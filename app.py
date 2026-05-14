import json
import os
import shutil
import subprocess
import time
from urllib.parse import urlparse

import ollama
from flask import Flask, Response, render_template, request, stream_with_context

app = Flask(__name__)


def _ollama_host():
    host = os.environ.get("OLLAMA_HOST", "").strip()
    if host:
        return host

    parsed = urlparse(os.environ.get("OLLAMA_URL", "http://127.0.0.1:11434"))
    if parsed.netloc:
        return parsed.netloc

    return "127.0.0.1:11434"


def _ollama_available(host):
    result = subprocess.run(
        ["ollama", "list"],
        env={**os.environ, "OLLAMA_HOST": host},
        stdout=subprocess.DEVNULL,
        stderr=subprocess.DEVNULL,
        check=False,
    )
    return result.returncode == 0


def ensure_ollama_running():
    """Start Ollama automatically when not already available."""
    if os.environ.get("NICOCHAT_USE_MOCK", "").lower() == "true":
        return
    if shutil.which("ollama") is None:
        return

    host = _ollama_host()
    if _ollama_available(host):
        return

    subprocess.Popen(
        ["ollama", "serve"],
        env={**os.environ, "OLLAMA_HOST": host},
        stdout=subprocess.DEVNULL,
        stderr=subprocess.DEVNULL,
        start_new_session=True,
    )

    try:
        ready_timeout_seconds = int(os.environ.get("OLLAMA_READY_TIMEOUT", "10"))
    except ValueError:
        ready_timeout_seconds = 10
    ready_timeout_seconds = max(ready_timeout_seconds, 1)

    for _ in range(ready_timeout_seconds):
        if _ollama_available(host):
            return
        time.sleep(1)


def get_ollama_models():
    """Return list of locally available Ollama model names."""
    try:
        ensure_ollama_running()
        response = ollama.list()
        return [m.model for m in response.models]
    except Exception:
        return []


@app.route("/")
def index():
    models = get_ollama_models()
    return render_template("index.html", models=models)


@app.route("/models")
def models():
    """Return available local Ollama models as JSON."""
    return {"models": get_ollama_models()}


@app.route("/chat", methods=["POST"])
def chat():
    """Stream a response from the selected Ollama model."""
    data = request.get_json(force=True)
    model = data.get("model", "").strip()
    messages = data.get("messages", [])

    if not model:
        return {"error": "No model specified."}, 400
    if not messages:
        return {"error": "No messages provided."}, 400

    def generate():
        try:
            stream = ollama.chat(model=model, messages=messages, stream=True)
            for chunk in stream:
                content = chunk.message.content
                if content:
                    yield f"data: {json.dumps({'content': content})}\n\n"
        except ollama.ResponseError as exc:
            yield f"data: {json.dumps({'error': str(exc)})}\n\n"
        except Exception as exc:
            yield f"data: {json.dumps({'error': str(exc)})}\n\n"
        yield "data: [DONE]\n\n"

    return Response(
        stream_with_context(generate()),
        mimetype="text/event-stream",
        headers={
            "Cache-Control": "no-cache",
            "X-Accel-Buffering": "no",
        },
    )


if __name__ == "__main__":
    app.run(debug=False, host="127.0.0.1", port=5000)

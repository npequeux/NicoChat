import json
import ollama
from flask import Flask, Response, render_template, request, stream_with_context

app = Flask(__name__)


def _format_ollama_error(exc: Exception) -> str:
    """Return an actionable client-facing error for Ollama failures."""
    raw = str(exc)
    normalized = raw.lower()

    if (
        "error sending request for url" in normalized
        or "connection refused" in normalized
        or "127.0.0.1:11434" in normalized
    ):
        return (
            "Unable to reach local Ollama instance at http://127.0.0.1:11434. "
            "Start it with 'ollama serve', verify with 'ollama list', "
            "or run in mock mode using NICOCHAT_USE_MOCK=true."
        )

    return raw


def get_ollama_models():
    """Return list of locally available Ollama model names."""
    try:
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
            yield f"data: {json.dumps({'error': _format_ollama_error(exc)})}\n\n"
        except Exception as exc:
            yield f"data: {json.dumps({'error': _format_ollama_error(exc)})}\n\n"
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

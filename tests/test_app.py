"""Unit tests for NicoChat Flask application."""

import json
import sys
import types
import unittest
from unittest.mock import MagicMock, patch

ROBOT_EMOJI = "\U0001F916"


# ---------------------------------------------------------------------------
# Helpers to build fake Ollama responses without requiring the real daemon
# ---------------------------------------------------------------------------

def _make_fake_models(*names):
    """Return a mock ollama.list() response with the given model names."""
    response = MagicMock()
    models = []
    for name in names:
        m = MagicMock()
        m.model = name
        models.append(m)
    response.models = models
    return response


def _make_fake_stream(*tokens):
    """Return an iterable that mimics ollama.chat(stream=True)."""
    chunks = []
    for token in tokens:
        chunk = MagicMock()
        chunk.message.content = token
        chunks.append(chunk)
    return iter(chunks)


# ---------------------------------------------------------------------------
# Test cases
# ---------------------------------------------------------------------------

class TestGetModels(unittest.TestCase):
    """Tests for the /models endpoint and get_ollama_models helper."""

    def setUp(self):
        import app as _app
        self.app_module = _app
        self.client = _app.app.test_client()

    def test_models_endpoint_returns_list(self):
        with patch.object(self.app_module, "get_ollama_models", return_value=["llama3:latest", "mistral:latest"]):
            resp = self.client.get("/models")
            self.assertEqual(resp.status_code, 200)
            data = resp.get_json()
            self.assertIn("models", data)
            self.assertIsInstance(data["models"], list)

    def test_models_endpoint_contains_expected_models(self):
        with patch.object(self.app_module, "get_ollama_models", return_value=["llama3:latest", "mistral:latest"]):
            resp = self.client.get("/models")
            data = resp.get_json()
            self.assertIn("llama3:latest", data["models"])
            self.assertIn("mistral:latest", data["models"])

    def test_models_returns_empty_list_when_ollama_unavailable(self):
        with patch("ollama.list", side_effect=Exception("connection refused")):
            result = self.app_module.get_ollama_models()
            self.assertEqual(result, [])

    def test_ensure_ollama_running_skips_in_mock_mode(self):
        with patch.dict(
            "os.environ",
            {"NICOCHAT_USE_MOCK": "true"},
            clear=False,
        ), patch("shutil.which") as which_mock, patch("subprocess.Popen") as popen_mock:
            self.app_module.ensure_ollama_running()
            which_mock.assert_not_called()
            popen_mock.assert_not_called()

    def test_ensure_ollama_running_starts_ollama_when_unavailable(self):
        with patch.dict(
            "os.environ",
            {
                "NICOCHAT_USE_MOCK": "false",
                "OLLAMA_URL": "http://127.0.0.1:11434",
                "OLLAMA_READY_TIMEOUT": "1",
            },
            clear=False,
        ), patch("shutil.which", return_value="/usr/bin/ollama"), patch(
            "subprocess.run",
            side_effect=[MagicMock(returncode=1), MagicMock(returncode=0)],
        ) as run_mock, patch("subprocess.Popen") as popen_mock, patch("time.sleep"):
            self.app_module.ensure_ollama_running()
            self.assertEqual(run_mock.call_count, 2)
            popen_mock.assert_called_once()


class TestIndexRoute(unittest.TestCase):
    """Tests for the / (index) route."""

    def setUp(self):
        import app as _app
        self.client = _app.app.test_client()
        self.app_module = _app

    def test_index_renders_200(self):
        with patch.object(self.app_module, "get_ollama_models", return_value=["llama3:latest"]):
            resp = self.client.get("/")
            self.assertEqual(resp.status_code, 200)
            self.assertIn(b"Select a model and send a message to begin.", resp.data)

    def test_index_contains_model_in_html(self):
        with patch.object(self.app_module, "get_ollama_models", return_value=["llama3:latest"]):
            resp = self.client.get("/")
            self.assertIn(b"llama3:latest", resp.data)

    def test_index_uses_clean_title_without_emoji(self):
        with patch.object(self.app_module, "get_ollama_models", return_value=["llama3:latest"]):
            resp = self.client.get("/")
            self.assertIn(b"<h1>NicoChat</h1>", resp.data)
            self.assertNotIn(ROBOT_EMOJI.encode("utf-8"), resp.data)

    def test_index_includes_model_selection_help(self):
        with patch.object(self.app_module, "get_ollama_models", return_value=["llama3:latest"]):
            resp = self.client.get("/")
            self.assertIn(b"Choose a model from the Ollama list above.", resp.data)

    def test_index_shows_no_models_when_empty(self):
        with patch.object(self.app_module, "get_ollama_models", return_value=[]):
            resp = self.client.get("/")
            self.assertEqual(resp.status_code, 200)
            self.assertIn(b"No models found", resp.data)


class TestChatRoute(unittest.TestCase):
    """Tests for the /chat streaming endpoint."""

    def setUp(self):
        import app as _app
        self.client = _app.app.test_client()
        self.app_module = _app

    def _collect_sse(self, response_data: bytes) -> list[dict]:
        """Parse Server-Sent Events (SSE) bytes and return a list of JSON payloads.

        Each line of the form ``data: <json>`` is decoded; the ``[DONE]``
        sentinel and non-data lines are skipped.
        """
        events = []
        for line in response_data.decode().splitlines():
            if line.startswith("data: ") and line[6:] != "[DONE]":
                events.append(json.loads(line[6:]))
        return events

    def test_missing_model_returns_400(self):
        resp = self.client.post(
            "/chat",
            data=json.dumps({"messages": [{"role": "user", "content": "hi"}]}),
            content_type="application/json",
        )
        self.assertEqual(resp.status_code, 400)

    def test_missing_messages_returns_400(self):
        resp = self.client.post(
            "/chat",
            data=json.dumps({"model": "llama3:latest"}),
            content_type="application/json",
        )
        self.assertEqual(resp.status_code, 400)

    def test_chat_streams_response(self):
        fake_stream = _make_fake_stream("Hello", " world", "!")
        with patch("ollama.chat", return_value=fake_stream):
            resp = self.client.post(
                "/chat",
                data=json.dumps({
                    "model": "llama3:latest",
                    "messages": [{"role": "user", "content": "Hi"}],
                }),
                content_type="application/json",
            )
            self.assertEqual(resp.status_code, 200)
            events = self._collect_sse(resp.data)
            contents = [e["content"] for e in events if "content" in e]
            self.assertEqual(contents, ["Hello", " world", "!"])

    def test_chat_includes_done_sentinel(self):
        fake_stream = _make_fake_stream("Hi")
        with patch("ollama.chat", return_value=fake_stream):
            resp = self.client.post(
                "/chat",
                data=json.dumps({
                    "model": "llama3:latest",
                    "messages": [{"role": "user", "content": "Hey"}],
                }),
                content_type="application/json",
            )
            self.assertIn(b"data: [DONE]", resp.data)

    def test_chat_returns_error_event_on_ollama_failure(self):
        import ollama as _ollama
        with patch("ollama.chat", side_effect=_ollama.ResponseError("model not found")):
            resp = self.client.post(
                "/chat",
                data=json.dumps({
                    "model": "nonexistent",
                    "messages": [{"role": "user", "content": "Hi"}],
                }),
                content_type="application/json",
            )
            self.assertEqual(resp.status_code, 200)
            events = self._collect_sse(resp.data)
            self.assertTrue(any("error" in e for e in events))


if __name__ == "__main__":
    unittest.main()

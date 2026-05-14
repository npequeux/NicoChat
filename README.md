# NicoChat

NicoChat is a local-network chat interface for a locally hosted AI model. It ships multiple backend options and a web UI — all running fully offline.

## What is included

- **Python/Flask backend** (`app.py`) — the simplest way to get started
- **Selectable compiled backend**: choose `.NET` or `Rust` for a production-ready deployment
- **Local-network web UI**: the server listens on `0.0.0.0` by default (compiled backends) or `127.0.0.1` (Python)
- **Android client scaffold**: a lightweight WebView app that points to the local server
- **Local AI integration**: all backends call a local [Ollama](https://ollama.com) instance — **no internet required**
- **Model selection**: choose from any model you have pulled with Ollama
- **Mock mode**: useful when validating the UI without a model installed

---

## Quick Start (Python)

The fastest way to run NicoChat locally:

```bash
# 1. Install Ollama and pull a model
ollama serve
ollama pull llama3          # or mistral, phi3, etc.

# 2. Install Python dependencies
pip install -r requirements.txt

# 3. Run the app
python app.py
# → http://127.0.0.1:5000
```


Select a model from the header dropdown list and start chatting. Responses stream in real time.

**If the model list is empty:**
- Run `ollama list` to see which models are installed locally.
- If no models are listed, install one with `ollama pull llama3` (or another model).
- Refresh the page after installing a model.

**If you see a "model not found" or similar error:**
- The backend and UI now provide clear, actionable error messages if a selected model is missing.
- Follow the instructions in the error message (usually: run `ollama list` and select an available model from the dropdown).

**Note:** NicoChat requires at least one model to be installed in Ollama. There is no fallback or default model—if none are available, the server and UI will prompt you to install one.

---

## Compiled Backends (.NET / Rust)

Build and run the compiled backend of your choice:

```bash
make build BACKEND=dotnet
make run   BACKEND=dotnet
# or
make build BACKEND=rust
make run   BACKEND=rust
```

`make run` now attempts to start the local Ollama service automatically when `NICOCHAT_USE_MOCK` is not enabled and the `ollama` CLI is installed.

The UI is then available at:

```text
http://<your-machine-ip>:5000
```

---


## Runtime configuration

Environment variables shared by all backends:

- `OLLAMA_URL` (default: `http://127.0.0.1:11434`)
- `OLLAMA_MODEL` (default: `qwen3`). If this model is not installed, you will be prompted to select an available model.
- `NICOCHAT_USE_MOCK=true` — bypass Ollama and return deterministic mock replies
- `PORT` — for the Rust backend
- `ASPNETCORE_URLS` — for the .NET backend

---

## Android app

The `android/` folder contains a minimal Android client that opens the locally hosted NicoChat UI inside a WebView. Enter the LAN URL of your server, for example:

```text
http://192.168.1.20:5000
```

---

## Project structure

```
NicoChat/
├── app.py                    # Python/Flask backend
├── requirements.txt          # Python dependencies
├── templates/
│   └── index.html            # Chat UI (Python backend)
├── tests/
│   └── test_app.py           # Python unit tests
├── web/                      # Shared web UI (compiled backends)
├── src/
│   ├── dotnet/               # .NET backend
│   └── rust/                 # Rust backend
├── android/                  # Android WebView client
└── Makefile                  # Build & run helpers
```

---

## Running Python tests

```bash
pip install pytest
pytest tests/
```

---

## Troubleshooting

- **Model list is empty:**
	- Run `ollama list` to check for installed models.
	- Install a model with `ollama pull llama3` (or another model).
	- Refresh the page.
- **Model not found error:**
	- The selected model is not installed. Use the dropdown to select an available model, or install the missing one.
- **Other backend errors:**
	- The UI now displays clear error messages with suggested actions. Follow the instructions provided.

If you encounter persistent issues, ensure that Ollama is running and at least one model is installed.

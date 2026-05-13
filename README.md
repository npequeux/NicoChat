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
If the list is empty, verify your local models with `ollama list` or pull one with
`ollama pull llama3`, then refresh the page.

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

The UI is then available at:

```text
http://<your-machine-ip>:5000
```

---

## Runtime configuration

Environment variables shared by all backends:

- `OLLAMA_URL` (default: `http://127.0.0.1:11434`)
- `OLLAMA_MODEL` (default: `qwen3`)
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

# NicoChat

NicoChat is a local-network chat interface for a locally hosted AI model.
The project is now simplified to a single Rust backend plus web UI.

## What is included

- Rust backend (`axum` + `reqwest`) in `src/rust/nicochat-rust`
- Web UI in `web/`
- Android WebView scaffold in `android/`
- Local AI integration via [Ollama](https://ollama.com)

## Quick Start

```bash
# 1. Start Ollama and install at least one model
ollama serve
ollama pull qwen3

# 2. Build and run NicoChat
make build
make run
```

Server URL:

```text
http://127.0.0.1:5000
```

`make run` tries to start `ollama serve` automatically when mock mode is not enabled and the `ollama` CLI is installed.

## Runtime configuration

- `OLLAMA_URL` (default: `http://127.0.0.1:11434`)
- `OLLAMA_MODEL` (default: `qwen3`)
- `NICOCHAT_USE_MOCK=true` to bypass Ollama with deterministic mock replies
- `PORT` (default: `5000`)

## Project structure

```text
NicoChat/
├── Makefile
├── src/
│   └── rust/
│       └── nicochat-rust/
├── web/
└── android/
```

## Troubleshooting

- If model list is empty, run `ollama list` and install one with `ollama pull qwen3`.
- If chat fails, verify Ollama is reachable with `ollama list`.
- If port 5000 is busy, stop existing NicoChat process and retry `make run`.

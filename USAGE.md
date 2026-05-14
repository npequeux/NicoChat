# Usage

NicoChat serves a local web chat UI backed by the Rust server.

## Requirements

- Rust toolchain (`cargo`) or a previously built binary
- optional local Ollama instance for real AI responses

## Start the server

From the repository root:

```bash
make run
```

When mock mode is disabled, `make run` checks whether Ollama is reachable and starts `ollama serve` automatically if needed.

Default URL:

```text
http://127.0.0.1:5000
```

LAN URL:

```text
http://<your-machine-ip>:5000
```

## Select a model

When the page loads, NicoChat fills the model dropdown using your local Ollama models.

- If the dropdown is empty, run `ollama list` and install one model (`ollama pull qwen3`), then refresh.
- If you get a model-not-found error, choose an installed model from the dropdown.

## Test without Ollama

```bash
NICOCHAT_USE_MOCK=true make run
```

## Runtime configuration

- `OLLAMA_URL` sets the Ollama server URL
- `OLLAMA_MODEL` sets default model name
- `NICOCHAT_USE_MOCK=true` enables mock replies
- `PORT` sets the Rust backend port

## Android client

The `android/` folder contains a WebView client.
Point it to your NicoChat URL, for example:

```text
http://192.168.1.20:5000
```

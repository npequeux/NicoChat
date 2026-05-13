# NicoChat

NicoChat is a small local-network chat interface for a locally hosted AI model. You can compile and run the server with either a **.NET** backend or a **Rust** backend, and both expose the same HTTP API and browser UI.

## What is included

- **Selectable backend at build time**: choose `.NET` or `Rust`
- **Local-network web UI**: the server listens on `0.0.0.0` by default
- **Android client scaffold**: a lightweight WebView app that points to the local server
- **Local AI integration**: both backends call a local Ollama instance by default
- **Mock mode**: useful when validating the UI without a model installed

## Backend selection

Build the backend you want:

```bash
make build BACKEND=dotnet
make build BACKEND=rust
```

Run the selected backend:

```bash
make run BACKEND=dotnet
make run BACKEND=rust
```

The UI is then available at:

```text
http://<your-machine-ip>:5000
```

## Runtime configuration

Environment variables shared by both backends:

- `OLLAMA_URL` (default: `http://127.0.0.1:11434`)
- `OLLAMA_MODEL` (default: `qwen3`)
- `NICOCHAT_USE_MOCK=true` to bypass Ollama and return deterministic mock replies
- `PORT` for the Rust backend
- `ASPNETCORE_URLS` for the .NET backend

## Android app

The `android/` folder contains a minimal Android client that opens the locally hosted NicoChat UI inside a WebView. Enter the LAN URL of your server, for example:

```text
http://192.168.1.20:5000
```

This keeps the Android client thin while the AI remains hosted locally on your machine.

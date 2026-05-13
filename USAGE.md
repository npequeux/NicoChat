# Usage

NicoChat serves a local web chat UI backed by either the `.NET` server or the `Rust` server.

## Requirements

- a built backend or the matching local toolchain
- optional: a local Ollama instance for real AI responses

## Start the server

Run one of the following commands from the repository root:

### .NET backend

```bash
make run BACKEND=dotnet
```

### Rust backend

```bash
make run BACKEND=rust
```

By default, NicoChat listens on:

```text
http://127.0.0.1:5000
```

To access it from another device on your local network, open:

```text
http://<your-machine-ip>:5000
```

## Test without Ollama

If you want to try the UI without a local model, enable mock mode:

```bash
NICOCHAT_USE_MOCK=true make run BACKEND=dotnet
```

or:

```bash
NICOCHAT_USE_MOCK=true make run BACKEND=rust
```

## Runtime configuration

- `OLLAMA_URL` sets the Ollama server URL
- `OLLAMA_MODEL` selects the Ollama model
- `NICOCHAT_USE_MOCK=true` enables mock replies
- `PORT` changes the port for the Rust backend
- `ASPNETCORE_URLS` changes the bind address for the `.NET` backend

## Android client

The `android/` folder contains a small WebView client. Point it to the NicoChat URL running on your machine, for example:

```text
http://192.168.1.20:5000
```

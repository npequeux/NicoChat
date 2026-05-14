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

When mock mode is disabled, `make run` first checks whether Ollama is reachable and starts `ollama serve` automatically if needed.

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

your local Ollama instance. Choose one from that list before sending a message.

## Select a model from the list

When the page loads, NicoChat fills the model dropdown with the models available in your local Ollama instance. Choose one from that list before sending a message.

**If the dropdown is empty:**
- Run `ollama list` to see which models are installed locally.
- If no models are listed, install one with `ollama pull llama3` (or another model).
- Refresh the page after installing a model.

**If you see a "model not found" or similar error:**
- The backend and UI now provide clear, actionable error messages if a selected model is missing.
- Follow the instructions in the error message (usually: run `ollama list` and select an available model from the dropdown).

**Note:** NicoChat requires at least one model to be installed in Ollama. There is no fallback or default model—if none are available, the server and UI will prompt you to install one.

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
- `OLLAMA_MODEL` selects the Ollama model. If this model is not installed, you will be prompted to select an available model.
- `NICOCHAT_USE_MOCK=true` enables mock replies
- `PORT` changes the port for the Rust backend
- `ASPNETCORE_URLS` changes the bind address for the `.NET` backend


## Android client

The `android/` folder contains a small WebView client. On launch it probes your local subnet for a reachable NicoChat server at `/models` on port `5000` and auto-loads the first match. You can also point it manually to the NicoChat URL running on your machine, for example:

```text
http://192.168.1.20:5000
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

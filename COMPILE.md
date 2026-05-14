# Compile

NicoChat can be compiled with either the `.NET` backend or the `Rust` backend.

## Requirements

- `make`
- One backend toolchain:
  - `.NET SDK 10` for the `.NET` backend
  - `Rust` and `cargo` for the `Rust` backend
- **Ollama must have at least one model installed** for NicoChat to function. There is no fallback or default model—if no models are available, the server and UI will prompt you to install one.

## Build the .NET backend

```bash
make build BACKEND=dotnet
```

This builds the project at:

```text
src/dotnet/NicoChat.DotNet/NicoChat.DotNet.csproj
```

## Build the Rust backend

```bash
make build BACKEND=rust
```

This builds the project at:

```text
src/rust/nicochat-rust/Cargo.toml
```

## Note on Model Availability

If no models are installed in Ollama, NicoChat will not function. You must install at least one model (e.g., `ollama pull llama3`) before running the server. If a selected model is missing, you will see a clear error message and instructions to run `ollama list` and select an available model.

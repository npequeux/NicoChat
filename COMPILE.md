# Compile

NicoChat now uses a single Rust backend.

## Requirements

- `make`
- Rust toolchain (`cargo`)
- Ollama with at least one installed model

## Build

```bash
make build
```

This compiles:

```text
src/rust/nicochat-rust/Cargo.toml
```

## Model availability

If no models are installed in Ollama, chat will not work.
Install one first, for example:

```bash
ollama pull qwen3
```

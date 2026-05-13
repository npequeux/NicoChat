# Compile

NicoChat can be compiled with either the `.NET` backend or the `Rust` backend.

## Requirements

- `make`
- one backend toolchain:
  - `.NET SDK 10` for the `.NET` backend
  - `Rust` and `cargo` for the `Rust` backend

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

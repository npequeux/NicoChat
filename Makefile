BACKEND ?= dotnet
PORT ?= 5000

.PHONY: build run

build:
ifeq ($(BACKEND),dotnet)
	dotnet build src/dotnet/NicoChat.DotNet/NicoChat.DotNet.csproj
else ifeq ($(BACKEND),rust)
	cargo build --manifest-path src/rust/nicochat-rust/Cargo.toml
else
	$(error Unknown BACKEND "$(BACKEND)". Use dotnet or rust)
endif

run:
ifeq ($(BACKEND),dotnet)
	ASPNETCORE_URLS=http://0.0.0.0:$(PORT) dotnet run --no-launch-profile --project src/dotnet/NicoChat.DotNet/NicoChat.DotNet.csproj
else ifeq ($(BACKEND),rust)
	PORT=$(PORT) cargo run --manifest-path src/rust/nicochat-rust/Cargo.toml
else
	$(error Unknown BACKEND "$(BACKEND)". Use dotnet or rust)
endif

BACKEND ?= dotnet
PORT ?= 5000
OLLAMA_URL ?= http://127.0.0.1:11434
OLLAMA_LOG_PATH ?=
OLLAMA_PID_PATH ?=
OLLAMA_READY_TIMEOUT ?= 10

.PHONY: build run ensure-ollama

build:
ifeq ($(BACKEND),dotnet)
	dotnet build src/dotnet/NicoChat.DotNet/NicoChat.DotNet.csproj
else ifeq ($(BACKEND),rust)
	cargo build --manifest-path src/rust/nicochat-rust/Cargo.toml
else
	$(error Unknown BACKEND "$(BACKEND)". Use dotnet or rust)
endif

run: ensure-ollama
ifeq ($(BACKEND),dotnet)
	ASPNETCORE_URLS=http://0.0.0.0:$(PORT) dotnet run --no-launch-profile --project src/dotnet/NicoChat.DotNet/NicoChat.DotNet.csproj
else ifeq ($(BACKEND),rust)
	PORT=$(PORT) cargo run --manifest-path src/rust/nicochat-rust/Cargo.toml
else
	$(error Unknown BACKEND "$(BACKEND)". Use dotnet or rust)
endif

ensure-ollama:
	@mock_mode="$$(printf '%s' '$(NICOCHAT_USE_MOCK)' | tr '[:upper:]' '[:lower:]')"; \
	if [ "$$mock_mode" = "true" ]; then \
		exit 0; \
	fi; \
	if ! command -v ollama >/dev/null 2>&1; then \
		echo "Warning: ollama CLI not found. Start Ollama manually or set NICOCHAT_USE_MOCK=true."; \
		exit 0; \
	fi; \
	host_env="$(OLLAMA_HOST)"; \
	if [ -z "$$host_env" ]; then \
		host_env="$(OLLAMA_URL)"; \
		host_env="$${host_env#http://}"; \
		host_env="$${host_env#https://}"; \
	fi; \
	if env OLLAMA_HOST="$$host_env" ollama list >/dev/null 2>&1; then \
		exit 0; \
	fi; \
	log_path="$(OLLAMA_LOG_PATH)"; \
	if [ -z "$$log_path" ]; then \
		log_path="$$(mktemp "$${TMPDIR:-/tmp}/nicochat-ollama.XXXXXX.log")"; \
	fi; \
	pid_path="$(OLLAMA_PID_PATH)"; \
	if [ -z "$$pid_path" ]; then \
		pid_path="$${log_path%.log}.pid"; \
	fi; \
	echo "Starting local Ollama service on $$host_env..."; \
	echo "Ollama startup log: $$log_path"; \
	nohup env OLLAMA_HOST="$$host_env" ollama serve >"$$log_path" 2>&1 & \
	ollama_pid="$$!"; \
	printf '%s\n' "$$ollama_pid" >"$$pid_path"; \
	attempt=1; \
	while [ "$$attempt" -le "$(OLLAMA_READY_TIMEOUT)" ]; do \
		if env OLLAMA_HOST="$$host_env" ollama list >/dev/null 2>&1; then \
			echo "Ollama is ready."; \
			exit 0; \
		fi; \
		sleep 1; \
		attempt=$$((attempt + 1)); \
	done; \
	echo "Warning: Ollama did not become ready within $(OLLAMA_READY_TIMEOUT) seconds. NicoChat will continue starting, but chat requests may fail until Ollama is ready; check $$log_path or start Ollama manually. PID: $$ollama_pid"

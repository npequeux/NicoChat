PORT ?= 5000
OLLAMA_URL ?= http://127.0.0.1:11434
OLLAMA_LOG_PATH ?=
OLLAMA_PID_PATH ?=
OLLAMA_READY_TIMEOUT ?= 10

.PHONY: build run ensure-ollama stop-app

build:
	cargo build --manifest-path src/rust/nicochat-rust/Cargo.toml

run: ensure-ollama stop-app
	@set +e; \
	PORT=$(PORT) cargo run --manifest-path src/rust/nicochat-rust/Cargo.toml; \
	exit_code=$$?; \
	if [ $$exit_code -eq 130 ] || [ $$exit_code -eq 143 ]; then \
		echo "NicoChat stopped."; \
		exit 0; \
	fi; \
	exit $$exit_code

stop-app:
	@existing_pids="$$(pgrep -x 'nicochat-rust' || true)"; \
	if [ -n "$$existing_pids" ]; then \
		echo "Stopping existing NicoChat process(es): $$existing_pids"; \
		kill $$existing_pids >/dev/null 2>&1 || true; \
		sleep 1; \
	fi

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

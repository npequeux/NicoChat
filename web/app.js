const messages = [];

const messagesElement = document.getElementById("messages");
const form = document.getElementById("chatForm");
const input = document.getElementById("messageInput");
const sendButton = document.getElementById("sendButton");
const modelSelect = document.getElementById("modelSelect");
const historyLengthInput = document.getElementById("historyLengthInput");
const speedModeSelect = document.getElementById("speedModeSelect");
const internetAccessToggle = document.getElementById("internetAccessToggle");

const gpuToggle = document.getElementById("gpuToggle");
const npuToggle = document.getElementById("npuToggle");
const remoteToggle = document.getElementById("remoteToggle");

const ollamaRestartNotice = document.getElementById("ollamaRestartNotice");
const restartOllamaBtn = document.getElementById("restartOllamaBtn");
const restartOllamaStatus = document.getElementById("restartOllamaStatus");

const acceleratorButtons = [
  { id: "gpu", element: gpuToggle },
  { id: "npu", element: npuToggle },
  { id: "remote", element: remoteToggle },
];

if (input) {
  input.addEventListener("keydown", (event) => {
    if (event.key === "Enter" && !event.shiftKey) {
      event.preventDefault();
      if (!sendButton.disabled) {
        sendButton.click();
      }
    }
  });
}

function appendMessage(role, content) {
  if (!messagesElement) return;

  const card = document.createElement("article");
  card.className = `message ${role}`;

  const roleLabel = document.createElement("p");
  roleLabel.className = "role";
  roleLabel.textContent = role;

  const body = document.createElement("p");
  body.textContent = content;

  card.append(roleLabel, body);
  messagesElement.append(card);
  messagesElement.scrollTop = messagesElement.scrollHeight;
}

function getHistoryLength() {
  const parsed = Number.parseInt(historyLengthInput?.value ?? "12", 10);
  if (Number.isNaN(parsed)) {
    return 12;
  }
  return Math.min(100, Math.max(0, parsed));
}

function getMessagesForRequest(allMessages) {
  const historyLength = getHistoryLength();
  if (historyLength === 0) {
    return allMessages.length > 0 ? [allMessages[allMessages.length - 1]] : [];
  }
  return allMessages.slice(-historyLength);
}

function getSpeedMode() {
  return speedModeSelect?.value || "balanced";
}

function getAccelerators() {
  return acceleratorButtons
    .filter((button) => button.element && button.element.classList.contains("active"))
    .map((button) => button.id);
}

function normalizeAccelerators(values) {
  const unique = [...new Set(values)];
  if (unique.includes("gpu") && unique.includes("npu")) {
    return unique.filter((value) => value !== "npu");
  }
  return unique;
}

function getInternetAccess() {
  return internetAccessToggle?.classList.contains("active") ?? true;
}

function formatChatError(error) {
  const raw = error instanceof Error ? error.message : "Unknown error";
  const normalized = raw.toLowerCase();

  if ((normalized.includes("model") && normalized.includes("not found")) || normalized.includes("not installed in ollama")) {
    return "Selected model is not installed in Ollama. Run 'ollama list', pick an available model in the dropdown, and retry.";
  }

  return `Unable to respond: ${raw}`;
}

async function refreshHealth() {
  const response = await fetch("/api/health", { cache: "no-store" });
  const payload = await response.json();

  document.getElementById("backendValue").textContent = payload.backend;
  document.getElementById("modeValue").textContent = payload.mode;
  document.getElementById("modelValue").textContent = payload.model;
  document.getElementById("acceleratorValue").textContent = payload.accelerator || "default";
}

async function loadModels() {
  try {
    const response = await fetch("/api/models", { cache: "no-store" });
    const payload = await response.json();
    const modelNames = payload.models || [];

    modelSelect.innerHTML = "";

    if (modelNames.length === 0) {
      const option = document.createElement("option");
      option.value = "";
      option.disabled = true;
      option.selected = true;
      option.textContent = "No models found";
      modelSelect.append(option);
      sendButton.disabled = true;
      return;
    }

    const savedModel = window.localStorage.getItem("nicochat-model");
    modelNames.forEach((name) => {
      const option = document.createElement("option");
      option.value = name;
      option.textContent = name;
      if (savedModel && savedModel === name) {
        option.selected = true;
      }
      modelSelect.append(option);
    });

    if (!modelSelect.value && modelSelect.options.length > 0) {
      modelSelect.selectedIndex = 0;
    }

    sendButton.disabled = false;
  } catch (error) {
    console.error("Failed to load models:", error);
    modelSelect.innerHTML = '<option value="" disabled selected>Unavailable</option>';
    sendButton.disabled = true;
  }
}

function applyInternetToggleState(enabled) {
  if (!internetAccessToggle) return;

  if (enabled) {
    internetAccessToggle.classList.add("active");
    internetAccessToggle.textContent = "Enabled";
  } else {
    internetAccessToggle.classList.remove("active");
    internetAccessToggle.textContent = "Disabled";
  }
}

function applyAcceleratorToggleState(activeValues) {
  acceleratorButtons.forEach((button) => {
    if (!button.element) return;
    if (activeValues.includes(button.id)) {
      button.element.classList.add("active");
    } else {
      button.element.classList.remove("active");
    }
  });
}

function showRestartNotice() {
  if (ollamaRestartNotice) {
    ollamaRestartNotice.style.display = "block";
  }
}

function hideRestartNotice() {
  if (ollamaRestartNotice) {
    ollamaRestartNotice.style.display = "none";
  }
  if (restartOllamaStatus) {
    restartOllamaStatus.textContent = "";
  }
}

function setupControls() {
  if (historyLengthInput) {
    const savedHistory = window.localStorage.getItem("nicochat-history-length");
    if (savedHistory !== null) {
      historyLengthInput.value = savedHistory;
    }

    historyLengthInput.addEventListener("change", () => {
      historyLengthInput.value = String(getHistoryLength());
      window.localStorage.setItem("nicochat-history-length", historyLengthInput.value);
    });
  }

  if (speedModeSelect) {
    const savedSpeed = window.localStorage.getItem("nicochat-speed-mode");
    if (savedSpeed) {
      speedModeSelect.value = savedSpeed;
    }
    speedModeSelect.addEventListener("change", () => {
      window.localStorage.setItem("nicochat-speed-mode", speedModeSelect.value);
    });
  }

  if (modelSelect) {
    modelSelect.addEventListener("change", () => {
      window.localStorage.setItem("nicochat-model", modelSelect.value);
    });
  }

  const savedInternet = window.localStorage.getItem("nicochat-internet-access");
  applyInternetToggleState(savedInternet !== "false");

  if (internetAccessToggle) {
    internetAccessToggle.addEventListener("click", () => {
      const enabled = !internetAccessToggle.classList.contains("active");
      applyInternetToggleState(enabled);
      window.localStorage.setItem("nicochat-internet-access", enabled ? "true" : "false");
    });
  }

  let savedAccelerators = ["gpu", "npu", "remote"];
  try {
    const value = JSON.parse(window.localStorage.getItem("nicochat-accelerators") || "null");
    if (Array.isArray(value) && value.length > 0) {
      savedAccelerators = value;
    }
  } catch (_) {
    // Ignore malformed localStorage data.
  }
  savedAccelerators = normalizeAccelerators(savedAccelerators);
  applyAcceleratorToggleState(savedAccelerators);
  window.localStorage.setItem("nicochat-accelerators", JSON.stringify(savedAccelerators));

  acceleratorButtons.forEach((button) => {
    if (!button.element) return;
    button.element.addEventListener("click", () => {
      button.element.classList.toggle("active");

      // GPU and NPU are mutually exclusive.
      if (button.id === "gpu" && button.element.classList.contains("active") && npuToggle) {
        npuToggle.classList.remove("active");
      }
      if (button.id === "npu" && button.element.classList.contains("active") && gpuToggle) {
        gpuToggle.classList.remove("active");
      }

      const active = getAccelerators();
      window.localStorage.setItem("nicochat-accelerators", JSON.stringify(active));
      showRestartNotice();
    });
  });

  if (restartOllamaBtn) {
    restartOllamaBtn.addEventListener("click", async () => {
      restartOllamaBtn.disabled = true;
      if (restartOllamaStatus) {
        restartOllamaStatus.textContent = "Restarting...";
      }

      try {
        const accelerators = getAccelerators();
        const accelerator = accelerators[0] || null;
        const response = await fetch("/api/restart-ollama", {
          method: "POST",
          headers: { "Content-Type": "application/json" },
          body: JSON.stringify({ accelerator, accelerators }),
        });
        if (response.ok) {
          if (restartOllamaStatus) {
            restartOllamaStatus.textContent = "Ollama restarted.";
          }
          window.setTimeout(hideRestartNotice, 2000);
        } else if (restartOllamaStatus) {
          restartOllamaStatus.textContent = "Failed to restart Ollama.";
        }
      } catch (_) {
        if (restartOllamaStatus) {
          restartOllamaStatus.textContent = "Error restarting Ollama.";
        }
      }

      restartOllamaBtn.disabled = false;
    });
  }
}

if (form) {
  form.addEventListener("submit", async (event) => {
    event.preventDefault();

    const content = input?.value.trim() || "";
    if (!content) return;

    const model = modelSelect?.value || "";
    if (!model) return;

    messages.push({ role: "user", content });
    appendMessage("user", content);

    input.value = "";
    sendButton.disabled = true;

    try {
      const requestMessages = getMessagesForRequest(messages);
      const speedMode = getSpeedMode();
      const accelerators = getAccelerators();
      const accelerator = accelerators[0] || null;
      const internetAccess = getInternetAccess();

      const response = await fetch("/api/chat", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          model,
          messages: requestMessages,
          speed_mode: speedMode,
          accelerator,
          accelerators,
          internet_access: internetAccess,
        }),
      });

      const payload = await response.json();
      if (!response.ok) {
        throw new Error(payload.error || payload.detail || "Chat request failed.");
      }

      messages.push({ role: payload.role, content: payload.content });
      appendMessage(payload.role, payload.content);
      await refreshHealth();
    } catch (error) {
      appendMessage("assistant", formatChatError(error));
      await loadModels();
    } finally {
      sendButton.disabled = false;
      input.focus();
    }
  });
}

if ("serviceWorker" in navigator && (window.isSecureContext || location.hostname === "localhost")) {
  navigator.serviceWorker
    .register("/sw.js", { updateViaCache: "none" })
    .then(async (registration) => {
      await registration.update();

      navigator.serviceWorker.addEventListener("controllerchange", () => {
        window.location.reload();
      });
    })
    .catch(() => {});
}

setupControls();

refreshHealth().catch(() => {
  document.getElementById("backendValue").textContent = "Unavailable";
  document.getElementById("modeValue").textContent = "Unavailable";
  document.getElementById("modelValue").textContent = "Unavailable";
  document.getElementById("acceleratorValue").textContent = "Unavailable";
});

loadModels().then(() => {
  appendMessage(
    "assistant",
    "Welcome to NicoChat. Select a model from the list above and start chatting."
  );
});

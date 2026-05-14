const internetAccessToggle = document.getElementById("internetAccessToggle");
function getInternetAccess() {
  return internetAccessToggle?.classList.contains("active");
}
const messages = [];
const messagesElement = document.getElementById("messages");

const form = document.getElementById("chatForm");
const input = document.getElementById("messageInput");
const sendButton = document.getElementById("sendButton");
const modelSelect = document.getElementById("modelSelect");
const historyLengthInput = document.getElementById("historyLengthInput");
const speedModeSelect = document.getElementById("speedModeSelect");
const acceleratorSelect = document.getElementById("acceleratorSelect");
const remoteEndpointInput = document.getElementById("remoteEndpointInput");

// Enable Enter to send, Shift+Enter for newline
if (input) {
  input.addEventListener("keydown", function(e) {
    if (e.key === "Enter" && !e.shiftKey) {
      e.preventDefault();
      if (!sendButton.disabled) {
        sendButton.click();
      }
    }
  });
}

function appendMessage(role, content) {
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

function getAccelerator() {
  return acceleratorSelect?.value || "cpu";
}

function getRemoteEndpoint() {
  return remoteEndpointInput?.value?.trim() || "";
}

function formatChatError(error) {
  const raw = error instanceof Error ? error.message : "Unknown error";
  const normalized = raw.toLowerCase();

  if (normalized.includes("not installed in ollama") || normalized.includes("model") && normalized.includes("not found")) {
    return "Selected model is not installed in Ollama. Run 'ollama list', pick an available model in the dropdown, and retry.";
  }

  return `Unable to respond: ${raw}`;
}

async function refreshHealth() {
  const response = await fetch("/api/health");
  const payload = await response.json();
  document.getElementById("backendValue").textContent = payload.backend;
  document.getElementById("modeValue").textContent = payload.mode;
  document.getElementById("modelValue").textContent = payload.model;
}

async function loadModels() {
  try {
    const response = await fetch("/api/models");
    const payload = await response.json();
    const modelNames = payload.models || [];

    modelSelect.innerHTML = "";
    if (modelNames.length === 0) {
      const opt = document.createElement("option");
      opt.value = "";
      opt.disabled = true;
      opt.selected = true;
      opt.textContent = "No models found";
      modelSelect.append(opt);
      sendButton.disabled = true;
    } else {
      modelNames.forEach((name) => {
        const opt = document.createElement("option");
        opt.value = name;
        opt.textContent = name;
        modelSelect.append(opt);
      });
    }
  } catch (error) {
    console.error("Failed to load models:", error);
    modelSelect.innerHTML = '<option value="" disabled selected>Unavailable</option>';
    sendButton.disabled = true;
  }
}

form.addEventListener("submit", async (event) => {
  event.preventDefault();

  const content = input.value.trim();
  if (!content) {
    return;
  }

  const model = modelSelect.value;
  if (!model) {
    return;
  }

  messages.push({ role: "user", content });
  appendMessage("user", content);
  input.value = "";
  sendButton.disabled = true;

  try {
    const requestMessages = getMessagesForRequest(messages);

    const speedMode = getSpeedMode();
    const accelerator = getAccelerator();

    const remoteEndpoint = getRemoteEndpoint();
    const internetAccess = getInternetAccess();
    const response = await fetch("/api/chat", {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ model, messages: requestMessages, speed_mode: speedMode, accelerator, remote_endpoint: remoteEndpoint, internet_access: internetAccess }),
    });
// Internet access toggle logic
if (internetAccessToggle) {
  const saved = window.localStorage.getItem("nicochat-internet-access");
  if (saved === "false") {
    internetAccessToggle.classList.remove("active");
    internetAccessToggle.textContent = "Disabled";
  } else {
    internetAccessToggle.classList.add("active");
    internetAccessToggle.textContent = "Enabled";
  }
  internetAccessToggle.addEventListener("click", () => {
    const enabled = !internetAccessToggle.classList.contains("active");
    if (enabled) {
      internetAccessToggle.classList.add("active");
      internetAccessToggle.textContent = "Enabled";
    } else {
      internetAccessToggle.classList.remove("active");
      internetAccessToggle.textContent = "Disabled";
    }
    window.localStorage.setItem("nicochat-internet-access", enabled ? "true" : "false");
  });
}

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

if ("serviceWorker" in navigator && (window.isSecureContext || location.hostname === "localhost")) {
  navigator.serviceWorker.register("/sw.js").catch(() => {});
}

if (historyLengthInput) {
  const savedValue = window.localStorage.getItem("nicochat-history-length");
  if (savedValue !== null) {
    historyLengthInput.value = savedValue;
  }

  historyLengthInput.addEventListener("change", () => {
    historyLengthInput.value = String(getHistoryLength());
    window.localStorage.setItem("nicochat-history-length", historyLengthInput.value);
  });
}

if (speedModeSelect) {
  const savedSpeed = window.localStorage.getItem("nicochat-speed-mode");
  if (savedSpeed) speedModeSelect.value = savedSpeed;
  speedModeSelect.addEventListener("change", () => {
    window.localStorage.setItem("nicochat-speed-mode", speedModeSelect.value);
  });
}

if (acceleratorSelect) {
  const savedAccel = window.localStorage.getItem("nicochat-accelerator");
  if (savedAccel) acceleratorSelect.value = savedAccel;
  acceleratorSelect.addEventListener("change", () => {
    window.localStorage.setItem("nicochat-accelerator", acceleratorSelect.value);
    if (acceleratorSelect.value === "remote") {
      remoteEndpointInput.style.display = "inline-block";
    } else {
      remoteEndpointInput.style.display = "none";
    }
  });
  // Initial show/hide
  if (acceleratorSelect.value === "remote") {
    remoteEndpointInput.style.display = "inline-block";
  } else {
    remoteEndpointInput.style.display = "none";
  }
}

if (remoteEndpointInput) {
  const savedRemote = window.localStorage.getItem("nicochat-remote-endpoint");
  if (savedRemote) remoteEndpointInput.value = savedRemote;
  remoteEndpointInput.addEventListener("change", () => {
    window.localStorage.setItem("nicochat-remote-endpoint", remoteEndpointInput.value);
  });
}

// Accelerator toggle logic
function updateAcceleratorToggles() {
  const saved = JSON.parse(window.localStorage.getItem("nicochat-accelerators") || '["gpu","npu","remote"]');
  [gpuToggle, npuToggle, remoteToggle].forEach((btn, i) => {
    if (!btn) return;
    const val = ["gpu","npu","remote"][i];
    if (saved.includes(val)) {
      btn.classList.add("active");
    } else {
      btn.classList.remove("active");
    }
    btn.addEventListener("click", () => {
      btn.classList.toggle("active");
      const acc = getAccelerators();
      window.localStorage.setItem("nicochat-accelerators", JSON.stringify(acc));
    });
  });
}
updateAcceleratorToggles();

// Show restart notice and handle restart button
const ollamaRestartNotice = document.getElementById("ollamaRestartNotice");
const restartOllamaBtn = document.getElementById("restartOllamaBtn");
const restartOllamaStatus = document.getElementById("restartOllamaStatus");

function showRestartNotice() {
  if (ollamaRestartNotice) ollamaRestartNotice.style.display = "block";
}
function hideRestartNotice() {
  if (ollamaRestartNotice) ollamaRestartNotice.style.display = "none";
  if (restartOllamaStatus) restartOllamaStatus.textContent = "";
}

[gpuToggle, npuToggle, remoteToggle].forEach(btn => {
  if (btn) {
    btn.addEventListener("click", showRestartNotice);
  }
});

if (restartOllamaBtn) {
  restartOllamaBtn.addEventListener("click", async () => {
    restartOllamaBtn.disabled = true;
    restartOllamaStatus.textContent = "Restarting...";
    try {
      const resp = await fetch("/api/restart-ollama", { method: "POST" });
      if (resp.ok) {
        restartOllamaStatus.textContent = "Ollama restarted.";
        setTimeout(hideRestartNotice, 2000);
      } else {
        restartOllamaStatus.textContent = "Failed to restart Ollama.";
      }
    } catch (e) {
      restartOllamaStatus.textContent = "Error restarting Ollama.";
    }
    restartOllamaBtn.disabled = false;
  });
}

refreshHealth().catch(() => {
  document.getElementById("backendValue").textContent = "Unavailable";
  document.getElementById("modeValue").textContent = "Unavailable";
  document.getElementById("modelValue").textContent = "Unavailable";
});

loadModels().then(() => {
  appendMessage("assistant", "Welcome to NicoChat. Select a model from the list above and start chatting — no internet required.");
});

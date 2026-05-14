const messages = [];
const messagesElement = document.getElementById("messages");
const form = document.getElementById("chatForm");
const input = document.getElementById("messageInput");
const sendButton = document.getElementById("sendButton");
const modelSelect = document.getElementById("modelSelect");

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

function formatChatError(error) {
  const raw = error instanceof Error ? error.message : "Unknown error";
  const normalized = raw.toLowerCase();

  if (
    normalized.includes("unable to reach local ollama") ||
    normalized.includes("connection refused") ||
    normalized.includes("127.0.0.1:11434")
  ) {
    return "Local Ollama is unavailable. Start it with 'ollama serve', check models with 'ollama list', then try again. You can also run with NICOCHAT_USE_MOCK=true.";
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
    const response = await fetch("/api/chat", {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ model, messages }),
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
  } finally {
    sendButton.disabled = false;
    input.focus();
  }
});

if ("serviceWorker" in navigator && (window.isSecureContext || location.hostname === "localhost")) {
  navigator.serviceWorker.register("/sw.js").catch(() => {});
}

refreshHealth().catch(() => {
  document.getElementById("backendValue").textContent = "Unavailable";
  document.getElementById("modeValue").textContent = "Unavailable";
  document.getElementById("modelValue").textContent = "Unavailable";
});

loadModels().then(() => {
  appendMessage("assistant", "Welcome to NicoChat. Select a model from the list above and start chatting — no internet required.");
});

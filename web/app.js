const messages = [];
const messagesElement = document.getElementById("messages");
const form = document.getElementById("chatForm");
const input = document.getElementById("messageInput");
const sendButton = document.getElementById("sendButton");

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

async function refreshHealth() {
  const response = await fetch("/api/health");
  const payload = await response.json();
  document.getElementById("backendValue").textContent = payload.backend;
  document.getElementById("modeValue").textContent = payload.mode;
  document.getElementById("modelValue").textContent = payload.model;
}

form.addEventListener("submit", async (event) => {
  event.preventDefault();

  const content = input.value.trim();
  if (!content) {
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
      body: JSON.stringify({ messages }),
    });

    const payload = await response.json();
    if (!response.ok) {
      throw new Error(payload.error || payload.detail || "Chat request failed.");
    }

    messages.push({ role: payload.role, content: payload.content });
    appendMessage(payload.role, payload.content);
    await refreshHealth();
  } catch (error) {
    appendMessage(
      "assistant",
      `Unable to respond: ${error instanceof Error ? error.message : "Unknown error"}`
    );
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

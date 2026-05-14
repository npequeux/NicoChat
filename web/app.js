const messages = [];

const messagesElement = document.getElementById("messages");
const form = document.getElementById("chatForm");
const input = document.getElementById("messageInput");
const sendButton = document.getElementById("sendButton");
const modelSelect = document.getElementById("modelSelect");
const historyLengthInput = document.getElementById("historyLengthInput");
const speedModeSelect = document.getElementById("speedModeSelect");
const roleSelect = document.getElementById("roleSelect");
const internetAccessToggle = document.getElementById("internetAccessToggle");
const importDocumentButton = document.getElementById("importDocumentButton");
const importDocumentInput = document.getElementById("importDocumentInput");

const gpuToggle = document.getElementById("gpuToggle");
const remoteToggle = document.getElementById("remoteToggle");

const ollamaRestartNotice = document.getElementById("ollamaRestartNotice");
const restartOllamaBtn = document.getElementById("restartOllamaBtn");
const restartOllamaStatus = document.getElementById("restartOllamaStatus");

const acceleratorButtons = [
  { id: "gpu", element: gpuToggle },
  { id: "remote", element: remoteToggle },
];

let thinkingMessageCard = null;

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

  if (role === "assistant") {
    const insights = extractResponseInsights(content);
    const rich = renderRichInsights(insights);
    if (rich) {
      card.append(rich);
    }
  }

  messagesElement.append(card);
  messagesElement.scrollTop = messagesElement.scrollHeight;
}

function importDocumentToComposer(file) {
  const messageInput = document.getElementById("messageInput");
  if (!file || !messageInput) return;

  const reader = new FileReader();
  reader.onload = () => {
    const text = typeof reader.result === "string" ? reader.result : "";
    const safeFileName = file.name.replace(/[\r\n<>]/g, "_");
    const importBlock = `[Imported document: ${safeFileName}]\n${text.trim()}`;
    messageInput.value = messageInput.value.trim()
      ? `${messageInput.value.trim()}\n\n${importBlock}`
      : importBlock;
    messageInput.focus();
  };
  reader.onerror = () => {
    appendMessage("assistant", "Unable to import this document. Please use a valid text-based file.");
  };
  reader.readAsText(file);
}

function extractResponseInsights(content) {
  const text = content || "";
  const insights = {
    location: null,
    weather: null,
    sourceUrl: null,
    searchContext: null,
    snippets: [],
  };

  const locationMatch = text.match(/Localisation[^:\n]*:\s*([^\n]+)/i);
  if (locationMatch) {
    insights.location = locationMatch[1].trim();
  }

  const weatherMatch = text.match(/Meteo[^:\n]*:\s*([^\n]+)/i) || text.match(/Weather[^:\n]*:\s*([^\n]+)/i);
  if (weatherMatch) {
    insights.weather = weatherMatch[1].trim();
  }

  const sourceMatch = text.match(/Source:\s*(https?:\/\/\S+)/i);
  if (sourceMatch) {
    insights.sourceUrl = sourceMatch[1].trim();
  }

  const searchContextMatch = text.match(/Search\s+(?:related\s+)?context(?:\s*\([^\)]*\))?:\s*([^\n]+)/i);
  if (searchContextMatch) {
    insights.searchContext = searchContextMatch[1].trim();
  }

  const snippetRegex = /Snippet:\s*([^\n]+)/gi;
  for (const match of text.matchAll(snippetRegex)) {
    const snippet = (match[1] || "").trim();
    if (snippet) {
      insights.snippets.push(snippet);
    }
    if (insights.snippets.length >= 2) {
      break;
    }
  }

  return insights;
}

function createInsightCard(title, value) {
  const card = document.createElement("div");
  card.className = "insight-card";

  const titleEl = document.createElement("p");
  titleEl.className = "insight-title";
  titleEl.textContent = title;

  const valueEl = document.createElement("p");
  valueEl.className = "insight-value";
  valueEl.textContent = value;

  card.append(titleEl, valueEl);
  return card;
}

function renderRichInsights(insights) {
  const container = document.createElement("div");
  container.className = "insight-grid";
  let hasContent = false;

  if (insights.location) {
    container.append(createInsightCard("Localisation", insights.location));
    hasContent = true;
  }

  if (insights.weather) {
    container.append(createInsightCard("Meteo", insights.weather));
    hasContent = true;
  }

  if (insights.searchContext) {
    container.append(createInsightCard("Contexte web", insights.searchContext));
    hasContent = true;
  }

  if (insights.snippets.length > 0) {
    container.append(createInsightCard("Extrait", insights.snippets[0]));
    hasContent = true;
  }

  if (insights.sourceUrl) {
    const linkCard = document.createElement("a");
    linkCard.className = "insight-card insight-link";
    linkCard.href = insights.sourceUrl;
    linkCard.target = "_blank";
    linkCard.rel = "noopener noreferrer";

    const t = document.createElement("p");
    t.className = "insight-title";
    t.textContent = "Source";

    const v = document.createElement("p");
    v.className = "insight-value";
    v.textContent = insights.sourceUrl;

    linkCard.append(t, v);
    container.append(linkCard);
    hasContent = true;
  }

  return hasContent ? container : null;
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

function syncDisplayedModel() {
  const modelValueElement = document.getElementById("modelValue");
  if (!modelValueElement) return;

  const selectedModel = modelSelect?.value?.trim();
  if (selectedModel) {
    modelValueElement.textContent = selectedModel;
  }
}

const ROLE_SYSTEM_PROMPTS = {
  default: "You are a cool companion ready to help.",
  psychologist: "You are a supportive psychologist. You listen carefully, show empathy, and provide thoughtful, supportive guidance.",
  geek: "You are a tech enthusiast who knows all the latest fancy geek stuff. You love talking about technology, programming, gadgets, and cutting-edge innovations.",
};

const DEFAULT_BREVITY_INSTRUCTION =
  "Default response style: reply in at most 3 sentences when possible. If the user explicitly asks for more detail, you can provide a longer answer.";

function getRoleSystemMessage() {
  const role = roleSelect?.value || "default";
  const prompt = ROLE_SYSTEM_PROMPTS[role] || ROLE_SYSTEM_PROMPTS.default;
  return { role: "system", content: `${prompt}\n\n${DEFAULT_BREVITY_INSTRUCTION}` };
}

function showThinkingIndicator() {
  if (!messagesElement || thinkingMessageCard) return;

  const card = document.createElement("article");
  card.className = "message assistant thinking-message";

  const roleLabel = document.createElement("p");
  roleLabel.className = "role";
  roleLabel.textContent = "assistant";

  const row = document.createElement("div");
  row.className = "thinking-row";

  const orbit = document.createElement("div");
  orbit.className = "thinking-orbit";
  orbit.setAttribute("aria-hidden", "true");

  const orb = document.createElement("span");
  orb.className = "thinking-orb";
  orbit.append(orb);

  const text = document.createElement("p");
  text.className = "thinking-text";
  text.textContent = "Thinking";

  const dots = document.createElement("span");
  dots.className = "thinking-dots";
  dots.setAttribute("aria-hidden", "true");
  dots.innerHTML = "<span></span><span></span><span></span>";

  row.append(orbit, text, dots);
  card.append(roleLabel, row);

  messagesElement.append(card);
  messagesElement.scrollTop = messagesElement.scrollHeight;
  thinkingMessageCard = card;
}

function hideThinkingIndicator() {
  if (!thinkingMessageCard) return;
  thinkingMessageCard.remove();
  thinkingMessageCard = null;
}

function getAccelerators() {
  return acceleratorButtons
    .filter((button) => button.element && button.element.classList.contains("active"))
    .map((button) => button.id);
}

function normalizeAccelerators(values) {
  const allowed = new Set(["gpu", "remote"]);
  const unique = [...new Set(values)];
  return unique.filter((value) => allowed.has(value));
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
  syncDisplayedModel();
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

    syncDisplayedModel();

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

  if (roleSelect) {
    const savedRole = window.localStorage.getItem("nicochat-role");
    if (savedRole && ROLE_SYSTEM_PROMPTS[savedRole]) {
      roleSelect.value = savedRole;
    }
    roleSelect.addEventListener("change", () => {
      window.localStorage.setItem("nicochat-role", roleSelect.value);
    });
  }

  if (modelSelect) {
    modelSelect.addEventListener("change", () => {
      window.localStorage.setItem("nicochat-model", modelSelect.value);
      syncDisplayedModel();
    });
  }

  if (importDocumentButton && importDocumentInput) {
    importDocumentButton.addEventListener("click", () => {
      importDocumentInput.click();
    });

    importDocumentInput.addEventListener("change", () => {
      const [file] = importDocumentInput.files || [];
      if (file) {
        importDocumentToComposer(file);
      }
      importDocumentInput.value = "";
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

  let savedAccelerators = ["gpu", "remote"];
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
    showThinkingIndicator();

    try {
      const requestMessages = getMessagesForRequest(messages);
      const speedMode = getSpeedMode();
      const accelerators = getAccelerators();
      const accelerator = accelerators[0] || null;
      const internetAccess = getInternetAccess();
      const roleSystemMessage = getRoleSystemMessage();

      const response = await fetch("/api/chat", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          model,
          messages: [roleSystemMessage, ...requestMessages],
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

      hideThinkingIndicator();
      messages.push({ role: payload.role, content: payload.content });
      appendMessage(payload.role, payload.content);
      await refreshHealth();
    } catch (error) {
      hideThinkingIndicator();
      appendMessage("assistant", formatChatError(error));
      await loadModels();
    } finally {
      hideThinkingIndicator();
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

# NicoChat – Local AI Chat

A local, offline ChatGPT-style chat application powered by [Ollama](https://ollama.com).  
**No internet connection required** once Ollama and at least one model are installed.

---

## Features

- 🤖 **Model selection** – choose from any model you have pulled with Ollama
- 🔒 **Fully offline** – all inference runs locally via the Ollama daemon
- 💬 **Multi-turn conversations** – full chat history is sent with each request
- ⚡ **Streaming responses** – tokens appear in real time
- 🗑 **Clear button** – start a fresh conversation at any time

---

## Prerequisites

1. **Python 3.10+**
2. **[Ollama](https://ollama.com/download)** installed and running

   ```bash
   # Start the Ollama daemon (runs on localhost:11434)
   ollama serve
   ```

3. At least one model pulled:

   ```bash
   ollama pull llama3          # ~4 GB, good general-purpose model
   # or
   ollama pull mistral
   # or any other model from https://ollama.com/library
   ```

---

## Quick Start

```bash
# 1. Clone the repo
git clone https://github.com/npequeux/NicoChat.git
cd NicoChat

# 2. Install Python dependencies
pip install -r requirements.txt

# 3. Run the app
python app.py
```

Then open **http://localhost:5000** in your browser.

---

## Usage

1. Select a model from the **drop-down in the header** (lists all locally available Ollama models).
2. Type your message in the text box and press **Enter** (or click **Send**).
3. The assistant's response streams in real time.
4. Press **Shift+Enter** for multi-line messages.
5. Click **Clear** to reset the conversation.

---

## Project Structure

```
NicoChat/
├── app.py               # Flask application & Ollama integration
├── requirements.txt     # Python dependencies
├── templates/
│   └── index.html       # Chat UI (HTML/CSS/JS, no external CDN)
└── tests/
    └── test_app.py      # Unit tests
```

---

## Running Tests

```bash
pip install pytest
pytest tests/
```

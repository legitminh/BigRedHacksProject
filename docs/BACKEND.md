# Waypoint API (sibling repo)

Desktop and API are **two git repos**. Develop them together in Cursor via `Waypoint.code-workspace`.

| Repo | Path | GitHub |
|---|---|---|
| Desktop (this) | `~/BigRedHacksProject` | https://github.com/legitminh/BigRedHacksProject |
| API | `~/BigRedHacksProjectBackend` | https://github.com/legitminh/BigRedHacksProjectBackend |

## Coach / Ollama (no Ollama on end-user Macs)

1. On the **API machine** (your Mac while developing, or a server in production):

```bash
brew install ollama   # API host only
ollama serve
ollama pull qwen2.5:0.5b
ollama pull moondream

cd ~/BigRedHacksProjectBackend
npm install
cp .env.example .env   # if needed
# Set SESSION_SECRET (≥32 chars), COACH_API_TOKEN, OLLAMA_BASE_URL=http://127.0.0.1:11434
npm run dev            # http://127.0.0.1:8787
```

2. In the **desktop** `src-tauri/secrets.toml`:

```toml
local_llm_base = "http://127.0.0.1:8787/v1/coach"
coach_api_token = "<same as COACH_API_TOKEN>"
local_llm_model = "qwen2.5:0.5b"
local_vision_model = "moondream"
```

3. Desktop lock-in calls:
   - `GET /v1/coach/api/tags`
   - `POST /v1/coach/api/generate`  
   with `Authorization: Bearer <coach_api_token>`. The API proxies to Ollama.

## Friend / teammate clone (both repos)

```bash
cd ~
git clone https://github.com/legitminh/BigRedHacksProject.git
git clone https://github.com/legitminh/BigRedHacksProjectBackend.git
cd BigRedHacksProject
git checkout cursor/waypoint-rust-study-nav-bd7b
npm install
cp src-tauri/secrets.example.toml src-tauri/secrets.toml
# fill gemini + coach_api_token (same as backend COACH_API_TOKEN)

cd ~/BigRedHacksProjectBackend
npm install
cp .env.example .env
# fill SESSION_SECRET, COACH_API_TOKEN, OLLAMA_BASE_URL, Google web client…
npm run dev   # :8787

cd ~/BigRedHacksProject
npm run app:dev
```

Shipped `Waypoint.app` users do **not** need local Rust, Node, Ollama, or ffmpeg — only the app, macOS permissions, and network to Gemini + your API host.

## Open both in Cursor

```bash
open ~/BigRedHacksProject/Waypoint.code-workspace
```

Or **File → Open Workspace from File…**.

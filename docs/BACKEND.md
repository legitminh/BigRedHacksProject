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

2. In the **desktop** `src-tauri/secrets.toml` (no Gemini / Google secrets):

```toml
waypoint_api_base = "http://127.0.0.1:8787"
local_llm_base = "http://127.0.0.1:8787/v1/coach"
coach_api_token = "<same as COACH_API_TOKEN>"   # optional; JWT preferred after Google sign-in
local_llm_model = "qwen2.5:0.5b"
local_vision_model = "moondream"
```

3. Desktop lock-in calls:
   - `GET /v1/coach/api/tags`
   - `POST /v1/coach/api/generate`  
   with `Authorization: Bearer <user JWT or coach_api_token>`. The API proxies to Ollama.

4. Copilot / study-memory consolidation call `POST /v1/gemini/chat` with the user JWT.  
   `GEMINI_API_KEY` stays in the **backend** `.env` only.

## Friend / teammate clone (both repos)

```bash
cd ~
git clone https://github.com/legitminh/BigRedHacksProject.git
git clone https://github.com/legitminh/BigRedHacksProjectBackend.git
cd BigRedHacksProject
git checkout cursor/waypoint-rust-study-nav-bd7b
npm install
cp src-tauri/secrets.example.toml src-tauri/secrets.toml
# set waypoint_api_base + optional coach_api_token — never Gemini/Google keys

cd ~/BigRedHacksProjectBackend
npm install
cp .env.example .env
# fill SESSION_SECRET, GEMINI_API_KEY, Google web client, DATABASE_URL…
# production on the website host: BIND_HOST=0.0.0.0 PUBLIC_BASE_URL=https://yoursite.com
npm run dev   # :8787

cd ~/BigRedHacksProject
npm run app:dev
```

Shipped `Waypoint.app` users do **not** need local Rust, Node, Ollama, or ffmpeg — only the app, macOS permissions, and network to **your API host** (Gemini runs there).

## Open both in Cursor

```bash
open ~/BigRedHacksProject/Waypoint.code-workspace
```

Or **File → Open Workspace from File…**.

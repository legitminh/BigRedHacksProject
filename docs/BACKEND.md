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

## Open both in Cursor

```bash
open ~/BigRedHacksProject/Waypoint.code-workspace
```

Or **File → Open Workspace from File…**.

# Waypoint API + desktop wiring

Desktop and API are **two git repos**. Develop them together in Cursor via `Waypoint.code-workspace`.

| Repo | Path | GitHub |
|---|---|---|
| Desktop (this) | `~/BigRedHacksProject` | https://github.com/legitminh/BigRedHacksProject |
| API | `~/BigRedHacksProjectBackend` | https://github.com/legitminh/BigRedHacksProjectBackend |

**Full production deploy (any powerful server + HTTPS + Ollama + TigerData):**  
see the API repo’s **[DEPLOY.md](https://github.com/legitminh/BigRedHacksProjectBackend/blob/main/DEPLOY.md)**.

This file is the short desktop-side companion: local coach wiring and how to **compile Waypoint against any API URL**.

---

## Point this Mac app at any API server

The API base URL is baked into the binary from `src-tauri/secrets.toml` (gitignored).

```bash
cd ~/BigRedHacksProject
cp src-tauri/secrets.example.toml src-tauri/secrets.toml
```

**Production / remote server** (must match the API’s `PUBLIC_BASE_URL`):

```toml
waypoint_api_base = "https://api.example.com"
local_llm_base = "https://api.example.com/v1/coach"
local_llm_model = "qwen2.5:0.5b"
local_vision_model = "moondream"
coach_api_token = ""
presage_api_key = ""
```

**Local API on this Mac:**

```toml
waypoint_api_base = "http://127.0.0.1:8787"
local_llm_base = "http://127.0.0.1:8787/v1/coach"
local_llm_model = "qwen2.5:0.5b"
local_vision_model = "moondream"
coach_api_token = ""   # or same as backend COACH_API_TOKEN for pre-login coach probes
presage_api_key = ""
```

Then:

```bash
npm run app:dev      # iterate (localhost OK; no release guard)
# or
npm run app:build    # ship Waypoint.app (sets WAYPOINT_RELEASE=1)
```

`npm run app:build` sets `WAYPOINT_RELEASE=1`, so `src-tauri/build.rs` refuses empty / localhost / `http://` `waypoint_api_base`. Point `secrets.toml` at your public `https://` origin first. Use `npm run app:dev` or `npm run app:build:debug` for local `http://127.0.0.1:8787` builds.

### Rules

| Do | Don’t |
|---|---|
| Put only the **public API origin** in `secrets.toml` | Put `gemini_api_key` / Google OAuth secrets in the app (build fails if present) |
| Use **HTTPS** for any non-localhost API | Ship `http://` API URLs for production (`app:build` fails if you try) |
| Rebuild after changing `waypoint_api_base` | Expect end users to edit config |

Google OAuth, Gemini, Postgres, and Ollama stay on the **API host** (`.env` there). Signed-in users call the API with a JWT.

---

## What the desktop calls on the API

| Feature | Endpoint | Auth |
|---|---|---|
| Google sign-in | `POST /v1/auth/google/start` + poll | — |
| **Connection status** | `GET /v1/status` | Optional JWT (enriches account + Google) |
| Copilot / study-memory consolidate | `POST /v1/gemini/chat` | User JWT |
| Study companion Live (Gemini Live proxy) | `WS /v1/companion/live` | User JWT via `Sec-WebSocket-Protocol`: `waypoint.live.v1` + `bearer.<jwt>` (query `?access_token=` is deprecated / prod-off) |
| Study companion typed fallback | `POST /v1/companion/chat` | User JWT |
| Lock-in coach | `/v1/coach/api/*` | User JWT (or optional `coach_api_token`) |
| Study heads-up TTS (Grok/xAI) | `POST /v1/voice/tts` | User JWT (or optional `coach_api_token`) |
| Camera observe | `POST /v1/camera/observe` | User JWT (+ `client_meta.brightness` on active clips) |
| Calendar / Drive / study-memory | `/v1/google/*`, `/v1/drive/*`, `/v1/study-memory`, … | User JWT |
| Delete everything | `DELETE /v1/me/data` | User JWT |

Lock-in cloud history uses **`PUT /v1/study-memory` only** — the Mac does not call `/v1/tasks` or `/v1/sessions`.

Settings → Connection is a thin client of `/v1/status` (Tauri `service_status`). Indicator meanings: API repo [docs/STATUS.md](https://github.com/legitminh/BigRedHacksProjectBackend/blob/main/docs/STATUS.md).

**Copilot chat provider** (`LOCAL_CHAT_PROVIDER` on the API):
- `gemini` / `cloud` (default) — Gemini first; silent Ollama fallback on quota/outage (`OLLAMA_CHAT_MODEL`, default `qwen2.5:7b`). Cloud companion stays on this path.
- `ollama` / `llama` / `local` — always Ollama; Gemini is never called for chat.

Lock-in coaching always uses Ollama via `/v1/coach` (never Gemini). The Mac UI should not show “cloud coach hit the limit” when the local path works.

### Study heads-up voice (Grok / xAI)

Lock-in overlays still show instantly. Spoken nudges try **xAI TTS** through the API so `XAI_API_KEY` never ships in Waypoint.app:

| Env (API `.env`) | Purpose |
|---|---|
| `XAI_API_KEY` | Required for Grok TTS. Blank → desktop uses macOS `say`. |
| `XAI_TTS_VOICE` | Optional voice id (default `eve`). |

**Latency / fallback:** the desktop budgets ~3.8s for lock-in heads-up `POST /v1/voice/tts` (API waits up to ~4s on xAI). On timeout, 503 (key unset), auth failure, or playback error, coach falls back to local macOS `say`. Settings **Test speak** uses the same Grok proxy with `extended_wait: true` (~15s client budget) when signed in — not local-only. Talk/Live requires `XAI_API_KEY` on the API (no `say` fallback for Live speak).

---

## Local API + Ollama (dev)

On the **API machine** (can be the same laptop):

```bash
# API host only
ollama serve
ollama pull qwen2.5:0.5b
ollama pull moondream
ollama pull qwen2.5:7b    # Copilot fallback

cd ~/BigRedHacksProjectBackend
npm install
cp .env.example .env
# SESSION_SECRET (≥32), Google web client, GEMINI_API_KEY, XAI_API_KEY (heads-up TTS),
# OLLAMA_*, optional DATABASE_URL
npm run dev               # http://127.0.0.1:8787
```

Then `npm run app:dev` in this repo with `waypoint_api_base = "http://127.0.0.1:8787"`.

---

## Friend / teammate clone

```bash
cd ~
git clone https://github.com/legitminh/BigRedHacksProject.git
git clone https://github.com/legitminh/BigRedHacksProjectBackend.git

cd BigRedHacksProjectBackend
npm install && cp .env.example .env
# fill .env — see DEPLOY.md for production, or local values for laptop
npm run dev

cd ~/BigRedHacksProject
git checkout cursor/waypoint-rust-study-nav-bd7b
npm install
cp src-tauri/secrets.example.toml src-tauri/secrets.toml
# set waypoint_api_base to that API (local or https://…)
npm run app:dev
```

Shipped `Waypoint.app` users need only the app, macOS permissions, and network to **your** API host.

## Open both in Cursor

```bash
open ~/BigRedHacksProject/Waypoint.code-workspace
```

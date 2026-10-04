# Study companion (Gemini Live) — smoke test

Blueprint behavior: [gemini_live_demo](https://github.com/legitminh/gemini_live_demo)  
Architecture: **Frontend ↔ Waypoint API `/v1/companion/live` ↔ Gemini Live**. The desktop never opens a Google socket and never holds `GEMINI_API_KEY`.

## Prerequisites

1. Backend `.env` has `GEMINI_API_KEY` (and optional `GEMINI_LIVE_MODEL`, default `gemini-3.8-live`).
2. API on `:8787`:
   ```bash
   cd ~/BigRedHacksProjectBackend && npm run dev
   ```
3. Desktop pointed at that API (`src-tauri/secrets.toml` → `waypoint_api_base = "http://127.0.0.1:8787"`).
4. Sign in with Google in Waypoint.

## Smoke steps (match demo chat behavior)

1. **Start a mission** (Lock in → objective + duration → Launch).
2. On the session panel, tap **Talk** (not the main Copilot composer).
   - Presence should move Connecting → Listening.
   - Mic stays open for continuous turns (demo-style), not “dictate 4s into the text box”.
3. **Speak a question** about your mission material.
   - Interim/final user transcript appears in the session log.
   - Companion replies with streaming assistant text + spoken audio (Gemini native audio via the API proxy).
4. **Type a follow-up** in the session field while Live is open → Enter.
   - Same Live socket (`type: "text"`), multi-turn context preserved.
5. **Barge-in**: while companion is speaking, talk over it — playback should clear and Listening returns.
6. **Screencap**: ask something that needs the screen (“what’s on my screen?” / “look at this error”). The companion should request a capture via the API; the desktop grabs JPEG and streams it back through `/v1/companion/live` (not directly to Google).
7. **End Live** with Talk/Live again, or **End mission** — mic tracks stop, WebSocket closes, no leftover audio.
8. Open **Copilot** (main chat): prior Copilot history still works; companion turns must not have polluted it.

## Typed-only fallback

If Live cannot connect, typing in the session composer still uses `POST /v1/companion/chat` (backend Gemini/Ollama). Still no secrets in the client.

## Verifier checklist

| Check | Expected |
| --- | --- |
| Not dictation-only | Talk opens Live; speech auto-turns without stuffing the textarea |
| Backend-mediated | Browser WS host is API (`/v1/companion/live`); no `generativelanguage.googleapis.com` from the app |
| Session teardown | End mission / session-ended stops mic + WS + playback |
| No client secrets | No `GEMINI_API_KEY` in desktop secrets / frontend |
| Copilot intact | Main `#chat-form` still uses `chat_send` |

## API-only curl sanity

```bash
# After sign-in, with ACCESS_TOKEN:
curl -s -o /dev/null -w "%{http_code}\n" \
  -X POST http://127.0.0.1:8787/v1/companion/chat \
  -H "Authorization: Bearer ACCESS_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"message":"Quiz me","history":[],"context":{"goals":"heaps","remaining_mins":20}}'
# expect 200
```

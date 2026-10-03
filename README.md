# Waypoint

Rust study navigation coach for Big Red Hacks (theme: **navigation**).

Waypoint helps you navigate school logistics through a Gemini chat grounded in Google Calendar + Drive, then runs **lock-in** sessions that watch your screen and webcam. [Presage](https://physiology.presagetech.com/) supplies stress/focus wellness signals; Gemini vision distinguishes distractions (phone vs calculator) and nudges you with text prompts.

## Stack

- **Tauri 2** (Rust core + Vite/TypeScript webview)
- **Gemini** — school chat + multimodal session coaching
- **Google OAuth** — Calendar + Drive (readonly)
- **Presage Physiology API** — HR / RR / HRV-style signals from short webcam clips

## Setup

1. Install [Rust](https://rustup.rs/), Node 20+, and [Tauri Linux prerequisites](https://tauri.app/start/prerequisites/).
2. Optional but recommended for Presage clips: `ffmpeg` on `PATH`.
3. Copy env and fill keys:

```bash
cp .env.example .env
```

| Variable | Purpose |
|---|---|
| `GEMINI_API_KEY` | Google AI Studio key |
| `GEMINI_MODEL` | defaults to `gemini-flash-latest` |
| `PRESAGE_API_KEY` | Presage Physiology API key |
| `GOOGLE_CLIENT_ID` / `GOOGLE_CLIENT_SECRET` | Google Cloud OAuth client (Desktop or Web) with redirect to `http://127.0.0.1:<port>/oauth2/callback` |

Google Cloud OAuth consent scopes used:

- `https://www.googleapis.com/auth/calendar.readonly`
- `https://www.googleapis.com/auth/drive.readonly`

Add `http://127.0.0.1` / loopback redirects as allowed for the OAuth client. Waypoint binds an ephemeral localhost port during Connect Google.

4. Run:

```bash
npm install
npm run tauri dev
```

## Demo flow

1. Connect Google (Calendar + Drive).
2. Ask Waypoint what to do next — replies use live agenda and recent Drive notes.
3. Start a lock-in: duration + goals (e.g. calc PSet). Waypoint classifies paper vs computer.
4. Stay in frame; keep the session window visible. Text coach prompts appear when you drift, look stuck, or stress rises.
5. End session for an on-task summary.

Wellness metrics are informational only — not medical diagnosis.

## Project layout

```
src/                 # webview UI
src-tauri/src/
  gemini.rs          # Gemini chat + vision coach
  google/            # OAuth, Calendar, Drive
  presage.rs         # Physiology API client
  capture/           # webcam + screen frames
  session.rs         # lock-in state
  coach.rs           # coaching loop
```

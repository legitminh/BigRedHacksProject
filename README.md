# Waypoint

Rust study navigation coach for Big Red Hacks (theme: **navigation**).

Waypoint helps you navigate school logistics through a Gemini chat grounded in Google Calendar + Drive, then runs **lock-in** sessions that watch your screen and webcam. [Presage](https://physiology.presagetech.com/) supplies stress/focus wellness signals; Gemini vision distinguishes distractions (phone vs calculator) and nudges you with text prompts.

## Stack

- **Tauri 2** (Rust core + Vite/TypeScript webview)
- **Gemini** — school chat + multimodal session coaching
- **Google OAuth** — Calendar + Drive (readonly)
- **Presage Physiology API** — HR / RR / HRV-style signals from short webcam clips

## Build the final Mac app (recommended)

On your **Mac** (Apple Silicon or Intel):

1. Install prerequisites:
   - [Xcode Command Line Tools](https://developer.apple.com/xcode/resources/): `xcode-select --install`
   - [Rust](https://rustup.rs/)
   - [Node 20+](https://nodejs.org/)
   - Optional: `brew install ffmpeg` (Presage clips)

2. Clone and install:

```bash
git clone https://github.com/legitminh/BigRedHacksProject.git
cd BigRedHacksProject
git checkout cursor/waypoint-rust-study-nav-bd7b
npm install
```

3. Create secrets (dev or beside the built app later):

```bash
cp .env.example .env
# edit .env — add GEMINI_API_KEY, GOOGLE_*, PRESAGE_API_KEY
```

| Variable | Purpose |
|---|---|
| `GEMINI_API_KEY` | Google AI Studio key |
| `GEMINI_MODEL` | defaults to `gemini-flash-latest` |
| `PRESAGE_API_KEY` | Presage Physiology API key |
| `GOOGLE_CLIENT_ID` / `GOOGLE_CLIENT_SECRET` | Google Cloud OAuth client with Calendar + Drive readonly |

Google scopes:

- `https://www.googleapis.com/auth/calendar.readonly`
- `https://www.googleapis.com/auth/drive.readonly`

Allow loopback redirects (`http://127.0.0.1`) on the OAuth client. Waypoint opens a local callback during Connect Google.

4. **Compile the distributable app:**

```bash
npm run app:build
```

When it finishes, open:

```text
src-tauri/target/release/bundle/macos/Waypoint.app
```

Also produced (if bundling succeeds):

```text
src-tauri/target/release/bundle/dmg/Waypoint_0.1.0_*.dmg
```

Double-click **Waypoint.app** (or install from the `.dmg`). First launch may need **System Settings → Privacy & Security** approval for camera/screen recording.

5. Put API keys where the shipped app can find them (pick one):

- `~/Library/Application Support/com.bigredhacks.waypoint/.env`  
  (Tauri/config dir may also be `~/Library/Application Support/waypoint/.env` — Waypoint creates a template `.env` under its config folder on first run), or
- A `.env` file next to the app binary inside the bundle (less convenient), or
- Keep developing with repo-root `.env` via `npm run app:dev`

After editing keys, restart Waypoint.

### Dev mode (hot reload, not the final app)

```bash
npm run app:dev
```

## Demo flow

1. Connect Google (Calendar + Drive).
2. Ask Waypoint what to do next — replies use live agenda and recent Drive notes.
3. Start a lock-in: duration + goals (e.g. calc PSet). Waypoint classifies paper vs computer.
4. Stay in frame; allow camera + screen capture. Text coach prompts appear when you drift.
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

## Note on Cloud Agent builds

Linux CI/cloud VMs cannot produce a signed macOS `.app`. Always run `npm run app:build` on your Mac for the final binary.

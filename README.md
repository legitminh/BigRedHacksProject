# Waypoint

End-user study navigation app for Big Red Hacks.

Users open the app and **Sign in with Google**. API keys are baked in at build time — they never edit config files.

---

## Exact steps: download + build on your Mac

### 0) One-time installs (skip if you already have them)

Open **Terminal** and run:

```bash
xcode-select --install
```

Install Rust:

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

Then close Terminal, open a **new** Terminal window, and confirm:

```bash
rustc --version
```

Install Node (if needed):

```bash
brew install node
```

Optional on the **API host only** (not required for people who just open a shipped `.app`):

```bash
brew install ollama
# see docs/BACKEND.md — Ollama runs next to BigRedHacksProjectBackend
```

Presage clips use a **bundled AVFoundation encoder** (no Homebrew ffmpeg for end users). ffmpeg remains an optional dev fallback.

---

### 1) Download the project

```bash
cd ~
git clone https://github.com/legitminh/BigRedHacksProject.git
cd BigRedHacksProject
git checkout cursor/waypoint-rust-study-nav-bd7b
npm install
```

---

### 2) Create Google OAuth credentials (needed so users can Sign in)

1. Go to [Google Cloud Console](https://console.cloud.google.com/)
2. Create/select a project
3. **APIs & Services → Library** → enable:
   - Google Calendar API
   - Google Drive API
4. **APIs & Services → OAuth consent screen**
   - User type: **External**
   - App name: `Waypoint`
   - Add your email as developer/test user
   - Scopes: add
     - `https://www.googleapis.com/auth/calendar.readonly`
     - `https://www.googleapis.com/auth/drive.readonly`
5. **APIs & Services → Credentials → Create credentials → OAuth client ID**
   - Application type: **Desktop app**
   - Name: `Waypoint`
   - Create → copy **Client ID** and **Client secret**

---

### 3) Bake secrets into the app (you do this once as the builder)

```bash
cd ~/BigRedHacksProject
cp src-tauri/secrets.example.toml src-tauri/secrets.toml
open -e src-tauri/secrets.toml
```

Fill it like this (keep quotes):

```toml
gemini_api_key = "YOUR_GEMINI_KEY"
gemini_model = "gemini-flash-latest"
presage_api_key = "YOUR_PRESAGE_KEY_OR_LEAVE_EMPTY"
google_client_id = "YOUR_GOOGLE_CLIENT_ID.apps.googleusercontent.com"
google_client_secret = "YOUR_GOOGLE_CLIENT_SECRET"
```

`gemini_api_key` is **required** — the Rust crate will not compile if it is empty (unless you export a non-empty `GEMINI_API_KEY`).  
`presage_api_key` unlocks webcam stress / HR–RR checks during lock-in.  
`local_llm_base` + `coach_api_token` point at the Waypoint API coach proxy (`http://127.0.0.1:8787/v1/coach`) so lock-in does **not** need Ollama on the user Mac — see [docs/BACKEND.md](docs/BACKEND.md).

Save the file.  
`src-tauri/secrets.toml` is **gitignored** — it will not go to GitHub. It gets compiled into `Waypoint.app`.

---

### 4) Build the final Mac app

```bash
cd ~/BigRedHacksProject
npm run app:build
```

Wait until it finishes (several minutes the first time).

---

### 5) Run it

```bash
open ~/BigRedHacksProject/src-tauri/target/release/bundle/macos/Waypoint.app
```

Or Finder → go to that folder → double-click **Waypoint.app**.

If macOS blocks it: **System Settings → Privacy & Security → Open Anyway**.

For lock-in, allow **Screen Recording** (required — Waypoint watches your screen) and **Camera** (for Presage wellness). End users do **not** need Homebrew, Ollama, or ffmpeg — only the `.app`, permissions, and network (Gemini + your API if coach is remote).

A `.dmg` (if produced) will be under:

```text
~/BigRedHacksProject/src-tauri/target/release/bundle/dmg/
```

---

### 6) End-user flow (what people see)

1. Open Waypoint  
2. Tap **Sign in with Google**  
3. Approve Calendar + Drive access  
4. Use **Ask** or **Lock in**

No API key screens. No `.env` for end users.

---

## Dev mode (optional, not the shipped app)

```bash
cd ~/BigRedHacksProject
npm run app:dev
```

---

## Notes

- Chat renders Markdown, including tables and code blocks. Use **Explain simply**, **Quiz me**, or **Make a plan** to draft a study request; edit it before sending. Enter sends, Shift+Enter adds a line, and **New chat** resets the conversation.
- Chat supports inline LaTeX (`$...$` or `\(...\)`) and display equations (`$$...$$` or `\[...\]`), with bundled KaTeX fonts. Temporary Gemini overloads and rate limits retry the unchanged message up to five times, showing a delay message; persistent failures show a friendly retry notice.
- Run `npm test` (Node 26+) for Markdown safety and chat interaction checks.
- If Calendar works but Drive does not, enable **Google Drive API** in the same Google Cloud project as the desktop OAuth client. Adding consent-screen scopes alone does not enable the API. Wait a few minutes after enabling it; reconnect Google if the error instead says permissions are missing.
- Drive chat reads excerpts from Google Docs, Slides, Sheets (first sheet only), and text files. Other formats currently provide filenames and metadata only. Ask with a specific title or topic for files outside the recent listing.
- Lock-in is screen-first: Gemini coaches from periodic screenshots against your goals. Presage runs separate ~20s webcam clips for stress / HR / RR when Camera + `presage_api_key` + `ffmpeg` are available. Lock-in still starts if the camera is denied; wellness just stays offline.
- Coach lines pop up in an always-on-top overlay and are spoken with a local macOS `say` stand-in (`waypoint-voice` crate). Swap that crate’s TTS/STT for Grok Voice later. `voice_listen_test` records a short mic clip through the stub STT pipeline.
- Keep `src-tauri/Info.plist` in the build: it declares why Waypoint needs camera access. Without `NSCameraUsageDescription`, macOS terminates the app when lock-in touches the camera. Reopen the rebuilt `.app` after updating; allow Camera and Screen Recording in System Settings.
- Rebuild after any change to `src-tauri/secrets.toml` (`npm run app:build` again).
- Embedded keys can be extracted from a desktop binary — fine for a hackathon demo; rotate keys after the event if the repo/app is shared widely.
- This Linux cloud environment cannot produce a macOS `.app`. Always build on your Mac.

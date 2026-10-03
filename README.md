# Waypoint

End-user study navigation app for Big Red Hacks.

Users download the Mac app, **Sign in with Google**, and use Copilot + Lock-in.  
**Gemini and Google OAuth secrets live only on the Waypoint API** (same host as the website). Nothing stealable is baked into the `.app` except optional Presage / coach-dev tokens.

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

Also clone and run the API (sibling repo) — see [docs/BACKEND.md](docs/BACKEND.md).

---

### 2) Google OAuth (API server only)

Create a **Web application** OAuth client in [Google Cloud Console](https://console.cloud.google.com/) and put `GOOGLE_CLIENT_ID` / `GOOGLE_CLIENT_SECRET` in the **backend** `.env` only.  
Authorized redirect URIs must match `PUBLIC_BASE_URL` (e.g. `https://yoursite.com/v1/auth/google/callback`).

Do **not** put Google client secrets in the Mac app.

---

### 3) Point the app at your API (builder only)

```bash
cd ~/BigRedHacksProject
cp src-tauri/secrets.example.toml src-tauri/secrets.toml
open -e src-tauri/secrets.toml
```

Ship / production example:

```toml
waypoint_api_base = "https://yoursite.com"
local_llm_base = "https://yoursite.com/v1/coach"
local_llm_model = "qwen2.5:0.5b"
local_vision_model = "moondream"
coach_api_token = ""
presage_api_key = ""
```

- **Never** set `gemini_api_key` or Google secrets here — they are ignored / unsafe if present.
- `coach_api_token` is optional for local lock-in before sign-in; signed-in users send their JWT.
- `src-tauri/secrets.toml` is **gitignored**. Keep it free of Gemini/Google keys.

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

For lock-in, allow **Screen Recording** (required — Waypoint watches your screen) and **Camera** (for Presage wellness). End users do **not** need Homebrew, Ollama, or ffmpeg — only the `.app`, permissions, and network to your Waypoint API.

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

No API key screens. No `.env` for end users. Copilot talks to Gemini **through your API** with their JWT.

---

## Dev mode (optional, not the shipped app)

```bash
cd ~/BigRedHacksProject
npm run app:dev
```

---

## Notes

- Desktop repo: this project. API: `BigRedHacksProjectBackend`.
- Production: set `BIND_HOST=0.0.0.0` and `PUBLIC_BASE_URL=https://yoursite.com` on the API host; put Gemini + Google secrets only in that server’s `.env`.

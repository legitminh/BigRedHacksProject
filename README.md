# Waypoint

End-user study navigation app for Big Red Hacks.

Users download the Mac app, **Sign in with Google**, and use Copilot + Lock-in.  
**Gemini, Google OAuth, Postgres, and Ollama live only on your Waypoint API server.** The Mac build only needs the public API URL.

| Repo | Role |
|---|---|
| **This repo** | Tauri Mac app (frontend / desktop) |
| [BigRedHacksProjectBackend](https://github.com/legitminh/BigRedHacksProjectBackend) | API + secrets + Ollama |

- **Deploy / run the API:** [Backend DEPLOY.md](https://github.com/legitminh/BigRedHacksProjectBackend/blob/main/DEPLOY.md)  
- **Desktop ↔ API wiring details:** [docs/BACKEND.md](docs/BACKEND.md)

---

## Prerequisites (builder Mac)

You need a **macOS** machine to build Waypoint (Tauri + Swift helpers). End users who only install a shipped `.app` do **not** need Rust, Node, or Ollama.

| Tool | Why | Install |
|---|---|---|
| **macOS** | Tauri desktop + Apple frameworks (Vision / AVFoundation) | — |
| **Xcode Command Line Tools** | `swiftc`, SDKs for OCR / face-detect / encode-clip helpers | `xcode-select --install` |
| **Rust** (stable) | Tauri / Cargo backend | [rustup](https://rustup.rs) |
| **Node.js 22+** | Vite frontend, Tauri CLI, tests (`--experimental-strip-types`) | `brew install node` (or nvm / fnm) |
| **Homebrew** | Convenient Node (and optional `ffmpeg`) | https://brew.sh |

```bash
xcode-select --install
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
# new terminal
rustc --version
node -v   # expect v22+
brew install node   # if needed
```

**Optional (dev only):** `brew install ffmpeg` — used only if the bundled `waypoint-encode-clip` Swift helper is missing; end-user `.app` bundles do not require Homebrew ffmpeg/ollama.

Confirm `swiftc` is on `PATH` (`/usr/bin/swiftc`). Without it, OCR / face presence / clip encode helpers will not compile.

---

## Clone + install + secrets

```bash
cd ~
git clone https://github.com/legitminh/BigRedHacksProject.git
cd BigRedHacksProject
git checkout cursor/waypoint-rust-study-nav-bd7b
npm install
cp src-tauri/secrets.example.toml src-tauri/secrets.toml
```

`src-tauri/secrets.toml` is **gitignored** and gets compiled into the binary. Do not invent or paste Gemini / Google / xAI keys here — those live only on the API (`.env`). Leave `coach_api_token` and `presage_api_key` empty (non-empty values fail release builds).

Run / deploy the API first ([DEPLOY.md](https://github.com/legitminh/BigRedHacksProjectBackend/blob/main/DEPLOY.md)). Note its public origin, e.g. `https://api.example.com` or `http://127.0.0.1:8787`.

Edit both bases together (same origin, **no trailing slash**). `local_llm_base` must be `waypoint_api_base` + `/v1/coach`.

### Local API on this Mac

```toml
waypoint_api_base = "http://127.0.0.1:8787"
local_llm_base = "http://127.0.0.1:8787/v1/coach"
local_llm_model = "qwen2.5:0.5b"
local_vision_model = "moondream"
coach_api_token = ""
presage_api_key = ""
```

Use with `npm run app:dev` or `npm run app:build:debug` / `npm run app:install`.  
`npm run app:build` (release) **refuses** localhost / `http://` when `WAYPOINT_RELEASE=1`.

### Remote / production API

```toml
waypoint_api_base = "https://api.example.com"
local_llm_base = "https://api.example.com/v1/coach"
local_llm_model = "qwen2.5:0.5b"
local_vision_model = "moondream"
coach_api_token = ""
presage_api_key = ""
```

- `waypoint_api_base` must match the API’s `PUBLIC_BASE_URL` (scheme + host, no trailing slash).  
- Changing the API URL requires a **rebuild** (values are baked in at compile time).  
- Google OAuth is configured **only** on the API. Details: backend DEPLOY / README.

More wiring notes: [docs/BACKEND.md](docs/BACKEND.md).

---

## Dev loop

| Script | What it does | When to use |
|---|---|---|
| `npm run app:dev` | `tauri dev` — hot reload against `secrets.toml` | Day-to-day local work (`http://127.0.0.1:8787` OK) |
| `npm run app:build:debug` | Debug `.app` + `app:bundle-helpers -- debug` | Packaged debug build without release URL guards |
| `npm run app:install` | `app:build:debug` then `ditto` → `/Applications/Waypoint.app` | Local install only — **not** for judges/ship |
| `npm run app:build` | `WAYPOINT_RELEASE=1 tauri build` + bundle helpers into **release** | Ship / demo — needs `https://` `waypoint_api_base` |

```bash
npm run app:dev           # iterate
# or
npm run app:build:debug   # debug .app under src-tauri/target/debug/bundle/macos/
npm run app:install       # debug → /Applications (local only)

# ship
npm run app:build
open src-tauri/target/release/bundle/macos/Waypoint.app
```

**Judges / demo / ship:** use `npm run app:build`, then open the release `.app` — not `app:install`.

If macOS blocks the binary: **System Settings → Privacy & Security → Open Anyway**.

### Switching backends

Edit `src-tauri/secrets.toml` → new `waypoint_api_base` / `local_llm_base` → rebuild (`app:dev`, `app:build:debug`, or `app:build`). Distribute a new `.app` if you ship.

---

## Helper binaries (`bundle-helpers`)

Lock-in OCR, camera clips, and local face presence use three Swift helpers compiled by `src-tauri/build.rs` into `src-tauri/bin/`:

| Binary | Role |
|---|---|
| `waypoint-ocr` | Apple Vision OCR on a focused-window JPEG |
| `waypoint-encode-clip` | AVFoundation short webcam MP4 (optional ffmpeg fallback in dev) |
| `waypoint-face-detect` | Local Vision face / presence for camera accountability |

They use **Apple frameworks only** — no Homebrew ffmpeg/ollama required on the end-user Mac. The desktop still talks to the Waypoint API for coach / LLM / observe uploads.

`npm run app:build` and `app:build:debug` finish with:

```bash
node ./scripts/bundle-helpers.mjs <debug|release>
```

That script copies the three binaries from `src-tauri/bin/` into:

`src-tauri/target/<profile>/bundle/macos/Waypoint.app/Contents/MacOS/`

and `chmod +x` them. If the `.app` or a helper is missing, it exits with an error — rebuild `src-tauri` first so `swiftc` produces the bins. `app:dev` resolves helpers from `src-tauri/bin/` without that copy step.

---

## Camera accountability + macOS permissions

**Camera accountability is opt-in and off by default** (Settings → *Camera accountability*, or the mission toggle). It requires **Sign in with Google** so clips can reach `POST /v1/camera/observe` with a user JWT. Guests cannot enable it.

When on (and Camera permission granted), short ~12s webcam clips / local face signals support desk presence. When off: no camera permission prompt, no observe loop.

Grant permissions in **System Settings → Privacy & Security**, then **quit and reopen** Waypoint (Settings → Permissions shows badges):

| Permission | Required? | Why |
|---|---|---|
| **Screen Recording** | Yes for Lock-in | Local focus checks / screen context |
| **Camera** | Only if camera accountability is on | Presence clips |
| **Accessibility & Automation** | Recommended | Frontmost app, window title, open browser tabs |
| **Microphone** | Optional | Copilot dictation / voice tests |

Server-side camera contract: [Backend CAMERA-ACCOUNTABILITY.md](https://github.com/legitminh/BigRedHacksProjectBackend/blob/main/docs/CAMERA-ACCOUNTABILITY.md).

---

## Common failures

| Symptom | Likely cause | Fix |
|---|---|---|
| Camera accountability stuck / “unavailable” | Camera TCC denied, or toggle on without sign-in | Sign in → enable toggle → **System Settings → Camera** for Waypoint → quit/reopen |
| Sign-in / Copilot / coach fails; Connection status red | API not running or unreachable | Start API (`npm run dev` in backend) or check host / HTTPS proxy ([DEPLOY.md](https://github.com/legitminh/BigRedHacksProjectBackend/blob/main/DEPLOY.md)) |
| App still hits old host after editing `secrets.toml` | URL is compile-time | Rebuild (`app:dev` / `app:build:debug` / `app:build`); restart the app |
| `WAYPOINT_RELEASE=1` / `app:build` panics on API base | Localhost or `http://` in `waypoint_api_base` | Point at public `https://…` matching `PUBLIC_BASE_URL`, or use `app:dev` / `app:build:debug` for local |
| Lock-in won’t start (Screen Recording) | TCC not granted for this binary path | Enable Waypoint under Screen Recording; quit/reopen (especially after `app:install` to `/Applications`) |
| OCR / presence / clips broken | Helpers missing (`swiftc` failed) | Ensure Xcode CLT; rebuild; for packaged apps confirm `app:bundle-helpers` ran |
| Tab coaching weak | Accessibility not granted | System Settings → Accessibility (+ Automation if prompted) |

---

## End-user flow (shipped app)

1. Open Waypoint  
2. **Sign in with Google** (browser)  
3. Approve Calendar + Drive  
4. Use **Copilot** / **Lock in**

No API keys on the device. Copilot uses Gemini on your server, with silent local Ollama fallback when Gemini is limited.

---

## Related repos & docs

| Doc | Purpose |
|---|---|
| [BigRedHacksProjectBackend](https://github.com/legitminh/BigRedHacksProjectBackend) | API repo |
| [Backend DEPLOY.md](https://github.com/legitminh/BigRedHacksProjectBackend/blob/main/DEPLOY.md) | Production / local API host setup |
| [docs/BACKEND.md](docs/BACKEND.md) | Compile this app against any API URL; endpoint map |
| `open Waypoint.code-workspace` | Edit desktop + API together in Cursor |

**Production API note:** HTTPS + reverse proxy; keep Node on `127.0.0.1` — see [DEPLOY.md](https://github.com/legitminh/BigRedHacksProjectBackend/blob/main/DEPLOY.md).

# Waypoint

End-user study navigation app for Big Red Hacks.

Users download the Mac app, **Sign in with Google**, and use Copilot + Lock-in.  
**Gemini, Google OAuth, Postgres, and Ollama live only on your Waypoint API server.** The Mac build only needs the public API URL.

| Repo | Role |
|---|---|
| **This repo** | Tauri Mac app |
| [BigRedHacksProjectBackend](https://github.com/legitminh/BigRedHacksProjectBackend) | API + secrets + Ollama |

- **Deploy API on any powerful server:** [Backend DEPLOY.md](https://github.com/legitminh/BigRedHacksProjectBackend/blob/main/DEPLOY.md)  
- **Wire / compile this app against that server:** [docs/BACKEND.md](docs/BACKEND.md)

---

## Build the Mac app against your API

### 0) Builder machine installs (once)

```bash
xcode-select --install
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
# new terminal
rustc --version
brew install node   # if needed
```

End users who only install a shipped `.app` do **not** need Rust, Node, or Ollama.

### 1) Clone

```bash
cd ~
git clone https://github.com/legitminh/BigRedHacksProject.git
cd BigRedHacksProject
git checkout cursor/waypoint-rust-study-nav-bd7b
npm install
```

Run / deploy the API first ([DEPLOY.md](https://github.com/legitminh/BigRedHacksProjectBackend/blob/main/DEPLOY.md)). Note its public origin, e.g. `https://api.example.com` or `http://127.0.0.1:8787`.

### 2) Point the app at that API

```bash
cp src-tauri/secrets.example.toml src-tauri/secrets.toml
```

**Any remote / production API:**

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
coach_api_token = ""
presage_api_key = ""
```

- `waypoint_api_base` must match the API’s `PUBLIC_BASE_URL` (scheme + host, no trailing slash).  
- **Never** put Gemini or Google client secrets in `secrets.toml` (unsafe and the build rejects non-empty values).  
- Changing the API URL requires a **rebuild**.

Google OAuth is configured **only** on the API (Web client + redirect URIs under that same public origin). Details in the backend DEPLOY / README.

### 3) Dev or ship

```bash
npm run app:dev      # hot reload against secrets.toml (localhost OK)
npm run app:build    # release Waypoint.app (WAYPOINT_RELEASE=1; needs https:// API base)
```

```bash
open src-tauri/target/release/bundle/macos/Waypoint.app
```

If macOS blocks it: **System Settings → Privacy & Security → Open Anyway**.  
Lock-in needs **Screen Recording**; camera is optional (Presage).

### 4) End-user flow

1. Open Waypoint  
2. **Sign in with Google** (browser)  
3. Approve Calendar + Drive  
4. Use **Ask** / **Lock in**

No API keys on the device. Copilot uses Gemini on your server, with silent local Ollama fallback when Gemini is limited.

---

## Switching backends

Edit `src-tauri/secrets.toml` → new `waypoint_api_base` / `local_llm_base` → `npm run app:build` again. Distribute the new `.app`.

---

## Notes

- Production API: HTTPS + reverse proxy; keep Node on `127.0.0.1` — see [DEPLOY.md](https://github.com/legitminh/BigRedHacksProjectBackend/blob/main/DEPLOY.md).  
- Workspace: `open Waypoint.code-workspace` to edit desktop + API together.

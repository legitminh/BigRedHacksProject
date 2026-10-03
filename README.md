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

Optional (Presage video clips):

```bash
brew install ffmpeg
```

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

Also allow **Camera** and **Screen Recording** when lock-in asks.

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

- Rebuild after any change to `src-tauri/secrets.toml` (`npm run app:build` again).
- Embedded keys can be extracted from a desktop binary — fine for a hackathon demo; rotate keys after the event if the repo/app is shared widely.
- This Linux cloud environment cannot produce a macOS `.app`. Always build on your Mac.

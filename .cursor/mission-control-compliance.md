# Mission Control — Figma compliance brief (pass 2)

Figma: https://www.figma.com/design/BR1qUqQ2xrpSCJdLgzhKDI/Waypoint-•-Mission-Control  
Repo: `/Users/aquoria/BigRedHacksProject`

## Critical correction
Previous agents built a **dark navy / neon glass** theme. The Figma is **soft lavender / cream light Mission Control** with flat cosmic illustrations.

### Brand tokens (match screenshots)
- Page bg: soft lavender ~`#E8E0F5` → `#F3EEF9` (not black)
- Cards: warm cream / off-white ~`#FBF8F3` / `#FFFDF8`
- Primary purple buttons: ~`#6B4CFF` / `#7C5CFF` pill
- Secondary: light lavender pills / outlined cream
- Text: deep indigo/navy ~`#2A2440`, muted mauve labels
- Kickers: `✦  WELCOME ABOARD`, `✦  READY WHEN YOU ARE`, `✦  YOUR COPILOT` (star glyph + caps)
- Font: clean sans (Inter ok if Figma uses similar); generous tracking on kickers

### Figma assets (USE THESE — do not invent CSS planets)
Under `src/assets/figma/`:
- `constellation.svg` — starfield background
- `planet-ringed.svg` — Saturn-style destination
- `ship.svg` — purple rocket
- `moon.svg` — personal-best moon
- `satellite.svg` — home satellite
- `astronaut.svg` — Copilot mascot
- `screens/*.png` — full-frame refs
- Reference PNGs also in `.cursor/figma-refs/01-welcome.png` … `19-first-flight.png`

### Screen refs (must match layout + copy)
| Screen | Ref PNG | Node |
|---|---|---|
| Welcome | `01-welcome.png` | 2:33 |
| Home | `02-home.png` | 2:34 |
| Copilot | `03-copilot.png` | 2:35 |
| Settings | `04-settings.png` | 2:36 |
| Mission setup | `05-mission-setup.png` | 2:37 |
| Active mission | `06-mission-active.png` | 2:38 |
| Quest complete | `12-quest-complete.png` | 2:44 |
| First flight | `19-first-flight.png` | 2:51 |

### Layout rules from Figma
- Top nav: **Home · Copilot · Lock in · Settings** (+ avatar on some)
- Welcome: constellation bg + ringed planet + rocket; cream sign-in card; **Sign in →** / **Continue as guest**
- Home: huge “Ready when you are.”; personal best card with moon; **Let’s lock in →**; satellite art right
- Active: soft bg; orbital dashed path with **ship.svg** from Launch → Destination; giant **13:00**; “REMAINING IN YOUR FLIGHT”; Pause + End mission; flight minutes line
- Copilot: cream chat card; astronaut; prompt pills; purple mic
- Setup: “01 / PREPARE FOR LAUNCH”; “What do you want to finish?”; duration chips; Launch mission
- Quest complete / First flight: soft cards, exact Figma headlines

### Agent rules
1. READ the matching `.cursor/figma-refs/*.png` before editing.
2. Prefer `<img src="/src/assets/figma/...">` for illustrations.
3. Replace dark `--mc-bg-void` theme with lavender/cream tokens.
4. Match Figma copy (Lock in, Let’s lock in, REMAINING IN YOUR FLIGHT).
5. One commit each; no push; don’t commit `.cursor/`.
6. Don’t break Tauri invokes / coach backend.

# Waypoint • Mission Control — implementation brief

Figma: https://www.figma.com/design/BR1qUqQ2xrpSCJdLgzhKDI/Waypoint-%E2%80%A2-Mission-Control?node-id=0-1  
Overview ref: `.cursor/figma-refs/mission-control-overview.png`  
App root: `/Users/aquoria/BigRedHacksProject` (Tauri + Vite, `index.html`, `src/main.ts`, `src/styles.css`, overlay)

## Visual language
- Dark navy/charcoal space background (not flat purple-on-white)
- Accents: vivid purple (~#7C3AED) + cyan/teal glow
- Large planet / orbital progress rings as hero visuals
- Pill buttons, rounded mission cards, clean light sans type on dark
- Pixel/stylized **spaceship** sprites as gamification avatars

## Screen map (from Figma frames ~00–37)
| Area | Frames (approx) | Maps to app |
|---|---|---|
| Intro / Welcome | Start here, Welcome | Home / first-run |
| Mission Control Home | Home, Big goals | Home dashboard |
| Copilot | Copilot | Ask chat |
| Mission setup | Mission setup / objectives | Lock-in start form |
| Active mission | Timer, Active mission, On a break | Session view |
| Gentle reminders | Overlay nags | Overlay toast + voice |
| Quest complete / Launch / First flight | Celebration | Summary + new reward flows |
| Flight log | Flight log, Ended sessions | Summary history |
| Settings / Permissions / Connection | Settings screens | Settings |
| Ship components | Ship sprite set | Shared gamification UI |

## Coordination rules for feature agents
1. Own ONLY your feature files/sections; avoid rewriting unrelated views.
2. Prefer CSS variables in `src/styles.css` for tokens; do not invent a second theme system.
3. Keep existing Tauri commands / coach backend working — UI rename lock-in → mission is OK if wired.
4. One focused commit per agent with HEREDOC; do not push; do not commit `.cursor/` or secrets.
5. Read the overview PNG before coding. Match Figma naming (Mission, Copilot, Flight log, Ship).
6. If you need a file another agent owns, stop and leave a `## blocked:` note in this brief.

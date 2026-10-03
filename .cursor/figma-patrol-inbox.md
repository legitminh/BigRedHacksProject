# Figma patrol inbox

Patrol appends `## open:` deviations vs Mission Control Figma.
Fixer marks `## done:` after a commit.
Compliance marks `## verified:` or reopens as `## open:` with `- reopened: true` and `- escalate: scrutinous`.

Figma: https://www.figma.com/design/BR1qUqQ2xrpSCJdLgzhKDI/Waypoint-•-Mission-Control  
Refs: `.cursor/figma-refs/*.png` · Assets: `src/assets/figma/` · Brief: `.cursor/mission-control-compliance.md`

```
## open: <short-id>
- screen: welcome | home | copilot | setup | active | summary | settings | overlay | global
- ref: <figma-refs png or node>
- deviation: <concrete visual/copy mismatch>
- fix_hint: <smallest change>
- escalate: none | scrutinous
```

---

## verified: fp-home-hero-title
- commit: 4cf9e19
- change: Home dashboard hero `<h1>` now reads “A little focus goes a long way.” per Figma 02-home.
- verified: 2026-10-03T13:41:07-04:00

## verified: fp-home-longest-badge
- commit: bf33c21
- change: Personal-best pill uses coral flag SVG (no ✦); uppercase label via `.mc-home-tag`.
- verified: 2026-10-03T13:41:07-04:00

## verified: fp-setup-top-nav
- screen: setup
- ref: `.cursor/figma-refs/05-mission-setup.png` · compare Home/Copilot/Settings/active: cream `settings-top-bar` / `welcome-nav--signed-in` / `copilot-nav`
- deviation: |
    EXPECTED (Figma 05 Mission setup top bar, left→right):
    1. Structure: full-width cream/white rounded pill bar (same MC chrome as Home/Settings/active), not a lone control.
    2. Brand (left): purple four-point star ✦ + bold “Waypoint” wordmark.
    3. Primary links (center): plain “Home”, plain “Copilot”, then “Lock in” as the active item — lavender pill background + stronger weight (`settings-nav-link--active` / `aria-current="page"`). Home and Copilot must NOT be pill-active.
    4. Trailing (right): plain “Settings” text control + circular teal avatar with user initials (Figma sample “JL”; app should use signed-in initials like `#settings-avatar` / `#session-avatar`).
    5. No back-chevron as the only chrome.
    ACTUAL (`#view-lockin` on HEAD):
    1. `<header class="mission-setup-head">` contains only `<button class="mission-setup-back" data-back>←</button>`.
    2. Missing brand ✦ Waypoint, missing Home/Copilot/Lock in row, missing Settings link, missing avatar.
    3. Claimed bf33c21 `settings-top-bar` mount is absent from current `index.html`.
- fix_hint: |
    Replace `mission-setup-head` with the same markup as `#view-session`’s `settings-top-bar session-top-bar`: brand `<p class="settings-brand">`, nav Home + Copilot buttons (`data-session-nav` or `data-settings-nav`), Lock in as non-button `settings-nav-link--active`, Settings button → `openSettings()`, `#setup-avatar` wired in `renderNavAvatar`. Keep `#goals-copilot-affordance` launch wiring untouched.
- commit: 3fa9f3d
- change: Replaced setup `←` head with shared `settings-top-bar` (✦ Waypoint, Home/Copilot, Lock in active, Settings + `#setup-avatar`); reused `data-session-nav` handlers; left in-field ✦ launch affordance alone.
- verified: 2026-10-03T14:10:57-04:00


## verified: fp-launch-cta-arrow
- commit: 4cf9e19
- change: Launch CTA uses “Launch mission →” in `index.html` and `missionLaunchLabel()`.
- verified: 2026-10-03T13:41:07-04:00

## verified: fp-active-figma-layout
- screen: active
- ref: `.cursor/figma-refs/06-mission-active.png`
- deviation: `#view-session` is a single centered card with “✦ Active mission” / Waypoint / “Mute voice”; Figma uses MC top nav, “✦ CURRENT MISSION” + goal header with Camera/Screen/Audio pills, a two-column layout (flight map + timer left, “✦ Your copilot” chat sidebar right).
- fix_hint: Add shared nav, rebuild `session-head` + two-column grid, and reuse `#view-chat` composer patterns for the sidebar; wire existing session/copilot invokes without changing backend contracts.
- commit: 3230044
- change: Active mission uses MC top bar, mission header + signal pills, two-column flight/copilot grid with in-session chat composer wired to `chat_send`.
- verified: 2026-10-03T13:41:07-04:00

## verified: fp-summary-top-nav
- screen: summary
- ref: `.cursor/figma-refs/12-quest-complete.png` · also `.cursor/figma-refs/07---Mission-recap.svg` if present; compare active session top bar
- deviation: |
    EXPECTED (Figma 12 Quest complete top bar, left→right):
    1. Structure: same cream MC top bar above the celebration card / space decor (CSS already reserves `.quest-complete-head` width).
    2. Brand (left): purple ✦ + “Waypoint”.
    3. Primary links (center): “Home”, “Copilot”, “Lock in” — Figma shows Lock in on a lavender pill (mission-flow active), not Home. Home/Copilot are plain text links.
    4. Trailing (right): “Settings” text link + teal circular avatar with initials (Figma “JL”).
    5. Bar sits above `.quest-complete-stage` / card; footer “Mission ended · …” stays below CTAs.
    ACTUAL (`#view-summary` on HEAD):
    1. Section opens directly at `<div class="quest-complete-stage">` — zero top nav markup.
    2. No `settings-top-bar`, no brand, no Home/Copilot/Lock in, no Settings, no avatar.
    3. Claimed bf33c21 mount is absent from current `index.html`.
- fix_hint: |
    Insert shared `settings-top-bar quest-complete-head` as first child of `#view-summary` (before `.quest-complete-stage`), Lock in `settings-nav-link--active`, Home/Copilot/Settings via existing `data-session-nav` handlers, `#summary-avatar` in `renderNavAvatar`.
- commit: 3fa9f3d
- change: Mounted `settings-top-bar quest-complete-head` above `.quest-complete-stage` with Lock in active, Settings + `#summary-avatar`, shared `data-session-nav` wiring.
- verified: 2026-10-03T14:10:57-04:00


## verified: fp-launch-overlay-scrim
- commit: 8b772b5
- change: Launch overlay uses soft lavender scrim, cream card, and `ship.svg` / `planet-ringed.svg` instead of dark fullscreen CSS shapes.
- verified: 2026-10-03T13:41:07-04:00

## verified: fp-copilot-sidebar-copy
- commit: 8b772b5
- change: Copilot sidebar body copy matches Figma (“Ask freely, talk through a roadblock…”).
- verified: 2026-10-03T13:41:07-04:00

## verified: fp-copilot-new-chat-btn
- commit: 8b772b5
- change: Removed “New chat” from `copilot-nav-end`; `#new-chat` kept as visually hidden control inside `.copilot-panel` for tests/behavior.
- verified: 2026-10-03T13:41:07-04:00

## verified: fp-active-status-chip
- screen: active
- ref: `.cursor/figma-refs/06-mission-active.png`
- deviation: `#session-status.status-chip` shows raw coach state (e.g. “watching”) under the orbit; Figma uses a green-dot “Mission in progress” pill plus “MISSION 001” label above the flight graphic, not a lowercase state chip.
- fix_hint: Replace inline status chip with Figma badges in `session-mission-card` header; map coach state to copy/colors separately if still needed for a11y.
- escalate: none
- commit: 3230044
- change: Card header shows green-dot progress pill + MISSION 001; coach state moved to visually hidden `#session-status` for screen readers.
- verified: 2026-10-03T13:41:07-04:00

## verified: fp-setup-objective-affordance
- screen: setup
- ref: `.cursor/figma-refs/05-mission-setup.png`
- deviation: “What do you want to finish?” field is a plain textarea; Figma shows the same lavender field with a small circular purple icon anchored on the right inside the input row.
- fix_hint: Wrap `#goals` in a positioned container and add the Figma mic/plus circle control (visual-only or wire to copilot) matching ref spacing.
- escalate: none
- commit: 3230044
- change: Objective textarea wrapped with in-field purple ✦ affordance button opening Copilot.
- verified: 2026-10-03T13:41:07-04:00

## verified: fp-active-earth-kepler-label
- screen: active
- ref: `.cursor/figma-refs/06---Mission-in-progress.svg` · `06-mission-active.png`
- deviation: Figma flight map panel shows an upper-left route label “EARTH → KEPLER”; `#session-orbit` only has Launch/Destination waypoint captions, no route kicker.
- fix_hint: Add a small uppercase label in `.session-flight-scene` (top-left) matching Figma copy/spacing; keep using `ship.svg` / `planet-ringed.svg` assets.
- escalate: none
- commit: ead9e40
- change: `.session-flight-route` kicker “Earth → Kepler” in top-left of `#session-orbit`.
- verified: 2026-10-03T13:41:07-04:00

## verified: fp-active-orbit-personal-best
- screen: active
- ref: `.cursor/figma-refs/06-mission-active.png`
- deviation: Orbit path includes a green flag marker and “Personal best 20 min” callout along the dashed route; dev scene has moon + planet but no PB waypoint.
- fix_hint: Place a flag icon + label on the arc (reuse home flag SVG styling or a small asset from `src/assets/figma/`) positioned per ref; wire copy from stored longest-flight minutes when available.
- escalate: none
- commit: ead9e40
- change: Green flag callout on orbit arc; `updateSessionOrbitPersonalBest()` from ship progress longest minutes.
- verified: 2026-10-03T13:41:07-04:00

## verified: fp-active-orbit-satellite
- screen: active
- ref: `.cursor/figma-refs/06---Mission-in-progress.svg`
- deviation: Figma mid-orbit illustration includes `satellite.svg` on the flight path; `#session-orbit` omits the satellite between Launch and Destination.
- fix_hint: Add `<img src="/src/assets/figma/satellite.svg">` inside `.session-flight-scene` with absolute positioning aligned to the dashed path in the ref.
- escalate: none
- commit: ead9e40
- change: `satellite.svg` on dashed path via `.session-flight-satellite` in active flight scene.
- verified: 2026-10-03T13:41:07-04:00

## verified: fp-active-next-step-timer
- screen: active
- ref: `.cursor/figma-refs/06-mission-active.png`
- deviation: Below flight minutes, Figma shows a lavender “NEXT STEP TIMER” pill with a countdown (e.g. 04:32) and helper line “One small step at a time.”; active card ends after flight-minutes hint with no next-step widget.
- fix_hint: Add footer row under `.session-flight-minutes-hint` with kicker pill + timer element; hook to existing coach/step timer if present or stub static layout matching ref.
- commit: 33a98f6
- change: Added `.session-next-step` lavender pill + countdown under flight-minutes hint; starts via suggest chip / defaults to 05:00; pauses with mission Pause.
- verified: 2026-10-03T14:10:57-04:00


## verified: fp-active-pause-dev-ui
- screen: active
- ref: `.cursor/figma-refs/06-mission-active.png`
- deviation: Figma primary “Pause” control is enabled on the cream card; `#session-pause` stays disabled and `.session-pause-note` exposes internal copy (“Pause isn’t wired yet…”).
- fix_hint: Hide or restyle the dev note for production UI; enable Pause styling to match Figma (wire pause invoke when ready, or visually match enabled state per design if behavior stays stubbed).
- commit: af13657
- change: Enabled Pause (aria-pressed Pause/Resume toggle); hid `.session-pause-note` via visually-hidden.
- verified: 2026-10-03T14:10:57-04:00


## verified: fp-settings-defaults-off
- screen: settings
- ref: `.cursor/figma-refs/04-settings.png` · also active pills on `.cursor/figma-refs/06-mission-active.png`
- deviation: |
    EXPECTED (Figma 04 “During a mission”, top→bottom):
    1. Copilot audio — toggle OFF
    2. Camera signals — toggle OFF
    3. Screen sharing — toggle OFF
    4. Reduce motion — toggle OFF
    Fresh install / no settings.json must render all four off. Active mission signal pills show “Audio off” when Copilot audio is off (Figma 06).
    ACTUAL (pre-fix): camera/screen/reduce-motion prefs defaulted false, but `UserSettings.silent_mode` defaulted false so `#setting-copilot-audio` synced ON after load.
- fix_hint: |
    Set `UserSettings::default().silent_mode = true` so Copilot audio renders off (matches Figma 04 all-off + Figma 06 “Audio off”). Keep HTML unchecked; do not remove Settings tabs.
- escalate: none
- reopened: true
- commit: f9c0f02
- change: Default `silent_mode` true so fresh installs show Copilot audio off (Figma 04/06); camera/screen/reduce-motion remain false prefs.
- verified: 2026-10-03T14:32:29-04:00


## verified: fp-active-waypoint-sublabels
- screen: active
- ref: `.cursor/figma-refs/06-mission-active.png`
- deviation: Flight waypoints only said “Launch” / “Destination”; Figma places “EARTH” under Launch and “KEPLER” under Destination.
- fix_hint: Add uppercase sublabels under each `.session-flight-waypoint`.
- commit: af13657
- change: Earth/Kepler sublabels stacked under Launch/Destination waypoints.
- verified: 2026-10-03T14:10:57-04:00


## verified: fp-active-copilot-suggest
- screen: active
- ref: `.cursor/figma-refs/06-mission-active.png`
- deviation: Session copilot sidebar jumps from chat log to composer; Figma shows a wide lavender “Set a five-minute timer” suggestion chip above the message field (plus Enter/mic hint under the composer).
- fix_hint: Add a `.session-copilot-suggest` button + composer hint line under `#session-chat-form` matching Figma copy; stub click or wire to existing step-timer if present.
- commit: 33a98f6
- change: Added “Set a five-minute timer” suggest chip above session composer; wires to next-step countdown.
- verified: 2026-10-03T14:10:57-04:00


## open: fp-setup-affordance-behavior
- screen: setup
- ref: `.cursor/figma-refs/05-mission-setup.png`
- deviation: In-field purple ✦ control is labeled “Launch mission” and `requestSubmit()`s `#lockin-form`; Figma shows a small assist/mic-style circle on the objective field (prior verified intent: open Copilot), not a second Launch control.
- fix_hint: Restore open-Copilot (or voice) behavior; set aria-label/title to match assist, not Launch.
- escalate: none
- note: Skipped this tick — user requested objective ✦ remain Launch; do not revert.

## verified: fp-active-waypoint-sublabels-uppercase
- screen: active
- ref: `.cursor/figma-refs/06-mission-active.png` · live `2:38` · nodes `6:189`/`6:190`
- deviation: |
    EXPECTED (live Figma 06 flight scene): waypoint captions are only “Launch” (`6:189`) and
    “Destination” (`6:190`). Route kicker alone carries “EARTH → KEPLER” (`6:168`). No EARTH/KEPLER
    sublabels under the waypoints.
    ACTUAL (`#session-orbit`): `.session-flight-waypoint-sub` still mounts “Earth”/“Kepler”
    (CSS-uppercased via 232e40d). Live frame no longer has those subs — uppercase fix is moot /
    adds extra labels vs Figma.
- fix_hint: |
    Remove `.session-flight-waypoint-sub` spans (and related CSS) so Launch/Destination stand alone;
    keep `.session-flight-route` “EARTH → KEPLER”. Do not change Pause/End or LTR orbit.
- escalate: scrutinous
- reopened: true
- commit: 44f3b0e
- change: Removed `.session-flight-waypoint-sub` Earth/Kepler spans + CSS; Launch/Destination only; route kicker “EARTH → KEPLER” kept.
- verified: 2026-10-03T15:32:34-04:00


## open: fp-end-confirm-figma-modal
- screen: active
- ref: `.cursor/figma-refs/11---End-confirmation.svg`
- deviation: `#end-session` uses `window.confirm("End this mission?…")`; Figma 11 is a designed End confirmation surface (cream modal / screen chrome), not a native browser dialog.
- fix_hint: Add a small MC-styled confirm dialog matching Figma 11; keep Pause/End wiring, only replace the confirm UI.
- escalate: none

## open: fp-gentle-checkin-session-panel
- screen: active
- ref: `.cursor/figma-refs/07-gentle-reminder-temp.png` · `08---Gentle-check-in.svg`
- deviation: Figma replaces the session copilot sidebar with a “✦ QUICK CHECK-IN” card (Still working… + On task / Got distracted / Take a break). App uses a separate toast overlay (`overlay.html`) without those three actions in the session layout.
- fix_hint: When a check-in fires, swap/overlay the session copilot panel with the Figma quick-check-in card; wire buttons to existing coach responses.
- escalate: none

## verified: fp-session-chat-meta-kickers
- screen: active
- ref: `.cursor/figma-refs/06-mission-active.png` · live `2:38` · `6:219`/`6:222`
- deviation: Session chat rows are plain `.bubble` nodes; Figma 06 prefixes each turn with uppercase meta kickers (“AT LAUNCH”, “YOU · JUST NOW”) above the message body.
- fix_hint: Extend `appendSessionChat` (and AT LAUNCH seed) to render a meta kicker line + body per Figma.
- escalate: none
- commit: 232e40d
- change: `appendSessionChat` wraps turns with `.session-chat-meta` kickers (YOU · JUST NOW / COPILOT · JUST NOW; optional AT LAUNCH override).
- verified: 2026-10-03T15:27:27-04:00

## verified: fp-session-mission-star-style
- screen: active
- ref: `.cursor/figma-refs/06-mission-active.png` · live `2:38`
- deviation: `.session-mission-star` is a purple filled gradient square with a ✦ glyph; Figma 06 uses a dark rounded square with a white outline star.
- fix_hint: Restyle `.session-mission-star` to dark navy fill + outline star (SVG or border glyph) matching the ref.
- escalate: none
- commit: 232e40d
- change: Dark `#282237` rounded square + lavender outline-star SVG from Figma 06 path.
- verified: 2026-10-03T15:27:27-04:00

## verified: fp-copilot-chip-plus
- screen: copilot
- ref: `.cursor/figma-refs/03-copilot.png` · live Figma `2:35`
- deviation: |
    Live Figma 03 Copilot prompt chips are plain labels only — no trailing `+` on any chip
    (including “Find a next step”). Ambient decorative `+` marks belong on page margins, not in chips.
- fix_hint: Remove all `.copilot-chip-plus` mounts; keep three lavender text chips.
- escalate: scrutinous
- commit: ffaa518
- change: Stripped chip trailing `+` entirely; decorative page-corner `+` marks remain outside chips.
- note: |
    Earlier 52749cd kept + on Find a next step per older PNG crops; live frame 2:35 (2026-10-03)
    no longer includes that glyph.
- verified: 2026-10-03T15:06:26-04:00

## verified: fp-copilot-chip-plus-glyph
- screen: copilot
- ref: `.cursor/figma-refs/03-copilot.png` · live Figma `2:35`
- deviation: Chip trailing `+` glyph styling superseded — live Figma has no chip plus.
- fix_hint: n/a — remove mount.
- escalate: none
- commit: ffaa518
- change: `.copilot-chip-plus` unused/hidden; no chip glyph to style.
- verified: 2026-10-03T15:06:26-04:00

## verified: fp-summary-decor-moon
- screen: summary
- ref: `.cursor/figma-refs/12-quest-complete.png`
- deviation: Figma quest-complete stage shows a cratered purple moon under the left ringed planet; `.quest-complete-decor` only mounts constellation, planet, ship, and satellite.
- fix_hint: Add `moon.svg` as `.quest-complete-moon` under the left planet with absolute placement matching the ref.
- escalate: none
- commit: 2971eae
- change: Mounted `moon.svg` as `.quest-complete-moon` under the left planet in quest-complete decor.
- verified: 2026-10-03T14:32:29-04:00

## verified: fp-settings-intro-kicker
- screen: settings
- ref: `.cursor/figma-refs/04-settings.png`
- deviation: Settings intro kicker rendered “✦ Make yourself at home”; Figma 04 uses plain uppercase “MAKE YOURSELF AT HOME” (kicker kept, no star).
- fix_hint: Remove the ✦ span from `.settings-kicker`; keep the kicker line (CSS already uppercases).
- escalate: none
- commit: 64a5890
- change: Dropped ✦ from `.settings-kicker`; plain “Make yourself at home” remains (uppercase via CSS).
- verified: 2026-10-03T14:32:29-04:00

## verified: fp-active-copilot-presence-dot
- screen: active
- ref: `.cursor/figma-refs/06-mission-active.png`
- deviation: Session copilot presence is plain lavender text “Here when you need me”; Figma shows a green-dot status pill (same language as the mission progress pill).
- fix_hint: Restyle `.session-copilot-presence` with a green leading dot matching `.session-progress-pill` / Figma.
- escalate: none
- commit: 2971eae
- change: Presence pill uses green leading dot matching `.session-progress-pill`.
- verified: 2026-10-03T14:32:29-04:00

## verified: fp-home-dest-orbits
- screen: home
- ref: `.cursor/figma-refs/02-home.png`
- deviation: Figma dark “Your next destination?” card includes faint orbital/constellation line art behind the planet/ship; `.mc-home-dest-art` has planet/ship/moon/satellite but no orbits/constellation layer.
- fix_hint: Add a low-opacity `orbits.svg` or constellation asset inside `.mc-home-dest-art` positioned behind the planet.
- escalate: none
- commit: 64a5890
- change: Added low-opacity `.mc-home-dest-orbits` (`orbits.svg`) behind planet/ship in destination card art.
- verified: 2026-10-03T14:32:29-04:00

## verified: fp-welcome-hero-orbits-moon
- screen: welcome
- ref: `.cursor/figma-refs/01-welcome.png`
- deviation: Figma welcome hero card shows orbital ellipses + pink cratered moon around the ringed planet; `.welcome-hero-art` only mounts `ship.svg` + `planet-ringed.svg`.
- fix_hint: Add low-opacity `orbits.svg` behind the planet and `moon.svg` on the orbit path inside `.welcome-hero-art` (keep LTR ship→planet).
- escalate: none
- commit: 64a5890
- change: Mounted `orbits.svg` + `moon.svg` in `.welcome-hero-art` behind LTR ship→planet.
- verified: 2026-10-03T14:32:29-04:00

## verified: fp-active-route-uppercase
- screen: active
- ref: `.cursor/figma-refs/06-mission-active.png`
- deviation: Flight route kicker is title case “Earth → Kepler”; Figma 06 uses all-caps “EARTH → KEPLER”.
- fix_hint: Change `.session-flight-route` copy (or CSS `text-transform: uppercase`) to match Figma casing.
- escalate: none
- commit: 3b12a30
- change: Set `.session-flight-route` copy to “EARTH → KEPLER” (CSS uppercase retained).
- verified: 2026-10-03T14:32:29-04:00

## verified: fp-home-kicker-plain
- screen: home
- ref: `.cursor/figma-refs/02-home.png`
- deviation: Home hero kicker renders “✦ Mission control”; Figma 02 shows plain uppercase “MISSION CONTROL” with no leading star glyph.
- fix_hint: Remove the ✦ span from `.mc-home-kicker` (keep uppercase via existing CSS).
- escalate: none
- commit: 3b12a30
- change: Removed ✦ from `.mc-home-kicker`; plain “Mission control” (uppercase via CSS).
- verified: 2026-10-03T14:32:29-04:00

## verified: fp-first-flight-kicker-plain
- screen: home
- ref: `.cursor/figma-refs/19-first-flight.png`
- deviation: First-flight hero kicker is “✦ YOUR FIRST MISSION”; Figma 19 shows “YOUR FIRST MISSION” without a leading star.
- fix_hint: Drop the ✦ from `.mc-first-flight` / `.mc-kicker--star` on that screen only.
- escalate: none
- commit: 3b12a30
- change: Dropped ✦ / `mc-kicker--star` from first-flight hero kicker; plain “YOUR FIRST MISSION”.
- verified: 2026-10-03T14:32:29-04:00

## verified: fp-session-at-launch-seed
- screen: active
- ref: `.cursor/figma-refs/06-mission-active.png` · live `2:38` · `6:219`/`6:220`
- deviation: Session copilot log empty state is muted helper copy; Figma seeds an “AT LAUNCH” system message stating the mission objective and time limit before any user chat.
- fix_hint: Seed an AT LAUNCH row in `#session-chat-log` when a mission starts (reuse goal + duration); keep empty helper for pre-start if needed.
- escalate: none
- commit: 7edce57
- change: Seed `#session-chat-log` with AT LAUNCH system row (goal + duration) once per mission via `ensureSessionAtLaunchSeed`.
- verified: 2026-10-03T15:27:27-04:00

## verified: fp-welcome-hero-body-plus
- screen: welcome
- ref: `.cursor/figma-refs/01-welcome.png` · live `2:33` · `5:39`
- deviation: Hero body ends with a purple `✦` (`.welcome-hero-star`); Figma ends the same sentence with a trailing “+” after “one step forward.”
- fix_hint: Replace the inline star span with a “+” (or match Figma asset) and tune color/size to the ref.
- escalate: none
- commit: 52749cd
- change: Replaced welcome hero body ✦ with trailing `+` (`.welcome-hero-plus`).
- verified: 2026-10-03T15:27:27-04:00

## verified: fp-settings-topnav-home-active
- screen: settings
- ref: `.cursor/figma-refs/04-settings.png` · live `2:36`
- deviation: On Settings, top nav leaves Home/Copilot/Lock in as plain links and marks “Settings” via `.settings-nav-current`; Figma shows Home on the lavender active pill (Settings is plain text on the right).
- fix_hint: Apply `settings-nav-link--active` to Home when `#view-settings` is active; render Settings as a normal trailing link, not the active pill.
- escalate: none
- commit: f4264a1
- change: Settings top nav marks Home with `settings-nav-link--active`; trailing Settings is plain `.settings-nav-link` (no current pill).
- note: |
    Regressed by 63701ab (“Settings marks Settings active”). Compliance proof-restored Home active /
    Settings plain to match live 2:36 (2026-10-03T15:27). Five-tab aside unchanged.
- verified: 2026-10-03T15:27:27-04:00

## verified: fp-settings-aside-figma-shortcuts
- screen: settings
- ref: `.cursor/figma-refs/04-settings.png`
- deviation: Figma left column is two actions—“Lock in” (lavender) and “Back to home” (outlined)—plus art; `#view-settings` uses a five-tab `.settings-aside-tabs` rail (Lock in, Connection, Permissions, Account, Voice) with no “Back to home” control.
- fix_hint: Product keeps aside tabs — do not remove.
- escalate: none
- note: Intentional product IA (user): aside is settings section tabs, not Lock-in/Back CTAs. Closed won’t-fix vs Figma shortcuts.
- commit: n/a
- change: Kept five-tab Settings rail; do not revert.
- verified: 2026-10-03T14:32:29-04:00

## verified: fp-summary-pb-banner-flag
- screen: summary
- ref: `.cursor/figma-refs/12-quest-complete.png` · live `2:44`/`2:45` · `6:1110`/`6:1212`
- deviation: “New longest flight! …” banner (`#summary-pb-banner`) is text-only from `renderSummary()`; Figma shows a small flag icon ahead of that line in `.quest-complete-pb-banner`.
- fix_hint: Insert the same flag SVG used on home longest-flight (or a Figma asset) inside the banner markup before the dynamic text.
- escalate: none
- commit: f4264a1
- change: `#summary-pb-banner` prepends home longest-flight flag SVG before “New longest flight! …” text.
- verified: 2026-10-03T15:27:27-04:00

## verified: fp-welcome-foot-plus
- screen: welcome
- ref: `.cursor/figma-refs/01-welcome.png` · live `2:33` · `5:57`
- deviation: Below the hero card, Figma shows “YOUR NEXT CHAPTER STARTS HERE” with a small “+” under the line; `.welcome-hero-foot` is plain text with no trailing/under “+”.
- fix_hint: Add a decorative “+” under or after `.welcome-hero-foot` matching the ref spacing.
- escalate: none
- commit: f4264a1
- change: Decorative purple `+` under `.welcome-hero-foot` via `.welcome-hero-foot-plus`.
- verified: 2026-10-03T15:27:27-04:00

## verified: fp-summary-ctas-in-card
- screen: summary
- ref: `.cursor/figma-refs/12-quest-complete.png`
- deviation: Figma places “Start another mission”, “Done”, and the “Mission ended · …” footer inside the cream quest card beneath the copilot note; `#view-summary` keeps `.quest-complete-actions` and `.quest-complete-footer` as siblings after `.quest-complete-stage`, so CTAs sit outside the card chrome.
- fix_hint: Move actions + footer inside `.quest-complete-card` (after note / before closing the card); keep existing button ids/handlers.
- commit: 58eadd7
- change: Moved `.quest-complete-actions` + footer inside `.quest-complete-card`; CTAs sit in a row under the copilot note.
- verified: 2026-10-03T14:10:57-04:00


## verified: fp-summary-flight-details-extra
- screen: summary
- ref: `.cursor/figma-refs/12-quest-complete.png`
- deviation: Quest complete card includes a “Flight details” `<details>` accordion; Figma 12 has no such disclosure between the copilot note and the CTAs.
- fix_hint: Hide the accordion for the default celebration layout (visually-hidden or remove from card flow); keep `#summary-body` available for tests if needed.
- commit: 58eadd7
- change: Hid Flight details `<details>` (`visually-hidden` + `hidden`); `#summary-body` kept in DOM for tests.
- verified: 2026-10-03T14:10:57-04:00


## verified: fp-home-dest-satellite
- screen: home
- ref: `.cursor/figma-refs/02-home.png`
- deviation: Figma dark destination card art includes a satellite near the ship/planet; `.mc-home-dest-art` only had planet, ship, and moon.
- fix_hint: Add `satellite.svg` into `.mc-home-dest-art` with absolute placement matching the ref.
- commit: 58eadd7
- change: Added `.mc-home-dest-satellite` (`satellite.svg`) to home destination card art.
- verified: 2026-10-03T14:10:57-04:00


## verified: fp-session-mic-hint-target
- screen: active
- ref: `.cursor/figma-refs/06-mission-active.png`
- deviation: Session mic feedback (`setComposerMicHint(..., "session")`) rewrites `#session-chat-empty` instead of a dedicated under-composer hint; Figma keeps a stable “Enter to send · Click the microphone…” line under the field.
- fix_hint: Add `#session-chat-hint` under the session composer and point session mic hints there (same pattern as `#chat-hint`).
- commit: 33a98f6
- change: Added `#session-chat-hint` under session composer; session mic feedback targets it instead of empty-state copy.
- verified: 2026-10-03T14:10:57-04:00

## open: fp-break-session-panel
- screen: active
- ref: `.cursor/figma-refs/10---On-a-break.svg`
- deviation: |
    EXPECTED (Figma 10 On a break): progress pill “On a break”; timer caption “REMAINING • TIMER PAUSED”; primary control “Resume mission”; session copilot sidebar replaced by an “ON A BREAK” card (“A little breathing room.” / “Your timer is paused…” / Resume mission / “Your spaceship will continue from right here.”).
    ACTUAL: `updateSessionProgressPill("paused")` sets “On a break” and Pause→“Resume”, but caption stays “REMAINING IN YOUR FLIGHT”, note is visually-hidden, and `.session-copilot-panel` stays the chat composer (no break card).
- fix_hint: |
    When `session.paused`, swap/overlay `.session-copilot-panel` with a Figma break card; set `.session-timer-caption` to “REMAINING • TIMER PAUSED”; keep existing pause invoke / Resume wiring (do not change End mission).
- escalate: none

## open: fp-relaunch-session-panel
- screen: active
- ref: `.cursor/figma-refs/09---Reset-&-relaunch.svg`
- deviation: |
    EXPECTED (Figma 09 Reset & relaunch): left progress pill “→ Ready to relaunch?”; primary row “Take a break” + “End mission”; right sidebar “READY TO RELAUNCH?” with next-step field, Relaunch + Take a break, footer “Same objective. Same flight. A fresh start.”
    ACTUAL: No relaunch UI surface — only a `RELAUNCH_FLAG_KEY` sessionStorage counter for summary stats; copilot sidebar never becomes a relaunch card.
- fix_hint: |
    Add a session-state panel matching Figma 09 (next-step input + Relaunch); wire Relaunch to existing relaunch flag / resume path without changing Pause/End contracts.
- escalate: none

## open: fp-connection-lost-panel
- screen: active
- ref: `.cursor/figma-refs/16---Connection-lost.svg`
- deviation: |
    EXPECTED (Figma 16): session copilot presence becomes “Reconnecting…”; chat area shows a lavender “Connection lost. Your flight keeps going.” callout plus “Retry connection” control above the composer.
    ACTUAL: Session presence is always “Here when you need me”; no connection-lost callout or retry affordance in `#view-session` (connection UI lives only under Settings → Connection).
- fix_hint: |
    On coach/Gemini disconnect during a mission, restyle `.session-copilot-presence` and insert a Figma-styled lost/retry block above the session composer; Retry can re-invoke existing status/connect helpers.
- escalate: none

## open: fp-timer-replace-modal
- screen: active
- ref: `.cursor/figma-refs/15---Timer-replacement.svg`
- deviation: |
    EXPECTED (Figma 15): tapping “Set a five-minute timer” while a next-step timer is already running opens a cream modal (“ONE TIMER AT A TIME” / “Replace your step timer?” / Keep current · Replace timer).
    ACTUAL: `#session-copilot-suggest` → `startNextStepTimer()` silently overwrites the running countdown; no confirm dialog.
- fix_hint: |
    If a next-step timer is active with remaining time, show an MC-styled modal matching Figma 15 before calling `startNextStepTimer(300)`; Keep current dismisses.
- escalate: none

## verified: fp-summary-flight-logged
- screen: summary
- ref: `.cursor/figma-refs/13-flight-logged.png` · live `2:45` · also quest `2:44`
- deviation: |
    EXPECTED (Figma 13/21): non–quest-complete / ended-early celebration uses badge “✦ FLIGHT LOGGED” and title “Every flight moves you forward.” (quest-complete keeps ✓ QUEST COMPLETE / “One mission. Well done.”).
    ACTUAL: `updateSummaryCelebration` only branches first-flight vs quest-complete — every non-first summary shows “✓ QUEST COMPLETE” / “One mission. Well done.” with no flight-logged variant.
- fix_hint: |
    Add a flight-logged celebration branch (badge + title + optional ended-early helper under objective choices) when ending early / not marking quest complete; keep quest-complete + first-flight paths.
- escalate: none
- commit: 7edce57
- change: Early End uses ✦ FLIGHT LOGGED / “Every flight moves you forward.”; timer-complete keeps QUEST COMPLETE; first-flight unchanged.
- verified: 2026-10-03T15:27:27-04:00

## verified: fp-copilot-live-match
- screen: copilot
- ref: `.cursor/figma-refs/03-copilot.png` · live Figma `BR1qUqQ2xrpSCJdLgzhKDI` node `2:35`
- deviation: |
    EXPECTED (live 03 • Copilot): cream pill top nav with Copilot active lavender pill; sidebar without
    ✦ YOUR COPILOT kicker; body “Ask a question, talk through a roadblock…”, divider, CTA, status
    “No mission running”, ringed planet; main title/sub; empty card without ✦ COPILOT kicker; three
    plain chips (no +); Mic + Send circular controls inside cream input pill; two hint lines; corner
    decorative + marks; no astronaut; no outer lavender #app frame.
    ACTUAL: kicker + old sidebar copy; empty kicker; astronaut; chip + on Find a next step; Send
    outside lavender input wrap; nav without cream bar chrome.
- fix_hint: Align `#view-chat` markup + `.view-copilot` CSS to live frame; keep chip/send/mic/nav wiring.
- escalate: scrutinous
- commit: ffaa518
- change: Matched live Figma 03 structure/copy/composer (in-pill Mic+Send), removed kickers/astronaut/chip +, cream nav + sidebar divider + deco pluses.
- verified: 2026-10-03T15:06:26-04:00

## verified: fp-setup-affordance-glyph
- screen: setup
- ref: `.cursor/figma-refs/05-mission-setup.png` · live `2:37`
- deviation: |
    EXPECTED (live Figma 05 objective field trailing control): small dark-purple concentric target /
    radio disc (outer ring + solid center dot), not a star glyph.
    ACTUAL (`#goals-copilot-affordance`): filled purple gradient circle containing a white star glyph.
- fix_hint: |
    Restyle `#goals-copilot-affordance` to a concentric target (CSS rings or tiny SVG). Keep Launch
    wiring / `requestSubmit()` (intentional product: objective control launches — do not reopen as Copilot).
- escalate: scrutinous
- commit: 4a33e9e
- change: Replaced star glyph with `#6750A4` concentric target SVG; Launch `requestSubmit()` wiring unchanged.
- verified: 2026-10-03T15:15:00-04:00

## verified: fp-settings-kicker-period
- screen: settings
- ref: `.cursor/figma-refs/04-settings.png` · live `2:36`
- deviation: |
    EXPECTED (live Figma text `5:306`): intro kicker “MAKE YOURSELF AT HOME” with no trailing period.
    ACTUAL after 4a33e9e: `.settings-kicker` had “Make yourself at home.” (period from bad OCR).
- fix_hint: Keep kicker without trailing period; leave five-tab rail alone.
- escalate: scrutinous
- commit: 4a33e9e
- change: Proof-fixed — removed trailing period so `.settings-kicker` matches live `5:306` (no period).
- verified: 2026-10-03T15:15:00-04:00

## open: fp-gentle-checkin-progress-pill
- screen: active
- ref: `.cursor/figma-refs/08-gentle-check-in.png` · live `2:40`
- deviation: |
    EXPECTED (live 08): while check-in is up, left progress pill becomes “✦ A gentle check-in”
    (not “Mission in progress”); timer caption stays “REMAINING IN YOUR FLIGHT”; Pause/End remain.
    ACTUAL: `#session-progress-label` stays “Mission in progress”; check-in only appears as
    `overlay.html` toast (see also `fp-gentle-checkin-session-panel`).
- fix_hint: |
    When a gentle check-in fires, set `#session-progress-label` to “A gentle check-in” (star via
    existing pill styles); restore “Mission in progress” when check-in dismisses. Keep Pause/End.
- escalate: scrutinous

## open: fp-end-confirm-modal-spec
- screen: active
- ref: `.cursor/figma-refs/11-end-confirmation.png` · live `2:43`
- deviation: |
    EXPECTED (live 11 overlay on active session): cream centered card with badge “END MISSION”,
    title “End this session?”, body “Your {n} earned flight minutes will be saved. You can reflect
    on your objective next.”, secondary “Keep working”, primary “End session”; dimmed scrim over
    the still-visible session chrome.
    ACTUAL (`#end-session` → `window.confirm("End this mission? Screen watching will stop.")`):
    native dialog, wrong copy, no scrim/card.
- fix_hint: |
    Replace `window.confirm` with an MC modal matching live 11 copy/buttons; Keep working dismisses;
    End session keeps existing end invoke. Complements `fp-end-confirm-figma-modal`.
- escalate: scrutinous

## open: fp-gentle-checkin-panel-copy
- screen: active
- ref: `.cursor/figma-refs/08-gentle-check-in.png` · live `2:40`
- deviation: |
    EXPECTED (live 08 right panel): badge “✦ QUICK CHECK-IN”; title “Still working on your mission?”;
    body “Your flight is still running. What would help right now?”; stacked “On task” (primary) /
    “Got distracted” / “Take a break”; footer “No answer needed if you're in the flow. This check-in
    will close on its own.”
    ACTUAL: `overlay.ts` toast shows coach `prompt.text` + kicker only — no three actions, no footer,
    not mounted in `.session-copilot-panel`.
- fix_hint: |
    Swap/overlay `.session-copilot-panel` with the Figma quick-check-in card + exact copy; wire
    buttons to existing coach responses; auto-dismiss per footer. Complements
    `fp-gentle-checkin-session-panel`.
- escalate: scrutinous

## verified: fp-active-next-step-clock
- screen: active
- ref: `.cursor/figma-refs/06-mission-active.png` · live `2:38`
- deviation: |
    EXPECTED (live 06 next-step chrome): lavender pill leads with a small clock/timer icon, then
    uppercase “NEXT STEP TIMER”, then the countdown (e.g. 04:32), with “One small step at a time.”
    beside it.
    ACTUAL (`.session-next-step-pill`): text kicker + time only — no leading clock icon.
- fix_hint: |
    Add a small clock SVG (or unicode) as first child of `.session-next-step-pill` before the kicker;
    keep existing timer wiring.
- escalate: scrutinous
- commit: 4a33e9e
- change: Leading clock SVG in `.session-next-step-pill` before kicker; timer wiring unchanged.
- verified: 2026-10-03T15:15:00-04:00

## verified: fp-setup-objective-compact
- screen: setup
- ref: `.cursor/figma-refs/05-mission-setup.png` · live `2:37` · node `5:421` (Editable field 544×64)
- deviation: |
    EXPECTED (live 05): objective control reads as a single-line compact field (~one text row) with
    the trailing affordance vertically centered on that row.
    ACTUAL (`#goals` textarea `rows="3"` + `.mission-field textarea { min-height: 3.25rem }`): tall
    multi-line box — overshoots Figma field height.
- fix_hint: |
    Drop to `rows="1"` (or 2) and lower min-height so the field matches the compact Figma row; keep
    resize/overflow usable for longer goals.
- escalate: scrutinous
- commit: dd85eb9
- change: `#goals` rows=1; min-height 2.5rem + tighter padding; resize/overflow kept for longer goals.
- verified: 2026-10-03T15:19:56-04:00

## open: fp-break-live-panel-copy
- screen: active
- ref: `.cursor/figma-refs/10-on-a-break.png` · live `2:42`
- deviation: |
    EXPECTED (OCR live 10): progress “On a break”; caption “REMAINING • TIMER PAUSED”; primary
    “Resume mission”; right card badge “ON A BREAK”, title “A little breathing room.”, body
    “Your timer is paused. Automatic check-ins are paused too.”, Resume CTA, footer
    “Your spaceship will continue from right here.”
    ACTUAL: Pause→Resume label flips, but caption stays “REMAINING IN YOUR FLIGHT”, note is
    visually-hidden, `.session-copilot-panel` stays chat (see `fp-break-session-panel`).
- fix_hint: |
    On pause: set caption + swap copilot panel to the break card with exact live copy; Resume keeps
    existing pause invoke. Do not change End mission.
- escalate: scrutinous

## open: fp-listening-session-ui
- screen: active
- ref: `.cursor/figma-refs/14-listening.png` · live `2:46`
- deviation: |
    EXPECTED (live 14 Listening screenshot): `.session-copilot-presence` → “● Listening…”;
    cream composer value/placeholder “Listening… click mic to stop”; in-pill mic becomes
    a ■ stop control beside Send ↑; footer hint stays
    “Enter to send · Click the microphone to start or stop a voice turn.”
    Mid-flight chrome (NEXT STEP TIMER + “Set a five-minute timer”) may remain — not the
    launch-only suggest swap (`fp-session-at-launch-chrome`).
    ACTUAL (`#session-chat-mic`): presence stays “Here when you need me”; mic → “…” and
    `disabled`; hint → “Listening for 4 seconds… speak now.”; `#session-chat-input` never
    shows listening copy. Verified Copilot `fp-copilot-listening-composer` already has ■ +
    listening placeholder — session path was not ported.
- fix_hint: |
    Mirror Copilot listening on session: presence “Listening…”, input listening copy, mic
    ■ (keep enabled to cancel), restore idle presence/placeholder/hint on end. Keep mic invoke.
- escalate: scrutinous

## open: fp-permission-handoff-modal
- screen: overlay
- ref: `.cursor/figma-refs/17-permission-request.png` · live `2:49`
- deviation: |
    EXPECTED (live 17): before OS/browser permission, cream MC modal over “YOUR CHOICE / Fly on
    your own terms.” — badge “PERMISSION HANDOFF”, title “Share an optional input?”, body about
    browser asking for camera/screen, actions “Not now” + “Continue”, footer that next step is
    the native prompt.
    ACTUAL: camera/screen permission goes straight to OS/Tauri prompts (`request_permission_*` /
    capture paths) with no MC handoff modal.
- fix_hint: |
    When enabling camera/screen from setup or settings, show the Figma handoff modal first;
    Continue proceeds to existing permission request; Not now cancels without prompting OS.
- escalate: scrutinous

## open: fp-permission-denied-modal
- screen: overlay
- ref: `.cursor/figma-refs/18-permission-denied.png` · live `2:50`
- deviation: |
    EXPECTED (live 18): after deny, cream modal — badge “INPUT REMAINS OFF”, title “You’re still
    cleared for launch.”, body that permission wasn’t granted / mission works without it, CTAs
    “Back to setup” + “View settings”, footer “Denied access never blocks starting a mission.”
    ACTUAL: no MC denied surface; deny paths leave toggles/status text only (Settings permissions
    list / silent continue on launch).
- fix_hint: |
    On camera/screen deny after handoff Continue, present the Figma denied modal; Back to setup →
    `#view-lockin`; View settings → Permissions tab. Do not block Launch.
- escalate: scrutinous

## verified: fp-session-composer-in-pill
- screen: active
- ref: `.cursor/figma-refs/06-mission-active.png` · `14-listening.png` · live `2:38` · node `6:231`
- deviation: |
    EXPECTED (live 06/14 composer): Mic + Send sit inside one cream input pill (same pattern as
    fixed Copilot 03 — both circular controls in `.copilot-input-wrap`).
    ACTUAL (`#session-chat-form`): `#session-chat-mic` is inside `.copilot-input-wrap`, but
    `#session-chat-send` is a sibling outside the wrap (pre–ffaa518 Copilot layout).
- fix_hint: |
    Move `#session-chat-send` into `.copilot-input-wrap` beside the mic; reuse `.view-copilot`
    in-pill composer CSS for `.session-copilot-composer` / `#view-session`. Keep send/mic wiring.
- escalate: none
- commit: dd85eb9
- change: Moved `#session-chat-send` into cream `.copilot-input-wrap` with Mic; reused Copilot in-pill CSS for session composer.
- verified: 2026-10-03T15:19:56-04:00

## open: fp-relaunch-preserved-banner
- screen: active
- ref: `.cursor/figma-refs/20-relaunched.png` · live `2:52`
- deviation: |
    EXPECTED (live 20 Relaunched): under the mission card, a left-aligned kicker
    “→ Relaunch {n} · all {m} flight minutes preserved”; mission header can show a teal
    “Next step: …” line under the objective.
    ACTUAL: after relaunch, `#view-session` has no preserved-minutes banner and no next-step
    subhead under `#session-goals` (only `RELAUNCH_FLAG_KEY` for summary stats). Distinct from
    open `fp-relaunch-session-panel` (frame 09 pre-relaunch card).
- fix_hint: |
    When resuming from relaunch, mount a `.session-relaunch-banner` under the mission card with
    live copy; optional next-step subline from coach/suggest. Keep Pause/End wiring.
- escalate: scrutinous

## verified: fp-setup-empty-launch-disabled
- screen: setup
- ref: `.cursor/figma-refs/32-objective-empty-disabled.png` · live `10:752`
- deviation: |
    EXPECTED (live 32 Objective empty — disabled): lead “One clear objective. A little room to
    focus.”; objective label without “(optional)”; with empty objective, Launch is a muted grey
    text CTA (not a filled purple button); foot “Type or speak an objective to enable launch.
    Both optional inputs are off.”
    ACTUAL (`#view-lockin`): lead allows blank (“…or leave it blank…”); label has “(optional)”;
    `#lockin-start` is always an enabled filled `.mission-launch` purple button; foot stays
    “You can fly with both inputs off…”.
- fix_hint: |
    Gate `#lockin-start` on non-empty `#goals` (disabled + text-only/muted style when empty);
    swap lead/foot/label to live 32 copy when empty; restore filled Launch + permission foot
    when objective present (live 05/26). Keep objective ✦ Launch wiring; do not reopen as Copilot.
- escalate: scrutinous
- proof: e9a210e — empty gates muted Launch + live 32 lead/label/foot; filled restores Launch + permission foot.

## verified: fp-setup-objective-listening
- screen: setup
- ref: `.cursor/figma-refs/30-objective-listening.png` · live `10:565`
- deviation: |
    EXPECTED (live 30 Objective — listening): objective field shows “Listening… tell me your
    objective.”; foot “Click the microphone again to stop. Your transcript appears here for you
    to edit.”; Launch stays muted until transcript lands.
    ACTUAL: no setup listening state — `#goals` has no listening placeholder/overlay; foot never
    changes; affordance only `requestSubmit()`s Launch (no dictate path). Distinct from open
    `fp-listening-session-ui` (session 14).
- fix_hint: |
    Add a setup dictate path that flips field + `.mission-setup-foot` to live 30 copy while
    listening, then writes transcript into `#goals`. Keep product ✦ = Launch on click if that
    stays intentional (e.g. separate long-press / secondary mic), or document the dictate entry.
- escalate: scrutinous
- proof: e9a210e — dictate path sets live 30 field/foot; Launch muted while listening; affordance Launch/`requestSubmit` when objective present.

## verified: fp-copilot-listening-composer
- screen: copilot
- ref: `.cursor/figma-refs/31-copilot-listening.png` · live `10:667`
- deviation: |
    EXPECTED (live 31 Copilot — listening): cream composer shows “Listening… click mic to stop”
    in the input; mic remains a stoppable control beside Send (in-pill).
    ACTUAL (`#chat-mic`): mic text → “…” and disables; hint becomes “Listening for 4 seconds…”;
    `#chat-input` placeholder/value never shows listening copy. Distinct from
    `fp-listening-session-ui`. Do not reopen verified live-03 chrome (kickers/astronaut/chip+);
    listening variant frame may still carry older chrome — match composer listening only.
- fix_hint: |
    While `chatMicListening` on Copilot: set `#chat-input` listening copy, keep mic enabled as
    stop (■) if cancel is wired, restore idle placeholder on end. Keep send/mic invokes.
- escalate: scrutinous
- commit: 5fdec0d
- change: Copilot mic listening sets `#chat-input` placeholder to “Listening… click mic to stop”, mic shows ■ (stays enabled); idle placeholder/label restored on end; stable hint unchanged.
- verified: 2026-10-03T15:27:27-04:00

## verified: fp-summary-objective-helper
- screen: summary
- ref: `.cursor/figma-refs/07-mission-recap.png` · `21-ended-early-reflection.png` · live `2:39` / `8:439`
- deviation: |
    EXPECTED (live 07/21 pre-answer summary): under Finished / Partly / Not yet, helper line
    “Your answer helps your copilot reflect on this flight. Only you decide whether the quest
    is complete.”
    ACTUAL (`.quest-objective-section`): choices only — no helper copy under the pills.
- fix_hint: |
    Add a muted helper paragraph under `.quest-objective-choices` with the live sentence; hide
    or keep when copilot note appears (see `fp-summary-note-gated`).
- escalate: none
- commit: 5fdec0d
- change: Added `.quest-objective-helper` under choices with live copy; hidden once copilot note appears.
- verified: 2026-10-03T15:27:27-04:00

## verified: fp-summary-note-gated
- screen: summary
- ref: `.cursor/figma-refs/07-mission-recap.png` · `13-flight-logged.png` · live `2:39` / `2:45`
- deviation: |
    EXPECTED: pre-answer (07 Mission recap / 21 reflection) has NO “✦ A NOTE FROM YOUR COPILOT”
    block — only objective choices + helper, then CTAs. After an answer (13 Flight logged with
    Partly selected), the note card appears between choices and CTAs.
    ACTUAL: `.quest-copilot-note` / `#summary-closing` always rendered on `#view-summary`
    (defaults objective to Finished and builds a note immediately).
- fix_hint: |
    Hide `.quest-copilot-note` until the user picks Finished/Partly/Not yet (or treat initial
    state as unanswered); then show note via existing `refreshSummaryCopilotNote`. Keep
    flight-logged vs quest-complete badge branching.
- escalate: scrutinous
- commit: 5fdec0d
- change: Summary starts unanswered (no pill selected); `.quest-copilot-note` hidden until Finished/Partly/Not yet; then `refreshSummaryCopilotNote` reveals it.
- verified: 2026-10-03T15:27:27-04:00


## open: fp-session-at-launch-chrome
- screen: active
- ref: `.cursor/figma-refs/36-launch-25-remaining.png` · live `10:1325`
- deviation: |
    EXPECTED (live 36 Launch — 25 minutes remaining): timer 25:00, “0 of 25 flight minutes
    earned”, Pause + End mission present; `#session-next-step` row ABSENT; suggest chip
    “How much time is left?”; AT LAUNCH seed + idle composer (Mic+↑) unchanged. Mid-flight
    frames (06/33/29/14/20) keep “Set a five-minute timer” + NEXT STEP TIMER once a step
    timer exists (or after suggest starts one).
    ACTUAL (`#view-session` / `index.html`): `#session-next-step` always mounts with default
    05:00; `#session-copilot-suggest` hard-coded “Set a five-minute timer” from launch.
- fix_hint: |
    At launch / before a next-step timer starts: hide `#session-next-step`; set suggest
    label to “How much time is left?” (reply with remaining flight time or send that
    prompt). After `startNextStepTimer`, show next-step row and restore five-minute suggest.
    Keep Pause/End / mic / send / AT LAUNCH seed wiring.
- escalate: scrutinous

## verified: fp-ended-early-finished-hero
- screen: summary
- ref: `.cursor/figma-refs/22-ended-early-finished.png` · live `8:531`
- deviation: |
    EXPECTED (live 22 Ended early — finished): after early End + selecting Finished, hero
    becomes ✓ QUEST COMPLETE / “One mission. Well done.” (layer name still “FLIGHT LOGGED”
    but visible text is quest-complete). Not-yet/partly keep ✦ FLIGHT LOGGED /
    “Every flight moves you forward.” (live 24).
    ACTUAL: `updateSummaryCelebration(..., endedEarly=true)` locks FLIGHT LOGGED /
    “Every flight…” for the whole summary; picking Finished never upgrades the badge/title.
- fix_hint: |
    When `lastSummaryObjective === "finished"`, restyle celebration to quest-complete
    (✓ QUEST COMPLETE + “One mission. Well done.”) even if the session ended early;
    keep flight-logged hero for partly/not-yet. Distinct from verified initial
    flight-logged branch on End.
- escalate: scrutinous
- commit: ca08f47
- change: Early-End + Finished upgrades hero to ✓ QUEST COMPLETE / “One mission. Well done.” via `syncSummaryCelebrationFromOutcome`; partly/not-yet keep FLIGHT LOGGED.
- verified: 2026-10-03T15:41:44-04:00

## verified: fp-summary-outcome-note-copy
- screen: summary
- ref: `.cursor/figma-refs/22-ended-early-finished.png` · `24-ended-early-not-yet.png` · live `8:531` / `8:715`
- deviation: |
    EXPECTED (live 22/24 after an objective pick): plain muted paragraph under the pills —
    no “✦ A note from your copilot” kicker. Finished: “You logged {n} flight minutes and
    said you finished {goal}. Your personal best remains {pb} minutes.” Not yet: “You
    logged {n} flight minutes and said your objective is not finished yet. Your time
    still counts, and your personal best remains {pb} minutes.”
    ACTUAL: `.quest-copilot-kicker` always shows; `buildCopilotNote` uses
    “finished your objective — {goal}” / “didn't finish your objective yet” and may append
    `closing_note` instead of the “personal best remains / time still counts” lines.
- fix_hint: |
    Hide `.quest-copilot-kicker` (or drop it); rewrite `buildCopilotNote` / outcome phrases
    to match live 22/24 sentences (include PB remains / time still counts). Keep
    note-gated visibility from verified `fp-summary-note-gated`.
- escalate: scrutinous
- commit: ca08f47
- change: Hid copilot-note kicker; plain muted note; `buildCopilotNote` matches live 22/24 PB-remains / time-still-counts sentences.
- verified: 2026-10-03T15:41:44-04:00

## verified: fp-summary-logged-banner
- screen: summary
- ref: `.cursor/figma-refs/22-ended-early-finished.png` · `24-ended-early-not-yet.png` · live `8:531` / `8:715`
- deviation: |
    EXPECTED: under the three stat tiles, a lavender pill always shows
    “{n} minutes logged · your best is still {pb} min” (or the new-longest variant when
    applicable — Figma samples use the “still” line when PB unchanged).
    ACTUAL (`#summary-pb-banner`): only rendered when `pb.isNew && pb.previous > 0` with
    “New longest flight! X → Y min”; otherwise hidden — non-PB summaries miss the logged
    / best-still line entirely.
- fix_hint: |
    Always show `#summary-pb-banner` after a mission: new-PB copy when `pb.isNew`, else
    “{n} minutes logged · your best is still {pb} min” (keep flag SVG for new-PB if Figma
    12 uses it). Distinct from verified flag-only new-PB mount.
- escalate: none
- commit: ca08f47
- change: `#summary-pb-banner` always shown — new-longest with flag when PB beaten, else “{n} minutes logged · your best is still {pb} min”.
- verified: 2026-10-03T15:41:44-04:00

## open: fp-screen-share-permission-modal
- screen: overlay
- ref: `.cursor/figma-refs/27-screen-sharing-permission.png` · live `8:1019`
- deviation: |
    EXPECTED (live 27 Screen-sharing permission): cream MC modal over “YOUR CHOICE /
    Fly on your own terms.” — badge “PERMISSION HANDOFF”, title “Share your screen?”,
    body about choosing a tab/window/screen and comparing visible activity to the
    objective, “Not now” + “Continue”, footer that nothing is recorded / can turn off.
    ACTUAL: no screen-specific handoff; screen enable goes straight to OS/Tauri share
    picker. Open `fp-permission-handoff-modal` covers the generic “Share an optional
    input?” camera-style frame 17 — this is the screen-share variant (frame 27).
- fix_hint: |
    When enabling screen sharing from setup/settings, show this screen-specific modal
    copy before the native picker; Continue → existing screen permission path; Not now
    cancels. Reuse handoff shell from `fp-permission-handoff-modal` with frame-27 strings.
- escalate: scrutinous

## verified: fp-summary-partly-note-copy
- screen: summary
- ref: `.cursor/figma-refs/23-ended-early-partly.png` · live `8:623` · note node `8:692`
- deviation: |
    EXPECTED (live 23 / design_context `8:692`, Partly selected, relaunch=0): plain muted
    14px paragraph under the pills — NO ✦ kicker, NO lavender card —
    “You logged 12 flight minutes and said you partly finished. Your time counts.
    Choose one small next step when you return.”
    (template: “You logged {n} flight {minute|minutes} and said you partly finished.
    Your time counts. Choose one small next step when you return.” — no goal fragment,
    no “still”, no PB-remains clause).
    ACTUAL (`buildCopilotNote` partly): “You logged {n} flight minutes and said you
    partly finished {goal}. Your time still counts, and your personal best remains
    {pb} minutes.” Wrong closer vs live 23. Do not reopen verified ca08f47 22/24 strings.
- fix_hint: |
    Specialize partly branch to the live 23 sentence (drop `{goal}` + PB-remains; use
    “Your time counts. Choose one small next step when you return.”). Keep
    `.quest-copilot-kicker` hidden on this zero-relaunch path.
- escalate: scrutinous
- commit: 5f9b96a
- change: `buildCopilotNote` partly uses live 23 sentence (no goal/PB closer; “Your time counts. Choose one small next step when you return.”).
- verified: 2026-10-03T16:06:11-04:00

## verified: fp-summary-relaunch-note
- screen: summary
- ref: `.cursor/figma-refs/25-flight-logged-not-yet.png` · live `8:817` · card `8:886` ·
  kicker `8:887` · body `8:888`
- deviation: |
    EXPECTED (live 25, Not yet + relaunch≥1): `.quest-copilot-note` becomes lavender
    bordered “Copilot summary” card (`8:886`) with teal kicker `✦  A NOTE FROM YOUR
    COPILOT` (`8:887`) and body (`8:888`):
    “You logged 25 flight minutes and relaunched once. You said your objective is not
    finished yet. Your time still counts; pick one small step for your next flight.”
    (pluralize “once” / “N times”). Distinct from plain muted 22/23/24 notes.
    ACTUAL: `buildCopilotNote` ignores `_relaunches`; kicker stays `hidden`; note has
    no card chrome (transparent / no border) — always the zero-relaunch plain style.
- fix_hint: |
    When relaunches>0 on flight-logged/not-yet (and matching 13): unhide teal kicker,
    apply card modifier, rewrite note to relaunch sentence. Keep plain muted notes for
    zero-relaunch 22/23/24. Complements open `fp-summary-relaunch-note-card`.
- escalate: scrutinous
- commit: c5010ec
- change: Not yet + relaunches>0 uses live 25 relaunch sentence + teal kicker; zero-relaunch 22/23/24 plain notes unchanged.
- verified: 2026-10-03T15:52:00-04:00

## verified: fp-summary-pb-stat-label
- screen: summary
- ref: `.cursor/figma-refs/23-ended-early-partly.png` · live `8:623` · tiles `8:681`–`8:683`
- deviation: |
    EXPECTED (live 23 design_context when PB unchanged): peach tile value “20 min”
    (`8:682`) + label “personal best” (`8:683`). Live 25 new-PB keeps “+5 min” /
    “new personal best”.
    ACTUAL (`renderSummary`): non-new label is “longest flight” (value
    `Math.max(flightMinutes, pb.previous)` OK when pb > flight).
- fix_hint: |
    Change non-new third-tile label to “personal best”; keep new-PB “+N min” /
    “new personal best”. Do not reopen always-on `#summary-pb-banner` (ca08f47).
- escalate: none
- commit: 5f9b96a
- change: Non-new third-tile label is “personal best”; new-PB “new personal best” unchanged.
- verified: 2026-10-03T16:06:11-04:00

## verified: fp-active-signal-on-fill
- screen: active
- ref: `.cursor/figma-refs/33-active-audio-on.png` · `34-active-camera-on.png` ·
    `35-active-screen-shared.png` · live `10:852` / `10:1009` / `10:1167`
- deviation: |
    EXPECTED (live 33/34/35): the active signal pill (“Audio on” / “Camera on” /
    “Screen on”) is a solid dark-purple filled pill with white label (~115px);
    inactive siblings stay cream/lavender outlined (“… off”).
    ACTUAL (`.session-signal-pill.is-on`): light lavender tint + darker text — reads as
    soft highlight, not the filled primary-on control in Figma.
- fix_hint: |
    Restyle `.session-signal-pill.is-on` to solid purple fill + white text (match Pause
    primary); keep Camera/Screen/Audio order and existing toggle wiring.
- escalate: none
- commit: 5f9b96a
- change: `.session-signal-pill.is-on` uses solid `--mc-accent-purple` fill + white text.
- verified: 2026-10-03T16:06:11-04:00

## verified: fp-summary-relaunch-note-card
- screen: summary
- ref: `.cursor/figma-refs/25-flight-logged-not-yet.png` · live `8:817` · node `8:886`
- deviation: |
    EXPECTED (live 25 Copilot summary `8:886`): when the relaunch note shows, mount is a
    lavender raised card with border, 20px padding, gap, teal uppercase kicker, then body.
    ACTUAL: `.quest-copilot-note` is `background: transparent; border: none; padding: 0`
    (styled for plain 22/24 notes). Complements `fp-summary-relaunch-note` copy/kicker.
- fix_hint: |
    Add a modifier (e.g. `.quest-copilot-note--card`) applied when relaunches>0 that
    restores lavender card chrome; leave default transparent for plain outcome notes.
- escalate: none
- commit: c5010ec
- change: `.quest-copilot-note--card` lavender chrome (20px pad, border, gap) when Not yet + relaunches>0; proof-tuned to live 8:886 tokens (#e9ddfd / #c8bfd7 / 28px / 10px gap / mint kicker).
- verified: 2026-10-03T15:52:00-04:00

## verified: fp-summary-partly-relaunch-note
- screen: summary
- ref: `.cursor/figma-refs/13-flight-logged.png` · live `2:45` · note `6:1224`/`6:1226`
- deviation: |
    EXPECTED (live 13, Partly selected + relaunch≥1): lavender Copilot summary card with teal
    kicker `✦  A NOTE FROM YOUR COPILOT` and body
    “You logged {n} flight minutes and relaunched {once|N times}. You said there’s more to
    do—and your time still counts. Pick up with one small step next time.”
    Distinct from verified zero-relaunch partly (23 / `fp-summary-partly-note-copy`) and from
    verified Not-yet relaunch (25 / `fp-summary-relaunch-note`).
    ACTUAL: `outcome === "partly"` always returns the plain 23 sentence (ignores `relaunches`);
    `summaryUsesRelaunchNote` is Not-yet-only so card chrome never applies on Partly+relaunch.
- fix_hint: |
    When partly + relaunches>0, use live 13 sentence + `--card` + teal kicker; keep zero-relaunch
    partly as plain 23. Do not reopen verified 25 not-yet strings.
- escalate: scrutinous
- commit: 6ee9efc
- change: Partly + relaunches>0 uses live 13 sentence + card/kicker; zero-relaunch partly stays plain 23.
- verified: 2026-10-03T16:01:56-04:00

## verified: fp-summary-finished-relaunch-note
- screen: summary
- ref: `.cursor/figma-refs/12-quest-complete.png` · live `2:44` · note `6:1123`/`6:1125`
- deviation: |
    EXPECTED (live 12, Finished selected + relaunch≥1 + new PB): lavender card + teal kicker and
    body “You logged {n} flight minutes, relaunched {once|N times}, and said you finished
    {goal}. That’s {delta} minutes beyond your previous longest flight.”
    (sample: problems 1–5 / 5 minutes). Zero-relaunch Finished stays plain 22 PB-remains note
    (verified `fp-summary-outcome-note-copy`).
    ACTUAL: Finished branch always uses “finished {goal}. Your personal best remains {pb}
    minutes.” — no relaunch clause, no “beyond your previous longest”, no card chrome.
- fix_hint: |
    Specialize Finished+relaunches>0 (optionally when `pb.isNew`) to live 12 sentence + card
    chrome; leave zero-relaunch Finished plain. Complements open partly-relaunch; leave 25 alone.
- escalate: scrutinous
- commit: 6ee9efc
- change: Finished + relaunches>0 + new PB uses live 12 sentence + card/kicker; zero-relaunch Finished stays plain 22.
- verified: 2026-10-03T16:01:56-04:00

## verified: fp-quest-complete-badge-mint
- screen: summary
- ref: `.cursor/figma-refs/12-quest-complete.png` · live `2:44` · badge `6:1097`
- deviation: |
    EXPECTED (live 12): `✓  QUEST COMPLETE` pill label color is mint/teal `#326c78` on lavender
    raised fill (same mint token as Copilot-note kickers). Live 13 `✦  FLIGHT LOGGED` stays
    amber/purple `#6750a4`.
    ACTUAL (`.view-quest-complete .quest-complete-badge`): both celebration badges use
    `color: var(--mc-accent-purple)` — quest-complete never switches to mint.
- fix_hint: |
    When badge is QUEST COMPLETE (`.mission-celebration--quest` / sync path), set badge text
    color to `#326c78`; keep FLIGHT LOGGED on purple/amber. No copy/wiring changes.
- escalate: none
- commit: fbe9617
- change: `.mission-celebration--quest .quest-complete-badge` uses mint `#326c78`; FLIGHT LOGGED stays purple.
- verified: 2026-10-03T15:59:00-04:00

## verified: fp-first-flight-badge-mint
- screen: home
- ref: `.cursor/figma-refs/19-first-flight.png` · live `2:51` · badge `7:565`
- deviation: |
    EXPECTED (live 19): card pill `✦  FIRST FLIGHT` uses mint label `#326c78` on lavender
    `#e9ddfd` fill (same mint as setup/welcome kickers).
    ACTUAL (`.mc-first-flight-badge`): `color: var(--mc-accent-purple)` — purple label, not mint.
    (Teal body line `.mc-first-flight-card-teal` is already OK.)
- fix_hint: |
    Set `.mc-first-flight-badge` color to `#326c78`; keep CTA/copy/wiring. Distinct from verified
    plain kicker `fp-first-flight-kicker-plain`.
- escalate: none
- commit: fbe9617
- change: `.mc-first-flight-badge` label color set to mint `#326c78`.
- verified: 2026-10-03T15:59:00-04:00

## verified: fp-setup-step-mint
- screen: setup
- ref: `.cursor/figma-refs/26-setup-camera-allowed.png` · `05-mission-setup.png` · live `8:919` /
  `2:37` · step `8:967`
- deviation: |
    EXPECTED (live 05/26): `01 / PREPARE FOR LAUNCH` step pill text is mint `#326c78` on
    lavender raised fill.
    ACTUAL (`.mission-setup-step`): mauve `#5c5478` — wrong token vs live mint kickers.
- fix_hint: |
    Change `.mission-setup-step` color to `#326c78` (match welcome/first-flight mint pills).
    Leave toggles / Launch / intentional objective ✦ wiring alone.
- escalate: none
- commit: fbe9617
- change: `.mission-setup-step` color set to mint `#326c78`.
- verified: 2026-10-03T15:59:00-04:00

## open: fp-welcome-signin-form
- screen: welcome
- ref: `.cursor/figma-refs/01-welcome.png` · live `2:33` · card `5:58` · fields `5:63`/`5:67` ·
  CTA `5:71` · lead `5:62` · foot `5:77`
- deviation: |
    EXPECTED (live 01 Sign in card): lead “Sign in to return to your space.”; Email field
    (placeholder `you@school.edu`) + Password field (`Enter your password`); primary
    “Sign in →”; “Continue as guest”; foot “Just here to focus? Guest mode has everything
    you need for your first mission.”
    ACTUAL (`#welcome-signin-form`): Google-only CTA (“Sign in with Google →”), lead about
    Calendar/Drive sync, no Email/Password fields, foot “Guest mode stays on this device
    only — no cloud sync. Accounts always use Google.”
- fix_hint: |
    Match live card chrome/copy (lead, Email/Password placeholders, “Sign in →”, guest foot).
    Keep Google auth wired — e.g. Sign in → still invokes `sign_in_waypoint_google` if email
    auth isn’t real yet; do not drop guest path.
- escalate: scrutinous

## verified: fp-welcome-signin-kicker-mint
- screen: welcome
- ref: `.cursor/figma-refs/01-welcome.png` · live `2:33` · kicker `5:59`/`5:60`
- deviation: |
    EXPECTED (live 01): `✦  WELCOME ABOARD` pill label is mint `#326c78` on lavender
    `#e9ddfd` fill (same mint token as setup/first-flight kickers).
    ACTUAL (`.welcome-signin-kicker`): `color: #6b4cff` on translucent purple tint —
    purple label, not mint. Distinct from verified quest/first-flight/setup mint (fbe9617).
- fix_hint: |
    Set `.welcome-signin-kicker` color to `#326c78` and raised fill `#e9ddfd`; leave form
    wiring / Google path alone (see `fp-welcome-signin-form`).
- escalate: none
- commit: dbc97fc
- change: `.welcome-signin-kicker` mint `#326c78` on raised `#e9ddfd` fill.
- verified: 2026-10-03T16:10:00-04:00

## open: fp-session-latest-response-card
- screen: active
- ref: `.cursor/figma-refs/06-mission-active.png` · `29-secondary-timer-replaced.png` ·
    live `2:38` / `8:1198` · Latest response `6:224` / `8:1320`
- deviation: |
    EXPECTED (live 06/29): assistant reply is a lavender raised card with border (`#e9ddfd` /
    `#c8bfd7`), ~18px padding, title line (18px medium dark) + body (14px muted) — NO
    “COPILOT · JUST NOW” meta above it (user/system keep their meta kickers).
    ACTUAL (`appendSessionChat("assistant")`): always mounts `COPILOT · JUST NOW` + a single
    `.bubble.assistant` block (lavender fill, no border, no title/body split).
- fix_hint: |
    For session assistant turns: omit/hide meta kicker; render `.session-latest-response`
    (or split first sentence → title, rest → body) matching live card tokens. Keep send/mic /
    AT LAUNCH / Pause/End wiring.
- escalate: scrutinous

## verified: fp-setup-toggle-row-chrome
- screen: setup
- ref: `.cursor/figma-refs/28-setup-screen-shared.png` · `05-mission-setup.png` · live
    `8:1096` / `2:37` · rows `8:1159`/`8:1164`
- deviation: |
    EXPECTED (live 05/28): Camera / Screen rows are plain stacked copy + toggle (no per-row
    fill/border); only the card itself is cream.
    ACTUAL (`.mission-toggle-row`): each row is a padded lavender mini-card
    (`background: rgba(240,235,250,0.55)` + border + radius) — heavier than live.
- fix_hint: |
    Flatten `.mission-toggle-row` to transparent/no-border list rows; keep toggle wiring and
    intentional objective ✦ = Launch. Distinct from open screen-permission modal.
- escalate: none
- commit: dbc97fc
- change: Flattened `.mission-toggle-row` to transparent/no-border list rows.
- verified: 2026-10-03T16:10:00-04:00

## verified: fp-session-user-turn-align
- screen: active
- ref: `.cursor/figma-refs/06-mission-active.png` · `29-secondary-timer-replaced.png` ·
    live `2:38` / `8:1198` · YOU meta `6:222` / `8:1318`
- deviation: |
    EXPECTED (live 06/29): session chat column is left-aligned throughout — “YOU · JUST NOW”
    and user body sit on the left under AT LAUNCH, same edge as the latest-response card.
    ACTUAL (`.session-chat-turn--user { align-items: flex-end }`): user meta + body hug the
    right edge of the copilot panel.
- fix_hint: |
    Left-align `.session-chat-turn--user` (remove flex-end); keep transparent user text style
    and existing chat wiring. Complements `fp-session-latest-response-card`.
- escalate: none
- commit: dbc97fc
- change: `.session-chat-turn--user` left-aligned (`flex-start`).
- verified: 2026-10-03T16:10:00-04:00

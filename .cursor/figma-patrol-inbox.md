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


## verified: fp-end-confirm-figma-modal
- screen: active
- ref: `.cursor/figma-refs/11---End-confirmation.svg`
- deviation: `#end-session` uses `window.confirm("End this mission?…")`; Figma 11 is a designed End confirmation surface (cream modal / screen chrome), not a native browser dialog.
- fix_hint: Add a small MC-styled confirm dialog matching Figma 11; keep Pause/End wiring, only replace the confirm UI.
- escalate: none
- commit: 10bb695
- change: |
    MC `#end-session-modal` (badge END MISSION / Keep working / End session)
    replaces native `window.confirm`.
- verified: 2026-10-03T17:52:09-04:00


## verified: fp-gentle-checkin-session-panel
- screen: active
- ref: `.cursor/figma-refs/07-gentle-reminder-temp.png` · `08---Gentle-check-in.svg`
- deviation: Figma replaces the session copilot sidebar with a “✦ QUICK CHECK-IN” card (Still working… + On task / Got distracted / Take a break). App uses a separate toast overlay (`overlay.html`) without those three actions in the session layout.
- fix_hint: When a check-in fires, swap/overlay the session copilot panel with the Figma quick-check-in card; wire buttons to existing coach responses.
- escalate: none
- commit: 10bb695
- change: |
    Session copilot column swaps to `#session-checkin-card` on overlay-prompt;
    three CTAs wired; chat restored on dismiss.
- verified: 2026-10-03T17:52:09-04:00


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

## verified: fp-break-session-panel
- screen: active
- ref: `.cursor/figma-refs/10-on-a-break.png` · live `2:42`
- deviation: |
    EXPECTED (Figma 10 On a break): progress pill “On a break”; timer caption “REMAINING • TIMER PAUSED”; primary control “Resume mission”; session copilot sidebar replaced by an “ON A BREAK” card (“A little breathing room.” / “Your timer is paused…” / Resume mission / “Your spaceship will continue from right here.”).
    ACTUAL: `updateSessionProgressPill("paused")` sets “On a break” and Pause→“Resume”, but caption stays “REMAINING IN YOUR FLIGHT”, note is visually-hidden, and `.session-copilot-panel` stays the chat composer (no break card).
- fix_hint: |
    When `session.paused`, swap/overlay `.session-copilot-panel` with a Figma break card; set `.session-timer-caption` to “REMAINING • TIMER PAUSED”; keep existing pause invoke / Resume wiring (do not change End mission).
- escalate: none
- commit: a79a5ce
- change: |
    Compact `#session-break-card` + “Resume mission” + hidden pause note via
    `is-session-break` (see sibling verified break items). Pill/caption via
    earlier b7de340 (`Ⅱ  On a break` / `REMAINING · TIMER PAUSED`).
- verified: 2026-10-03T17:33:30-04:00
- note: |
    Live `2:42`/`6:828` match HEAD a79a5ce + b7de340. Stale open claimed no
    break card / “Resume” only — superseded by verified compact/copy/label items.
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

## verified: fp-timer-replace-modal
- screen: active
- ref: `.cursor/figma-refs/15---Timer-replacement.svg`
- deviation: |
    EXPECTED (Figma 15): tapping “Set a five-minute timer” while a next-step timer is already running opens a cream modal (“ONE TIMER AT A TIME” / “Replace your step timer?” / Keep current · Replace timer).
    ACTUAL: `#session-copilot-suggest` → `startNextStepTimer()` silently overwrites the running countdown; no confirm dialog.
- fix_hint: |
    If a next-step timer is active with remaining time, show an MC-styled modal matching Figma 15 before calling `startNextStepTimer(300)`; Keep current dismisses.
- escalate: none
- commit: 62f7171
- change: |
    Suggest chip opens `#timer-replace-modal` when a next-step timer is
    already active; Keep current dismisses; Replace timer restarts 05:00.
- verified: 2026-10-03T18:13:56-04:00

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
- ref: `.cursor/figma-refs/08-gentle-check-in.png` · live `2:40` ·
  pill `6:403` / label `6:404` (was stale `6:421` star ellipse)
- expected: |
    Live 08 Flight status pill (`6:403`, 132×31) reads “✦  A gentle
    check-in”; caption stays “REMAINING IN YOUR FLIGHT”; Pause/End remain.
- actual: |
    Session check-in card/CTA/compact chrome verified (10bb695), but
    `updateSessionProgressPill` never sets a check-in label — still
    “Mission in progress” while `is-session-checkin`. Overlay toast may also
    show; left pill does not flip.
- deviation: |
    Progress pill copy vs live `6:404` during check-in. Distinct from
    verified panel/CTA/compact opens — only the left pill remains.
- fix_hint: |
    In `setSessionCheckinUi(true)`, set `#session-progress-label` to
    “A gentle check-in” (✦ via pill styles); restore “Mission in progress”
    on dismiss/auto-close. Keep Pause/End.
- escalate: scrutinous

## verified: fp-end-confirm-modal-spec
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
- commit: 10bb695
- change: |
    Live 11 copy + CTA hierarchy in `#end-session-modal`; earned minutes interpolated.
- verified: 2026-10-03T17:52:09-04:00


## verified: fp-gentle-checkin-panel-copy
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
- commit: 10bb695
- change: |
    Exact live 08 badge/title/body/footer copy on `#session-checkin-card`.
- verified: 2026-10-03T17:52:09-04:00


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

## verified: fp-break-live-panel-copy
- screen: active
- ref: `.cursor/figma-refs/10-on-a-break.png` · live `2:42` · badge `8:431` ·
  title `8:432` · body `8:433` · CTA `8:435` · foot `8:438`
- deviation: |
    EXPECTED (live 10 right card): badge “Ⅱ  ON A BREAK” (`8:431`); title “A little
    breathing room.”; body “Your timer is paused. Automatic check-ins are paused
    too.”; primary “Resume mission”; footer “Your spaceship will continue from
    right here.” Left primary is also “Resume mission” (`6:819`).
    PARTIAL (b7de340 verified): progress pill already “Ⅱ  On a break”; caption
    already “REMAINING · TIMER PAUSED”; next-step + suggest hidden
    (`fp-break-hides-session-chrome`).
    ACTUAL: `.session-copilot-panel` stays chat — no break card; Pause button
    label is “Resume” (not “Resume mission”).
- fix_hint: |
    Swap `.session-copilot-panel` to the break card with exact live copy + “Ⅱ  ”
    badge; optionally align Pause→“Resume mission”. Keep End + pause invoke.
    Complements `fp-break-session-panel`.
- escalate: scrutinous
- commit: a79a5ce
- change: |
    `#session-break-card` uses live badge/title/body/divider/Resume/footer;
    left `#session-pause` → “Resume mission” while paused.
- verified: 2026-10-03T17:33:30-04:00
- note: |
    Live get_screenshot `6:828` / metadata `8:431`–`8:438` + `6:819` match HEAD.
    Stale ACTUAL (no card / “Resume”) rewritten — now verified with compact siblings.
## open: fp-listening-session-ui
- screen: active
- ref: `.cursor/figma-refs/14-listening.png` · live `2:46` · presence `6:1373` ·
  composer `6:1388` · stop `6:1389`
- deviation: |
    EXPECTED (live 14 get_screenshot tick11): presence “●  Listening…”; cream
    composer “Listening… click mic to stop”; in-pill mic → ■ stop (`6:1389`)
    beside Send ↑; footer hint stays “Enter to send · Click the microphone…”.
    Suggest “Set a five-minute timer” stays (`6:1383`). NEXT STEP TIMER absent
    (verified `fp-listening-hides-next-step`).
    ACTUAL after c00ab73: session Live uses Talk label + phase fills
    (“Live”/“…”) via `applyMicLiveUi(..., "Talk")`; presence stays idle;
    `#session-chat-input` never gets listening placeholder; STT path still
    flashes “Listening for 4 seconds…”. Copilot path now verified tonal “Mic”
    + listening placeholder (`fp-copilot-mic-live-phase-chrome`) — session 14
    still needs ■ stop chrome, not Copilot’s sticky Mic.
- fix_hint: |
    On session listen: presence “●  Listening…”, input listening copy, mic ■
    (enabled to cancel), restore idle on end. Keep mic invoke + suggest.
    Do not re-show next-step while listening. Do not copy Copilot sticky Mic.
- escalate: scrutinous

## verified: fp-permission-handoff-modal
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
- commit: b7ea57b
- change: |
    Thin `#permission-handoff-modal` shell before OS camera prompt when enabling
    `#lockin-camera` / `#setting-camera-signals`; Continue → `request_camera_permission`;
    Not now cancels. Camera-specific live copy (not stale layer name). YOUR CHOICE page
    still open (`fp-permissions-choice-surface`).
- verified: 2026-10-03T17:05:46-04:00

## verified: fp-permission-denied-modal
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
- commit: b7ea57b
- change: |
    Thin `#permission-denied-modal` after handoff Continue deny; Back to setup →
    `#view-lockin`; View settings → Permissions tab; Launch never blocked.
- verified: 2026-10-03T17:05:46-04:00

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
- expected: |
    Live 01 Sign in card: lead “Sign in to return to your space.”; Email
    (`you@school.edu`) + Password (`Enter your password`); primary
    “Sign in →”; “Continue as guest”; foot “Just here to focus? Guest
    mode has everything you need for your first mission.”
- actual: |
    Tick14: card stack + labeled-field tokens + CTA solid/height 52
    (`fp-welcome-signin-cta-height` / `9681808`) + foot muted verified.
    Form still Google-only — no Email/Password fields; lead
    Calendar/Drive sync; CTA “Sign in with Google →”; foot
    Google-approval sentence.
- deviation: |
    Form structure/copy vs live `5:58` (fields + lead + CTA/foot strings).
    Distinct from verified guest Quiet / title Medium 32 / CTA solid /
    CTA height 52 / foot muted / card-stack-gap / labeled-field-stack.
- fix_hint: |
    Match live card chrome/copy (lead, Email/Password placeholders,
    “Sign in →”, guest foot). Keep Google auth wired — e.g. Sign in →
    still invokes `sign_in_waypoint_google` if email auth isn’t real yet;
    do not drop guest path.
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

## verified: fp-home-footnote-rail
- screen: home
- ref: `.cursor/figma-refs/02-home.png` · live `2:34` · footnote `5:169` · satellite `22:1311`
- deviation: |
    EXPECTED (live 02): “No streaks to protect. Every flight counts.” sits in the right
    rail under the personal-best column (x≈969, y≈750) with the decorative satellite
    below it — beside the copilot invitation card, not under it.
    ACTUAL (`.mc-home-bottom`): footnote is a full-width row under the entire
    `.mc-home-grid` (left-aligned), so it drops below the copilot card instead of
    occupying the PB/right gutter.
- fix_hint: |
    Place `.mc-home-footnote` (+ bottom satellite) in the right column under
    `.mc-home-best` (same vertical band as the copilot card). Keep CTA / Open copilot
    wiring. Distinct from dest-card satellite.
- escalate: none
- commit: 43e68a5
- change: Moved `.mc-home-bottom` into grid col 2 / row 2 under `.mc-home-best` (footnote + satellite).
- verified: 2026-10-03T16:25:30-04:00

## verified: fp-home-copilot-shortcut-row
- screen: home
- ref: `.cursor/figma-refs/02-home.png` · live `2:34` · shortcut `5:164` · copy `5:165`
- deviation: |
    EXPECTED (live 02 Copilot invitation): title “Need a hand finding your next step?”
    alone on the first row; second row is a horizontal shortcut — muted
    “Talk it through with your copilot.” + CTA on the same baseline (`5:164`).
    ACTUAL: “Talk it through…” is `.mc-home-copilot-sub` stacked under the `<h2>` in
    `.mc-home-copilot-copy`, so the muted line sits in the title stack rather than the
    shortcut row with the button.
- fix_hint: |
    Restructure `.mc-home-copilot`: title full-width; put Talk-it-through text and
    `#home-copilot-cta` in one flex row matching `5:164`. Keep Open-copilot → show chat.
- escalate: none
- commit: 43e68a5
- change: Title alone on first row; Talk-it-through + `#home-copilot-cta` in `.mc-home-copilot-shortcut` row (`5:164`). CTA remains “Open copilot  →”.
- verified: 2026-10-03T16:25:30-04:00

## verified: fp-home-copilot-cta-arrow
- screen: home
- ref: `.cursor/figma-refs/02-home.png` · live `2:34` · CTA `5:166`
- deviation: |
    EXPECTED (live 02): copilot shortcut CTA label is “Open copilot  ↗”
    (northeast arrow glyph).
    ACTUAL (`renderHome` → `#home-copilot-cta`): “Open copilot →” (right arrow).
- fix_hint: |
    Change the button label to “Open copilot  ↗” (match live spacing/glyph). Keep
    `show("view-chat")` wiring. Complements `fp-home-copilot-shortcut-row`.
- escalate: none
- commit: 5873ecf
- change: Home copilot CTA label matches live `5:166` text “Open copilot  →” (instance name ↗ is stale; characters are →). Tiny proof restored after 5873ecf over-corrected to ↗.
- verified: 2026-10-03T16:19:28-04:00

## verified: fp-home-best-extraneous-moon
- screen: home
- ref: `.cursor/figma-refs/02-home.png` · live `2:34` · Personal best `5:154`
- deviation: |
    EXPECTED (live 02): Personal best card is cream copy only (flag kicker, big minutes,
    divider, note, disclaimer) — no moon illustration inside `5:154`. Moon (`22:1301`)
    belongs only on the dark destination art.
    ACTUAL (`.mc-home-best`): mounts `<img class="mc-home-moon" … moon.svg>` in the
    card corner — extra art vs live.
- fix_hint: |
    Remove `.mc-home-moon` from the personal-best card (and related CSS). Keep
    `.mc-home-moon-sm` on destination art + bottom-rail satellite.
- escalate: none
- commit: 5873ecf
- change: Removed `.mc-home-moon` from personal-best card (+ unused CSS); dest-art moon kept.
- verified: 2026-10-03T16:19:28-04:00

## verified: fp-copilot-responses-footnote
- screen: copilot
- ref: `.cursor/figma-refs/03-copilot.png` · live `2:35` · hint `5:256` · foot `5:257`
- deviation: |
    EXPECTED (live 03): under the composer, only
    “Enter to send · Click the microphone to start or stop a voice turn.”
    A separate page-level line “Responses always appear as text. Audio is yours to
    turn on.” sits below (`5:257`), not stacked inside the composer hint.
    ACTUAL (`#chat-hint`): both sentences are one `<p>` joined with `<br />`, so the
    audio/responses line reads as a second composer-hint row.
- fix_hint: |
    Keep Enter/mic copy in `#chat-hint`; move the Responses/Audio sentence to its own
    `.copilot-responses-foot` (or equivalent) under the composer matching live
    placement. Leave Mic/Send/listening wiring alone.
- escalate: none
- commit: 5873ecf
- change: `#chat-hint` keeps Enter/mic only; Responses/Audio moved to `.copilot-responses-foot`.
- verified: 2026-10-03T16:19:28-04:00

## verified: fp-welcome-signin-title
- screen: welcome
- ref: `.cursor/figma-refs/01-welcome.png` · live `2:33` · title `5:61`
- expected: |
    Live 01 Sign in card H2 is “Your seat is ready.” above lead
    “Sign in to return to your space.”
- actual: |
    `.welcome-signin-title` reads “Sign in to continue.”
- deviation: |
    EXPECTED (live `5:61`): card title “Your seat is ready.”
    ACTUAL (`#welcome-signin-form`): “Sign in to continue.” Distinct from open
    `fp-welcome-signin-form` (fields/CTA/foot) — title string alone is wrong.
- fix_hint: |
    Set `.welcome-signin-title` to “Your seat is ready.”; leave Google wiring and
    the broader form open (`fp-welcome-signin-form`) for Email/Password/guest.
- escalate: scrutinous
- commit: cc11ce5
- change: `.welcome-signin-title` → “Your seat is ready.” (live `5:61`); form/Google path untouched.
- verified: 2026-10-03T16:45:50-04:00

## open: fp-relaunch-live-panel-copy
- screen: active
- ref: `.cursor/figma-refs/09-reset-relaunch.png` · live `2:41` ·
  pill `6:581` · badge `8:415`
- expected: |
    Live 09 (tick9 get_screenshot): progress pill “→  Ready to relaunch?”;
    caption “REMAINING · TIMER PAUSED”; primary row “Take a break” + “End
    mission”; right card badge “→  READY TO RELAUNCH?”, title “Let’s pick
    one small next step.”, body “Your {n} earned minutes are safe. Take a
    breath and start small.”, labeled Next step field, Relaunch + Take a
    break, footer “Same objective. Same flight. A fresh start.”
    Glyph is → (U+2192), not ↗ — layer names may still say ↗.
- actual: |
    No relaunch session surface (only `RELAUNCH_FLAG_KEY` stats counter).
    Sibling opens lock → glyph, footer mint, field fill, pill mint.
- deviation: |
    Complements open `fp-relaunch-session-panel` with exact live 09 copy +
    left chrome (→ pill, TIMER PAUSED caption, Take a break replacing Pause).
- fix_hint: |
    When relaunch state fires: set progress/caption/controls per live 09
    with → glyphs; swap `.session-copilot-panel` to the relaunch card with
    Next step + Relaunch. Keep End mission wiring; do not change
    intentional Pause/End contracts on normal active.
- escalate: scrutinous

## verified: fp-timer-replace-body-copy
- screen: active
- ref: `.cursor/figma-refs/15-timer-replacement.png` · live `2:47` · body `6:1566`
- expected: |
    Live 15 modal body: “Your current timer has {m:ss} left. Replace it with a
    new five-minute timer?” under title “Replace your step timer?” / badge
    “ONE TIMER AT A TIME”; actions Keep current · Replace timer.
- actual: |
    Suggest chip silently overwrites the countdown (open `fp-timer-replace-modal`
    covers missing modal shell) — no remaining-time sentence either.
- deviation: |
    When the replace modal is built, body must interpolate the live remaining
    countdown (sample 4:32), not a generic “replace?” only.
- fix_hint: |
    In the MC modal for `fp-timer-replace-modal`, set body from live `6:1566`
    template using current `#session-next-step-timer` value.
- escalate: scrutinous
- commit: 62f7171
- change: |
    `#timer-replace-modal-body` interpolates live remaining as m:ss
    (“Your current timer has {m:ss} left. Replace it with a new five-minute
    timer? …”).
- verified: 2026-10-03T18:13:56-04:00

## verified: fp-session-pb-marker-stack
- screen: active
- ref: `.cursor/figma-refs/06-mission-active.png` · live `2:38` · marker `6:191`–
    `6:193`
- expected: |
    Personal best callout stacks two lines: “⚑ Personal best” then “20 min”
    under it (`6:192` / `6:193`).
- actual: |
    `#session-orbit-pb-label` is a single inline string
    “Personal best {n} min” beside the flag (`.session-flight-personal-best`
    is `inline-flex` row).
- deviation: |
    Live marker is a vertical stack (label over minutes), not one horizontal
    phrase.
- fix_hint: |
    Split into kicker + minutes elements (or two lines in the label) matching
    `6:191`; keep `updateSessionOrbitPersonalBest()` wiring.
- escalate: none
- commit: cc11ce5
- change: PB callout stacks flag+“Personal best” then `{n} min`; `updateSessionOrbitPersonalBest` sets minutes only.
- verified: 2026-10-03T16:45:50-04:00

## verified: fp-listening-hides-next-step
- screen: active
- ref: `.cursor/figma-refs/14-listening.png` · live `2:46`
- expected: |
    Live 14 Listening: left flight card has NO `#session-next-step` /
    Secondary timer row (dashboard ends at earned-minutes); right panel keeps
    “Set a five-minute timer” + listening composer (● Listening… / ■ / 
    “Listening… click mic to stop”).
- actual: |
    Open `fp-listening-session-ui` notes next-step “may remain”; app always
    mounts `#session-next-step` during mid-flight, including while mic is
    listening.
- deviation: |
    Live 14 removes the left NEXT STEP TIMER row while listening (height 662
    vs 714 on 06). Complements `fp-listening-session-ui` presence/composer work.
- fix_hint: |
    While session mic is listening, hide `#session-next-step`; restore when
    idle. Keep suggest chip + Pause/End. Distinct from launch-chrome hide
    (`fp-session-at-launch-chrome`).
- escalate: scrutinous
- commit: cc11ce5
- change: `#view-session.is-session-listening` hides `#session-next-step`; toggled via `setSessionListeningUi` on session mic start/end.
- verified: 2026-10-03T16:45:50-04:00

## verified: fp-break-ii-glyph
- screen: active
- ref: `.cursor/figma-refs/10-on-a-break.png` · live `2:42` · pill `6:760` · badge `8:431`
- expected: |
    Live 10 progress pill is “Ⅱ  On a break” (`6:760`); right-card badge is
    “Ⅱ  ON A BREAK” (`8:431`) — leading double-bar / II glyph before the label.
    Caption uses middle-dot “REMAINING · TIMER PAUSED” (`6:817`).
- actual: |
    `updateSessionProgressPill("paused")` sets plain “On a break” (no Ⅱ);
    open break-panel work also targets plain “ON A BREAK” without the glyph.
    Older opens used bullet “•” in the caption; live is “·”.
- deviation: |
    Break chrome glyphs/punctuation vs live 10 — distinct from panel shell/copy
    opens (`fp-break-session-panel` / `fp-break-live-panel-copy`).
- fix_hint: |
    When paused: prefix progress label + break-card badge with “Ⅱ  ”; set
    `.session-timer-caption` to “REMAINING · TIMER PAUSED”. Keep Resume/End.
- escalate: scrutinous
- commit: b7de340
- change: |
    Progress pill → “Ⅱ  On a break”; `#session-timer-caption` → “REMAINING · TIMER PAUSED”
    while paused (restored on resume). Break-card badge deferred — no panel shell yet
    (`fp-break-session-panel`).
- verified: 2026-10-03T16:45:50-04:00
- note: |
    Live `6:760`/`6:817` match HEAD. Badge `8:431` “Ⅱ  ON A BREAK” remains with open
    panel work (`fp-break-live-panel-copy` / `fp-break-session-panel`).

## verified: fp-break-hides-session-chrome
- screen: active
- ref: `.cursor/figma-refs/10-on-a-break.png` · live `2:42`
- expected: |
    Live 10 On a break: left flight card has NO `#session-next-step` /
    NEXT STEP TIMER row; right break card replaces chat — NO
    “Set a five-minute timer” suggest chip.
- actual: |
    Mid-flight `#session-next-step` + `#session-copilot-suggest` stay mounted
    while paused (only listening hides next-step via `is-session-listening`).
- deviation: |
    Break state must strip secondary-timer + suggest chrome, same pattern as
    relaunch/connection-lost hides. Complements break panel opens.
- fix_hint: |
    When `session.paused` (break): hide `#session-next-step` and
    `#session-copilot-suggest`; restore on Resume. Keep Pause/End wiring.
- escalate: scrutinous
- commit: b7de340
- change: |
    `#view-session.is-session-break` (via `syncPauseControls`) hides
    `#session-next-step` + `#session-copilot-suggest`; cleared on resume.
- verified: 2026-10-03T16:45:50-04:00

## open: fp-relaunch-hides-session-chrome
- screen: active
- ref: `.cursor/figma-refs/09-reset-relaunch.png` · live `2:41`
- expected: |
    Live 09 Reset & relaunch: left card has NO NEXT STEP TIMER row; right is
    the relaunch card (Next step field + Relaunch) — NO five-minute suggest.
- actual: |
    No relaunch surface yet; mid-flight next-step + suggest remain visible
    whenever those nodes mount. Distinct from open panel/copy work.
- deviation: |
    Relaunch state must hide secondary-timer + suggest (metadata: no
    NEXT STEP TIMER / no “Set a five-minute timer” on `2:41`).
- fix_hint: |
    When relaunch UI shows: hide `#session-next-step` +
    `#session-copilot-suggest`; restore after Relaunch/idle active.
    Pair with `fp-relaunch-live-panel-copy` / `fp-relaunch-session-panel`.
- escalate: scrutinous

## open: fp-connection-lost-body-copy
- screen: active
- ref: `.cursor/figma-refs/16-connection-lost.png` · live `2:48` ·
  title `6:1720` · body `6:1721` · presence `6:1712`
- expected: |
    Live 16 callout: title “Connection lost. Your flight keeps going.” plus
    body “The timer, pause, and end controls still work. Reconnect whenever
    you're ready.” Presence pill “○  Reconnecting…” (hollow ring + ellipsis).
    Retry connection under the body.
- actual: |
    Open `fp-connection-lost-panel` covers title + Retry + presence word only —
    no body sentence; app has neither callout nor “○ Reconnecting…”.
- deviation: |
    Missing helper paragraph + presence ring glyph vs live `6:1721` / `6:1712`.
- fix_hint: |
    In the connection-lost block: add live body copy; set presence to
    “○  Reconnecting…”. Keep Retry → existing reconnect helpers.
- escalate: scrutinous

## open: fp-connection-lost-hides-session-chrome
- screen: active
- ref: `.cursor/figma-refs/16-connection-lost.png` · live `2:48`
- expected: |
    Live 16 Connection lost: left flight card has NO NEXT STEP TIMER;
    right keeps chat history + lost callout + composer — NO
    “Set a five-minute timer” suggest chip.
- actual: |
    Mid-flight next-step + suggest stay visible; no connection-lost UI yet
    (`fp-connection-lost-panel`).
- deviation: |
    While disconnected: hide `#session-next-step` + `#session-copilot-suggest`
    (metadata confirms neither on `2:48`). Pause/End remain.
- fix_hint: |
    On coach/Gemini disconnect during a mission, hide next-step + suggest;
    restore when reconnected. Pair with `fp-connection-lost-panel` /
    `fp-connection-lost-body-copy`.
- escalate: scrutinous

## verified: fp-gentle-checkin-hides-session-chrome
- screen: active
- ref: `.cursor/figma-refs/08-gentle-check-in.png` · live `2:40`
- expected: |
    Live 08 Gentle check-in: left flight dashboard height 662 — NO
    `#session-next-step` / NEXT STEP TIMER row (ends at earned-minutes);
    right panel is the check-in card — NO “Set a five-minute timer”
    suggest. Pause/End remain; caption stays “REMAINING IN YOUR FLIGHT”.
- actual: |
    Mid-flight next-step + suggest stay mounted; check-in only appears as
    overlay toast (see panel/progress opens). Break/listening already hide
    via `is-session-break` / `is-session-listening` — no check-in class.
- deviation: |
    Check-in must strip secondary-timer + suggest chrome (metadata: no
    Secondary timer on `2:40`). Complements `fp-gentle-checkin-session-panel`
    / `fp-gentle-checkin-progress-pill` / `fp-gentle-checkin-panel-copy`.
- fix_hint: |
    When gentle check-in is showing: hide `#session-next-step` +
    `#session-copilot-suggest` (e.g. `is-session-checkin`); restore on
    dismiss. Keep Pause/End. Pair with panel swap opens.
- escalate: scrutinous
- commit: 57c9d92
- change: |
    `#view-session.is-session-checkin` hides `#session-next-step` +
    `#session-copilot-suggest`; toggled via global `overlay-prompt` /
    `overlay-clear` (+ toast-duration timeout). Caption/Pause/End unchanged.
- verified: 2026-10-03T17:05:46-04:00

## verified: fp-end-confirm-hides-next-step
- screen: active
- ref: `.cursor/figma-refs/11-end-confirmation.png` · live `2:43`
- expected: |
    Live 11 End confirmation: under the scrim, left flight card has NO
    NEXT STEP TIMER (dashboard 662, ends at earned-minutes). Right still
    shows chat + “Set a five-minute timer” + composer (dimmed). Modal
    “END MISSION” / “End this session?” / Keep working · End session.
- actual: |
    No MC end modal yet (`fp-end-confirm-figma-modal` /
    `fp-end-confirm-modal-spec`); mid-flight `#session-next-step` stays
    mounted whenever a step timer is running.
- deviation: |
    When End confirmation is up, hide left `#session-next-step` to match
    live `2:43` underlying chrome. Distinct from modal shell/copy opens;
    suggest may remain under scrim.
- fix_hint: |
    While end-confirm modal is open: hide `#session-next-step`; restore
    on Keep working / after End. Keep Pause/End wiring; pair with modal
    opens.
- escalate: scrutinous
- commit: 57c9d92
- change: |
    `#view-session.is-session-ending` hides `#session-next-step` only
    (suggest stays); toggled around native `window.confirm` on `#end-session`
    until MC end modal lands. Full modal deferred (`fp-end-confirm-figma-modal`).
- verified: 2026-10-03T17:05:46-04:00

## verified: fp-timer-replace-keeps-running
- screen: active
- ref: `.cursor/figma-refs/15-timer-replacement.png` · live `2:47` · body `6:1566`
- expected: |
    Live 15 modal body closer after replace question (design_context /
    tick16+): “Your mission timer keeps running.”
- actual: |
    Superseded by `62f7171` `#timer-replace-modal` body (remaining m:ss +
    “Your mission timer keeps running.”).
- deviation: |
    Earlier OCR without “timer” conflicted with design_context; shipped
    closer matches `fp-timer-replace-closer-has-timer`.
- commit: 62f7171
- change: |
    Modal body includes mission-timer closer; stale no-“timer” guidance
    retired.
- escalate: scrutinous
- verified: 2026-10-03T18:13:56-04:00

## open: fp-permissions-choice-surface
- screen: overlay
- ref: `.cursor/figma-refs/17-permission-request.png` ·
  `18-permission-denied.png` · live `2:49` / `2:50` · intro `7:393`/`7:476`
- expected: |
    Live 17/18 modals sit over a full MC page (not setup toggles alone):
    cream top nav Lock-in active; kicker “YOUR CHOICE”; title
    “Fly on your own terms.”; sub “Optional signals. Clear controls.
    No recordings.”; left ringed planet + right ship art.
- actual: |
    Tick14: handoff modal shell + camera-live title/body/foot verified
    (`9681808` / `fp-permission-handoff-camera-live`); surface/title
    Medium 30 + pad 32 / gap 22 still OK. Modals still mount as fixed
    overlays over setup/Settings — no “YOUR CHOICE” / “Fly on your own
    terms.” interstitial page or art plane under them.
- deviation: |
    Missing interstitial permissions-choice page that hosts the handoff
    and denied modals in Figma. Distinct from verified modal shell/
    camera-live copy / surface/title-scale/stack-gap.
- fix_hint: |
    Add a lightweight MC “YOUR CHOICE” view (or setup substate) matching
    live intro + art; show handoff/denied modals over it. Keep five-tab
    Settings; do not block Launch.
- escalate: scrutinous

## verified: fp-permission-denied-body-copy
- screen: overlay
- ref: `.cursor/figma-refs/18-permission-denied.png` · live `2:50` ·
  body `7:502` · footer `7:510`
- expected: |
    Live 18 denied modal body: “Permission wasn’t granted, so the input
    stays off. Your mission works just as well without it.” Footer:
    “Denied access never blocks starting a mission.” CTAs
    “Back to setup” + “View settings”; badge “INPUT REMAINS OFF”;
    title “You’re still cleared for launch.”
- actual: |
    Open `fp-permission-denied-modal` paraphrases body (“permission
    wasn’t granted / mission works without it”) — exact two-sentence
    body + footer string not locked; app has no denied surface yet.
- deviation: |
    Exact denied body/footer copy vs live `7:502` / `7:510`. Complements
    shell open `fp-permission-denied-modal`.
- fix_hint: |
    When building the denied modal, use the live body + footer verbatim;
    Back to setup → `#view-lockin`; View settings → Permissions tab.
- escalate: scrutinous
- commit: b7ea57b
- change: |
    Denied modal uses live `7:502` / `7:510` body + footer (works just as
    well); expected above refreshed off stale “will still work”.
- verified: 2026-10-03T17:05:46-04:00

## verified: fp-permission-handoff-camera-copy
- screen: overlay
- ref: `.cursor/figma-refs/17-permission-request.png` · live `2:49` ·
  title `7:418` · body `7:419` · footer `7:427`
- expected: |
    Live 17 modal (design_context): badge “PERMISSION HANDOFF”; title
    “Allow camera signals?”; body “Your browser asks to use your camera.
    Camera signals may inform a gentle check-in; they never prove you’re
    distracted.”; Not now + Continue; footer “Nothing is recorded. You can
    turn this input off at any time from your mission controls.”
    (Layer/frame name still “Share an optional input?” — stale; visible
    text is camera-specific.)
- actual: |
    Open `fp-permission-handoff-modal` still expects generic title “Share
    an optional input?” + browser camera/screen body + “next step is the
    native prompt” footer. App has no handoff modal.
- deviation: |
    Live 17 copy moved to camera-specific title/body/footer. Distinct from
    screen-share frame 27 (`fp-screen-share-permission-modal`) and from
    the YOUR CHOICE page shell (`fp-permissions-choice-surface`).
- fix_hint: |
    When building camera handoff, use live `7:418`/`7:419`/`7:427`
    verbatim (not the stale layer name). Keep Not now / Continue wiring.
- escalate: scrutinous
- commit: b7ea57b
- change: |
    Handoff modal uses live camera title/body/footer (`7:418`/`7:419`/
    `7:427`); badge PERMISSION HANDOFF; Not now · Continue.
- verified: 2026-10-03T17:05:46-04:00

## verified: fp-permission-denied-works-copy
- screen: overlay
- ref: `.cursor/figma-refs/18-permission-denied.png` · live `2:50` ·
  body `7:502`
- expected: |
    Live 18 body (design_context `7:502`): “Permission wasn’t granted, so
    the input stays off. Your mission works just as well without it.”
- actual: |
    Open `fp-permission-denied-body-copy` expected “Your mission will
    still work without it.” — wrong closer vs live. App has no denied
    surface yet.
- deviation: |
    Exact second sentence is “works just as well”, not “will still work”.
    Complements shell `fp-permission-denied-modal`; supersedes stale
    expected in `fp-permission-denied-body-copy`.
- fix_hint: |
    Use live `7:502` verbatim when building the denied modal body.
- escalate: scrutinous
- commit: b7ea57b
- change: |
    Denied body uses live “works just as well without it.” (`7:502`).
- verified: 2026-10-03T17:05:46-04:00

## verified: fp-timer-replace-mission-timer-copy
- screen: active
- ref: `.cursor/figma-refs/15-timer-replacement.png` · live `2:47` ·
  body `6:1566`
- expected: |
    Live design_context closer includes the word “timer”: “Your mission
    timer keeps running.”
- actual: |
    Superseded by `62f7171` (same string as `fp-timer-replace-closer-has-timer`).
- deviation: |
    False OCR path that dropped “timer” retired; shipped modal matches
    design_context.
- commit: 62f7171
- change: |
    Inbox closed as superseded — modal body uses mission-timer closer.
- escalate: scrutinous
- verified: 2026-10-03T18:13:56-04:00

## verified: fp-timer-replace-keeps-next-step
- screen: active
- ref: `.cursor/figma-refs/15-timer-replacement.png` · live `2:47`
- expected: |
    Live 15 under scrim: flight dashboard height 714 — Secondary timer
    (“◷  NEXT STEP TIMER” / 04:32) STAYS visible; suggest chip “Set a
    five-minute timer” STAYS. Mission in progress + Pause/End unchanged.
- actual: |
    No replace modal yet. End-confirm / check-in / break / listening /
    relaunch / connection-lost hide next-step (662 chrome). Risk: fixer
    copies that hide pattern onto timer-replace.
- deviation: |
    Timer-replace must NOT hide `#session-next-step` or suggest (opposite
    of `fp-end-confirm-hides-next-step` / check-in chrome). Distinct from
    modal shell/copy opens.
- fix_hint: |
    When showing the replace modal, leave next-step + suggest mounted;
    do not toggle `is-session-ending` / break / check-in hide classes.
- escalate: scrutinous
- commit: 62f7171
- change: |
    `openTimerReplaceModal` does not toggle `is-session-ending` / break /
    check-in; `#session-next-step` + suggest stay visible under scrim.
- verified: 2026-10-03T18:13:56-04:00

## verified: fp-permission-denied-cta-primary
- screen: overlay
- ref: `.cursor/figma-refs/18-permission-denied.png` · live `2:50` ·
  CTAs `7:504` / `7:507`
- expected: |
    Live 18 actions: LEFT “Back to setup” is filled primary (M3 enabled
    button); RIGHT “View settings” is tonal/raised lavender secondary.
- actual: |
    Open denied shell/body opens list both CTAs but do not lock which is
    filled. App has no denied modal.
- deviation: |
    CTA hierarchy vs live — Back to setup primary, View settings
    secondary. Complements `fp-permission-denied-modal`.
- fix_hint: |
    Style Back to setup as filled purple primary; View settings as
    lavender secondary. Wire → `#view-lockin` / Permissions tab.
- escalate: none
- commit: b7ea57b
- change: |
    Left “Back to setup” filled primary; right “View settings” lavender
    secondary; wired to setup / Permissions tab.
- verified: 2026-10-03T17:05:46-04:00

## verified: fp-copilot-sidebar-cta-fullwidth
- screen: copilot
- ref: `.cursor/figma-refs/03-copilot.png` · live `2:35` · CTA `5:223`
- expected: |
    Live 03 sidebar “Start a mission” is a full-width filled pill spanning
    the sidebar content column (262×52 under `5:217`).
- actual: |
    `#copilot-start-mission.copilot-sidebar-cta` uses `align-self: flex-start`
    + hug padding — button is content-width, not full sidebar row.
- deviation: |
    Copilot priority tick — CTA width vs live `5:223`. Copy/wiring OK
    (verified live-03 chrome); width alone mismatches.
- fix_hint: |
    Set `#copilot-start-mission` / `.copilot-sidebar-cta` to `align-self:
    stretch` / `width: 100%`; keep `show("view-lockin")` wiring.
- escalate: scrutinous
- commit: b1c106a
- change: |
    `.copilot-sidebar-cta` now `align-self: stretch` + `width: 100%`
    (full sidebar row); `#copilot-start-mission` → `show("view-lockin")`
    unchanged.
- verified: 2026-10-03T17:33:30-04:00
- note: |
    Live get_screenshot/metadata `5:223` = 262×52 under sidebar `5:217`
    content column; HEAD `.copilot-sidebar-cta` stretch + width 100% matches.

## verified: fp-copilot-composer-surface
- screen: copilot
- ref: `.cursor/figma-refs/03-copilot.png` · live `2:35` · input `5:249`
- expected: |
    Live 03 composer pill fill is page lavender `#f7f2ff` (`color-bg`) with
    `#c8bfd7` border + 16px radius (Mic tonal + Send filled inside).
- actual: |
    `.view-copilot .copilot-input-wrap` uses `background: var(--mc-bg-card)`
    (cream/white surface) — reads as a card chip, not page-bg input.
- deviation: |
    Composer surface token vs live `5:249`. Distinct from verified in-pill
    Mic/Send / responses-foot. Copilot priority.
- fix_hint: |
    Set Copilot (and optionally session) input-wrap fill to page lavender
    `#f7f2ff` / `--mc-bg-page`; keep Mic/Send/listening wiring.
- escalate: scrutinous
- commit: b1c106a
- change: |
    `.view-copilot` / session `.copilot-input-wrap` fill `#f7f2ff`, border
    `#c8bfd7`, 16px radius; Mic/Send/listening wiring unchanged.
- verified: 2026-10-03T17:33:30-04:00
- note: |
    Live get_screenshot `5:249` pale lavender page-bg + light purple border;
    HEAD `#f7f2ff` / `#c8bfd7` / 1rem radius matches.

## verified: fp-timer-replace-cta-hierarchy
- screen: active
- ref: `.cursor/figma-refs/15-timer-replacement.png` · live `2:47` ·
  actions `6:1568` / `6:1571`
- expected: |
    Live 15 dialog actions: LEFT “Keep current” tonal/raised lavender
    secondary; RIGHT “Replace timer” filled purple primary (equal ~252px).
- actual: |
    No replace modal yet (`fp-timer-replace-modal`). Existing opens lock
    badge/title/body copy but not which CTA is filled.
- deviation: |
    CTA hierarchy vs live — Keep current secondary, Replace timer primary.
    Complements shell/body/keeps-next-step opens.
- fix_hint: |
    When building the replace modal, style Keep current lavender secondary
    (dismiss) and Replace timer filled primary → `startNextStepTimer(300)`.
- escalate: scrutinous
- commit: 62f7171
- change: |
    LEFT Keep current = lavender secondary dismiss; RIGHT Replace timer =
    filled primary → `startNextStepTimer(300)`.
- verified: 2026-10-03T18:13:56-04:00

## open: fp-connection-lost-latest-card
- screen: active
- ref: `.cursor/figma-refs/16-connection-lost.png` · live `2:48` ·
  callout `6:1719` · Retry `6:1722`
- expected: |
    Live 16 mounts the lost message as a “Latest response” card (`6:1719`)
    — lavender raised/bordered block with title + body (same chrome as
    assistant latest-response on 06/29). “Retry connection” sits in the
    suggest-chip slot (full-width tonal, y≈478), not inside the card.
- actual: |
    Open `fp-connection-lost-panel` / `fp-connection-lost-body-copy` cover
    presence + title/body/Retry existence only — not Latest-response card
    chrome or Retry-as-suggest placement. App has neither.
- deviation: |
    Callout uses Latest-response card tokens; Retry replaces suggest.
    Complements panel/body/chrome-hide; pairs with
    `fp-session-latest-response-card` token reuse.
- fix_hint: |
    Render lost callout via `.session-latest-response` (or shared card);
    place Retry where `#session-copilot-suggest` sits; hide suggest while
    disconnected.
- escalate: scrutinous

## verified: fp-welcome-guest-button-chrome
- screen: welcome
- ref: `.cursor/figma-refs/01-welcome.png` · live `2:33` · guest `5:74`
- expected: |
    Live 01 “Continue as guest” is a full-width (438×52) Quiet pill under
    “Sign in →” — fill `#fffbff` (not tonal `#e9ddfd`), same width as the
    primary, not a plain text link.
- actual: |
    Tick9: `254b24b` mounted `#welcome-continue-guest` Quiet `#fffbff`
    438×52 pill + guest unlock (verified via `fp-welcome-guest-quiet-fill`).
    Chrome/fill match live `5:74`. Remaining welcome gap is Email/Password
    form (`fp-welcome-signin-form` / `fp-welcome-field-height`), not guest CTA.
- deviation: |
    Guest CTA chrome satisfied by 254b24b — open kept only so patrol can
    retire; no further guest-button work needed. Prefer closing vs form opens.
- fix_hint: |
    No guest-chrome fix left. Patrol/compliance may promote to verified or
    drop; focus remaining welcome work on Email/Password card
    (`fp-welcome-signin-form`).
- escalate: scrutinous
- commit: 254b24b
- change: Quiet `#fffbff` 438×52 guest pill; guest unlock wiring kept.
- verified: 2026-10-03T18:32:50-04:00

## verified: fp-break-extraneous-pause-note
- screen: active
- ref: `.cursor/figma-refs/10-on-a-break.png` · live `2:42` · tick13
- expected: |
    Live 10 under the Resume/End row: only earned-minutes + hint — NO
    coaching-hold note. Break copy lives in the right card only.
- actual: |
    `syncPauseControls(true)` sets `#session-pause-note.hidden = false`
    with “Timer paused — coaching is on hold until you resume.” CSS is
    visible (not visually-hidden). Live frame has no such line.
- deviation: |
    Extraneous pause note vs live 10. Distinct from break panel/copy/
    Resume-label opens.
- fix_hint: |
    Keep `#session-pause-note` hidden (or remove) while paused; rely on
    break card body for coaching-paused copy. Keep Pause/End wiring.
- escalate: scrutinous
- commit: a79a5ce
- change: |
    `syncPauseControls` always keeps `#session-pause-note` hidden; CSS
    `display: none` so the coaching-hold line never appears under Pause/End.
- verified: 2026-10-03T17:33:30-04:00
- note: |
    Live metadata `2:42`: after `6:818` controls only `6:825` earned-minutes —
    no coaching-hold line. HEAD keeps `#session-pause-note` hidden + `display:none`.

## verified: fp-break-resume-mission-label
- screen: active
- ref: `.cursor/figma-refs/10-on-a-break.png` · live `2:42` · left `6:819`
- expected: |
    Live 10 left primary is exactly “Resume mission” (`6:819`, 210×52
    filled); End mission stays tonal secondary. Card CTA matches.
- actual: |
    `syncPauseControls` sets `#session-pause` text to “Resume” (not
    “Resume mission”). Open `fp-break-live-panel-copy` only notes this
    optionally.
- deviation: |
    Left control label must be “Resume mission” per live `6:819`.
    Elevates soft note in panel-copy open.
- fix_hint: |
    When paused: `btn.textContent = "Resume mission"`; idle → “Pause”.
    Keep pause invoke / End. Pair with break panel shell.
- escalate: scrutinous
- commit: a79a5ce
- change: |
    Paused `#session-pause` label is “Resume mission”; idle “Pause”.
    Left control stays filled primary while paused; End mission unchanged.
- verified: 2026-10-03T17:33:30-04:00
- note: |
    Live `6:819` “Resume mission” + card `8:435` match HEAD
    `syncPauseControls` paused label + `#session-break-resume`.

## open: fp-relaunch-cta-hierarchy
- screen: active
- ref: `.cursor/figma-refs/09-reset-relaunch.png` · live `2:41` ·
  left `6:644`/`6:647` · card `8:423`/`8:426`
- expected: |
    Live 09: LEFT “Take a break” filled primary + “End mission” tonal;
    RIGHT card “Relaunch” filled primary full-width + “Take a break”
    tonal secondary full-width under it.
- actual: |
    No relaunch surface (`fp-relaunch-session-panel` /
    `fp-relaunch-live-panel-copy`). Opens lock copy/existence, not which
    CTAs are filled.
- deviation: |
    CTA hierarchy vs live — left Take a break primary; card Relaunch
    primary / Take a break secondary. Complements panel/copy/chrome-hide.
- fix_hint: |
    When building relaunch UI: style left Take a break as filled (Pause
    slot), End tonal; card Relaunch filled → relaunch path, Take a break
    tonal → same pause path. Keep End wiring.
- escalate: scrutinous

## open: fp-connection-lost-composer-stays
- screen: active
- ref: `.cursor/figma-refs/16-connection-lost.png` · live `2:48` ·
  composer `6:1725`
- expected: |
    Live 16 keeps the idle message composer under Retry (placeholder
    “Message your copilot…”, Mic + Send ↑, stable hint). Only suggest
    becomes “Retry connection”; presence “○  Reconnecting…”.
- actual: |
    Opens cover presence/callout/Retry/latest-card/chrome-hide — not
    that composer must remain mounted and enabled. Risk: fixer disables
    chat while disconnected.
- deviation: |
    Composer stays idle while lost; do not strip Mic/Send. Distinct from
    `fp-connection-lost-latest-card` (Retry-as-suggest).
- fix_hint: |
    On disconnect: swap suggest→Retry + show lost Latest card; leave
    `#session-chat-form` / Mic / Send as idle. Restore suggest on reconnect.
- escalate: scrutinous

## verified: fp-break-panel-compact
- screen: active
- ref: `.cursor/figma-refs/10-on-a-break.png` · live `2:42` · panel `6:828`
- expected: |
    Live 10 right panel height 376 — badge “Ⅱ  ON A BREAK”, title, body,
    divider, Resume mission, footer only. NO ✦ Your copilot header,
    presence, chat history, suggest, or composer.
- actual: |
    Open `fp-break-session-panel` / `fp-break-live-panel-copy` say swap
    to break card but don’t lock compact chrome (no chat stack). App
    keeps full `.session-copilot-panel` chat column while paused.
- deviation: |
    Break panel is a short status card, not a chat sidebar with a card
    overlay. Complements panel/copy; pairs with chrome-hide (already
    verified for next-step/suggest).
- fix_hint: |
    Replace copilot column contents with the compact break card (no
    chat/composer/presence); restore chat on Resume. Keep End + pause.
- escalate: scrutinous
- commit: a79a5ce
- change: |
    `#view-session.is-session-break` swaps `.session-copilot-chat` for
    `#session-break-card` (badge/title/body/divider/Resume/footer). Card
    Resume clicks `#session-pause`; chat column restores on resume.
- verified: 2026-10-03T17:33:30-04:00
- note: |
    Live get_screenshot/metadata `6:828` = 434×376 compact card only;
    HEAD `is-session-break` hides chat stack + shows `#session-break-card`.

## verified: fp-copilot-chip-height
- screen: copilot
- ref: `.cursor/figma-refs/03-copilot.png` · live `2:35` · prompts `5:240` ·
  chips `5:241`/`5:243`/`5:245`
- expected: |
    Live 03 suggested prompts row is 52px tall — Explain simply 210×52,
    I’m stuck 180×52, Find a next step 230×52 (10px gaps).
- actual: |
    `.copilot-chip` uses `min-height: 2.75rem` (~44px) + hug padding — chips
    read shorter than live 52px pills. Copy/no-+ already verified; CTA width +
    composer fill done in b1c106a.
- deviation: |
    Copilot priority tick14 — chip height vs live `5:240`. Distinct from
    verified chip-plus removal and done CTA/composer surface.
- fix_hint: |
    Set `.copilot-chip` to `min-height: 52px` (and optional min-widths
    210/180/230); keep plain labels + study wiring.
- escalate: scrutinous
- commit: ab57008
- change: `.copilot-chip` `min-height: 52px` to match live `5:240` suggested-prompt pills; labels + study wiring unchanged.
- verified: 2026-10-03T17:52:09-04:00


## verified: fp-copilot-placeholder-ellipsis
- screen: copilot
- ref: `.cursor/figma-refs/03-copilot.png` · live `2:35` · input `5:250` ·
  also session `6:1011` / `6:1727`
- expected: |
    Live composer placeholder is “Message your copilot…” with a single
    unicode ellipsis (U+2026) — same on Copilot 03 and session composers.
- actual: |
    `#chat-input` + `CHAT_INPUT_IDLE_PLACEHOLDER` correctly use
    “Message your copilot…” (U+2026) after ab57008. `#session-chat-input`
    regressed in 10bb695 to “Ask your companion…” (still U+2026, wrong copy
    vs live session composers).
- deviation: |
    Session idle placeholder copy vs live `5:250`/`6:1011` — Copilot path OK;
    session still mismatches Figma string. Reopened after ab57008 “done”.
- fix_hint: |
    Restore `#session-chat-input` idle placeholder to “Message your copilot…”
    (U+2026); keep Talk/Mic + send wiring. Do not touch Copilot `#chat-input`.
- commit: adecf0c
- change: |
    Restored `#session-chat-input` HTML placeholder to “Message your
    copilot…” (U+2026) after 10bb695 companion Live regression.
- reopen_note: |
    Tick8 live verify: Copilot OK; session placeholder overwritten by 10bb695
    companion Live to “Ask your companion…”.
- verified: 2026-10-03T18:13:56-04:00

## open: fp-relaunch-next-step-field
- screen: active
- ref: `.cursor/figma-refs/09-reset-relaunch.png` · live `2:41` ·
  field `8:419`/`8:421`
- expected: |
    Live 09 relaunch card includes a labeled “Next step” stack (`8:419`):
    label + Editable field 386×64 (`8:421`) above Relaunch / Take a break —
    not a bare textarea or missing input.
- actual: |
    No relaunch surface yet (`fp-relaunch-session-panel` /
    `fp-relaunch-live-panel-copy`). Those opens mention “Next step field” in
    copy lists but do not lock label + 64px field chrome.
- deviation: |
    Next-step field chrome vs live `8:419`/`8:421`. Complements panel/copy/
    CTA-hierarchy; under-covered relaunch surface.
- fix_hint: |
    When building the relaunch card, mount labeled Next step + 64px MC
    editable (prefill coach next-step text); keep Relaunch wiring.
- escalate: scrutinous

## open: fp-connection-lost-keeps-history
- screen: active
- ref: `.cursor/figma-refs/16-connection-lost.png` · live `2:48` ·
  AT LAUNCH `6:1714` · YOU `6:1717` · Latest `6:1719`
- expected: |
    Live 16 keeps prior chat turns (AT LAUNCH seed + YOU · JUST NOW) above
    the Connection-lost Latest-response card; disconnect does not clear the
    log.
- actual: |
    Opens cover presence/callout/Retry/latest-card/composer-stays/chrome-hide
    — not that existing session turns must remain mounted when lost UI
    appears. App has no lost UI yet.
- deviation: |
    History stays above lost Latest card. Distinct from
    `fp-connection-lost-composer-stays` and `fp-connection-lost-latest-card`.
- fix_hint: |
    On disconnect: append/show lost Latest card + Retry; do not wipe
    `#session-chat-log` turns. Restore suggest on reconnect.
- escalate: scrutinous

## open: fp-welcome-field-height
- screen: welcome
- ref: `.cursor/figma-refs/01-welcome.png` · live `2:33` · Email `5:65` ·
  Password `5:69`
- expected: |
    Live 01 Email/Password Editable fields are 438×64 each under bold
    labels (`5:63`/`5:67` stacks) — same 64px field height as setup/relaunch
    editables.
- actual: |
    Open `fp-welcome-signin-form` covers field existence/placeholders; app
    is still Google-only (no Email/Password). No lock on 64px field height
    when the card is built. CSS `.welcome-signin-card input` padding hugs
    shorter than 64.
- deviation: |
    Welcome field height vs live `5:65`/`5:69`. Complements form + guest
    chrome opens; under-covered sign-in surface.
- fix_hint: |
    When mounting Email/Password, set inputs to min-height 64px (full card
    column width); keep Sign in → / guest wiring per sibling opens.
- escalate: scrutinous

## open: fp-relaunch-panel-compact
- screen: active
- ref: `.cursor/figma-refs/09-reset-relaunch.png` · live `2:41` · panel `6:653`
- expected: |
    Live 09 right panel is 434×560 — badge “→  READY TO RELAUNCH?” (→ not ↗;
    see `fp-relaunch-live-panel-copy`), title, body, divider, Next step field,
    Relaunch, Take a break, footer only. NO ✦ Your copilot header, presence,
    chat history, suggest, or composer.
- actual: |
    Opens cover panel existence/copy/CTA/field/chrome-hide — not that the
    relaunch surface is a short status card replacing the chat column.
    App has no relaunch UI; mid-flight keeps full `.session-copilot-panel`.
- deviation: |
    Relaunch panel is a compact card (mirror verified `fp-break-panel-compact`),
    not a chat sidebar with a card overlay. Complements panel/copy/CTA opens.
- fix_hint: |
    When relaunch shows: swap `.session-copilot-chat` for the compact relaunch
    card only; restore chat after Relaunch/idle. Keep End + Take a break wiring.
- escalate: scrutinous

## verified: fp-gentle-checkin-cta-hierarchy
- screen: active
- ref: `.cursor/figma-refs/08-gentle-check-in.png` · live `2:40` ·
  CTAs `8:404` / `8:407` / `8:410`
- expected: |
    Live 08 stacked actions (all 386×52 full-width): “On task” filled primary
    (`8:404`); “Got distracted” + “Take a break” tonal/raised lavender
    secondary (`8:407`/`8:410`).
- actual: |
    Opens cover panel shell/copy/progress pill — not which CTA is filled.
    App still uses overlay toast without the three actions.
- deviation: |
    CTA hierarchy vs live — On task primary; Got distracted / Take a break
    secondary. Complements `fp-gentle-checkin-panel-copy`.
- fix_hint: |
    When building the check-in card: style On task filled → dismiss/on-task
    path; Got distracted + Take a break tonal (Take a break → pause path).
    Keep Pause/End on the left flight card.
- escalate: scrutinous
- commit: 10bb695
- change: |
    Stacked 52px CTAs: On task filled primary → dismiss; Got distracted tonal
    → soft distracted orbit/pill; Take a break tonal → `#session-pause`.
- verified: 2026-10-03T17:52:09-04:00


## verified: fp-end-confirm-cta-hierarchy
- screen: active
- ref: `.cursor/figma-refs/11-end-confirmation.png` · live `2:43` ·
  actions `6:1026` / `6:1029`
- expected: |
    Live 11 dialog actions (equal 252×52): LEFT “Keep working” tonal/raised
    lavender secondary; RIGHT “End session” filled purple primary.
- actual: |
    Opens cover modal shell/copy (`fp-end-confirm-figma-modal` /
    `fp-end-confirm-modal-spec`) — not which CTA is filled. App still uses
    native `window.confirm`.
- deviation: |
    CTA hierarchy vs live — Keep working secondary, End session primary.
    Same pattern as open `fp-timer-replace-cta-hierarchy`.
- fix_hint: |
    When building the end modal: Keep working lavender secondary (dismiss);
    End session filled primary → existing end invoke. Keep Pause wiring.
- escalate: scrutinous
- commit: 10bb695
- change: |
    Replaced `window.confirm` with MC `#end-session-modal`: Keep working
    lavender secondary; End session filled primary → `stop_lock_in`.
    Keeps `is-session-ending` next-step hide.
- verified: 2026-10-03T17:52:09-04:00


## open: fp-connection-lost-keeps-mission-running
- screen: active
- ref: `.cursor/figma-refs/16-connection-lost.png` · live `2:48` ·
  pill `6:1636` · caption `6:1696` · Pause `6:1698`
- expected: |
    Live 16 left flight chrome stays mid-mission: progress “●  Mission in
    progress”; caption “REMAINING IN YOUR FLIGHT”; primary row “Pause” +
    “End mission” — NOT paused / Take a break / TIMER PAUSED. Body states
    timer/pause/end still work.
- actual: |
    Opens cover presence/callout/Retry/history/composer/chrome-hide — not
    that disconnect must leave left controls in the running-mission state.
    Risk: fixer applies relaunch/break Pause→Take a break swap.
- deviation: |
    Connection lost does not pause the flight. Distinct from
    `fp-connection-lost-hides-session-chrome` (next-step/suggest only).
- fix_hint: |
    On disconnect: keep `#session-progress-label` / caption / Pause label as
    idle mid-flight; only swap presence + Latest card + Retry. Do not enter
    break/relaunch control chrome.
- escalate: scrutinous

## verified: fp-gentle-checkin-panel-compact
- screen: active
- ref: `.cursor/figma-refs/08-gentle-check-in.png` · live `2:40` · panel `6:475`
- expected: |
    Live 08 right panel height 536 — badge “✦ QUICK CHECK-IN”, title, body,
    divider, On task / Got distracted / Take a break, footer only. NO ✦ Your
    copilot header, presence, chat history, suggest, or composer.
- actual: |
    Opens say swap/overlay to check-in card but don’t lock compact chrome
    (no chat stack). App keeps toast overlay + full chat column.
- deviation: |
    Check-in panel is a short status card (mirror `fp-break-panel-compact` /
    open `fp-relaunch-panel-compact`), not a chat sidebar overlay.
- fix_hint: |
    Replace copilot column contents with the compact check-in card; restore
    chat on dismiss / auto-close. Pair with CTA-hierarchy + panel-copy.
- escalate: scrutinous
- commit: 10bb695
- change: |
    `#session-checkin-card` replaces chat column under `is-session-checkin`
    (badge/title/body/divider/3 CTAs/footer); chat restored on dismiss/auto-close.
- verified: 2026-10-03T17:52:09-04:00


## verified: fp-copilot-chip-widths
- screen: copilot
- ref: `.cursor/figma-refs/03-copilot.png` · live `2:35` · prompts `5:240` ·
  chips `5:241`/`5:243`/`5:245`
- expected: |
    Live 03 suggested prompts are fixed-width pills: Explain simply 210×52,
    I’m stuck 180×52, Find a next step 230×52 (10px gaps in a 640×52 row).
- actual: |
    `ab57008` locked `.copilot-chip` `min-height: 52px` only — chips still
    hug content width via horizontal padding (no 210/180/230 min-widths).
    CTA/composer/ellipsis already match.
- deviation: |
    Copilot priority tick16 spot-check after ab57008 — chip widths vs live
    `5:240`. Distinct from done chip-height / verified chip-plus removal.
- fix_hint: |
    Set `.copilot-chip` min-widths 210 / 180 / 230 (or per-chip modifiers);
    keep 52px height + plain labels + study wiring.
- escalate: scrutinous
- commit: 62162e9
- change: |
    Per-`data-study` chip widths 210/180/230×52 with 10px gaps; study
    wiring + plain labels kept.
- verified: 2026-10-03T18:13:56-04:00

## verified: fp-copilot-empty-surface
- screen: copilot
- ref: `.cursor/figma-refs/03-copilot.png` · live `2:35` · card `5:235`
- expected: |
    Live empty “Copilot response” card (`5:235`) is surface `#fffbff` with
    `#c8bfd7` border, 28px radius, 24px padding — not cream.
- actual: |
    `.copilot-panel` uses `background: var(--mc-bg-card)` (`#fffdf8` cream)
    for the empty-state card shell. Copy/heading OK; b1c106a fixed composer
    fill only.
- deviation: |
    Empty-card surface token vs live `5:235`. Copilot priority after
    b1c106a composer-surface (composer lavender OK; card still cream).
- fix_hint: |
    Set empty-state `.copilot-panel` / `#chat-empty` fill to `#fffbff` +
    keep `#c8bfd7` / ~28px radius; leave Mic/Send/chip wiring.
- escalate: scrutinous
- commit: 62162e9
- change: |
    Empty-state `.copilot-panel:has(#chat-empty)` uses `#fffbff` +
    `#c8bfd7` / 28px radius; cream cards elsewhere unchanged.
- verified: 2026-10-03T18:13:56-04:00

## verified: fp-copilot-composer-height
- screen: copilot
- ref: `.cursor/figma-refs/03-copilot.png` · live `2:35` · input `5:249`
- expected: |
    Live 03 Composer input is 868×78 — `py` 12px around 52×52 Mic/Send
    (`5:251`/`5:254`), `px` 16, gap 12; fill/border already match.
- actual: |
    `.view-copilot .copilot-input-wrap` uses `padding: 0.65rem …` (~10.4px
    vertical) → wrap reads ~73px tall with 52px controls. Surface tokens
    OK after b1c106a; ellipsis OK after ab57008.
- deviation: |
    Composer height/padding vs live `5:249` 78px. Distinct from verified
    surface-token + done placeholder.
- fix_hint: |
    Set Copilot (and session) input-wrap vertical padding to 12px so the
    pill is 78px with 52px Mic/Send; keep `#f7f2ff` / `#c8bfd7` / 16px radius.
- escalate: scrutinous
- commit: 62162e9
- change: |
    Copilot + session `.copilot-input-wrap` padding 12×16, gap 12 → ~78px
    with 52px Mic/Send; fill/border/ellipsis preserved.
- verified: 2026-10-03T18:13:56-04:00

## verified: fp-timer-replace-closer-has-timer
- screen: active
- ref: `.cursor/figma-refs/15-timer-replacement.png` · live `2:47` ·
  body `6:1566`
- expected: |
    Live get_design_context `6:1566` verbatim closer: “Your mission timer
    keeps running.” — the word “timer” IS present after “mission”.
- actual: |
    Opens `fp-timer-replace-keeps-running` + `fp-timer-replace-mission-timer-copy`
    still instruct fixers to DROP “timer” (“Your mission keeps running.”)
    from a prior false OCR. App has no replace modal yet.
- deviation: |
    Tick16 live proof reverted the false reading — body must include
    “mission timer keeps running.” Supersedes the no-timer expected in
    those sibling opens when building `fp-timer-replace-modal`.
- fix_hint: |
    When building the replace modal body, use live closer with “timer”:
    “…five-minute timer? Your mission timer keeps running.” Ignore the
    no-timer guidance on the sibling opens.
- escalate: scrutinous
- commit: 62f7171
- change: |
    Body closer uses verbatim “Your mission timer keeps running.” (timer
    word included per live `6:1566`).
- verified: 2026-10-03T18:13:56-04:00

## open: fp-connection-lost-keeps-chat-chrome
- screen: active
- ref: `.cursor/figma-refs/16-connection-lost.png` · live `2:48` ·
  panel `6:1707` · header `6:1708`
- expected: |
    Live 16 right column stays a full chat shell (434×678): ✦ Your copilot
    header, ○ Reconnecting… presence, AT LAUNCH / YOU history, Latest
    lost card, Retry, composer. NOT a compact status card.
- actual: |
    Opens cover presence/callout/Retry/history/composer/mission-running —
    not that lost UI must KEEP the chat chrome. Risk: fixer copies
    break/check-in/relaunch `*-panel-compact` and strips header/history.
- deviation: |
    Connection lost is NOT compact (opposite of verified break / done
    check-in / open relaunch compact). Under-covered `2:48` surface.
- fix_hint: |
    On disconnect: keep `.session-copilot` header + log; only swap presence,
    append Latest lost card, suggest→Retry. Do not apply
    `is-session-break` / check-in compact swap.
- escalate: scrutinous

## open: fp-relaunch-badge-arrow
- screen: active
- ref: `.cursor/figma-refs/09-reset-relaunch.png` · live `2:41` ·
  pill `6:581` · badge `8:415`
- expected: |
    Live design_context characters are right-arrow → (not ↗): left progress
    “→  Ready to relaunch?”; card badge “→  READY TO RELAUNCH?”. Layer names
    still say ↗ — stale (same trap as verified home “Open copilot →”).
- actual: |
    Opens `fp-relaunch-live-panel-copy` / `fp-relaunch-panel-compact` still
    instruct ↗. App has no relaunch UI yet.
- deviation: |
    Tick17 live proof — badge/pill glyph is →. Supersedes ↗ guidance on
    sibling relaunch opens when building the surface.
- fix_hint: |
    Use → (U+2192) for progress pill + card badge; ignore ↗ layer names.
    Pair with panel/copy/compact opens. Keep End + Take a break wiring.
- escalate: scrutinous

## open: fp-relaunch-footer-mint
- screen: active
- ref: `.cursor/figma-refs/09-reset-relaunch.png` · live `2:41` ·
  footer `8:429`
- expected: |
    Live relaunch card footer “Same objective. Same flight. A fresh start.”
    is mint `#326c78` at 13px — not muted gray (`#645d73`).
- actual: |
    Opens lock footer string only (`fp-relaunch-live-panel-copy` /
    `fp-relaunch-session-panel`). No color token. App has no relaunch card.
- deviation: |
    Footer mint token vs live `8:429`. Distinct from copy/CTA/field opens.
- fix_hint: |
    Style relaunch footer with mint `#326c78` (same token as welcome/setup
    kickers). Keep copy verbatim; Relaunch wiring unchanged.
- escalate: scrutinous

## verified: fp-welcome-field-fill
- screen: welcome
- ref: `.cursor/figma-refs/01-welcome.png` · live `2:33` · Email `5:65` ·
  Password `5:69`
- expected: |
    Live Email/Password Editables: fill `#f7f2ff`, border `#c8bfd7`,
    radius 12px, 438×64 (same MC editable as relaunch/setup).
- actual: |
    Open `fp-welcome-field-height` locks 64px only. Existing CSS
    `.welcome-signin-card input` uses `#f3eef9` fill, mauve
    `rgba(42,36,64,0.12)` border, 14px radius — wrong tokens even before
    Email/Password mount (Google-only form).
- deviation: |
    Welcome field fill/border/radius vs live `5:65`/`5:69`. Complements
    form + field-height + guest-chrome opens.
- fix_hint: |
    Set welcome inputs to `#f7f2ff` / `#c8bfd7` / 12px radius + min-height
    64px; keep Sign in → / guest path per sibling opens.
- escalate: scrutinous
- commit: 62f7171
- change: |
    `.welcome-signin-card input` → fill `#f7f2ff`, border `#c8bfd7`,
    radius 12px, min-height 64px (Google-only form; tokens ready for fields).
- verified: 2026-10-03T18:13:56-04:00

## verified: fp-timer-replace-modal-surface
- screen: active
- ref: `.cursor/figma-refs/15-timer-replacement.png` · live `2:47` ·
  modal `6:1562`
- expected: |
    Live replace dialog (`6:1562`, 580×301): surface `#fffbff`, border
    `#c8bfd7`, radius 28px, padding 32px, gap 22px — same MC modal chrome
    as permission/end dialogs. Badge ONE TIMER AT A TIME on `#e9ddfd`.
- actual: |
    Opens cover shell/body/CTA/keeps-next-step/closer-has-timer — not
    surface tokens. App still silently overwrites (`fp-timer-replace-modal`).
- deviation: |
    Modal surface tokens vs live `6:1562`. Under-covered `2:47` chrome.
- fix_hint: |
    When building the replace modal, use `#fffbff` / `#c8bfd7` / 28px /
    32px pad; Keep current tonal + Replace timer filled per CTA open.
- escalate: scrutinous
- commit: 62f7171
- change: |
    `.mc-perm-modal__card--timer-replace` uses `#fffbff` / `#c8bfd7` /
    28px radius / 32px pad / 22px gap / max-width 580px.
- verified: 2026-10-03T18:13:56-04:00

## open: fp-permissions-choice-title-size
- screen: overlay
- ref: `.cursor/figma-refs/17-permission-request.png` · live `2:49` ·
  intro `7:393` · title `7:395`
- expected: |
    Live YOUR CHOICE intro: kicker “YOUR CHOICE” purple `#6750a4` bold
    12px; title “Fly on your own terms.” medium 44px dark; sub
    “Optional signals. Clear controls. No recordings.” muted 16px.
- actual: |
    Open `fp-permissions-choice-surface` locks page existence + copy
    strings but not title type scale (44px) or kicker purple (not mint).
    App still has no interstitial under handoff/denied modals.
- deviation: |
    Intro type/color tokens vs live `7:394`/`7:395`/`7:396`. Complements
    choice-surface shell; under-covered YOUR CHOICE.
- fix_hint: |
    When adding the YOUR CHOICE view, set title ~44px medium + purple
    kicker (not mint); keep Lock-in nav + handoff/denied over the page.
    Preserve Settings five-tab; do not block Launch.
- escalate: scrutinous

## verified: fp-copilot-sidebar-title-scale
- screen: copilot
- ref: `.cursor/figma-refs/03-copilot.png` · live `2:35` · sidebar `5:217` ·
  title `5:220`
- expected: |
    Live 03 sidebar title (`5:220`) is Medium 26px / leading 1.4, two
    lines (“A sounding board” / “for your next step.”) on raised
    `#e9ddfd` card — not semibold ~24px.
- actual: |
    Copilot priority tick18 spot-check after dones `62162e9` / `ab57008` /
    `b1c106a` / `adecf0c` (chips/empty-surface/composer/placeholder OK).
    `.copilot-sidebar-title` is `1.5rem` (~24px) `font-weight: 600`.
- deviation: |
    Sidebar title type scale/weight vs live `5:220`. Distinct from
    verified sidebar-copy / CTA-fullwidth.
- fix_hint: |
    Set `.copilot-sidebar-title` to 26px medium (500); keep Start a
    mission wiring + `#e9ddfd` card fill.
- escalate: scrutinous
- commit: 254b24b
- change: `.copilot-sidebar-title` → Medium 26px / line-height 1.4 (`font-weight: 500`, `1.625rem`).
- verified: 2026-10-03T18:13:56-04:00

## verified: fp-copilot-empty-heading-weight
- screen: copilot
- ref: `.cursor/figma-refs/03-copilot.png` · live `2:35` · card `5:235` ·
  heading `5:238`
- expected: |
    Live empty-card heading “What are you working on today?” is Regular
    22px (`font-normal`) on `#fffbff` card with 24px pad / 14px gap.
- actual: |
    Tick18 Copilot spot-check: empty surface `#fffbff` OK after `62162e9`;
    `.copilot-empty-heading` is `1.375rem` (22px) but `font-weight: 500`.
    Gap `0.85rem` vs live 14px (secondary).
- deviation: |
    Empty heading weight vs live `5:238` Regular. Complements done
    empty-surface; copy strings already match.
- fix_hint: |
    Set `.copilot-empty-heading` to `font-weight: 400` (optional gap
    14px); leave Mic/Send/chip wiring.
- escalate: scrutinous
- commit: 254b24b
- change: `.copilot-empty-heading` → Regular (`font-weight: 400`); empty-card gap 14px.
- verified: 2026-10-03T18:13:56-04:00

## open: fp-relaunch-next-step-field-fill
- screen: active
- ref: `.cursor/figma-refs/09-reset-relaunch.png` · live `2:41` ·
  field `8:421`
- expected: |
    Live 09 Next step Editable (`8:421`, 386×64): fill `#f7f2ff`, border
    `#c8bfd7`, radius 12px, px 18 / py 17 — same MC editable as welcome/
    setup.
- actual: |
    Open `fp-relaunch-next-step-field` locks label + 64px height only.
    App has no relaunch card yet — no fill/border tokens when built.
- deviation: |
    Next-step field surface tokens vs live `8:421`. Complements field
    chrome / panel-compact / CTA opens; under-covered `2:41`.
- fix_hint: |
    When mounting the relaunch Next step input, use `#f7f2ff` / `#c8bfd7`
    / 12px radius + 64px height; keep Relaunch wiring.
- escalate: scrutinous

## open: fp-relaunch-progress-pill-mint
- screen: active
- ref: `.cursor/figma-refs/09-reset-relaunch.png` · live `2:41` ·
  pill text `6:581` · card badge `8:415`
- expected: |
    Live left progress label (`6:581`) is mint `#326c78` “→  Ready to
    relaunch?”. Right card badge (`8:415`) stays purple `#6750a4`
    “→  READY TO RELAUNCH?” — two different label colors.
- actual: |
    Opens lock → glyph (`fp-relaunch-badge-arrow`) + footer mint
    (`fp-relaunch-footer-mint`) but not left-pill mint vs card-badge
    purple. App has no relaunch UI.
- deviation: |
    Tick18 live proof — progress pill mint, card badge purple. Distinct
    from footer mint + badge arrow.
- fix_hint: |
    Style left `#session-progress-label` relaunch state mint `#326c78`;
    card badge purple `#6750a4`; both use →. Pair with panel/copy opens.
- escalate: scrutinous

## verified: fp-welcome-guest-quiet-fill
- screen: welcome
- ref: `.cursor/figma-refs/01-welcome.png` · live `2:33` · guest `5:74`
- expected: |
    Live “Continue as guest” (`5:74`, 438×52) is Quiet state: fill
    `#fffbff` (surface), not tonal `#e9ddfd` lavender — full-width pill
    under Sign in →.
- actual: |
    Open `fp-welcome-guest-button-chrome` still says “tonal/raised
    lavender secondary”. Live design_context is Quiet `#fffbff`. App
    still Google-only (no guest control).
- deviation: |
    Tick18 live supersedes lavender guest guidance — Quiet surface fill.
    Complements guest-button-chrome size/width + signin-form.
- fix_hint: |
    When mounting guest CTA: `#fffbff` Quiet pill 438×52 (not `#e9ddfd`);
    keep guest unlock wiring. Ignore lavender secondary on sibling open.
- escalate: scrutinous
- commit: 254b24b
- change: Mounted `#welcome-continue-guest` as full-width Quiet `#fffbff` 52px pill; wired `sign_in_waypoint_guest` + `guest_mode` unlock.
- verified: 2026-10-03T18:13:56-04:00

## verified: fp-welcome-card-surface
- screen: welcome
- ref: `.cursor/figma-refs/01-welcome.png` · live `2:33` · card `5:58`
- expected: |
    Live Sign in card (`5:58`, 510×668): surface `#fffbff`, border `#c8bfd7`,
    radius ~28px, padding 37 — large MC card hosting kicker/title/fields/CTAs.
- actual: |
    `#welcome-signin-form` / `.welcome-signin-card` uses cream `#fbf8f3`,
    mauve `rgba(42,36,64,0.08)` border, `border-radius: 24px`,
    `width: min(100%, 24rem)` (~384px) — wrong surface + too narrow.
    Open `fp-welcome-signin-form` locks Email/Password/CTA copy only.
- deviation: |
    Tick19 live — welcome card chrome vs live `5:58`. Distinct from form
    fields (`fp-welcome-field-height`) + verified field-fill / guest Quiet.
- fix_hint: |
    Set `.welcome-signin-card` to `#fffbff` / `#c8bfd7` / ~28px radius /
    ~510px max-width + 37px pad; keep Google/guest wiring per form opens.
- escalate: scrutinous
- commit: 69037e0
- change: `.welcome-signin-card` → `#fffbff` / `#c8bfd7` / 28px radius / 37px pad / `min(100%, 510px)`; kicker/fields/guest/Sign in wiring unchanged.
- verified: 2026-10-03T18:32:50-04:00

## open: fp-relaunch-title-scale
- screen: active
- ref: `.cursor/figma-refs/09-reset-relaunch.png` · live `2:41` · title `8:416`
- expected: |
    Live relaunch card title (`8:416`, 386×78): two-line “Let’s pick one /
    small next step.” — Medium 28px / leading 1.4 (per
    `fp-relaunch-title-medium-28`; not ~36px, not 1.55rem break-card scale).
- actual: |
    Opens lock title string (`fp-relaunch-live-panel-copy` /
    `fp-relaunch-panel-compact`) but not type scale. App has no relaunch
    card; break title CSS is `1.55rem` / 700 — risk of wrong-scale reuse.
- deviation: |
    Title type scale vs live `8:416` Medium 28px. Complements panel/copy/
    compact opens; under-covered `2:41`.
- fix_hint: |
    When mounting relaunch title, use `font-size: 28px; font-weight: 500;
    line-height: 1.4` two-line stack (~386×78 box); do not copy
    `.session-break-title` 1.55rem or ship ~36px. Keep Relaunch wiring.
- escalate: scrutinous

## open: fp-relaunch-card-surface
- screen: active
- ref: `.cursor/figma-refs/09-reset-relaunch.png` · live `2:41` · panel `6:653`
- expected: |
    Live relaunch panel (`6:653`, 434×560): cream/surface `#fffbff` card
    with border `#c8bfd7` + large radius (same MC panel chrome as break /
    check-in compact cards), pad ~25.
- actual: |
    Opens cover compact swap / copy / CTA / field fill — not panel surface
    tokens. App has no relaunch card yet.
- deviation: |
    Relaunch card surface tokens vs live `6:653`. Distinct from
    `fp-relaunch-panel-compact` (chrome presence) + field-fill.
- fix_hint: |
    Style relaunch card `#fffbff` / `#c8bfd7` / MC radius + ~25px pad inside
    434-wide column; pair with compact swap. Keep End / Take a break.
- escalate: scrutinous

## open: fp-connection-lost-retry-tonal
- screen: active
- ref: `.cursor/figma-refs/16-connection-lost.png` · live `2:48` ·
  Retry `6:1722`
- expected: |
    Live “Retry connection” (`6:1722`, 386×52) is full-width tonal/raised
    lavender (`#e9ddfd`) — same suggest-chip slot chrome, NOT filled primary.
- actual: |
    Open `fp-connection-lost-latest-card` locks Retry-as-suggest placement
    only. Risk: fixer styles Retry as filled purple primary. App has no
    lost Retry yet.
- deviation: |
    Retry tonal hierarchy vs live `6:1722`. Complements latest-card /
    composer-stays / keeps-chat-chrome; under-covered `2:48`.
- fix_hint: |
    Style Retry like `#session-copilot-suggest` tonal 52px full-width
    (`#e9ddfd`); wire reconnect helper. Do not use filled primary.
- escalate: scrutinous

## verified: fp-permission-handoff-modal-surface
- screen: overlay
- ref: `.cursor/figma-refs/17-permission-request.png` · live `2:49` ·
  modal `7:415`
- expected: |
    Live handoff dialog (`7:415`, 580×340): surface `#fffbff`, border
    `#c8bfd7`, radius ~28px, padding ~33 — same MC modal chrome as verified
    timer-replace (`6:1562`). Sits over YOUR CHOICE page.
- actual: |
    `#permission-handoff-modal` `.mc-perm-modal__card` is
    `width: min(100%, 26.5rem)` (~424px), cream `var(--mc-bg-card)`,
    `border-radius: 1.35rem`, pad ~1.85rem — narrower + wrong tokens.
    Copy/badge verified (`b7ea57b`); timer-replace got 580px surface,
    handoff did not.
- deviation: |
    Tick19 live — handoff modal surface vs live `7:415`. Distinct from
    verified camera copy + open YOUR CHOICE page/title-size.
- fix_hint: |
    Apply timer-replace-like surface to handoff card: 580 max-width /
    `#fffbff` / `#c8bfd7` / 28px / ~33px pad; keep Not now · Continue.
    Preserve Settings five-tab; do not block Launch.
- escalate: scrutinous
- commit: 69037e0
- change: Base `.mc-perm-modal__card` → 580px / `#fffbff` / `#c8bfd7` / 28px / 33px pad (handoff + denied + end-session); Continue/Not now CTAs unchanged.
- verified: 2026-10-03T18:32:50-04:00

## verified: fp-copilot-mic-idle-label
- screen: copilot
- ref: `.cursor/figma-refs/03-copilot.png` · live `2:35` · Mic `5:251` ·
  listening `10:667` / `10:736`
- expected: |
    Live 03 idle Mic (`5:251`, 52×52) is tonal `#e9ddfd` circle labeled
    “Mic” (title case). Live 31 listening (`10:736`) keeps the same “Mic”
    label while the field shows “Listening… click mic to stop” — session
    14 uses ■ (`6:1389`), Copilot 31 does not.
- actual: |
    Tick20 Copilot priority after `254b24b` / `62162e9` / `b8b30e0`:
    `#chat-mic` idle label is “Talk” (HTML + `applyMicLiveUi(..., "Talk")`).
    Verified `fp-copilot-listening-composer` taught ■ for Copilot STT, but
    Live path never restores “Mic”.
- deviation: |
    Idle (and Copilot listening) control label vs live `5:251` / `10:736`.
    Distinct from chip/empty/composer surface dones. Escalate after Live
    companion rewrote Mic → Talk.
- fix_hint: |
    Idle `#chat-mic` text “Mic” to match live 03/31; keep Live start/end
    wiring (aria can say live voice). Do not use “Talk” on Copilot tab.
    Preserve Send ↑ + chip wiring.
- escalate: scrutinous
- commit: c00ab73
- change: |
    `#chat-mic` idle/live label → “Mic” (HTML + `applyMicLiveUi(..., "Mic",
    { stickyLabel: true })`); Live start/end wiring kept; session Talk unchanged.
- verified: 2026-10-03T18:52:40-04:00

## verified: fp-copilot-composer-hint-copy
- screen: copilot
- ref: `.cursor/figma-refs/03-copilot.png` · live `2:35` · hint `5:256`
- expected: |
    Live 03 under-composer hint (`5:256`): “Enter to send · Click the
    microphone to start or stop a voice turn.” (muted). Verified via
    `fp-copilot-responses-footnote` / live-match — Enter/mic only in
    `#chat-hint`.
- actual: |
    Tick20: `#chat-hint` reads “Talk for live voice · type + Enter for a
    turn · Live ends when you tap Live again” — Live companion copy
    replaced Figma Enter/mic line.
- deviation: |
    Composer hint regression vs live `5:256` after Live land. Complements
    mic-idle-label; not covered by chip/height/empty dones (`62162e9` /
    `254b24b`).
- fix_hint: |
    Restore `#chat-hint` to live Enter/mic sentence; keep Live affordance
    in aria/title or Settings Voice — not this hint. Leave responses-foot
    as its own open.
- escalate: scrutinous
- commit: c00ab73
- change: |
    `#chat-hint` idle → live `5:256` Enter/mic sentence; temporary Live
    status hints still ok and restore idle copy when Live ends.
- verified: 2026-10-03T18:52:40-04:00

## verified: fp-copilot-responses-foot-copy
- screen: copilot
- ref: `.cursor/figma-refs/03-copilot.png` · live `2:35` · foot `5:257`
- expected: |
    Live 03 responses foot (`5:257`): “Responses always appear as text.
    Audio is yours to turn on.” — verified `fp-copilot-responses-footnote`.
- actual: |
    Tick20: `.copilot-responses-foot` reads “Live voice talks with your
    coach. Typed chat still works when Live is off.” — diverges from live
    and from verified foot copy.
- deviation: |
    Responses-foot regression vs live `5:257` / verified footnote. Copilot
    priority tick20; distinct from hint-line open.
- fix_hint: |
    Restore `.copilot-responses-foot` to live Responses/Audio sentence;
    document Live elsewhere if needed. Keep Mic/Send wiring.
- escalate: scrutinous
- commit: c00ab73
- change: |
    `.copilot-responses-foot` → live `5:257` “Responses always appear as
    text. Audio is yours to turn on.”
- verified: 2026-10-03T18:52:40-04:00

## verified: fp-copilot-mic-live-phase-chrome
- screen: copilot
- ref: `.cursor/figma-refs/03-copilot.png` · `31-copilot-listening.png` ·
  live `2:35` / `10:667` · Mic `5:251` / `10:736`
- expected: |
    Live Copilot mic chrome stays tonal `#e9ddfd` + dark “Mic” label in
    idle and listening (31). No purple/green/brown fills; listening only
    changes field copy to “Listening… click mic to stop”.
- actual: |
    Spot-check `b8b30e0` restored `.view-copilot .copilot-mic.is-live|
    .is-thinking|.is-speaking|.is-connecting` fills (`#6b4cff` / `#3d2a99` /
    `#2f6b45` / `#5c5348`) with labels “Live” / “…”. Not present on live
    03/31. Field never shows listening placeholder during Live.
- deviation: |
    Live-phase mic colors/labels vs Figma Copilot mic instances. Product
    Live states overshoot live 03/31 chrome; pairs with idle-label + hint
    opens.
- fix_hint: |
    Prefer Figma tonal Mic during Copilot voice; if Live needs phase
    feedback, use presence/hint — not non-Figma mic fills/labels. Keep
    end-Live click wiring.
- escalate: scrutinous
- commit: c00ab73
- change: |
    Copilot `.copilot-mic.is-*` stays tonal `#e9ddfd` + “Mic”; session
    Talk phase fills kept; listening sets `#chat-input` placeholder to
    “Listening… click mic to stop”.
- verified: 2026-10-03T18:52:40-04:00

## verified: fp-welcome-title-scale
- screen: welcome
- ref: `.cursor/figma-refs/01-welcome.png` · live `2:33` · title `5:61`
- expected: |
    Live Sign in title “Your seat is ready.” (`5:61`, 438×45): Medium
    32px / leading 1.4 on `#fffbff` card (tick21 token correction; not
    ~36px). String already verified (`fp-welcome-signin-title`).
- actual: |
    Tick20 welcome spot-check after `69037e0` card surface (no live
    regression on `#fffbff`/`#c8bfd7`/28px). `.welcome-signin-title` is
    `clamp(1.45rem, 2.8vw, 1.75rem)` (~23–28px) `font-weight: 700` —
    under-scaled vs 45px box. Form fields still Google-only (open
    `fp-welcome-signin-form`).
- deviation: |
    Title type scale vs live `5:61`. Distinct from verified string + card
    surface; under-covered welcome form chrome.
- fix_hint: |
    Set `.welcome-signin-title` to 32px / `font-weight: 500` / line-height
    1.4; keep Google/guest wiring + card surface tokens from `69037e0`.
- escalate: scrutinous
- commit: bb10d7d
- change: |
    `.welcome-signin-title` → Medium 32px / line-height 1.4 / weight 500
    (live `5:61`); string “Your seat is ready.” unchanged.
- verified: 2026-10-03T18:52:40-04:00

## open: fp-relaunch-title-medium-28
- screen: active
- ref: `.cursor/figma-refs/09-reset-relaunch.png` · live `2:41` · title `8:416`
- expected: |
    Tick21 design_context: relaunch title (`8:416`) is Medium 28px /
    leading 1.4, two lines (“Let’s pick one” / “small next step.”) —
    not ~36px display. Box still ~386×78 from line stack.
- actual: |
    Open `fp-relaunch-title-scale` expected ~36px medium; that overshoots
    live tokens. App has no relaunch card yet.
- deviation: |
    Tick21 live token correction — title is 28px Medium, not ~36px.
    Prefer this over `fp-relaunch-title-scale` when mounting. Under-covered
    `2:41`.
- fix_hint: |
    Style relaunch title `font-size: 28px; font-weight: 500; line-height:
    1.4` two-line; keep Relaunch / Take a break wiring. Ignore ~36px on
    sibling title-scale open.
- escalate: scrutinous

## open: fp-relaunch-divider
- screen: active
- ref: `.cursor/figma-refs/09-reset-relaunch.png` · live `2:41` ·
  divider `8:418`
- expected: |
    Live 09 relaunch card: 1px `#c8bfd7` hairline (`8:418`, 386 wide)
    between body (“Your 12 earned minutes…”) and the Next step field
    stack — panel gap ~20px / pad ~24px.
- actual: |
    Opens cover panel compact / copy / field / CTA / surface — not the
    body→field divider. App has no relaunch card.
- deviation: |
    Missing divider chrome vs live `8:418`. Distinct from field-fill /
    panel-compact / title opens; under-covered `2:41`.
- fix_hint: |
    When mounting relaunch card, insert `#c8bfd7` 1px rule between body
    and Next step label; keep End / Relaunch wiring.
- escalate: scrutinous

## open: fp-connection-lost-presence-pill
- screen: active
- ref: `.cursor/figma-refs/16-connection-lost.png` · live `2:48` ·
  pill `6:1711` · text `6:1712`
- expected: |
    Live 16 presence (`6:1711`): raised `#e9ddfd` pill, purple `#6750a4`
    Regular 12px “○  Reconnecting…” (hollow ring + ellipsis) — same
    raised chrome as suggest/progress pills, not plain text.
- actual: |
    Open `fp-connection-lost-body-copy` locks presence string “○
    Reconnecting…” only. App still shows “Here when you need me” with no
    lost UI (`fp-connection-lost-panel`).
- deviation: |
    Presence pill surface/color tokens vs live `6:1711`. Complements
    string lock + panel/latest-card; under-covered `2:48`.
- fix_hint: |
    On disconnect: restyle `.session-copilot-presence` as `#e9ddfd` pill +
    `#6750a4` “○  Reconnecting…”; restore idle on reconnect. Keep composer.
- escalate: scrutinous

## open: fp-permissions-choice-art
- screen: overlay
- ref: `.cursor/figma-refs/17-permission-request.png` · live `2:49` ·
  planet `7:397` · ship `7:402`
- expected: |
    Live YOUR CHOICE page under handoff scrim: left ringed planet
    (`7:397`, ~218×218 at y≈456) + right ship (`7:402`, ~160×110 at
    y≈570) on constellation field — intro at `7:393` above.
- actual: |
    Open `fp-permissions-choice-surface` mentions planet+ship art in
    expected copy but does not lock sizes/placement. App still has no
    interstitial page under handoff/denied modals.
- deviation: |
    Tick21 — art plane geometry for YOUR CHOICE backdrop. Distinct from
    surface shell + title-size (44px / purple kicker). Under-covered
    permissions choice.
- fix_hint: |
    When adding the YOUR CHOICE view, mount ringed planet left + ship
    right matching live `7:397`/`7:402`; keep handoff/denied over the
    page. Preserve Settings five-tab; do not block Launch.
- escalate: scrutinous

## verified: fp-welcome-title-medium-32
- screen: welcome
- ref: `.cursor/figma-refs/01-welcome.png` · live `2:33` · title `5:61`
- expected: |
    Tick21 design_context: Sign in title (`5:61`) is Medium 32px /
    leading 1.4 (“Your seat is ready.”) on `#fffbff` — not ~36px and not
    `font-weight: 700`. Height ~45px matches 32×1.4.
- actual: |
    Open `fp-welcome-title-scale` expected ~36px; live tokens are 32px
    Medium. App `.welcome-signin-title` still clamp ~23–28px / 700 after
    `69037e0` card surface.
- deviation: |
    Tick21 live token correction — prefer 32px Medium over sibling ~36px
    guidance. Distinct NEW from form/field-height; under-covered welcome.
- fix_hint: |
    Set `.welcome-signin-title` to 32px / `font-weight: 500` / line-height
    1.4; keep Google/guest wiring + card surface. Prefer over
    `fp-welcome-title-scale` ~36px hint.
- escalate: scrutinous
- commit: bb10d7d
- change: |
    `.welcome-signin-title` → Medium 32px / line-height 1.4 / weight 500
    (live `5:61`); string “Your seat is ready.” unchanged.
- verified: 2026-10-03T18:52:40-04:00

## verified: fp-copilot-page-title-scale
- screen: copilot
- ref: `.cursor/figma-refs/03-copilot.png` · live `2:35` · title `5:233`
- expected: |
    Live 03 page title “A little help, a clearer path.” (`5:233`, ~880×53 /
    text raster ~449×36): Medium ~36px / weight 500 on the conversation
    column — not Semibold. Spot-check after `c00ab73` Mic/hint/foot/tonal
    (those match live `5:251` / `5:256` / `5:257` / `#e9ddfd`).
- actual: |
    Tick22 Copilot priority: `.copilot-page-title` is
    `clamp(1.85rem, 3.2vw, 2.375rem)` with `font-weight: 600` / line-height
    1.3 — overshoots Medium 36 and uses Semibold.
- deviation: |
    Page title type scale/weight vs live `5:233`. Distinct from verified
    sidebar-title-scale / empty-heading-weight; Mic chrome dones OK.
- fix_hint: |
    Set `.copilot-page-title` to 36px / `font-weight: 500` / line-height
    ~1.4; keep Start a mission + Mic/Send wiring. Preserve Pause/End /
    Settings five-tab / `#app` padding 0.
- escalate: scrutinous
- commit: b0e40e8
- change: |
    `.copilot-page-title` → Medium 36px / line-height 1.4 / weight 500
    (live `5:233`); Mic/Send + Start a mission wiring unchanged.
- verified: 2026-10-03T19:12:40-04:00

## verified: fp-copilot-intro-gap
- screen: copilot
- ref: `.cursor/figma-refs/03-copilot.png` · live `2:35` · title `5:233` ·
  sub `5:234`
- expected: |
    Live Conversation stack: title at y=0 (h≈53) then sub “Ask freely.
    Your next step can be small.” at y=75 (h≈16–22) — ~22px gap between
    title box and sub.
- actual: |
    `.copilot-intro` uses `gap: 0.85rem` (~13.6px) between
    `.copilot-page-title` and `.copilot-page-sub`. Sub copy/size (~16px)
    already match.
- deviation: |
    Intro vertical gap vs live `5:233`→`5:234`. Complements page-title
    scale; Copilot priority tick22 after Mic dones.
- fix_hint: |
    Set `.copilot-intro` gap to 22px (or margin-top on sub); leave chip /
    composer / Mic wiring.
- escalate: scrutinous
- commit: b0e40e8
- change: |
    `.copilot-intro` gap → `1.375rem` (22px) for live `5:233`→`5:234`;
    chip / composer / Mic wiring unchanged.
- verified: 2026-10-03T19:12:40-04:00

## open: fp-relaunch-take-break-quiet-fill
- screen: active
- ref: `.cursor/figma-refs/09-reset-relaunch.png` · live `2:41` ·
  secondary `8:426` · field `8:421`
- expected: |
    DEFER — Tick23 live `8:426` is raised tonal `#e9ddfd`, not Quiet
    field fill. Treat `fp-relaunch-take-break-tonal-raised` as source of
    truth for Take a break fill. Do not implement Quiet `#f7f2ff` here.
- actual: |
    Prior Quiet-fill guidance was a crop misread; superseded by Tick23
    `fp-relaunch-take-break-tonal-raised`. App still has no relaunch card.
- deviation: |
    Stale Quiet-fill open retained for history; fixers should follow
    tonal-raised instead. Under-covered `2:41`.
- fix_hint: |
    No-op for Quiet fill. When mounting relaunch CTAs, use tonal-raised
    open: Relaunch filled `#6750a4`; Take a break `#e9ddfd`. Keep End /
    Take a break → pause wiring.
- escalate: scrutinous
- deferred_to: fp-relaunch-take-break-tonal-raised

## open: fp-connection-lost-composer-hint-copy
- screen: active
- ref: `.cursor/figma-refs/16-connection-lost.png` · live `2:48` ·
  hint `6:1734` · composer `6:1725`
- expected: |
    Live 16 under-composer hint (`6:1734`): “Enter to send · Click the
    microphone to start or stop a voice turn.” while disconnected —
    same Enter/mic sentence as Copilot idle `5:256`. Composer stays with
    Mic + Send (not Talk).
- actual: |
    Opens cover composer-stays / Mic presence / Retry — not hint string.
    App `#session-chat-hint` is “Talk for live voice · type + Enter for a
    turn · ends with the mission” and mic label stays “Talk”.
- deviation: |
    Lost-state composer hint (+ Mic label) vs live `6:1734` / `6:1728`.
    Distinct from presence-pill / latest-card / retry-tonal; under-covered
    `2:48`.
- fix_hint: |
    On disconnect: set `#session-chat-hint` to live Enter/mic sentence and
    `#session-chat-mic` label “Mic”; restore Talk hint/label on reconnect.
    Keep composer mounted + Retry wiring.
- escalate: scrutinous

## verified: fp-permission-handoff-title-scale
- screen: overlay
- ref: `.cursor/figma-refs/17-permission-request.png` · live `2:49` ·
  title `7:418` · modal `7:415`
- expected: |
    Live handoff title (`7:418`, 516×42): Medium ~28–30px / weight 500
    (not Bold ~25px). Frame title string is “Share an optional input?”;
    camera-specific “Allow camera signals?” remains a product variant
    (verified camera-copy). Badge + surface already match.
- actual: |
    Tick22 YOUR CHOICE spot-check: `.mc-perm-modal__title` is `1.55rem`
    (~24.8px) `font-weight: 700`. Copy strings OK after `b7ea57b`;
    interstitial page still open (`fp-permissions-choice-surface`).
- deviation: |
    Handoff title type scale/weight vs live `7:418`. Distinct from
    choice-surface / title-size (page intro) / art; under-covered
    permissions choice.
- fix_hint: |
    Set `.mc-perm-modal__title` to ~28px / `font-weight: 500` / line-height
    ~1.4; keep Not now · Continue + camera copy. Preserve Settings
    five-tab; do not block Launch.
- escalate: scrutinous
- commit: b0e40e8
- change: |
    `.mc-perm-modal__title` → Medium 30px / line-height 1.4 / weight 500
    (live `7:418`); “Allow camera signals?” + denied shared class; Not now /
    Continue wiring unchanged.
- verified: 2026-10-03T19:12:40-04:00

## open: fp-relaunch-take-break-tonal-raised
- screen: active
- ref: `.cursor/figma-refs/09-reset-relaunch.png` · live `2:41` ·
  secondary `8:426` · field `8:421`
- expected: |
    Tick23 design_context + isolated `get_screenshot` on `8:426`: card
    “Take a break” is raised tonal `#e9ddfd` (Secondary), 386×52 — NOT
    Quiet field fill `#f7f2ff`. Next step Editable stays `#f7f2ff` /
    `#c8bfd7`. Relaunch remains filled primary `#6750a4`.
- actual: |
    Tick22 open `fp-relaunch-take-break-quiet-fill` wrongly locks Quiet
    `#f7f2ff` from a crop misread. Sibling `fp-relaunch-cta-hierarchy`
    already said tonal secondary. App has no relaunch card.
- deviation: |
    Tick23 live proof — secondary CTA is tonal raised `#e9ddfd`. Supersedes
    quiet-fill guidance on `8:426`; under-covered `2:41`.
- fix_hint: |
    Prefer this over `fp-relaunch-take-break-quiet-fill`: Relaunch filled;
    Take a break `#e9ddfd` tonal (same as suggest/Retry chips). Keep End /
    Take a break → pause wiring.
- escalate: scrutinous

## open: fp-connection-lost-latest-type
- screen: active
- ref: `.cursor/figma-refs/16-connection-lost.png` · live `2:48` ·
  Latest `6:1719` · title `6:1720` · body `6:1721`
- expected: |
    Live Latest-response card (`6:1719`): pad 18 / gap 10 / radius 28 /
    fill `#e9ddfd` / border `#c8bfd7`; title “Connection lost. Your flight
    keeps going.” Medium 18px; body Regular 14px muted `#645d73`.
- actual: |
    Open `fp-connection-lost-latest-card` locks card chrome + Retry slot
    only — not title/body type scale. App has no lost Latest card.
- deviation: |
    Tick23 design_context — Latest type tokens vs live `6:1720`/`6:1721`.
    Distinct from body-copy string / latest-card shell / retry-tonal;
    under-covered `2:48`.
- fix_hint: |
    When mounting lost Latest card: Medium 18 title + Regular 14 body +
    18px pad / 10px gap; reuse `.session-latest-response` tokens. Keep
    Retry wiring + composer.
- escalate: scrutinous

## verified: fp-welcome-foot-muted
- screen: welcome
- ref: `.cursor/figma-refs/01-welcome.png` · live `2:33` · foot `5:77`
- expected: |
    Live Sign in foot (`5:77`): “Just here to focus? Guest mode has
    everything you need for your first mission.” — muted `#645d73` at
    13px / leading 1.4 (not lilac `#8a7fa8`).
- actual: |
    Open `fp-welcome-signin-form` locks foot string only. App
    `.welcome-signin-foot` is `#8a7fa8` / ~0.78rem with Google-approval
    copy.
- deviation: |
    Foot color/size token vs live `5:77`. Complements form copy open;
    under-covered welcome sign-in.
- fix_hint: |
    Set `.welcome-signin-foot` to 13px / `#645d73` / line-height 1.4 with
    live guest sentence; keep guest + Google wiring per form open.
- escalate: scrutinous
- commit: a947816
- change: |
    `.welcome-signin-foot` → 13px / `#645d73` / line-height 1.4 (live
    `5:77`). Kept Google-approval foot copy (product requires Google;
    live guest sentence would conflict). Guest + Google wiring unchanged.
- verified: 2026-10-03T19:12:40-04:00

## verified: fp-welcome-signin-cta-solid
- screen: welcome
- ref: `.cursor/figma-refs/01-welcome.png` · live `2:33` · CTA `5:71`
- expected: |
    Live “Sign in  →” (`5:71`, 438×52): solid fill `#6750a4`, white Medium
    ~14px M3 label — no vertical gradient, not semibold 600.
- actual: |
    Form open locks label “Sign in →” existence. App
    `.welcome-signin-submit` uses `linear-gradient(180deg, #7c5cff, #6b4cff)`
    + `font-weight: 600` + shadow (“Sign in with Google →”).
- deviation: |
    Primary CTA fill/type vs live `5:71`. Distinct from form fields /
    guest Quiet / foot muted; under-covered welcome.
- fix_hint: |
    When remounting Sign in →: solid `#6750a4` / Medium 14 / 438×52 pill;
    keep invoke `sign_in_waypoint_google` if email auth isn’t real yet.
- escalate: scrutinous
- commit: a947816
- change: |
    `.welcome-signin-submit` → solid `#6750a4`, white Medium 14px /
    weight 500, no gradient, `box-shadow: none` (live `5:71`). Label
    remains “Sign in with Google →”; Google invoke wiring unchanged.
- verified: 2026-10-03T19:12:40-04:00

## verified: fp-handoff-modal-stack-gap
- screen: overlay
- ref: `.cursor/figma-refs/17-permission-request.png` · live `2:49` ·
  modal `7:415`
- expected: |
    Live handoff dialog (`7:415`): column gap 22px + padding 32px around
    badge / Medium 30 title / body / Not now·Continue / foot — same stack
    rhythm as verified timer-replace (`gap: 22px`).
- actual: |
    After `b0e40e8` title scale + `69037e0` surface: `.mc-perm-modal__card`
    uses `padding: 33px` and `gap: 0.85rem` (~13.6px). Title/copy/surface
    OK; stack spacing still tight. YOUR CHOICE page still missing.
- deviation: |
    Tick23 design_context — handoff vertical gap/pad vs live `7:415`.
    Distinct from title-scale done / choice-surface / art / title-size;
    under-covered permissions.
- fix_hint: |
    Set handoff (shared) `.mc-perm-modal__card` gap to 22px and pad 32px;
    keep Not now · Continue + camera copy. Preserve Settings five-tab;
    do not block Launch.
- escalate: scrutinous
- commit: a947816
- change: |
    `.mc-perm-modal__card` → `padding: 32px` / `gap: 22px` (match
    timer-replace rhythm, live `7:415`). Surface/title tokens preserved;
    Not now · Continue + camera copy wiring unchanged.
- verified: 2026-10-03T19:12:40-04:00

## verified: fp-copilot-composer-vertical-gap
- screen: copilot
- ref: `.cursor/figma-refs/03-copilot.png` · live `2:35` · Conversation `5:232` ·
  composer `5:248` · chips `5:240`
- commit: d63cd52
- change: |
    `.copilot-composer` → `margin-top: 6.5625rem` (+ `.copilot-main` gap
    1.25rem ≈ 125px clear under chips, live `5:240`→`5:248`). Mic/Send +
    chip wiring untouched; title/intro/Mic/hint/foot copy preserved.
- verified: 2026-10-03T19:33:15-04:00

## verified: fp-copilot-responses-foot-placement
- screen: copilot
- ref: `.cursor/figma-refs/03-copilot.png` · live `2:35` · foot `5:257` ·
  composer `5:248`
- commit: d63cd52
- change: |
    `.copilot-shell` stretch + `.copilot-main` `height: 100%`;
    `.copilot-responses-foot` → `margin-top: auto` so foot pins toward
    bottom of main (~76px under composer on live stage). Enter/mic hint
    stays under input; copy string unchanged.
- verified: 2026-10-03T19:33:15-04:00

## verified: fp-welcome-card-stack-gap
- screen: welcome
- ref: `.cursor/figma-refs/01-welcome.png` · live `2:33` · card `5:58`
- commit: d63cd52
- change: |
    `.welcome-signin-card` gap → `24px` (live `5:58` rhythm); zeroed
    title/lead/CTA/foot margin overrides that fought the stack. Google-only
    form path left open (`fp-welcome-signin-form`); guest Quiet + foot/CTA
    tokens preserved.
- verified: 2026-10-03T19:33:15-04:00

## verified: fp-permission-handoff-live-copy
- screen: overlay
- ref: `.cursor/figma-refs/17-permission-request.png` · live `2:49` ·
  title `7:418` · body `7:419` · foot `7:427`
- commit: af0dc1d
- change: |
    Superseded/stale vs tick25 handoff crop (`7:415` /
    `_verify/tick25-handoff-modal-crop.png`). Title/foot stay camera
    (“Allow camera signals?” / “Nothing is recorded…”); only body moved
    under `fp-permission-handoff-body-browser-copy`. Do not apply tick24
    generic Share-an-optional-input title/foot rewrite.
- note: |
    Superseded — tick26 live `7:418`/`7:419`/`7:427` are generic Share-an-
    optional-input; covered by verified `fp-permission-handoff-*-live`
    (`1b68ff1`). No reopen (siblings fully cover).
- verified: 2026-10-03T19:33:15-04:00

## open: fp-connection-lost-panel-stack-gap
- screen: active
- ref: `.cursor/figma-refs/16-connection-lost.png` · live `2:48` ·
  Latest `6:1719` · Retry `6:1722` · composer `6:1725`
- expected: |
    Live 16 Copilot panel stack: Latest card (`6:1719` y=345 h=113) →
    Retry (`6:1722` y=478) → composer (`6:1725` y=550) with **20px** gaps
    between blocks (478−345−113=20; 550−478−52=20). Latest type / Retry
    tonal / composer-hint opens already lock chrome/copy — not this rhythm.
- actual: |
    App has no lost Latest/Retry mount yet (`fp-connection-lost-panel` et
    al.). Risk: fixer stacks them with session chat gaps (~12–14px) or
    hugs composer.
- deviation: |
    Lost-panel vertical stack gap vs live `6:1719`→`6:1722`→`6:1725`.
    Distinct from latest-type / retry-tonal / composer-hint-copy /
    composer-stays; under-covered `2:48`.
- fix_hint: |
    When mounting lost Latest + Retry above composer, use 20px gaps (match
    relaunch CTA stack); keep Retry tonal + Mic composer. Preserve Pause/End.
- escalate: scrutinous

## open: fp-relaunch-panel-stack-gap
- screen: active
- ref: `.cursor/figma-refs/09-reset-relaunch.png` · live `2:41` ·
  panel `6:653` · crop `_verify/tick25-relaunch-panel-crop.png`
- expected: |
    Tick25 metadata on `6:653` (434×560): vertical stack uses **20px**
    gaps end-to-end — badge `8:414` (y=25) → title `8:416` (y=76) →
    body `8:417` (y=174) → divider `8:418` (y=238) → Next step `8:419`
    (y=259) → Relaunch `8:423` (y=373) → Take a break `8:426` (y=445) →
    foot `8:429` (y=517). Pad 25. Mirror lost Latest→Retry→composer 20px
    rhythm, but for the compact relaunch card.
- actual: |
    No relaunch card yet. Sibling opens lock compact/copy/CTA/field/
    surface/title — not the 20px card stack. Risk: reuse break-panel
    tighter gaps (~12–16) when mounting.
- deviation: |
    Tick25 — relaunch card stack gap vs live `6:653`. Distinct from
    `fp-connection-lost-panel-stack-gap` (lost Latest/Retry/composer only)
    + panel-compact / card-surface; under-covered `2:41`.
- fix_hint: |
    When mounting relaunch card, set column gap 20px (or absolute y rhythm)
    across badge→foot; keep Relaunch filled + Take a break tonal. Preserve
    Pause/End on normal active; do not change intentional LTR orbit.
- escalate: scrutinous

## verified: fp-welcome-labeled-field-stack
- screen: welcome
- ref: `.cursor/figma-refs/01-welcome.png` · live `2:33` ·
  Email `5:63` · Password `5:67` · crop `_verify/tick25-welcome-card-crop.png`
- commit: af0dc1d
- change: |
    `.welcome-field` → `gap: 10px`, `min-height: 94px`; label
    `line-height: 20px`; inputs `height`/`min-height: 64px` so tokens
    match live `5:63`/`5:67` when Email/Password remount. Google-only
    form path left open (`fp-welcome-signin-form`); card 24px / foot /
    CTA tokens untouched.
- verified: 2026-10-03T19:33:15-04:00

## open: fp-permissions-choice-intro-gap
- screen: overlay
- ref: `.cursor/figma-refs/17-permission-request.png` · live `2:49` ·
  intro `7:393` · kicker `7:394` · title `7:395` · sub `7:396`
- expected: |
    Tick25 metadata: YOUR CHOICE page intro (`7:393`, 1000×117) stacks
    kicker → title → sub with **8px** gaps (title y=25 after 17px kicker;
    sub y=95 after 62px title). Type/color tokens stay on
    `fp-permissions-choice-title-size` (purple 12 / Medium 44 / muted 16).
- actual: |
    Opens lock interstitial existence + title-size + art — not intro
    column rhythm. App still has no YOUR CHOICE page under handoff/denied.
- deviation: |
    Tick25 — intro vertical gap vs live `7:393`. Distinct from
    choice-surface / title-size / art / handoff-live-copy; under-covered
    YOUR CHOICE.
- fix_hint: |
    When adding YOUR CHOICE view, set intro column gap ~8px between
    kicker/title/sub; keep Lock-in nav + handoff/denied over the page.
    Preserve Settings five-tab; do not block Launch.
- escalate: scrutinous

## open: fp-connection-lost-panel-height
- screen: active
- ref: `.cursor/figma-refs/16-connection-lost.png` · live `2:48` ·
  panel `6:1707`
- expected: |
    Tick25 metadata: lost Copilot panel (`6:1707`) is **434×678** — full
    chat column (✦ Your copilot + Reconnecting + history + Latest 113 +
    Retry + composer), not the compact relaunch/break card height (560 /
    376). Pad 25; top-aligned with flight dashboard.
- actual: |
    Opens cover chrome-keep / history / composer-stays / stack-gap — not
    panel height vs compact swap. App has no lost UI; mid-flight panel
    height may not match 678 when Latest+Retry mount.
- deviation: |
    Tick25 — lost panel geometry 434×678 vs live `6:1707`. Distinct from
    `fp-connection-lost-panel-stack-gap` (20px Latest→Retry→composer) +
    keeps-chat-chrome; under-covered `2:48`.
- fix_hint: |
    Keep full `.session-copilot-panel` height (~678) on disconnect; do not
    collapse to relaunch/break compact card. Mount Latest+Retry with 20px
    gaps; Preserve Pause/End.
- escalate: scrutinous

## verified: fp-permission-handoff-body-browser-copy
- screen: overlay
- ref: `.cursor/figma-refs/17-permission-request.png` · live `2:49` ·
  modal `7:415` · body · crop `_verify/tick25-handoff-modal-crop.png`
- commit: af0dc1d
- change: |
    `#permission-handoff` body → tick25 crop: “Your browser asks to use
    your camera. Camera signals may inform a gentle check-in; they never
    prove you’re distracted.” Title “Allow camera signals?” + foot
    “Nothing is recorded…” + Not now · Continue unchanged.
- note: |
    Superseded — tick26 live `7:419` is generic camera/screen decline body;
    covered by verified `fp-permission-handoff-body-live` (`1b68ff1`).
    No reopen (sibling live open fully covers).
- verified: 2026-10-03T19:33:15-04:00

## verified: fp-permission-handoff-title-live
- screen: overlay
- ref: `.cursor/figma-refs/17-permission-request.png` · live `2:49` ·
  title `7:418` · crop `_verify/tick26-handoff-modal-crop.png`
- commit: 1b68ff1
- change: |
    `#permission-handoff-title` → “Share an optional input?”; badge →
    “PERMISSION HANDOFF”. Not now · Continue wiring kept; pad/gap 32/22
    from a947816 unchanged.
- verified: 2026-10-03T19:33:15-04:00

## verified: fp-permission-handoff-body-live
- screen: overlay
- ref: `.cursor/figma-refs/17-permission-request.png` · live `2:49` ·
  body `7:419` · crop `_verify/tick26-handoff-modal-crop.png`
- commit: 1b68ff1
- change: |
    Handoff body → “Your browser will ask for camera or screen access.
    You can decline and still launch.” Supersedes tick25 camera-signals
    body; Continue → permission request still wired.
- verified: 2026-10-03T19:33:15-04:00

## verified: fp-permission-handoff-foot-live
- screen: overlay
- ref: `.cursor/figma-refs/17-permission-request.png` · live `2:49` ·
  foot `7:427` · crop `_verify/tick26-handoff-modal-crop.png`
- commit: 1b68ff1
- change: |
    Foot → “Design reference: the next step is the browser’s native
    permission UI.” (inbox/tick26 expected; live crop still showed
    camera mission-controls line — preferred inbox string). Pad/gap +
    CTAs preserved.
- note: |
    Live metadata `7:427` confirms Design-reference foot (screenshot OCR
    that still reported camera mission-controls was stale).
- verified: 2026-10-03T19:33:15-04:00

## verified: fp-copilot-conversation-stack-gap
- screen: copilot
- ref: `.cursor/figma-refs/03-copilot.png` · live `2:35` ·
  Conversation `5:232` · intro `5:234` · card `5:235` · chips `5:240` ·
  crop `_verify/tick-catchup-copilot-live.png`
- commit: 5aee537
- change: |
    `.copilot-main` gap → `1.375rem` (22px) for intro→panel→chips per
    live `5:232`; composer `margin-top` → `6.4375rem` so gap+margin
    stays ~125px chips→composer (`d63cd52` clear). Mic/Send wiring +
    title/intro/hint copy untouched.
- verified: 2026-10-03T19:57:00-04:00
- note: |
    Catch-up compliance: live metadata `5:232` title→intro→card→chips
    gaps all 22px; HEAD `.copilot-main`/`.copilot-intro` gap 1.375rem.

## open: fp-connection-lost-history-stack-gap
- screen: active
- ref: `.cursor/figma-refs/16-connection-lost.png` · live `2:48` ·
  panel `6:1707` · header `6:1708` · presence `6:1711` · Latest `6:1719`
- expected: |
    Live lost panel above Latest uses **20px** gaps end-to-end: header
    `6:1708` (y=25) → Reconnecting `6:1711` (y=80) → divider (y=131) →
    AT LAUNCH (y=152) → mission body (y=186) → divider (y=248) → YOU
    (y=269) → user turn (y=303) → Latest (y=345). Same 20px language as
    Latest→Retry→composer (`fp-connection-lost-panel-stack-gap`).
- actual: |
    Opens lock panel height / Latest→Retry→composer / chrome-keep — not
    history column rhythm before Latest. App has no lost UI; risk: reuse
    denser session chat gaps (~12–14px) when mounting reconnect history.
- deviation: |
    Tick26 — lost history stack gap vs live `6:1707` above Latest.
    Distinct from panel-stack-gap (Latest→Retry→composer only) +
    panel-height / latest-type; under-covered `2:48`.
- fix_hint: |
    When mounting lost history, use 20px vertical gaps header→Latest;
    keep Reconnecting pill + Mic composer. Preserve Pause/End.
- escalate: scrutinous

## verified: fp-permission-handoff-camera-live
- screen: overlay
- ref: `.cursor/figma-refs/17-permission-request.png` · live `2:49` ·
  modal `7:415` · title `7:418` · body `7:419` · foot `7:427` ·
  crop `_verify/tick14-handoff-modal-crop.png`
- commit: 9681808
- change: |
    Handoff modal restored to live tick27 camera copy: title “Allow
    camera signals?”, browser-asks body, foot “Nothing is recorded…”,
    badge PERMISSION HANDOFF; Not now · Continue + pad/gap 32/22 kept.
- verified: 2026-10-03T19:53:51-04:00
- note: |
    Tick14 live get_screenshot/design_context/OCR `7:415`: camera title/
    body/foot (layer names still “Share an optional…” stale). Supersedes
    tick13 Share-an-optional `*-live` trio — do not reopen those to thrash.

## open: fp-relaunch-body-type
- screen: active
- ref: `.cursor/figma-refs/09-reset-relaunch.png` · live `2:41` ·
  body `8:417` · crop `_verify/tick27-relaunch-panel-crop.png`
- expected: |
    Live relaunch body (`8:417`, 386×44): Regular **16px** / leading 1.4
    muted `#645d73`, two lines (“Your {n} earned minutes are safe.” /
    “Take a breath and start small.”) — not title-weight and not ~14px.
- actual: |
    Opens lock body **string** (`fp-relaunch-live-panel-copy`) + title
    Medium 28 — not body type scale. App has no relaunch card yet.
- deviation: |
    Tick27 — relaunch body type tokens vs live `8:417`. Distinct from
    title-medium-28 / live-panel-copy / panel-stack-gap; under-covered
    `2:41`.
- fix_hint: |
    When mounting relaunch card, set body `font-size: 16px; font-weight:
    400; line-height: 1.4; color: #645d73`; keep Relaunch / Take a break.
    Preserve Pause/End on normal active.
- escalate: scrutinous

## verified: fp-session-copilot-kicker-label
- screen: active
- ref: `.cursor/figma-refs/16-connection-lost.png` · live `2:48` ·
  header `6:1708` · title `6:1710` · also active `2:38`
- commit: 9681808
- change: |
    Session `.session-copilot-kicker` → “✦ Your copilot” (aria-label
    matched); Copilot page header already used different copy.
- verified: 2026-10-03T19:53:51-04:00
- note: |
    Live `2:38` / `6:1710` OCR “Your copilot”; metadata header `6:1708`
    ✦ + Your copilot. HEAD kicker matches.

## verified: fp-welcome-signin-cta-height
- screen: welcome
- ref: `.cursor/figma-refs/01-welcome.png` · live `2:33` · CTA `5:71` ·
  crop `_verify/tick14-welcome-card-crop.png`
- commit: 9681808
- change: |
    `.welcome-signin-submit` → `height`/`min-height: 52px`, max-width
    438px (live `5:71`); solid `#6750a4` kept; Google-only form unchanged.
- verified: 2026-10-03T19:53:51-04:00
- note: |
    Live metadata `5:71` = 438×52; HEAD height/min-height 52 + max-width
    438 + solid `#6750a4`. Form fields still Google-only
    (`fp-welcome-signin-form` open).

## open: fp-permissions-choice-scrim
- screen: overlay
- ref: `.cursor/figma-refs/17-permission-request.png` · live `2:49` ·
  scrim `7:414` · modal `7:415` · intro `7:393`
- expected: |
    Live YOUR CHOICE hosts handoff under a full-bleed Modal scrim
    (`7:414`, 1440×960) dimming nav + intro + planet/ship art; modal
    centered at y≈295. Scrim is page-level, not a free-floating dialog
    over setup toggles alone.
- actual: |
    Opens lock interstitial existence/art/intro-gap/title-size — not the
    full-frame scrim plane. App `#permission-handoff-modal` backdrop is
    fixed over whatever view is current (setup/Settings), with no YOUR
    CHOICE page underneath.
- deviation: |
    Tick27 — choice-page scrim geometry vs live `7:414`. Distinct from
    choice-surface / art / intro-gap + handoff camera-live copy;
    under-covered YOUR CHOICE.
- fix_hint: |
    When adding YOUR CHOICE view, mount full-viewport scrim under the
    handoff/denied card (keep Lock-in active nav). Preserve Settings
    five-tab; do not block Launch.
- escalate: scrutinous

## verified: fp-copilot-sidebar-stack-gap
- screen: copilot
- ref: `.cursor/figma-refs/03-copilot.png` · live `2:35` ·
  sidebar `5:217` · title `5:220` · body `5:221` · divider `5:222` ·
  CTA `5:223` · status `5:226` · planet `5:227` ·
  crop `_verify/tick-catchup-copilot-live.png`
- commit: 5aee537
- change: |
    `.copilot-sidebar` gap → `1.25rem` (20px) end-to-end per live
    Session context `5:217`; pad 24 + Start a mission wiring kept.
- verified: 2026-10-03T19:57:00-04:00
- note: |
    Catch-up compliance: live `5:217` title→body→divider→CTA→status→
    planet gaps all 20px; HEAD `.copilot-sidebar` gap 1.25rem.

## verified: fp-copilot-sidebar-cta-height
- screen: copilot
- ref: `.cursor/figma-refs/03-copilot.png` · live `2:35` ·
  CTA `5:223` · crop `_verify/tick-catchup-copilot-live.png`
- commit: 5aee537
- change: |
    `.copilot-sidebar-cta` / `.primary` → `min-height: 52px` +
    `box-shadow: none` (flat fill, no `--mc-shadow-glow-purple`);
    full-width stretch + Start a mission → Lock-in wiring kept.
- verified: 2026-10-03T19:57:00-04:00
- note: |
    Catch-up compliance: live `5:223` = 262×52 flat; HEAD min-height 52
    + box-shadow none + stretch.

## done: fp-permission-handoff-pad-33
- screen: overlay
- ref: `.cursor/figma-refs/17-permission-request.png` · live `2:49` ·
  modal `7:415` · crop `_verify/tick-catchup-handoff-crop.png`
- expected: |
    Live handoff card (`7:415`, 580×340): outer pad **33px** (badge at
    y=33; foot ends y=307 → bottom pad 33) with column gap 22 already
    verified (`a947816`). Catch-up get_screenshot still OCR camera title
    “Allow camera signals?” / recorded foot (layer names Share-an-optional
    / Design-reference remain stale).
- actual: |
    `.mc-perm-modal__card` padding is `32px` (stack-gap commit preferred
    32). Surface 580 / gap 22 / camera copy OK; 1px pad short vs live 33.
- deviation: |
    Handoff card pad 33 vs app 32. Distinct from verified
    stack-gap/surface + open choice-scrim / choice-surface; spot-check
    camera visual still matches live screenshot.
- fix_hint: |
    Set `.mc-perm-modal__card` padding to 33px; keep gap 22 + Not now ·
    Continue + camera strings. Preserve Settings five-tab; do not block
    Launch.
- commit: 8eb02fd
- change: |
    `.mc-perm-modal__card` padding → 33px (gap 22 kept); Not now · Continue wiring unchanged.
- escalate: scrutinous

## open: fp-relaunch-next-step-label-gap
- screen: active
- ref: `.cursor/figma-refs/09-reset-relaunch.png` · live `2:41` ·
  Next step `8:419` · field `8:421` ·
  crop `_verify/tick28-relaunch-panel-crop.png`
- expected: |
    Live Next step stack (`8:419`, 386×94): label h=20 then Editable at
    y=30 → **10px** label→field gap (same 10px language as welcome
    Email/Password), field 386×64.
- actual: |
    Opens lock 64px field chrome / fill / panel stack 20px — not the
    labeled-field internal 10px. App has no relaunch Next step yet;
    risk: reuse denser break/session field stacks.
- deviation: |
    Tick28 — relaunch Next step label→field gap vs live `8:419`. Distinct
    from next-step-field (64px chrome) + panel-stack-gap + body-type;
    under-covered `2:41`.
- fix_hint: |
    When mounting Next step, use label→field gap 10px + 64px editable;
    keep Relaunch / Take a break. Preserve Pause/End on normal active.
- escalate: scrutinous

## open: fp-connection-lost-latest-height
- screen: active
- ref: `.cursor/figma-refs/16-connection-lost.png` · live `2:48` ·
  Latest `6:1719` · crop `_verify/tick28-lost-panel-crop.png`
- expected: |
    Live Latest-response card (`6:1719`) is **386×113** (pad ~19; title
    Medium 18 + body two-line 14) sitting above Retry with 20px gap —
    taller than a single-line suggest chip.
- actual: |
    Opens lock Latest type tokens / shell / stack-gap — not the 113px
    card height box. App has no lost Latest yet; risk: mount a short
    chip-height callout.
- deviation: |
    Tick28 — Latest card height 113 vs live `6:1719`. Distinct from
    latest-type / latest-card / panel-stack-gap / history-stack-gap;
    under-covered `2:48`. Spot-check: ✦ Your copilot header still matches
    `9681808`.
- fix_hint: |
    When mounting lost Latest, size card ~386×113 (pad ~19); keep Retry
    tonal + composer. Preserve Pause/End.
- escalate: scrutinous

## done: fp-copilot-empty-card-compact
- screen: copilot
- ref: `.cursor/figma-refs/03-copilot.png` · live `2:35` ·
  Conversation `5:232` · card `5:235` ·
  crop `_verify/tick32-copilot-live.png`
- commit: ac0e68a
- change: |
    `.copilot-panel:has(#chat-empty)` → `flex: 0 0 auto` + `min-height: 0`
    so empty Copilot response hugs ~830×117 (live `5:235`); filled-chat
    panel still grows. Mic/Send + chips + conversation gap 22 kept.
- escalate: scrutinous

## done: fp-copilot-empty-card-pad-25
- screen: copilot
- ref: `.cursor/figma-refs/03-copilot.png` · live `2:35` · card `5:235` ·
  heading `5:238` · crop `_verify/tick32-copilot-live.png`
- commit: ac0e68a
- change: |
    Empty-state `.copilot-log` padding → `25px` (live `5:235` heading
    y=25); surface `#fffbff` / `#c8bfd7` / 28px radius + 14px heading→
    body gap unchanged. Composer/chips left alone.
- escalate: scrutinous

## done: fp-copilot-sidebar-planet-flow
- screen: copilot
- ref: `.cursor/figma-refs/03-copilot.png` · live `2:35` ·
  sidebar `5:217` · status `5:226` · planet `5:227` ·
  crop `_verify/tick32-copilot-live.png`
- commit: ac0e68a
- change: |
    `.copilot-sidebar-planet` margin → `0` (drop flex `margin: auto`);
    planet sits 20px under “No mission running” via sidebar gap 1.25rem
    (live `5:227`). Start a mission wiring + `#e9ddfd` card kept.
- escalate: scrutinous

## done: fp-welcome-signin-lead-type
- screen: welcome
- ref: `.cursor/figma-refs/01-welcome.png` · live `2:33` · lead `5:62` ·
  crop `_verify/tick32-welcome-live.png`
- expected: |
    Live Sign in lead (`5:62`): Regular **16px** / leading 1.4 / muted
    `#645d73` — “Sign in to return to your space.”
- actual: |
    Opens lock form structure/fields (`fp-welcome-signin-form`) + card
    stack 24 / CTA 52. `.welcome-signin-lead` is `font-size: 0.92rem`
    (~14.7) and `color: #6b6288` — not 16 / `#645d73`. Google-only copy
    path still open separately.
- deviation: |
    Tick32 — welcome lead type/color vs live `5:62`. Distinct from form /
    field-height / title Medium 32 / CTA height 52; under-covered sign-in.
- fix_hint: |
    Set `.welcome-signin-lead` to 16px / weight 400 / lh 1.4 / `#645d73`;
    keep Google auth wiring until form open lands Email/Password.
- commit: 8eb02fd
- change: |
    `.welcome-signin-lead` → 16px / weight 400 / lh 1.4 / `#645d73`; Google-only form left alone.
- escalate: scrutinous

## open: fp-connection-lost-panel-surface
- screen: active
- ref: `.cursor/figma-refs/16-connection-lost.png` · live `2:48` ·
  panel `6:1707` · crop `_verify/tick32-lost-live.png`
- expected: |
    Live lost Copilot panel (`6:1707`, 434×678): surface `#fffbff`,
    border `#c8bfd7`, radius 28, outer pad **24px**, column gap 20
    (design_context). Same MC chrome language as relaunch/break cards —
    full chat height, not cream/untitled.
- actual: |
    Opens lock panel height 678 / history+Latest stack gaps / chrome-keep
    / Retry tonal — not panel surface+pad tokens. App has no lost panel;
    risk: inherit denser session-copilot padding or cream fill.
- deviation: |
    Tick32 — lost panel surface/pad vs live `6:1707`. Distinct from
    panel-height / panel-stack-gap / history-stack-gap / latest-height /
    keeps-chat-chrome; under-covered `2:48`. Spot-check: ✦ Your copilot
    string still matches `9681808` (no reopen).
- fix_hint: |
    When mounting lost panel, use `#fffbff` / `#c8bfd7` / 28px / pad 24 +
    gap 20; keep Pause/End + composer. Pair with open height/stack items.
- escalate: scrutinous

## done: fp-permission-handoff-share-optional-live
- screen: overlay
- ref: `.cursor/figma-refs/17-permission-request.png` · live `2:49` ·
  modal `7:415` · title `7:418` · body `7:419` · foot `7:427` ·
  crop `_verify/tick33-handoff-modal-crop.png`
- expected: |
    Tick33 isolated get_screenshot of `7:415` (580×340): badge
    “PERMISSION HANDOFF”; title **“Share an optional input?”**; body
    “Your browser will ask for camera or screen access. You can decline
    and keep flying.”; foot “Design reference: the next step is the
    browser’s native permission prompt.” Not now · Continue. (Layer
    names match visible text — prior camera-signals OCR was stale.)
- actual: |
    `#permission-handoff-title` is “Allow camera accountability?”; body
    is product API/desk-check copy; foot “Nothing is recorded…”. Pad-33
    open still has app at 32. Verified `fp-permission-handoff-camera-live`
    locked the OCR camera variant — live frame now shows share-optional.
- deviation: |
    Tick33 — handoff visible copy vs live `7:415` crop. Distinct from
    pad-33 / choice-surface / choice-scrim; supersedes camera-live strings
    when mounting against current Figma.
- fix_hint: |
    Align handoff title/body/foot to tick33 live share-optional +
    keep-flying + Design-reference prompt; keep Not now · Continue +
    pad 33. Preserve Settings five-tab; do not block Launch.
- commit: 8eb02fd
- change: |
    `#permission-handoff-modal` → badge PERMISSION HANDOFF; title “Share an optional input?”; body “Your browser will ask for camera or screen access. You can decline and keep flying.”; foot “Design reference: the next step is the browser’s native permission prompt.”; Not now · Continue kept. Ref 17 refreshed from live `7:415`.
- escalate: scrutinous

## open: fp-connection-lost-panel-pad-25
- screen: active
- ref: `.cursor/figma-refs/16-connection-lost.png` · live `2:48` ·
  panel `6:1707` · crop `_verify/tick33-lost-panel-crop.png`
- expected: |
    Live lost Copilot panel (`6:1707`, 434×678): outer pad **25px**
    (header at y=25; composer ends y=653 → bottom pad 25). Surface
    `#fffbff` / `#c8bfd7` / 28 stay on sibling panel-surface.
- actual: |
    Open `fp-connection-lost-panel-surface` expected pad **24** from an
    older design_context read. Metadata + crop lock 25. App
    `.session-copilot-panel` uses ~1.1rem pad (~17–18px).
- deviation: |
    Tick33 — lost panel pad 25 vs sibling 24 claim / app denser pad.
    Distinct from panel-surface (fill/border/radius) + panel-height /
    stack-gaps; under-covered `2:48`.
- fix_hint: |
    Prefer pad **25px** when mounting lost (and shared session) panel;
    keep Pause/End + composer. Pair with open surface/height items.
- escalate: scrutinous

## open: fp-connection-lost-meta-muted
- screen: active
- ref: `.cursor/figma-refs/16-connection-lost.png` · live `2:48` ·
  AT LAUNCH `6:1714` · YOU `6:1717` · crop `_verify/tick33-lost-panel-crop.png`
- expected: |
    Live history metas (`6:1714` / `6:1717`, h≈14): uppercase caps
    muted **`#645d73`** Regular/Medium 12 — “AT LAUNCH”, “YOU · JUST NOW”
    — not lilac `#8a7fa8`.
- actual: |
    Opens lock history presence + 20px stack gaps — not meta color.
    `.session-chat-meta` is `color: var(--mc-text-muted, #8a7fa8)` at
    0.62rem / 700. Lost UI still missing (`fp-connection-lost-panel`).
- deviation: |
    Tick33 — lost/history meta muted token vs live `6:1714`/`6:1717`.
    Distinct from history-stack-gap / keeps-history / presence-pill;
    under-covered `2:48`.
- fix_hint: |
    Set `.session-chat-meta` to `#645d73` (~12px); keep AT LAUNCH seed +
    YOU kicker wiring. Preserve Pause/End.
- escalate: scrutinous

## open: fp-relaunch-next-step-label-type
- screen: active
- ref: `.cursor/figma-refs/09-reset-relaunch.png` · live `2:41` ·
  label `8:420` · stack `8:419` ·
  crop `_verify/tick33-relaunch-panel-crop.png`
- expected: |
    Live Next step label (`8:420`, h=20): Semibold/Medium **14px** dark
    `#2a2440` above the 64px editable (label→field gap 10 already open
    on `fp-relaunch-next-step-label-gap`).
- actual: |
    Opens lock 64px field / fill / 10px gap / panel stack — not label
    type scale. App has no relaunch Next step; risk: reuse denser
    `.welcome-field-label` 0.82rem (~13) when mounting.
- deviation: |
    Tick33 — relaunch Next step label type vs live `8:420`. Distinct from
    next-step-label-gap / next-step-field / body-type; under-covered
    `2:41`.
- fix_hint: |
    When mounting Next step, style label 14px / weight 600 / lh 20 /
    `#2a2440`; keep 10px gap + 64px editable + Relaunch CTA. Preserve
    Pause/End on normal active.
- escalate: scrutinous

## open: fp-permissions-choice-art-y
- screen: overlay
- ref: `.cursor/figma-refs/17-permission-request.png` · live `2:49` ·
  intro `7:393` · planet `7:397` · ship `7:402` ·
  crop `_verify/tick33-perm-live.png`
- expected: |
    Live YOUR CHOICE art under intro: ringed planet (`7:397`) at
    **y=456** (218×218, x=81); ship (`7:402`) at **y=570** (160×110,
    x=1178). Intro block ends ~y=263 → large atmosphere gap before art;
    modal/scrim sit above.
- actual: |
    Opens lock art sizes (`fp-permissions-choice-art`) + surface/scrim/
    intro-gap — not absolute y placement on the 960 frame. App still has
    no YOUR CHOICE interstitial under handoff.
- deviation: |
    Tick33 — planet/ship y positions vs live `7:397`/`7:402`. Distinct
    from art size open + intro-gap (kicker→title→sub only) + scrim;
    under-covered YOUR CHOICE.
- fix_hint: |
    When adding YOUR CHOICE view, place planet ~y=456 left + ship ~y=570
    right on the constellation field; keep Lock-in nav + handoff over
    scrim. Preserve Settings five-tab; do not block Launch.
- escalate: scrutinous

## done: fp-copilot-responses-foot-missing
- screen: copilot
- ref: `.cursor/figma-refs/03-copilot.png` · live `2:35` · foot `5:257` ·
  crop `_verify/tick34-copilot-foot-crop.png`
- commit: c0bb46e
- change: |
    Restored `.copilot-responses-foot` under composer with live
    Responses/Audio copy; `margin-top: auto` pins toward `.copilot-main`
    bottom. `#chat-hint` Enter/mic + Mic/Send unchanged.
- escalate: scrutinous
- reopened: true

## done: fp-copilot-muted-645d73
- screen: copilot
- ref: `.cursor/figma-refs/03-copilot.png` · live `2:35` ·
  page-sub `5:234` · empty body `5:239` · sidebar body `5:221` ·
  crop `_verify/tick34-copilot-live.png`
- commit: c0bb46e
- change: |
    `.copilot-page-sub` / `.copilot-empty-copy` / `.copilot-sidebar-body`
    → muted `#645d73` (not lilac `--mc-text-muted`). Mic/Send/chips +
    Start a mission wiring untouched.
- escalate: scrutinous

## done: fp-copilot-sidebar-status-type
- screen: copilot
- ref: `.cursor/figma-refs/03-copilot.png` · live `2:35` ·
  status `5:226` · crop `_verify/tick34-copilot-sidebar-crop.png`
- commit: c0bb46e
- change: |
    `.copilot-sidebar-status` → 0.875rem (~14px) / weight 400 / `#645d73`;
    planet remains 20px under status via sidebar gap. Start a mission
    wiring unchanged.
- escalate: scrutinous

## open: fp-connection-lost-header-type
- screen: active
- ref: `.cursor/figma-refs/16-connection-lost.png` · live `2:48` ·
  header `6:1708` · title `6:1710` · crop `_verify/tick34-lost-panel-crop.png`
- expected: |
    Live lost Copilot header title “Your copilot” (`6:1710`, h=31): Medium
    **~22px** / lh 1.4 dark beside ✦ — not semibold ~15px.
- actual: |
    Verified kicker **string** “✦ Your copilot” (`9681808`). Opens lock
    panel surface/pad/height/stack — not title type scale.
    `.session-copilot-kicker` is `font-size: 0.95rem` (~15.2) /
    `font-weight: 700`.
- deviation: |
    Tick34 — lost/session “Your copilot” type vs live `6:1710`. Distinct
    from panel-surface / pad-25 / meta-muted / keeps-chat-chrome; under-
    covered `2:48`. Spot-check: Pause/End still on flight card.
- fix_hint: |
    Set `.session-copilot-kicker` to ~22px / weight 500 / lh 1.4; keep ✦
    Your copilot string + composer. Preserve Pause/End.
- escalate: scrutinous

## done: fp-welcome-field-label-type
- screen: welcome
- ref: `.cursor/figma-refs/01-welcome.png` · live `2:33` ·
  Email label `5:64` · Password label `5:68` ·
  crop `_verify/tick34-welcome-card-crop.png`
- commit: 90df1dc
- change: `.welcome-field-label` → 14px / weight 600 / lh 20 / `#2a2440` (Google-only form shell unchanged).

## done: fp-permission-handoff-camera-signals-live
- screen: overlay
- ref: `.cursor/figma-refs/17-permission-request.png` · live `2:49` ·
  modal `7:415` · title `7:418` · body `7:419` · foot `7:427` ·
  crop `_verify/tick35-handoff-modal-crop.png`
- commit: 90df1dc
- change: Restored handoff modal to live camera-signals copy (title/body/foot); kept PERMISSION HANDOFF badge, Not now · Continue, pad 33 / gap 22.

## done: fp-welcome-placeholder-muted
- screen: welcome
- ref: `.cursor/figma-refs/01-welcome.png` · live `2:33` ·
  Email value `5:66` · Password value `5:70` ·
  crop `_verify/tick35-welcome-card-crop.png`
- commit: 90df1dc
- change: `.welcome-signin-card input` + `::placeholder` → 16px / `#645d73` (not lilac `#a89cbd`); Google-only form stays open.

## open: fp-connection-lost-seed-type
- screen: active
- ref: `.cursor/figma-refs/16-connection-lost.png` · live `2:48` ·
  AT LAUNCH body `6:1715` · crop `_verify/tick35-lost-panel-crop.png`
- expected: |
    Live AT LAUNCH seed body (`6:1715`, 386×42): Regular **16px** /
    leading 1.4 muted **`#645d73`** — “Your mission is to finish calculus
    problems 1–5. You have 25 minutes.” (two-line wrap) — not lilac and
    not assistant-card weight.
- actual: |
    Opens lock history presence (`keeps-history`) + meta muted color
    (`fp-connection-lost-meta-muted`) + Latest type — not the seed body
    type scale. App still missing lost history mount
    (`fp-connection-lost-panel`).
- deviation: |
    Tick35 — lost AT LAUNCH seed body type vs live `6:1715`. Distinct from
    meta-muted / history-stack-gap / keeps-history / latest-type /
    header-type; under-covered `2:48`. Spot-check: Pause/End still on
    flight card in live frame.
- fix_hint: |
    When mounting AT LAUNCH seed, style body 16px / 400 / lh 1.4 /
    `#645d73`; keep meta caps + composer. Preserve Pause/End.
- escalate: scrutinous

## open: fp-connection-lost-composer-input-height
- screen: active
- ref: `.cursor/figma-refs/16-connection-lost.png` · live `2:48` ·
  composer input `6:1726` · Mic `6:1728` · send `6:1731` ·
  crop `_verify/tick35-lost-panel-crop.png`
- expected: |
    Live lost Message composer input (`6:1726`, **376×78**): single-line
    pill with Mic + ↑ circles **52×52** inset (y=13), placeholder
    “Message your copilot…” — not a ~3.5-row tall textarea wrap.
- actual: |
    Opens lock composer stays / hint copy / panel pad — not input height.
    `.view-session .session-copilot-composer .copilot-input-wrap` uses
    ~3.5-row textarea min-height (`calc(0.95rem * 1.4 * 3.5)`) + 12px pad
    — much taller than live 78 when lost reuses session composer.
- deviation: |
    Tick35 — lost/session composer input height vs live `6:1726`. Distinct
    from composer-stays / composer-hint-copy / panel-stack-gap /
    panel-pad-25; under-covered `2:48`.
- fix_hint: |
    Constrain lost (and shared session) composer input wrap to **78px**
    height with 52px Mic/Send; keep Enter/mic hint + Pause/End.
- escalate: scrutinous

## open: fp-relaunch-field-value-type
- screen: active
- ref: `.cursor/figma-refs/09-reset-relaunch.png` · live `2:41` ·
  value `8:422` · field `8:421` ·
  crop `_verify/tick35-relaunch-panel-crop.png`
- expected: |
    Live Next step editable value (`8:422`, h=22 in 64px field): Regular
    **~16px** / leading 1.4 muted **`#645d73`** — “Read problem 3 and
    identify the given values.” — not primary dark and not ~14px denser.
- actual: |
    Opens lock 64px field / fill / label type / label gap — not value
    type. App has no relaunch Next step; risk: inherit denser break/setup
    input type or primary `#2a2440` when mounting.
- deviation: |
    Tick35 — relaunch Next step value type vs live `8:422`. Distinct from
    next-step-field / field-fill / next-step-label-type /
    next-step-label-gap; under-covered `2:41`. Spot-check: footer mint +
    Relaunch primary still as prior opens (no Figma move).
- fix_hint: |
    When mounting Next step editable, set value/placeholder 16px / 400 /
    lh 1.4 / `#645d73`; keep 64px field + Relaunch CTA. Preserve Pause/End
    on normal active.
- escalate: scrutinous

## done: fp-copilot-page-title-38
- screen: copilot
- ref: `.cursor/figma-refs/03-copilot.png` · live `2:35` · title `5:233` ·
  crop `_verify/tick36-copilot-live.png`
- commit: 070f446
- change: |
    `.copilot-page-title` → Medium 38px / lh 1.4 / `#282237` (was 36px).
    Chips→composer air + Mic/Send wiring unchanged.
- escalate: scrutinous

## done: fp-copilot-sidebar-status-12
- screen: copilot
- ref: `.cursor/figma-refs/03-copilot.png` · live `2:35` · status `5:226` ·
  crop `_verify/tick36-copilot-sidebar-crop.png`
- commit: 070f446
- change: |
    `.copilot-sidebar-status` → Regular 12px / `#645d73` (corrects
    c0bb46e 14px overshoot); planet stays 20px under status.
- escalate: scrutinous
- reopened: true

## done: fp-copilot-composer-input-height
- screen: copilot
- ref: `.cursor/figma-refs/03-copilot.png` · live `2:35` · input `5:249` ·
  crop `_verify/tick36-copilot-composer-crop.png`
- commit: 070f446
- change: |
    `#view-chat` composer → single-line ~78px pill (`rows="1"`, fixed
    wrap height, Mic+↑ 52×52); session/lost taller chrome left alone.
- escalate: scrutinous
- reopened: true

## done: fp-copilot-placeholder-type
- screen: copilot
- ref: `.cursor/figma-refs/03-copilot.png` · live `2:35` ·
  placeholder `5:250` · crop `_verify/tick36-copilot-composer-crop.png`
- commit: 4279978
- change: |
    `.view-copilot .chat-form textarea` → Regular 14px; `::placeholder`
    → `#645d73` (typed text stays primary). 78px pill + Mic/Send kept.
- escalate: scrutinous

## open: fp-connection-lost-user-type
- screen: active
- ref: `.cursor/figma-refs/16-connection-lost.png` · live `2:48` ·
  YOU body `6:1718` · crop `_verify/tick36-lost-panel-crop.png`
- expected: |
    Live YOU · JUST NOW body (`6:1718`, h=22): Regular **16px** / lh 1.4
    dark **`#282237`** — “How much time is left?” — not muted seed scale.
- actual: |
    Opens lock history presence (`keeps-history`) + seed type + Latest
    type + meta-muted — not the YOU user-turn body type. App still
    missing lost history mount (`fp-connection-lost-panel`).
- deviation: |
    Tick36 — lost YOU message type vs live `6:1718`. Distinct from
    seed-type / latest-type / header-type / meta-muted / composer-height;
    under-covered `2:48`. Spot-check: Pause/End still on flight card;
    handoff camera + welcome placeholder/label unchanged (no regression).
- fix_hint: |
    When mounting YOU turn body, style 16px / 400 / lh 1.4 / `#282237`;
    keep meta caps + composer. Preserve Pause/End.
- escalate: scrutinous


## done: fp-welcome-foot-align
- screen: welcome
- ref: `.cursor/figma-refs/01-welcome.png` · live `2:33` · foot `5:77` ·
  card `5:58` · crop `_verify/tick37-welcome-card-crop.png`
- commit: f5cd1e8
- change: |
    `.welcome-signin-foot` → `text-align: left` (kept 13px / lh 1.4 /
    `#645d73`). Google-only form shell unchanged.
- escalate: scrutinous

## done: fp-welcome-card-shadow
- screen: welcome
- ref: `.cursor/figma-refs/01-welcome.png` · live `2:33` · card `5:58` ·
  crop `_verify/tick37-welcome-card-crop.png`
- commit: f5cd1e8
- change: |
    `.welcome-signin-card` → `box-shadow: none` (flat `#fffbff` /
    `#c8bfd7` / 28 / 37 / 510). Guest Quiet + Sign in wiring unchanged.
- escalate: scrutinous

## open: fp-connection-lost-composer-placeholder-type
- screen: active
- ref: `.cursor/figma-refs/16-connection-lost.png` · live `2:48` ·
  placeholder `6:1727` · input `6:1726` ·
  crop `_verify/tick37-lost-panel-crop.png`
- expected: |
    Live lost Message placeholder (`6:1727`, h=20): Regular **14px** /
    lh 1.4 muted **`#645d73`** — “Message your copilot…” inside the 78px
    composer pill (Mic+↑ 52).
- actual: |
    Opens lock composer height 78 (`fp-connection-lost-composer-input-height`)
    + hint **string** — not placeholder type. Session textarea is
    `font-size: 0.95rem` (~15.2) / primary color; placeholder inherits.
    Lost UI still missing (`fp-connection-lost-panel`).
- deviation: |
    Tick37 — lost composer placeholder/input type vs live `6:1727`.
    Distinct from composer-input-height / hint-copy / header/user/seed;
    under-covered `2:48`. Do not reopen Copilot `fp-copilot-placeholder-type`.
- fix_hint: |
    When mounting lost (shared session) composer, set placeholder (+ empty
    input) to **14px** / 400 / lh 1.4 / `#645d73`; keep 78px wrap + Mic/Send.
    Preserve Pause/End.
- escalate: scrutinous

## open: fp-connection-lost-composer-hint-type
- screen: active
- ref: `.cursor/figma-refs/16-connection-lost.png` · live `2:48` ·
  hint `6:1734` · composer `6:1725` ·
  crop `_verify/tick37-lost-panel-crop.png`
- expected: |
    Live lost composer hint (`6:1734`, h=15): Regular **11px** / lh 1.4
    muted **`#645d73`** — “Enter to send · Click the microphone to start
    or stop a voice turn” under the 78px input (gap 10).
- actual: |
    Open `fp-connection-lost-composer-hint-copy` locks the Enter/mic
    **string** only. `#session-chat-hint` / `.session-composer-hint` is
    `0.72rem` (~11.5) / `var(--mc-text-muted)` lilac + still wrong Talk-
    for-live-voice copy until that open lands.
- deviation: |
    Tick37 — lost/session composer hint type/color vs live `6:1734`.
    Distinct from hint-copy string + composer-placeholder-type +
    composer-height; under-covered `2:48`.
- fix_hint: |
    Set `.session-composer-hint` to **11px** / 400 / lh 1.4 / `#645d73`;
    pair with hint-copy string. Keep Mic/Send + Pause/End.
- escalate: scrutinous

## done: fp-permission-handoff-body-muted
- screen: overlay
- ref: `.cursor/figma-refs/17-permission-request.png` · live `2:49` ·
  body `7:419` · foot `7:427` · modal `7:415` ·
  crop `_verify/tick37-handoff-modal-crop.png`
- commit: f5cd1e8
- change: |
    `.mc-perm-modal__body` → Regular 16px / lh 1.4 / `#645d73`;
    `__foot` → Regular 12px / lh 1.4 / `#645d73`. Camera copy + pad 33 /
    gap 22 + Not now · Continue unchanged.
- escalate: scrutinous

## done: fp-copilot-composer-hint-type
- screen: copilot
- ref: `.cursor/figma-refs/03-copilot.png` · live `2:35` ·
  hint `5:256` · composer `5:248` ·
  crop `_verify/tick38-copilot-hint-crop.png`
- commit: 4279978
- change: |
    `.view-copilot .copilot-hint` → Regular 11px / lh 1.4 / `#645d73`
    (not lilac muted); 10px gap under 78px wrap. Enter/mic string +
    Mic/Send unchanged.
- escalate: scrutinous

## done: fp-copilot-responses-foot-type
- screen: copilot
- ref: `.cursor/figma-refs/03-copilot.png` · live `2:35` ·
  foot `5:257` · crop `_verify/tick38-copilot-foot-crop.png`
- commit: 4279978
- change: |
    `.view-copilot .copilot-responses-foot` → Regular 12px / lh 1.4 /
    `#645d73`; `margin-top: auto` pin + Responses/Audio string kept.
- escalate: scrutinous

## open: fp-relaunch-badge-type
- screen: active
- ref: `.cursor/figma-refs/09-reset-relaunch.png` · live `2:41` ·
  badge `8:414` · text `8:415` ·
  crop `_verify/tick38-relaunch-badge-crop.png`
- expected: |
    Live relaunch card badge text (`8:415`, glyph ~9–11 in 31px pill):
    Bold/Semibold **~11–12px** uppercase purple **`#6750a4`** on raised
    `#e9ddfd` — “→  READY TO RELAUNCH?” (→ glyph via
    `fp-relaunch-badge-arrow`).
- actual: |
    Opens lock → glyph (`fp-relaunch-badge-arrow`) + left-pill mint vs
    card purple (`fp-relaunch-progress-pill-mint`) + footer mint — not
    badge **type scale**. App has no relaunch card yet.
- deviation: |
    Tick38 — relaunch card badge type vs live `8:415`. Distinct from
    badge-arrow / progress-pill-mint / footer-mint / title-medium-28;
    under-covered `2:41`. Spot-check: Take a break + End on flight card.
- fix_hint: |
    When mounting relaunch badge, set label ~11–12px / weight 700 /
    `#6750a4` on `#e9ddfd` pill; pair with → glyph open. Preserve
    Pause/End on normal active.
- escalate: scrutinous

## open: fp-connection-lost-composer-pill
- screen: active
- ref: `.cursor/figma-refs/16-connection-lost.png` · live `2:48` ·
  input `6:1726` · composer `6:1725` ·
  crop `_verify/tick38-lost-composer-crop.png`
- expected: |
    Live lost Message composer input (`6:1726`, 376×78): **pill**
    radius (full stadium) fill `#f7f2ff` / border `#c8bfd7` with Mic+↑
    52 circles — same chrome language as Copilot `5:249`, not a 16px
    rounded rectangle.
- actual: |
    Opens lock composer **height** 78 (`fp-connection-lost-composer-input-height`)
    + placeholder-type / hint-type — not radius. Session wrap
    `.view-session .session-copilot-composer .copilot-input-wrap` uses
    `border-radius: 1rem` (multi-row look). Lost UI still missing
    (`fp-connection-lost-panel`).
- deviation: |
    Tick38 — lost/session composer pill radius vs live `6:1726`. Distinct
    from composer-input-height / placeholder-type / hint-type /
    composer-stays; under-covered `2:48`. Pause/End still on flight card.
- fix_hint: |
    When mounting lost (shared session) 78px composer, use
    `border-radius: var(--radius-pill)` + `#f7f2ff` / `#c8bfd7`; keep
    Mic/Send + Pause/End.
- escalate: scrutinous

## done: fp-welcome-kicker-type
- screen: welcome
- ref: `.cursor/figma-refs/01-welcome.png` · live `2:33` ·
  kicker `5:59`/`5:60` · crop `_verify/tick38-welcome-kicker-crop.png`
- commit: 4877bf7
- change: |
    Superseded by `fp-welcome-kicker-regular-12` (tick43 Regular 12 /
    lh 1.4 / `#326c78` is source of truth; not Bold ~11 / 0.08em).
- escalate: scrutinous
## done: fp-permission-handoff-badge-type
- screen: overlay
- ref: `.cursor/figma-refs/17-permission-request.png` · live `2:49` ·
  badge `7:416` · text `7:417` ·
  crop `_verify/tick39-handoff-modal-crop.png`
- commit: 84b0882
- change: |
    `.mc-perm-modal__badge` → 12px / weight 700 / tracking 0.08em /
    `#6750a4` on `#e9ddfd`. Camera copy + pad 33 / Not now · Continue kept.
- escalate: scrutinous

## done: fp-permission-handoff-not-now-ink
- screen: overlay
- ref: `.cursor/figma-refs/17-permission-request.png` · live `2:49` ·
  Not now `7:421` · actions `7:420` ·
  crop `_verify/tick39-handoff-modal-crop.png`
- commit: 84b0882
- change: |
    `.mc-perm-modal__btn--secondary` → Regular 16px / lh 1.4 / `#282237`
    on `#e9ddfd` (not purple `#4a3d78`). Continue primary + dismiss wiring kept.
- escalate: scrutinous

## done: fp-welcome-field-label-weight
- screen: welcome
- ref: `.cursor/figma-refs/01-welcome.png` · live `2:33` ·
  Email label `5:64` · Password label `5:68` ·
  crop `_verify/tick39-welcome-card-crop.png`
- commit: 84b0882
- change: |
    `.welcome-field-label` → font-weight 700 / 14px / lh 20 / `#282237`.
    Google-only form shell unchanged.
- escalate: scrutinous

## open: fp-relaunch-primary-label-type
- screen: active
- ref: `.cursor/figma-refs/09-reset-relaunch.png` · live `2:41` ·
  Relaunch `8:423` · crop `_verify/tick39-relaunch-panel-crop.png`
- expected: |
    Live card “Relaunch” (`8:423`, 386×52 filled `#6750a4`): M3 Label
    Large — Medium **~14px** / weight 500 / white `#fff` centered on the
    pill (same primary label token as welcome Sign in / Continue).
- actual: |
    Opens lock fill hierarchy (`fp-relaunch-cta-hierarchy`) + Take a break
    tonal (`fp-relaunch-take-break-tonal-raised`) — not Relaunch **label
    type**. App has no relaunch card yet.
- deviation: |
    Tick39 — Relaunch primary label type vs live `8:423`. Distinct from
    cta-hierarchy / take-break-tonal / badge-type / title-medium-28;
    under-covered `2:41`. Spot-check: left Take a break + End still on
    flight card.
- fix_hint: |
    When mounting Relaunch CTA, set label ~14px / weight 500 / `#fff` on
    filled `#6750a4` 52px pill; pair with tonal Take a break. Preserve
    Pause/End on normal active.
- escalate: scrutinous

## open: fp-connection-lost-retry-label-type
- screen: active
- ref: `.cursor/figma-refs/16-connection-lost.png` · live `2:48` ·
  Retry `6:1722` · crop `_verify/tick39-lost-panel-crop.png`
- expected: |
    Live “Retry connection” (`6:1722`, 386×52 tonal `#e9ddfd`): Regular
    **16px** / lh 1.4 ink **`#282237`** — same secondary-label token as
    handoff Not now / relaunch Take a break, not dense/semibold purple.
- actual: |
    Opens lock Retry tonal fill (`fp-connection-lost-retry-tonal`) +
    Latest/suggest slot (`fp-connection-lost-latest-card`) — not Retry
    **label type**. App has no lost Retry yet; risk: reuse suggest chip
    type that drifts from 16/400/`#282237`.
- deviation: |
    Tick39 — Retry label type vs live `6:1722`. Distinct from retry-tonal /
    latest-card / latest-type / composer-*; under-covered `2:48`.
    Spot-check: Pause/End still on flight card; Copilot type tokens
    (`4279978`) no regression on glance.
- fix_hint: |
    When mounting Retry, set label 16px / 400 / `#282237` on `#e9ddfd`
    52px full-width; keep composer + Pause/End.
- escalate: scrutinous

## done: fp-copilot-chip-label-type
- screen: copilot
- ref: `.cursor/figma-refs/03-copilot.png` · live `2:35` ·
  chips `5:241`/`5:243`/`5:245` · crop `_verify/tick40-copilot-chips-crop.png`
- commit: 5de8f4a
- change: |
    `.copilot-chip` → Regular 16px / weight 400 / lh 1.4 / `#282237` on
    tonal `#e9ddfd`; 52px height + fixed widths + chip→send wiring kept.
- escalate: scrutinous

## done: fp-copilot-mic-label-type
- screen: copilot
- ref: `.cursor/figma-refs/03-copilot.png` · live `2:35` ·
  Mic `5:251` · crop `_verify/tick40-copilot-composer-crop.png`
- commit: 5de8f4a
- change: |
    `.view-copilot .copilot-mic` → Regular 16px / weight 400 / lh 1.4 /
    `#282237`; “Mic” string + 52×52 tonal circle + voice wiring kept.
- escalate: scrutinous

## done: fp-copilot-sidebar-cta-label-type
- screen: copilot
- ref: `.cursor/figma-refs/03-copilot.png` · live `2:35` ·
  CTA `5:223` · crop `_verify/tick40-copilot-sidebar-crop.png`
- commit: 5de8f4a
- change: |
    `#copilot-start-mission` / `.copilot-sidebar-cta.primary` → Medium
    14px / weight 500 / white (override `.primary` 700); full-width +
    52px height + no glow + `show("view-lockin")` kept.
- escalate: scrutinous

## open: fp-relaunch-take-break-label-type
- screen: active
- ref: `.cursor/figma-refs/09-reset-relaunch.png` · live `2:41` ·
  Take a break `8:426` · crop `_verify/tick40-relaunch-panel-crop.png`
- expected: |
    Live card “Take a break” (`8:426`, 386×52 tonal `#e9ddfd`): Regular
    **16px** / lh 1.4 ink **`#282237`** — same secondary-label token as
    handoff Not now / lost Retry (not purple/semibold).
- actual: |
    Opens lock tonal/raised fill (`fp-relaunch-take-break-tonal-raised` /
    quiet-fill) + Relaunch **primary** label type — not Take a break
    **label type**. App has no relaunch card yet; risk: reuse `.primary`
    bold or chip Medium when mounting.
- deviation: |
    Tick40 — Take a break label type vs live `8:426`. Distinct from
    take-break-tonal / cta-hierarchy / primary-label-type / badge-type;
    under-covered `2:41`. Spot-check: left Take a break + End on flight
    card; Pause/End preserved on normal active / lost.
- fix_hint: |
    When mounting Take a break, set label 16px / 400 / `#282237` on
    `#e9ddfd` 52px full-width; pair with Relaunch filled Medium 14.
    Preserve Pause/End on normal active.
- escalate: scrutinous

## done: fp-permission-handoff-badge-weight
- screen: overlay
- ref: `.cursor/figma-refs/17-permission-request.png` · live `2:49` ·
  badge text `7:417` · crop `_verify/tick40-handoff-modal-crop.png`
- commit: 91131e0
- change: |
    `.mc-perm-modal__badge` → font-weight 400 (keep 12px / tracking
    0.08em / `#6750a4` on `#e9ddfd`). Not now · Continue + camera copy +
    pad 33 kept.
- escalate: scrutinous

## done: fp-connection-lost-mic-label-type
- screen: active
- ref: `.cursor/figma-refs/16-connection-lost.png` · live `2:48` ·
  Mic `6:1728` · composer `6:1725` ·
  crop `_verify/tick41-lost-mic.png` · panel `_verify/tick41-lost-panel-crop.png`
- commit: 91131e0
- change: |
    Extended Mic type override to
    `.view-session .session-copilot-composer .copilot-mic` → Regular 16px /
    weight 400 / lh 1.4 / `#282237` (match Copilot `5de8f4a`). 52 circle +
    Mic wiring + Pause/End kept.
- escalate: scrutinous

## open: fp-relaunch-progress-pill-type
- screen: active
- ref: `.cursor/figma-refs/09-reset-relaunch.png` · live `2:41` ·
  pill `6:580` · text `6:581` ·
  crop `_verify/tick41-relaunch-progress-pill.png`
- expected: |
    Live left flight status pill (`6:581`, ~143×31): → + sentence-case
    **“Ready to relaunch?”** — Bold/Semibold **~11–12px** mint **`#326c78`**
    on raised `#e9ddfd` (NOT uppercase; NOT purple card-badge type).
- actual: |
    Opens lock mint color (`fp-relaunch-progress-pill-mint`) + → glyph
    (`fp-relaunch-badge-arrow`) + card badge **uppercase** type
    (`fp-relaunch-badge-type`) — not left-pill **type/casing**. App has no
    relaunch progress state yet.
- deviation: |
    Tick41 — left progress pill type/casing vs live `6:581`. Distinct from
    progress-pill-mint / badge-type (card `8:415` UPPERCASE purple) /
    badge-arrow; under-covered `2:41`. Spot-check: left Take a break + End.
- fix_hint: |
    When mounting relaunch progress pill, set ~11–12px / weight 700 /
    mint `#326c78` / sentence-case “Ready to relaunch?”; keep card badge
    UPPERCASE purple. Preserve Pause/End on normal active.
- escalate: scrutinous

## open: fp-relaunch-flight-take-break-label-type
- screen: active
- ref: `.cursor/figma-refs/09-reset-relaunch.png` · live `2:41` ·
  left Take a break `6:644` · End `6:647` ·
  crop `_verify/tick41-relaunch-live.png`
- expected: |
    Live left flight-card “Take a break” (`6:644`, 210×52 filled `#6750a4`):
    M3 Label Large — Medium **~14px** / weight **500** / white `#fff`
    (same primary-label token as card Relaunch / welcome Sign in), not
    Regular 16 dark like the card’s tonal Take a break.
- actual: |
    Opens lock left filled hierarchy (`fp-relaunch-cta-hierarchy`) + card
    tonal Take a break **label** (`fp-relaunch-take-break-label-type`) +
    card Relaunch primary label — not left filled Take a break **label
    type**. App has no relaunch left-row swap yet; risk: reuse Pause
    bold/700 when mounting.
- deviation: |
    Tick41 — left filled Take a break label type vs live `6:644`. Distinct
    from cta-hierarchy / take-break-label-type (card `8:426`) /
    primary-label-type (card Relaunch); under-covered `2:41`.
- fix_hint: |
    When swapping Pause → Take a break on relaunch, set label ~14px /
    weight 500 / `#fff` on filled `#6750a4` 52px pill; End stays tonal
    Regular 16. Preserve Pause/End on normal active / lost.
- escalate: scrutinous

## done: fp-permission-handoff-continue-label-type
- screen: overlay
- ref: `.cursor/figma-refs/17-permission-request.png` · live `2:49` ·
  Continue `7:424` · actions `7:420` ·
  crop `_verify/tick41-handoff-continue.png` · modal `_verify/tick41-handoff-modal-crop.png`
- commit: 91131e0
- change: |
    `.mc-perm-modal__btn--primary` → Medium 14px / weight 500 / white +
    min-height 52 (override 0.95rem/650). Not now Regular 16 / `#282237`
    + Continue wiring + camera copy + pad 33 kept.
- escalate: scrutinous

## open: fp-permissions-choice-sub-type
- screen: overlay
- ref: `.cursor/figma-refs/17-permission-request.png` · live `2:49` ·
  sub `7:396` · intro `7:393` ·
  crop `_verify/tick41-choice-intro-crop.png`
- expected: |
    Live YOUR CHOICE sub (`7:396`): Regular **16px** / lh 1.4 / muted
    **`#645d73`** — “Optional signals. Clear controls. No recordings.”
    (title Medium 44 + purple kicker stay on title-size).
- actual: |
    Opens lock interstitial existence / title-size (bundles “muted 16”) /
    intro-gap / art / scrim — not sub **type tokens** (weight/lh/ink).
    App still has no YOUR CHOICE page under handoff/denied.
- deviation: |
    Tick41 — choice sub type vs live `7:396`. Distinct from title-size /
    intro-gap / choice-surface / art-y / scrim; under-covered YOUR CHOICE.
- fix_hint: |
    When adding YOUR CHOICE view, set sub 16px / 400 / lh 1.4 / `#645d73`;
    keep purple kicker + Medium 44 title + Lock-in nav. Preserve Settings
    five-tab; do not block Launch.
- escalate: scrutinous

## done: fp-copilot-send-fill
- screen: copilot
- ref: `.cursor/figma-refs/03-copilot.png` · live `2:35` ·
  send `5:254` · input `5:249` ·
  crop `_verify/tick42-copilot-composer-crop.png`
- commit: 97e3086
- change: |
    `.view-copilot .copilot-send.primary` → solid `#6750a4` (scoped;
    session Send left on `--mc-accent-purple`); 52×52 circle + ↑ +
    send wiring kept.
- escalate: scrutinous

## done: fp-copilot-sidebar-cta-fill
- screen: copilot
- ref: `.cursor/figma-refs/03-copilot.png` · live `2:35` ·
  CTA `5:223` · sidebar `5:217` ·
  crop `_verify/tick42-copilot-sidebar-crop.png`
- commit: 97e3086
- change: |
    `#copilot-start-mission` / `.copilot-sidebar-cta.primary` → solid
    `#6750a4` + no glow; Medium 14 white / full-width / 52px + Launch
    wiring kept.
- escalate: scrutinous

## done: fp-copilot-sidebar-body-type
- screen: copilot
- ref: `.cursor/figma-refs/03-copilot.png` · live `2:35` ·
  body `5:221` · sidebar `5:217` ·
  crop `_verify/tick42-sidebar-full.png`
- commit: 97e3086
- change: |
    `.copilot-sidebar-body` → Regular 16px / weight 400 / lh 1.4 /
    `#645d73`; Start a mission + planet flow unchanged.
- escalate: scrutinous

## done: fp-copilot-empty-heading-ink
- screen: copilot
- ref: `.cursor/figma-refs/03-copilot.png` · live `2:35` ·
  heading `5:238` · card `5:235` ·
  crop `_verify/tick42-empty-card.png`
- commit: 02df760
- change: |
    `.copilot-empty-heading` → ink `#282237` (Regular 22px / lh 1.4 kept);
    empty copy `#645d73` + Mic/Send/chip wiring unchanged.
- escalate: scrutinous

## open: fp-relaunch-end-label-type
- screen: active
- ref: `.cursor/figma-refs/09-reset-relaunch.png` · live `2:41` ·
  End `6:647` · controls `6:643` ·
  crop `_verify/tick42-end-mission.png` · panel `_verify/tick42-relaunch-live.png`
- expected: |
    Live left “End mission” (`6:647`, 180×52 tonal `#e9ddfd`): Regular
    **16px** / lh 1.4 ink **`#282237`** (no border stroke) — same
    secondary-label token as card Take a break / lost Retry.
- actual: |
    Opens lock left CTA hierarchy (Take a break filled + End tonal) +
    flight-take-break **label** — not End **label type**. App
    `#end-session.session-control-pill--end` is ~0.9rem / weight **600**
    on bordered lavender (`--mc-surface-lavender`), not 16/400/`#282237`
    on `#e9ddfd`.
- deviation: |
    Tick42 — End mission label type/fill vs live `6:647`. Distinct from
    flight-take-break-label-type / cta-hierarchy / take-break-label-type /
    primary-label-type; under-covered `2:41`. Spot-check: handoff
    Continue/badge (`91131e0`) + session Mic — no live regression.
- fix_hint: |
    Align `.session-control-pill--end` to 16px / 400 / `#282237` on
    `#e9ddfd` 52px (drop border); keep End wiring. Preserve Pause/End on
    normal active / lost.
- escalate: scrutinous

## done: fp-relaunch-timer-caption-type
- screen: active
- ref: `.cursor/figma-refs/09-reset-relaunch.png` · live `2:41` ·
  caption `6:642` · countdown `6:640` ·
  crop `_verify/tick43-relaunch-caption-crop.png` · panel `_verify/tick43-relaunch-full.png`
- commit: c6b2e58
- change: |
    `.session-timer-caption` → Regular 11px / weight 400 / lh 1.4 /
    muted `#645d73` (drop 0.12em tracking + weight 600). TIMER PAUSED
    string + Pause/End wiring unchanged.
- escalate: scrutinous

## open: fp-relaunch-earned-type
- screen: active
- ref: `.cursor/figma-refs/09-reset-relaunch.png` · live `2:41` ·
  earned `6:650` · line `6:651` · hint `6:652` ·
  crop `_verify/tick43-relaunch-earned-crop.png`
- expected: |
    Live earned block (`6:650`): stack gap **6px** — primary Regular
    **15px** mint **`#326c78`** “{n} of {m} flight minutes earned”
    (`6:651`) + hint Regular **12px** muted **`#645d73`** “Time in
    session, excluding breaks and resets.” (`6:652`).
- actual: |
    `#session-flight-minutes` is sole line — `0.92rem` / weight **600** /
    `var(--mc-accent-green-dim)` — no hint subline. Opens lock relaunch
    panel/chrome only, not earned type tokens.
- deviation: |
    Tick43 — earned minutes type + missing hint vs live `6:651`/`6:652`.
    Distinct from live-panel-copy / timer-caption-type / flight CTA
    labels; under-covered `2:41` (also on lost `2:48` flight card).
- fix_hint: |
    Style `.session-flight-minutes` 15px / 400 / `#326c78`; add muted
    12px hint under it; keep earned wiring. Preserve Pause/End.
- escalate: scrutinous

## done: fp-welcome-title-ink
- screen: welcome
- ref: `.cursor/figma-refs/01-welcome.png` · live `2:33` ·
  title `5:61` · card `5:58` ·
  crop `_verify/tick43-welcome-title-crop.png`
- commit: 4877bf7
- change: |
    `.welcome-signin-title` → ink `#282237` (Medium 32 / lh 1.4 kept);
    Google/guest wiring unchanged.
- escalate: scrutinous

## done: fp-welcome-kicker-regular-12
- screen: welcome
- ref: `.cursor/figma-refs/01-welcome.png` · live `2:33` ·
  kicker `5:59`/`5:60` ·
  crop `_verify/tick43-welcome-kicker-crop.png`
- commit: 4877bf7
- change: |
    `.welcome-signin-kicker` → Regular 12px / weight 400 / lh 1.4 /
    mint `#326c78` on `#e9ddfd` (drop Bold 700 + 0.12em tracking).
    Supersedes `fp-welcome-kicker-type`. Google wiring kept.
- escalate: scrutinous

## done: fp-connection-lost-send-fill
- screen: active
- ref: `.cursor/figma-refs/16-connection-lost.png` · live `2:48` ·
  send `6:1731` · composer `6:1726` ·
  crop `_verify/tick43-lost-composer-crop.png` · full `_verify/tick43-lost-full.png`
- commit: 4877bf7
- change: |
    `.view-session .session-copilot-composer .copilot-send.primary` →
    solid `#6750a4` (match Copilot Send after `97e3086`); 52 circle + ↑
    + Enter/mic + Pause/End kept.
- escalate: scrutinous

## done: fp-copilot-sidebar-title-ink
- screen: copilot
- ref: `.cursor/figma-refs/03-copilot.png` · live `2:35` ·
  title `5:220` · sidebar `5:217` ·
  crop `_verify/tick44-copilot-sidebar-crop.png`
- commit: 02df760
- change: |
    `.copilot-sidebar-title` → ink `#282237` (Medium 26 / lh 1.4 kept);
    body `#645d73` + Start a mission / planet wiring unchanged.
- escalate: scrutinous

## done: fp-copilot-sidebar-divider
- screen: copilot
- ref: `.cursor/figma-refs/03-copilot.png` · live `2:35` ·
  divider `5:222` · sidebar `5:217` ·
  crop `_verify/tick44-copilot-sidebar-crop.png`
- commit: 02df760
- change: |
    `.copilot-sidebar-divider` → solid hairline `#c8bfd7` (1px; not
    rgba `--mc-border-default`); Start a mission + planet flow kept.
- escalate: scrutinous

## open: fp-relaunch-timer-digit-type
- screen: active
- ref: `.cursor/figma-refs/09-reset-relaunch.png` · live `2:41` ·
  digits `6:641` · countdown `6:640` ·
  crop `_verify/tick44-relaunch-digits.png` · full `_verify/tick44-relaunch-live.png`
- expected: |
    Live relaunch countdown digits (`6:641`, ~222×120 box): Bold **~86px**
    / tight leading / ink **`#282237`** “13:00” — caption under stays
    Regular 11 muted (`fp-relaunch-timer-caption-type`).
- actual: |
    Opens lock caption type only. `.session-timer` is
    `clamp(3.4rem, 12vw, 5rem)` (max **80px**) / weight **600** /
    `color: var(--mc-text-primary)` (`#2a2440`) — undersized + lighter
    weight/ink vs live Bold 86 / `#282237`.
- deviation: |
    Tick44 — relaunch timer digit type vs live `6:641`. Distinct from
    timer-caption-type / earned-type / end-label-type; under-covered
    `2:41`. Spot-check: welcome title/kicker + session Send (`4877bf7`)
    — no live regression on 01/16 glance.
- fix_hint: |
    Set `.view-session .session-timer` to ~86px / weight 700 / `#282237`
    (tabular); keep TIMER PAUSED caption open + Pause/End swap.
- escalate: scrutinous

## done: fp-connection-lost-header-ink
- screen: active
- ref: `.cursor/figma-refs/16-connection-lost.png` · live `2:48` ·
  title `6:1710` · header `6:1708` ·
  crop `_verify/tick44-lost-live.png`
- commit: c6b2e58
- change: |
    `.session-copilot-kicker` → ink `#282237` (scale/weight left for open
    `fp-connection-lost-header-type`); ✦ Your copilot + Pause/End kept.
- escalate: scrutinous

## open: fp-permissions-choice-kicker-type
- screen: overlay
- ref: `.cursor/figma-refs/17-permission-request.png` · live `2:49` ·
  kicker `7:394` · intro `7:393` ·
  crop `_verify/tick44-choice-intro-crop.png`
- expected: |
    Live YOUR CHOICE kicker (`7:394`, h≈17): Regular **12px** / weight
    **400** / lh 1.4 / purple **`#6750a4`** uppercase “YOUR CHOICE” —
    not Bold/700 and not mint (mint is welcome/setup badges).
- actual: |
    Open `fp-permissions-choice-title-size` bundles “purple `#6750a4` bold
    12px” with Medium 44 title — locks size/color family, not kicker
    **weight/lh**. `fp-permissions-choice-sub-type` locks sub only.
    App still has no YOUR CHOICE page under handoff/denied.
- deviation: |
    Tick44 — choice kicker type vs live `7:394` (Regular 12 / 400 /
    `#6750a4`). Distinct from title-size / sub-type / choice-surface /
    intro-gap / scrim; under-covered YOUR CHOICE.
- fix_hint: |
    When adding YOUR CHOICE view, set kicker 12px / 400 / lh 1.4 /
    `#6750a4` (no heavy tracking); keep Medium 44 title + muted 16 sub +
    Lock-in nav. Preserve Settings five-tab; do not block Launch.
- escalate: scrutinous

## open: fp-relaunch-title-ink
- screen: active
- ref: `.cursor/figma-refs/09-reset-relaunch.png` · live `2:41` ·
  title `8:416` · panel `6:653` ·
  crop `_verify/tick45-relaunch-panel-crop.png`
- expected: |
    Live relaunch title (`8:416`): Medium 28 / lh 1.4 ink **`#282237`**
    (“Let’s pick one / small next step.”) — same darkest text token as
    welcome/copilot titles (`var(--color-text)`).
- actual: |
    Open `fp-relaunch-title-medium-28` locks size/weight/lh only — not
    ink. App has no relaunch card; risk: inherit `--mc-text-primary`
    (`#2a2440`) when mounting.
- deviation: |
    Tick45 — relaunch title ink vs live `8:416` `#282237`. Distinct from
    title-medium-28 / title-scale / body-type / live-panel-copy;
    under-covered `2:41`. Spot-check: Copilot empty/sidebar ink +
    divider (`02df760`) — no regression on `2:35` glance. Local refs
    match live (hash identical).
- fix_hint: |
    When mounting relaunch title, set color `#282237` with Medium 28 /
    lh 1.4; keep Relaunch / Take a break. Preserve Pause/End.
- escalate: scrutinous

## open: fp-connection-lost-divider
- screen: active
- ref: `.cursor/figma-refs/16-connection-lost.png` · live `2:48` ·
  divider `6:1713` · mid `6:1716` · panel `6:1707` ·
  crop `_verify/tick45-lost-panel-crop.png`
- expected: |
    Live lost Copilot panel hairlines (`6:1713` under presence,
    `6:1716` under AT LAUNCH seed): 1px **`#c8bfd7`** / 386 wide —
    same border token as relaunch divider `8:418` / panel chrome.
- actual: |
    Opens lock panel surface/pad/stack / presence / history — not the
    history **divider** strokes. App has no lost panel; session chat
    has no `#c8bfd7` rules between presence → AT LAUNCH → YOU.
- deviation: |
    Tick45 — lost panel dividers vs live `6:1713`/`6:1716`. Distinct from
    panel-surface / header-ink / meta-muted / history-stack-gap /
    latest-*; under-covered `2:48`. Relaunch divider open does not cover
    lost.
- fix_hint: |
    When mounting lost history, insert `#c8bfd7` 1px rules after presence
    and after AT LAUNCH seed (before YOU); keep composer + Pause/End.
- escalate: scrutinous

## open: fp-permissions-choice-title-ink
- screen: overlay
- ref: `.cursor/figma-refs/17-permission-request.png` · live `2:49` ·
  title `7:395` · intro `7:393` ·
  crop `_verify/tick45-choice-intro-crop.png`
- expected: |
    Live YOUR CHOICE title (`7:395`): Medium ~44 ink **`#282237`**
    “Fly on your own terms.” — `var(--color-text)`, not lilac/primary
    purple.
- actual: |
    Open `fp-permissions-choice-title-size` locks Medium 44 + “dark”
    only — not exact ink. Kicker purple / sub muted stay on sibling
    opens. App still has no YOUR CHOICE page under handoff/denied.
- deviation: |
    Tick45 — choice title ink vs live `7:395` `#282237`. Distinct from
    title-size / kicker-type / sub-type / choice-surface / scrim;
    under-covered YOUR CHOICE.
- fix_hint: |
    When adding YOUR CHOICE view, set title color `#282237` with Medium
    44; keep purple Regular-12 kicker + muted 16 sub + Lock-in nav.
    Preserve Settings five-tab; do not block Launch.
- escalate: scrutinous

## open: fp-connection-lost-latest-title-ink
- screen: active
- ref: `.cursor/figma-refs/16-connection-lost.png` · live `2:48` ·
  Latest title `6:1720` · card `6:1719` ·
  crop `_verify/tick45-lost-panel-crop.png`
- expected: |
    Live Latest-response title (`6:1720`): Medium ~18 ink **`#282237`**
    “Connection lost. Your flight keeps going.” — darkest text on
    lavender card (body stays muted `#645d73` via latest-type).
- actual: |
    Open `fp-connection-lost-latest-type` locks Medium 18 + body 14/muted
    — not title **ink**. App has no lost Latest card; risk: inherit
    `--mc-text-primary` (`#2a2440`) or muted for the title line.
- deviation: |
    Tick45 — lost Latest title ink vs live `6:1720` `#282237`. Distinct
    from latest-type / latest-card / latest-height / body-copy /
    header-ink; under-covered `2:48`.
- fix_hint: |
    When mounting Latest card, set title color `#282237` (Medium 18);
    keep body `#645d73` 14 + Retry tonal + composer. Preserve Pause/End.
- escalate: scrutinous

## done: fp-relaunch-controls-gap
- screen: active
- ref: `.cursor/figma-refs/09-reset-relaunch.png` · live `2:41` ·
  controls `6:643` · Take a break `6:644` · End `6:647` ·
  crop `_verify/tick45-relaunch-live.png`
- commit: c6b2e58
- change: |
    `.session-controls` → `gap: 12px` between Take a break / End pills
    (was 0.55rem); Pause/End / Take a break wiring unchanged.
- escalate: scrutinous

## done: fp-copilot-composer-radius
- screen: copilot
- ref: `.cursor/figma-refs/03-copilot.png` · live `2:35` ·
  input `5:249` · composer `5:248` ·
  crop `_verify/tick46-copilot-composer-crop.png`
- commit: d02b66c
- change: |
    `.view-copilot .copilot-input-wrap` → `border-radius: 16px` (was
    pill 9999px); keep 78px height / `#f7f2ff` / `#c8bfd7` / Mic+↑ 52 /
    `#6750a4` Send. Session/lost wrap stay `1rem`.
- escalate: scrutinous

## done: fp-copilot-sidebar-body-15
- screen: copilot
- ref: `.cursor/figma-refs/03-copilot.png` · live `2:35` ·
  body `5:221` · sidebar `5:217` ·
  crop `_verify/tick46-copilot-sidebar-crop.png`
- commit: d02b66c
- change: |
    `.copilot-sidebar-body` → Regular **15px** / lh 1.4 / `#645d73`
    (correct overshoot from `97e3086` 16px). Start a mission + planet
    unchanged.
- escalate: scrutinous

## done: fp-session-mission-title-type
- screen: active
- ref: `.cursor/figma-refs/09-reset-relaunch.png` · live `2:41` ·
  objective `6:566` · title `6:568` · kicker `6:567` ·
  crop `_verify/tick46-relaunch-live.png`
- commit: d02b66c
- change: |
    `.session-mission-title` → Medium **28px** / lh 1.4 ink `#282237`
    (was Bold clamp / `--mc-text-primary`). CURRENT MISSION kicker +
    Pause/End / signal pills unchanged.
- escalate: scrutinous

## open: fp-connection-lost-seed-15
- screen: active
- ref: `.cursor/figma-refs/16-connection-lost.png` · live `2:48` ·
  AT LAUNCH body `6:1715` · panel `6:1707` ·
  crop `_verify/tick46-lost-panel-crop.png`
- expected: |
    Live AT LAUNCH seed body (`6:1715`, 386×42): Regular **15px** /
    lh 1.4 muted **`#645d73`** (two-line wrap) — same 15px muted token
    as Copilot sidebar body live.
- actual: |
    Open `fp-connection-lost-seed-type` expected **16px** — overstated
    vs live `get_design_context` **15px**. App still missing lost history
    mount (`fp-connection-lost-panel`).
- deviation: |
    Tick46 — lost seed scale correction vs live `6:1715` (15 not 16).
    Distinct from seed-type color/presence framing / meta-muted /
    user-type / divider; under-covered `2:48`. Prefer 15 when mounting.
- fix_hint: |
    When mounting AT LAUNCH seed, style body **15px** / 400 / lh 1.4 /
    `#645d73` (supersede 16 in seed-type); keep meta + composer.
    Preserve Pause/End.
- escalate: scrutinous

## open: fp-permissions-choice-kicker-bold
- screen: overlay
- ref: `.cursor/figma-refs/17-permission-request.png` · live `2:49` ·
  kicker `7:394` · intro `7:393` ·
  crop `_verify/tick46-choice-intro-crop.png`
- expected: |
    Live YOUR CHOICE kicker (`7:394`): **Bold 12px** / weight **700** /
    lh 1.4 purple **`#6750a4`** uppercase — not Regular/400 (welcome
    mint kickers stay Regular 12 mint).
- actual: |
    Open `fp-permissions-choice-kicker-type` expected Regular 400 — now
    wrong vs live `get_design_context` Bold 700. Title-size still says
    “purple bold 12” bundled with Medium 44. App has no YOUR CHOICE page.
- deviation: |
    Tick46 — choice kicker weight vs live `7:394` (Bold 700). Distinct
    from kicker-type (Regular claim) / title-ink / title-size / sub-type /
    intro-gap; under-covered YOUR CHOICE. Do not re-add kicker-type.
- fix_hint: |
    When adding YOUR CHOICE view, set kicker **12px / 700** / lh 1.4 /
    `#6750a4`; keep Medium 44 title `#282237` + muted 16 sub + Lock-in
    nav. Preserve Settings five-tab; do not block Launch.
- escalate: scrutinous

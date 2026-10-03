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


## done: fp-settings-defaults-off
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

## done: fp-copilot-chip-plus
- screen: copilot
- ref: `.cursor/figma-refs/03-copilot.png`
- deviation: Figma prompt chips “Explain simply”, “I'm stuck”, and “Find a next step” each show a trailing “+”; only `.copilot-chip--next` includes `.copilot-chip-plus`.
- fix_hint: Add `<span class="copilot-chip-plus">+</span>` to the explain/stuck chips to match the next-step chip.
- escalate: none
- commit: 2971eae
- change: Added trailing `+` on Explain simply / I'm stuck chips; all chips share inline-flex layout.

## done: fp-summary-decor-moon
- screen: summary
- ref: `.cursor/figma-refs/12-quest-complete.png`
- deviation: Figma quest-complete stage shows a cratered purple moon under the left ringed planet; `.quest-complete-decor` only mounts constellation, planet, ship, and satellite.
- fix_hint: Add `moon.svg` as `.quest-complete-moon` under the left planet with absolute placement matching the ref.
- escalate: none
- commit: 2971eae
- change: Mounted `moon.svg` as `.quest-complete-moon` under the left planet in quest-complete decor.

## done: fp-settings-intro-kicker
- screen: settings
- ref: `.cursor/figma-refs/04-settings.png`
- deviation: Settings intro kicker rendered “✦ Make yourself at home”; Figma 04 uses plain uppercase “MAKE YOURSELF AT HOME” (kicker kept, no star).
- fix_hint: Remove the ✦ span from `.settings-kicker`; keep the kicker line (CSS already uppercases).
- escalate: none
- commit: f8e6fb3
- change: Dropped ✦ from `.settings-kicker`; plain “Make yourself at home” remains (uppercase via CSS).

## done: fp-active-copilot-presence-dot
- screen: active
- ref: `.cursor/figma-refs/06-mission-active.png`
- deviation: Session copilot presence is plain lavender text “Here when you need me”; Figma shows a green-dot status pill (same language as the mission progress pill).
- fix_hint: Restyle `.session-copilot-presence` with a green leading dot matching `.session-progress-pill` / Figma.
- escalate: none
- commit: 2971eae
- change: Presence pill uses green leading dot matching `.session-progress-pill`.

## done: fp-home-dest-orbits
- screen: home
- ref: `.cursor/figma-refs/02-home.png`
- deviation: Figma dark “Your next destination?” card includes faint orbital/constellation line art behind the planet/ship; `.mc-home-dest-art` has planet/ship/moon/satellite but no orbits/constellation layer.
- fix_hint: Add a low-opacity `orbits.svg` or constellation asset inside `.mc-home-dest-art` positioned behind the planet.
- escalate: none
- commit: f8e6fb3
- change: Added low-opacity `.mc-home-dest-orbits` (`orbits.svg`) behind planet/ship in destination card art.

## done: fp-welcome-hero-orbits-moon
- screen: welcome
- ref: `.cursor/figma-refs/01-welcome.png`
- deviation: Figma welcome hero card shows orbital ellipses + pink cratered moon around the ringed planet; `.welcome-hero-art` only mounts `ship.svg` + `planet-ringed.svg`.
- fix_hint: Add low-opacity `orbits.svg` behind the planet and `moon.svg` on the orbit path inside `.welcome-hero-art` (keep LTR ship→planet).
- escalate: none
- commit: f8e6fb3
- change: Mounted `orbits.svg` + `moon.svg` in `.welcome-hero-art` behind LTR ship→planet.

## open: fp-active-route-uppercase
- screen: active
- ref: `.cursor/figma-refs/06-mission-active.png`
- deviation: Flight route kicker is title case “Earth → Kepler”; Figma 06 uses all-caps “EARTH → KEPLER”.
- fix_hint: Change `.session-flight-route` copy (or CSS `text-transform: uppercase`) to match Figma casing.
- escalate: none
- note: CSS already has `text-transform: uppercase` on `.session-flight-route`; confirm visual then close.

## open: fp-home-kicker-plain
- screen: home
- ref: `.cursor/figma-refs/02-home.png`
- deviation: Home hero kicker renders “✦ Mission control”; Figma 02 shows plain uppercase “MISSION CONTROL” with no leading star glyph.
- fix_hint: Remove the ✦ span from `.mc-home-kicker` (keep uppercase via existing CSS).
- escalate: none

## open: fp-first-flight-kicker-plain
- screen: home
- ref: `.cursor/figma-refs/19-first-flight.png`
- deviation: First-flight hero kicker is “✦ YOUR FIRST MISSION”; Figma 19 shows “YOUR FIRST MISSION” without a leading star.
- fix_hint: Drop the ✦ from `.mc-first-flight` / `.mc-kicker--star` on that screen only.
- escalate: none

## open: fp-session-at-launch-seed
- screen: active
- ref: `.cursor/figma-refs/06-mission-active.png`
- deviation: Session copilot log empty state is muted helper copy; Figma seeds an “AT LAUNCH” system message stating the mission objective and time limit before any user chat.
- fix_hint: Seed an AT LAUNCH row in `#session-chat-log` when a mission starts (reuse goal + duration); keep empty helper for pre-start if needed.
- escalate: none

## open: fp-welcome-hero-body-plus
- screen: welcome
- ref: `.cursor/figma-refs/01-welcome.png`
- deviation: Hero body ends with a purple `✦` (`.welcome-hero-star`); Figma ends the same sentence with a trailing “+” after “one step forward.”
- fix_hint: Replace the inline star span with a “+” (or match Figma asset) and tune color/size to the ref.
- escalate: none

## open: fp-settings-topnav-home-active
- screen: settings
- ref: `.cursor/figma-refs/04-settings.png`
- deviation: On Settings, top nav leaves Home/Copilot/Lock in as plain links and marks “Settings” via `.settings-nav-current`; Figma shows Home on the lavender active pill (Settings is plain text on the right).
- fix_hint: Apply `settings-nav-link--active` to Home when `#view-settings` is active; render Settings as a normal trailing link, not the active pill.
- escalate: none

## open: fp-settings-aside-figma-shortcuts
- screen: settings
- ref: `.cursor/figma-refs/04-settings.png`
- deviation: Figma left column is two actions—“Lock in” (lavender) and “Back to home” (outlined)—plus art; `#view-settings` uses a five-tab `.settings-aside-tabs` rail (Lock in, Connection, Permissions, Account, Voice) with no “Back to home” control.
- fix_hint: Product keeps aside tabs — do not remove. Optional: add a “Back to home” control under the tab rail without collapsing the five tabs.
- escalate: none
- note: Preserve Settings aside tabs (user constraint).

## open: fp-summary-pb-banner-flag
- screen: summary
- ref: `.cursor/figma-refs/12-quest-complete.png`
- deviation: “New longest flight! …” banner (`#summary-pb-banner`) is text-only from `renderSummary()`; Figma shows a small flag icon ahead of that line in `.quest-complete-pb-banner`.
- fix_hint: Insert the same flag SVG used on home longest-flight (or a Figma asset) inside the banner markup before the dynamic text.
- escalate: none

## open: fp-welcome-foot-plus
- screen: welcome
- ref: `.cursor/figma-refs/01-welcome.png`
- deviation: Below the hero card, Figma shows “YOUR NEXT CHAPTER STARTS HERE” with a small “+” under the line; `.welcome-hero-foot` is plain text with no trailing/under “+”.
- fix_hint: Add a decorative “+” under or after `.welcome-hero-foot` matching the ref spacing.
- escalate: none

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


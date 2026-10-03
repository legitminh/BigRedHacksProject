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

## done: fp-setup-top-nav
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
- escalate: scrutinous
- reopened: true
- commit: 3fa9f3d
- change: Replaced setup `←` head with shared `settings-top-bar` (✦ Waypoint, Home/Copilot, Lock in active, Settings + `#setup-avatar`); reused `data-session-nav` handlers; left in-field ✦ launch affordance alone.

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

## done: fp-summary-top-nav
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
- escalate: scrutinous
- reopened: true
- commit: 3fa9f3d
- change: Mounted `settings-top-bar quest-complete-head` above `.quest-complete-stage` with Lock in active, Settings + `#summary-avatar`, shared `data-session-nav` wiring.

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

## done: fp-active-next-step-timer
- screen: active
- ref: `.cursor/figma-refs/06-mission-active.png`
- deviation: Below flight minutes, Figma shows a lavender “NEXT STEP TIMER” pill with a countdown (e.g. 04:32) and helper line “One small step at a time.”; active card ends after flight-minutes hint with no next-step widget.
- fix_hint: Add footer row under `.session-flight-minutes-hint` with kicker pill + timer element; hook to existing coach/step timer if present or stub static layout matching ref.
- escalate: none
- commit: 33a98f6
- change: Added `.session-next-step` lavender pill + countdown under flight-minutes hint; starts via suggest chip / defaults to 05:00; pauses with mission Pause.

## done: fp-active-pause-dev-ui
- screen: active
- ref: `.cursor/figma-refs/06-mission-active.png`
- deviation: Figma primary “Pause” control is enabled on the cream card; `#session-pause` stays disabled and `.session-pause-note` exposes internal copy (“Pause isn’t wired yet…”).
- fix_hint: Hide or restyle the dev note for production UI; enable Pause styling to match Figma (wire pause invoke when ready, or visually match enabled state per design if behavior stays stubbed).
- escalate: none
- commit: af13657
- change: Enabled Pause (aria-pressed Pause/Resume toggle); hid `.session-pause-note` via visually-hidden.

## done: fp-settings-defaults-off
- screen: settings
- ref: `.cursor/figma-refs/04-settings.png`
- deviation: “During a mission” toggles shipped with `checked` / pref defaults true; Figma shows all four switches off.
- fix_hint: Remove default `checked`; set camera/screen/reduce-motion pref defaults to false (keep saved localStorage values).
- escalate: none
- commit: af13657
- change: HTML toggles unchecked; pref defaults false for camera/screen/reduce-motion.

## done: fp-active-waypoint-sublabels
- screen: active
- ref: `.cursor/figma-refs/06-mission-active.png`
- deviation: Flight waypoints only said “Launch” / “Destination”; Figma places “EARTH” under Launch and “KEPLER” under Destination.
- fix_hint: Add uppercase sublabels under each `.session-flight-waypoint`.
- escalate: none
- commit: af13657
- change: Earth/Kepler sublabels stacked under Launch/Destination waypoints.

## done: fp-active-copilot-suggest
- screen: active
- ref: `.cursor/figma-refs/06-mission-active.png`
- deviation: Session copilot sidebar jumps from chat log to composer; Figma shows a wide lavender “Set a five-minute timer” suggestion chip above the message field (plus Enter/mic hint under the composer).
- fix_hint: Add a `.session-copilot-suggest` button + composer hint line under `#session-chat-form` matching Figma copy; stub click or wire to existing step-timer if present.
- escalate: none
- commit: 33a98f6
- change: Added “Set a five-minute timer” suggest chip above session composer; wires to next-step countdown.

## open: fp-setup-affordance-behavior
- screen: setup
- ref: `.cursor/figma-refs/05-mission-setup.png`
- deviation: In-field purple ✦ control is labeled “Launch mission” and `requestSubmit()`s `#lockin-form`; Figma shows a small assist/mic-style circle on the objective field (prior verified intent: open Copilot), not a second Launch control.
- fix_hint: Restore open-Copilot (or voice) behavior; set aria-label/title to match assist, not Launch.
- escalate: none
- note: Skipped this tick — user requested objective ✦ remain Launch; do not revert.

## open: fp-copilot-chip-plus
- screen: copilot
- ref: `.cursor/figma-refs/03-copilot.png`
- deviation: Figma prompt chips “Explain simply”, “I'm stuck”, and “Find a next step” each show a trailing “+”; only `.copilot-chip--next` includes `.copilot-chip-plus`.
- fix_hint: Add `<span class="copilot-chip-plus">+</span>` to the explain/stuck chips to match the next-step chip.
- escalate: none

## open: fp-summary-decor-moon
- screen: summary
- ref: `.cursor/figma-refs/12-quest-complete.png`
- deviation: Figma quest-complete stage shows a cratered purple moon under the left ringed planet; `.quest-complete-decor` only mounts constellation, planet, ship, and satellite.
- fix_hint: Add `moon.svg` as `.quest-complete-moon` under the left planet with absolute placement matching the ref.
- escalate: none

## open: fp-settings-intro-kicker
- screen: settings
- ref: `.cursor/figma-refs/04-settings.png`
- deviation: Settings intro renders “✦ Make yourself at home” above the H1; Figma 04 starts with “Your space, your settings.” and the lead — no kicker line.
- fix_hint: Hide or remove `.settings-kicker` on `#view-settings` so the hero matches the ref.
- escalate: none

## open: fp-active-copilot-presence-dot
- screen: active
- ref: `.cursor/figma-refs/06-mission-active.png`
- deviation: Session copilot presence is plain lavender text “Here when you need me”; Figma shows a green-dot status pill (same language as the mission progress pill).
- fix_hint: Restyle `.session-copilot-presence` with a green leading dot matching `.session-progress-pill` / Figma.
- escalate: none

## open: fp-home-dest-orbits
- screen: home
- ref: `.cursor/figma-refs/02-home.png`
- deviation: Figma dark “Your next destination?” card includes faint orbital/constellation line art behind the planet/ship; `.mc-home-dest-art` has planet/ship/moon/satellite but no orbits/constellation layer.
- fix_hint: Add a low-opacity `orbits.svg` or constellation asset inside `.mc-home-dest-art` positioned behind the planet.
- escalate: none

## done: fp-summary-ctas-in-card
- screen: summary
- ref: `.cursor/figma-refs/12-quest-complete.png`
- deviation: Figma places “Start another mission”, “Done”, and the “Mission ended · …” footer inside the cream quest card beneath the copilot note; `#view-summary` keeps `.quest-complete-actions` and `.quest-complete-footer` as siblings after `.quest-complete-stage`, so CTAs sit outside the card chrome.
- fix_hint: Move actions + footer inside `.quest-complete-card` (after note / before closing the card); keep existing button ids/handlers.
- escalate: none
- commit: 353c851
- change: Moved `.quest-complete-actions` + footer inside `.quest-complete-card`; CTAs sit in a row under the copilot note.

## done: fp-summary-flight-details-extra
- screen: summary
- ref: `.cursor/figma-refs/12-quest-complete.png`
- deviation: Quest complete card includes a “Flight details” `<details>` accordion; Figma 12 has no such disclosure between the copilot note and the CTAs.
- fix_hint: Hide the accordion for the default celebration layout (visually-hidden or remove from card flow); keep `#summary-body` available for tests if needed.
- escalate: none
- commit: 353c851
- change: Hid Flight details `<details>` (`visually-hidden` + `hidden`); `#summary-body` kept in DOM for tests.

## done: fp-home-dest-satellite
- screen: home
- ref: `.cursor/figma-refs/02-home.png`
- deviation: Figma dark destination card art includes a satellite near the ship/planet; `.mc-home-dest-art` only had planet, ship, and moon.
- fix_hint: Add `satellite.svg` into `.mc-home-dest-art` with absolute placement matching the ref.
- escalate: none
- commit: 353c851
- change: Added `.mc-home-dest-satellite` (`satellite.svg`) to home destination card art.

## done: fp-session-mic-hint-target
- screen: active
- ref: `.cursor/figma-refs/06-mission-active.png`
- deviation: Session mic feedback (`setComposerMicHint(..., "session")`) rewrites `#session-chat-empty` instead of a dedicated under-composer hint; Figma keeps a stable “Enter to send · Click the microphone…” line under the field.
- fix_hint: Add `#session-chat-hint` under the session composer and point session mic hints there (same pattern as `#chat-hint`).
- escalate: none
- commit: 33a98f6
- change: Added `#session-chat-hint` under session composer; session mic feedback targets it instead of empty-state copy.

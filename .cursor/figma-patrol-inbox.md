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

## done: fp-active-waypoint-sublabels-uppercase
- screen: active
- ref: `.cursor/figma-refs/06-mission-active.png`
- deviation: Waypoint subs render title case “Earth” / “Kepler”; Figma 06 shows all-caps “EARTH” / “KEPLER” under Launch/Destination. `.session-flight-waypoint-sub` has letter-spacing but no `text-transform: uppercase`.
- fix_hint: Add `text-transform: uppercase` on `.session-flight-waypoint-sub` (or change HTML copy).
- escalate: none
- commit: 232e40d
- change: Added `text-transform: uppercase` on `.session-flight-waypoint-sub`.

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

## done: fp-session-chat-meta-kickers
- screen: active
- ref: `.cursor/figma-refs/06-mission-active.png`
- deviation: Session chat rows are plain `.bubble` nodes; Figma 06 prefixes each turn with uppercase meta kickers (“AT LAUNCH”, “YOU · JUST NOW”) above the message body.
- fix_hint: Extend `appendSessionChat` (and AT LAUNCH seed) to render a meta kicker line + body per Figma.
- escalate: none
- commit: 232e40d
- change: `appendSessionChat` wraps turns with `.session-chat-meta` kickers (YOU · JUST NOW / COPILOT · JUST NOW; optional AT LAUNCH override).

## done: fp-session-mission-star-style
- screen: active
- ref: `.cursor/figma-refs/06-mission-active.png`
- deviation: `.session-mission-star` is a purple filled gradient square with a ✦ glyph; Figma 06 uses a dark rounded square with a white outline star.
- fix_hint: Restyle `.session-mission-star` to dark navy fill + outline star (SVG or border glyph) matching the ref.
- escalate: none
- commit: 232e40d
- change: Dark `#282237` rounded square + lavender outline-star SVG from Figma 06 path.

## done: fp-copilot-chip-plus
- screen: copilot
- ref: `.cursor/figma-refs/03-copilot.png` · `.cursor/figma-refs/03---Copilot.svg`
- deviation: |
    EXPECTED (Figma 03 Copilot prompt-chip row under empty card, left of astronaut; PNG + SVG raster):
    1. Row: `.copilot-prompts-row` → `.study-actions.copilot-shortcuts` — three lavender pills, left→right:
       - `button.copilot-chip[data-study="explain"]` → text “Explain simply” only — NO trailing `+`, no `.copilot-chip-plus` child.
       - `button.copilot-chip[data-study="stuck"]` → text “I'm stuck” only — NO trailing `+`, no `.copilot-chip-plus` child.
       - `button.copilot-chip.copilot-chip--next[data-study="plan"]` → “Find a next step” + ONE trailing thin dark `+` glyph inside the pill (plain stroke/character after a small gap past “step”; same ink family as the label).
    2. Ambient decorative `+` marks near the astronaut / page margins are NOT chip children.
    ACTUAL (`index.html` `#view-chat` `.copilot-prompts-row .copilot-shortcuts`, HEAD after 2971eae):
    1. Explain chip mounts `<span class="copilot-chip-plus" aria-hidden="true">+</span>` after “Explain simply”.
    2. Stuck chip mounts the same `<span class="copilot-chip-plus">+</span>` after “I'm stuck”.
    3. Plan/next chip also mounts `<span class="copilot-chip-plus">+</span>` (correct presence; style covered by sibling open).
    4. Overshoot vs Figma: two extra trailing pluses on explain + stuck.
- fix_hint: |
    In `index.html`, delete the `.copilot-chip-plus` span from `[data-study="explain"]` and `[data-study="stuck"]` only. Keep the span on `.copilot-chip--next` / `[data-study="plan"]`. Do not touch Settings five-tab, Pause/End, LTR orbit, objective ✦ Launch, or `silent_mode` default true.
- escalate: scrutinous
- reopened: true
- commit: 52749cd
- change: Removed `.copilot-chip-plus` from explain + stuck chips; kept only on `copilot-chip--next` / plan.
- note: |
    Compliance 2026-10-03 — prior claim that all three chips should have + does not match Figma 03; reopen to strip extras.
    Scrutinous rewrite 2026-10-03T14:35 — pixel crops of 03-copilot.png + 03---Copilot.svg confirm only “Find a next step” carries a trailing +.

## done: fp-copilot-chip-plus-glyph
- screen: copilot
- ref: `.cursor/figma-refs/03-copilot.png` · `.cursor/figma-refs/03---Copilot.svg`
- deviation: |
    EXPECTED (Figma 03 trailing mark on “Find a next step” only): plain thin dark charcoal/`#`-ink `+` glyph after the label inside the pill — no nested circle, no fill disc behind the plus.
    ACTUAL (`src/styles.css` `.copilot-chip-plus`): `width/height: 1.35rem`, `border-radius: 50%`, `background: rgba(107, 76, 255, 0.12)`, bold `color: var(--mc-accent-purple)` — reads as a circular badge control nested in the chip, not Figma’s thin trailing glyph.
- fix_hint: |
    Restyle `.copilot-chip-plus` to a plain inline glyph (drop circle dimensions/background/radius); keep mounted only on `.copilot-chip--next` after `fp-copilot-chip-plus` strip. Tune spacing with `.copilot-chip` `gap` / margin so “step” → `+` matches the ref gap.
- escalate: none
- commit: 52749cd
- change: Restyled `.copilot-chip-plus` as thin plain ink glyph (no circular lavender badge).

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

## open: fp-session-at-launch-seed
- screen: active
- ref: `.cursor/figma-refs/06-mission-active.png`
- deviation: Session copilot log empty state is muted helper copy; Figma seeds an “AT LAUNCH” system message stating the mission objective and time limit before any user chat.
- fix_hint: Seed an AT LAUNCH row in `#session-chat-log` when a mission starts (reuse goal + duration); keep empty helper for pre-start if needed.
- escalate: none

## done: fp-welcome-hero-body-plus
- screen: welcome
- ref: `.cursor/figma-refs/01-welcome.png`
- deviation: Hero body ends with a purple `✦` (`.welcome-hero-star`); Figma ends the same sentence with a trailing “+” after “one step forward.”
- fix_hint: Replace the inline star span with a “+” (or match Figma asset) and tune color/size to the ref.
- escalate: none
- commit: 52749cd
- change: Replaced welcome hero body ✦ with trailing `+` (`.welcome-hero-plus`).

## done: fp-settings-topnav-home-active
- screen: settings
- ref: `.cursor/figma-refs/04-settings.png`
- deviation: On Settings, top nav leaves Home/Copilot/Lock in as plain links and marks “Settings” via `.settings-nav-current`; Figma shows Home on the lavender active pill (Settings is plain text on the right).
- fix_hint: Apply `settings-nav-link--active` to Home when `#view-settings` is active; render Settings as a normal trailing link, not the active pill.
- escalate: none
- commit: f4264a1
- change: Settings top nav marks Home with `settings-nav-link--active`; trailing Settings is plain `.settings-nav-link` (no current pill).

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

## done: fp-summary-pb-banner-flag
- screen: summary
- ref: `.cursor/figma-refs/12-quest-complete.png`
- deviation: “New longest flight! …” banner (`#summary-pb-banner`) is text-only from `renderSummary()`; Figma shows a small flag icon ahead of that line in `.quest-complete-pb-banner`.
- fix_hint: Insert the same flag SVG used on home longest-flight (or a Figma asset) inside the banner markup before the dynamic text.
- escalate: none
- commit: f4264a1
- change: `#summary-pb-banner` prepends home longest-flight flag SVG before “New longest flight! …” text.

## done: fp-welcome-foot-plus
- screen: welcome
- ref: `.cursor/figma-refs/01-welcome.png`
- deviation: Below the hero card, Figma shows “YOUR NEXT CHAPTER STARTS HERE” with a small “+” under the line; `.welcome-hero-foot` is plain text with no trailing/under “+”.
- fix_hint: Add a decorative “+” under or after `.welcome-hero-foot` matching the ref spacing.
- escalate: none
- commit: f4264a1
- change: Decorative purple `+` under `.welcome-hero-foot` via `.welcome-hero-foot-plus`.

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

## open: fp-summary-flight-logged
- screen: summary
- ref: `.cursor/figma-refs/13---Flight-logged.svg` · also `21---Ended-early---reflection.svg`
- deviation: |
    EXPECTED (Figma 13/21): non–quest-complete / ended-early celebration uses badge “✦ FLIGHT LOGGED” and title “Every flight moves you forward.” (quest-complete keeps ✓ QUEST COMPLETE / “One mission. Well done.”).
    ACTUAL: `updateSummaryCelebration` only branches first-flight vs quest-complete — every non-first summary shows “✓ QUEST COMPLETE” / “One mission. Well done.” with no flight-logged variant.
- fix_hint: |
    Add a flight-logged celebration branch (badge + title + optional ended-early helper under objective choices) when ending early / not marking quest complete; keep quest-complete + first-flight paths.
- escalate: none


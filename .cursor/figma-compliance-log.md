# Figma compliance log

Verifier appends pass/fail notes after each 10m check.

```
## check: <iso-time>
- reviewed: <done ids>
- pass: <ids>
- fail: <ids reopened>
- notes: <one line>
```

---

## check: 2025-10-03T17:30:00-04:00
- reviewed: (none)
- pass: (none)
- fail: (none)
- notes: figma-patrol-inbox had zero `## done:` entries below the separator; nothing to verify or reopen this cycle.

## check: 2026-10-03T13:41:07-04:00
- reviewed: fp-home-hero-title, fp-home-longest-badge, fp-setup-top-nav, fp-launch-cta-arrow, fp-active-figma-layout, fp-summary-top-nav, fp-launch-overlay-scrim, fp-copilot-sidebar-copy, fp-copilot-new-chat-btn, fp-active-status-chip, fp-setup-objective-affordance, fp-active-earth-kepler-label, fp-active-orbit-personal-best, fp-active-orbit-satellite
- pass: fp-home-hero-title, fp-home-longest-badge, fp-launch-cta-arrow, fp-active-figma-layout, fp-launch-overlay-scrim, fp-copilot-sidebar-copy, fp-copilot-new-chat-btn, fp-active-status-chip, fp-setup-objective-affordance, fp-active-earth-kepler-label, fp-active-orbit-personal-best, fp-active-orbit-satellite
- fail: fp-setup-top-nav, fp-summary-top-nav
- notes: ead9e40 orbit route/PB/satellite match refs; bf33c21 nav fixes absent on HEAD so setup+summary still lack MC top bar (escalated scrutinous).

## check: 2026-10-03T14:10:57-04:00
- reviewed: fp-setup-top-nav, fp-summary-top-nav, fp-active-next-step-timer, fp-active-pause-dev-ui, fp-settings-defaults-off, fp-active-waypoint-sublabels, fp-active-copilot-suggest, fp-summary-ctas-in-card, fp-summary-flight-details-extra, fp-home-dest-satellite, fp-session-mic-hint-target
- pass: fp-setup-top-nav, fp-summary-top-nav, fp-active-next-step-timer, fp-active-pause-dev-ui, fp-active-waypoint-sublabels, fp-active-copilot-suggest, fp-summary-ctas-in-card, fp-summary-flight-details-extra, fp-home-dest-satellite, fp-session-mic-hint-target
- fail: fp-settings-defaults-off
- notes: 33a98f6/58eadd7 active+summary/home items match refs; setup/summary MC nav present; reopened settings-defaults (Copilot audio still ON via silent_mode false) → scrutinous rewrite + tiny fix f9c0f02 (silent_mode default true). Preserved Settings tabs, Pause/End, LTR orbit, objective ✦ Launch.

## check: 2026-10-03T14:32:29-04:00
- reviewed: fp-settings-defaults-off, fp-copilot-chip-plus, fp-summary-decor-moon, fp-settings-intro-kicker, fp-active-copilot-presence-dot, fp-home-dest-orbits, fp-welcome-hero-orbits-moon, fp-settings-aside-figma-shortcuts, fp-active-route-uppercase, fp-home-kicker-plain, fp-first-flight-kicker-plain
- pass: fp-settings-defaults-off, fp-summary-decor-moon, fp-settings-intro-kicker, fp-active-copilot-presence-dot, fp-home-dest-orbits, fp-welcome-hero-orbits-moon, fp-settings-aside-figma-shortcuts, fp-active-route-uppercase, fp-home-kicker-plain, fp-first-flight-kicker-plain
- fail: fp-copilot-chip-plus
- notes: f9c0f02 silent_mode default true OK; 2971eae moon+presence OK; 64a5890 orbits/kicker OK; 3b12a30 route/home/first-flight kickers OK; aside five-tab won’t-fix kept; reopened chip-plus (Figma 03 only + on Find a next step). Preserved Pause/End, LTR orbit, objective ✦ Launch, silent_mode true. No code edits (fixer may be concurrent).

## check: 2026-10-03T15:06:26-04:00
- reviewed: fp-copilot-live-match, fp-copilot-chip-plus, fp-copilot-chip-plus-glyph
- pass: fp-copilot-live-match, fp-copilot-chip-plus, fp-copilot-chip-plus-glyph
- fail: (none)
- notes: Live Figma 2:35 confirms plain chips (no + on any, including Find a next step); ffaa518/#view-chat matches cream nav, sidebar copy/divider/CTA/planet, empty card, in-pill Mic+Send, deco + only. Preserved Settings five-tab, objective ✦ Launch, Pause/End, LTR orbit, silent_mode true, #app padding:0. No code edits.

## check: 2026-10-03T15:15:02-04:00
- reviewed: fp-settings-kicker-period, fp-setup-affordance-glyph, fp-active-next-step-clock
- pass: fp-settings-kicker-period, fp-setup-affordance-glyph, fp-active-next-step-clock
- fail: (none)
- notes: Live 2:36 text 5:306 is “MAKE YOURSELF AT HOME” (no period) — proof-removed 4a33e9e OCR period; live 5:421 concentric target + Launch requestSubmit OK; live 6:207/6:209 clock-led NEXT STEP TIMER OK. Preserved Settings five-tab, objective Launch wiring, Pause/End, LTR orbit, silent_mode true, #app padding:0, Copilot live-match.

## check: 2026-10-03T15:19:56-04:00
- reviewed: fp-session-composer-in-pill, fp-setup-objective-compact
- pass: fp-session-composer-in-pill, fp-setup-objective-compact
- fail: (none)
- notes: Live 2:38/6:231 Mic+↑ both in cream Composer input; HEAD dd85eb9 `#session-chat-send` inside `.copilot-input-wrap` + shared Copilot in-pill CSS. Live 2:37/5:421 Editable field 544×64 single-row; `#goals` rows=1 + min-height 2.5rem + centered affordance. Preserved Settings five-tab, ✦ Launch wiring, Pause/End, LTR orbit, silent_mode true, #app padding:0. No code edits.

## check: 2026-10-03T15:27:27-04:00
- reviewed: fp-session-chat-meta-kickers, fp-session-mission-star-style, fp-session-at-launch-seed, fp-welcome-hero-body-plus, fp-welcome-foot-plus, fp-settings-topnav-home-active, fp-summary-pb-banner-flag, fp-summary-flight-logged, fp-active-waypoint-sublabels-uppercase, fp-copilot-listening-composer, fp-summary-objective-helper, fp-summary-note-gated; spot-check ffaa518/4a33e9e/dd85eb9
- pass: fp-session-chat-meta-kickers, fp-session-mission-star-style, fp-session-at-launch-seed, fp-welcome-hero-body-plus, fp-welcome-foot-plus, fp-settings-topnav-home-active, fp-summary-pb-banner-flag, fp-summary-flight-logged, fp-copilot-listening-composer, fp-summary-objective-helper, fp-summary-note-gated; prior ffaa518/4a33e9e/dd85eb9 still match live
- fail: fp-active-waypoint-sublabels-uppercase
- notes: Live 2:38 waypoints Launch/Destination only — reopened uppercase escalate:scrutinous. Settings Home-pill regressed by 63701ab → proof-restored vs live 2:36. 5fdec0d listening composer + summary helper/note-gated match live. Scrutinous opens still accurate. Preserved Settings five-tab, ✦ Launch, Pause/End, LTR orbit, silent_mode true, #app padding:0. No push.

## check: 2026-10-03T15:32:00-04:00
- reviewed: fp-setup-empty-launch-disabled, fp-setup-objective-listening
- pass: fp-setup-empty-launch-disabled, fp-setup-objective-listening
- fail: (none)
- notes: Live 10:752/10:565 confirm e9a210e — empty muted Launch + lead/label/foot; listening field/foot + Launch muted until transcript; affordance `requestSubmit` when objective present. Preserved Settings five-tab, Pause/End, LTR orbit, silent_mode true. No code edits. No push.

## check: 2026-10-03T15:32:34-04:00
- reviewed: fp-active-waypoint-sublabels-uppercase
- pass: fp-active-waypoint-sublabels-uppercase
- fail: (none)
- notes: Live 2:38 metadata 6:189=Launch, 6:190=Destination, 6:168=EARTH→KEPLER only; HEAD 44f3b0e removed `.session-flight-waypoint-sub`; route kicker kept. Preserved Pause/End, LTR orbit, Settings five-tab, ✦ Launch, silent_mode true. No code edits; no push.

## check: 2026-10-03T15:41:44-04:00
- reviewed: fp-ended-early-finished-hero, fp-summary-outcome-note-copy, fp-summary-logged-banner
- pass: fp-ended-early-finished-hero, fp-summary-outcome-note-copy, fp-summary-logged-banner
- fail: (none)
- notes: Live 8:531/8:715 + OCR confirm ca08f47 — Finished upgrades ✓ QUEST COMPLETE / “One mission. Well done.”; not-yet keeps ✦ FLIGHT LOGGED; kicker hidden + PB-remains / time-still-counts notes; `#summary-pb-banner` always-on (“still” / new-longest). Preserved Settings five-tab, ✦ Launch, Pause/End, LTR orbit, silent_mode true. No code edits; no push.

## check: 2026-10-03T15:47:52-04:00
- reviewed: (no fresh `## done:`) spot-check ca08f47 / e9a210e / 44f3b0e verified; EXTRA SCRUTINOUS on escalate opens
- pass: (none new — ca08f47 fp-ended-early-finished-hero, fp-summary-outcome-note-copy, fp-summary-logged-banner still match live 8:531/8:715; e9a210e setup + 44f3b0e waypoints remain verified)
- fail: (none)
- notes: Zero `## done:` this tick. Live reconfirm ca08f47 code+Figma OK. Rewrote scrutinous opens fp-summary-partly-note-copy (8:692 exact), fp-summary-relaunch-note (8:886–888), fp-session-at-launch-chrome (10:1325), fp-listening-session-ui (2:46); tightened fp-summary-pb-stat-label. Refreshed local 23 PNG from live. Preserved Settings five-tab, ✦ Launch, Pause/End, LTR orbit, silent_mode true, #app padding:0. No code edits; no push.

## check: 2026-10-03T15:50:04-04:00
- reviewed: fp-summary-partly-note-copy, fp-summary-pb-stat-label, fp-active-signal-on-fill
- pass: fp-summary-partly-note-copy, fp-summary-pb-stat-label, fp-active-signal-on-fill
- fail: (none)
- notes: 5f9b96a matches live — 8:692 partly note exact; 8:683 “personal best”; `.session-signal-pill.is-on` purple fill+white vs 33/34/35. Inbox commit hashes 4f72419→5f9b96a + verified stamps. Preserved Settings five-tab, ✦ Launch, Pause/End, LTR orbit, silent_mode true. No push.

## check: 2026-10-03T15:52:00-04:00
- reviewed: fp-summary-relaunch-note, fp-summary-relaunch-note-card
- pass: fp-summary-relaunch-note, fp-summary-relaunch-note-card
- fail: (none)
- notes: c5010ec vs live 8:886–888 — Not yet + relaunches>0 uses exact relaunch body + `✦  A NOTE FROM YOUR COPILOT` kicker + `.quest-copilot-note--card`; zero-relaunch 22/23/24 stay plain. Tiny proof CSS to live tokens (#e9ddfd/#c8bfd7/28px/10px/mint #326c78/15px body). Preserved Settings five-tab, ✦ Launch, Pause/End, LTR orbit, silent_mode true. No push.

## check: 2026-10-03T16:00:29-04:00
- reviewed: fp-quest-complete-badge-mint, fp-first-flight-badge-mint, fp-setup-step-mint
- pass: fp-quest-complete-badge-mint, fp-first-flight-badge-mint, fp-setup-step-mint
- fail: (none)
- notes: fbe9617 vs live — 6:1097 / 7:565 / 8:967 all `var(--color-mint,#326c78)`; HEAD quest `--quest` badge + `.mc-first-flight-badge` + `.mission-setup-step` mint; FLIGHT LOGGED base stays purple. Inbox stamped verified. Preserved Settings five-tab, ✦ Launch, Pause/End, LTR orbit, silent_mode true. No code edits; no push.

## check: 2026-10-03T16:01:56-04:00
- reviewed: fp-summary-partly-relaunch-note, fp-summary-finished-relaunch-note
- pass: fp-summary-partly-relaunch-note, fp-summary-finished-relaunch-note
- fail: (none)
- notes: 6ee9efc vs live 6:1226/6:1125 — Partly+relaunch and Finished+relaunch+new-PB body strings exact (curly ’ / em-dash —); card chrome via summaryUsesRelaunchNote + `.quest-copilot-note--card` + mint `✦  A NOTE FROM YOUR COPILOT`; zero-relaunch 22/23 and Not-yet 25 untouched. Preserved Settings five-tab, ✦ Launch, Pause/End, LTR orbit, silent_mode true. No code edits; no push.

## check: 2026-10-03T16:06:11-04:00
- reviewed: fp-summary-partly-note-copy, fp-summary-pb-stat-label, fp-active-signal-on-fill; reconfirm fbe9617 mint (fp-quest-complete-badge-mint, fp-first-flight-badge-mint, fp-setup-step-mint); reconfirm 6ee9efc (fp-summary-partly-relaunch-note, fp-summary-finished-relaunch-note); spot escalate:scrutinous opens
- pass: fp-summary-partly-note-copy, fp-summary-pb-stat-label, fp-active-signal-on-fill; fbe9617 mint + 6ee9efc relaunch notes remain verified
- fail: (none)
- notes: Live 8:692/8:683/10:852 + 6:1097/7:565/8:967 + 6:1226/6:1125 match HEAD 5f9b96a/fbe9617/6ee9efc. Promoted leftover ## done: headers → verified. Scrutinous opens still accurate (incl. fp-session-at-launch-chrome vs live 10:1325) — no rewrite. Preserved Settings five-tab, ✦ Launch, Pause/End, LTR orbit, silent_mode true, #app padding:0. No code edits; no push.

## check: 2026-10-03T16:10:00-04:00
- reviewed: fp-welcome-signin-kicker-mint, fp-setup-toggle-row-chrome, fp-session-user-turn-align
- pass: fp-welcome-signin-kicker-mint, fp-setup-toggle-row-chrome, fp-session-user-turn-align
- fail: (none)
- notes: dbc97fc vs live — 5:59 vars mint `#326c78` / raised `#e9ddfd` on WELCOME ABOARD; 8:1159/8:1164 Camera+Screen plain transparent rows (no lavender mini-cards); 2:38 YOU·JUST NOW + user body left-aligned (`flex-start`). Inbox stamped verified (commit hash e18583f→dbc97fc). Preserved Settings five-tab, ✦ Launch, Pause/End, LTR orbit, silent_mode true. No code edits; no push.

## check: 2026-10-03T16:19:28-04:00
- reviewed: fp-home-copilot-cta-arrow, fp-home-best-extraneous-moon, fp-copilot-responses-footnote
- pass: fp-home-copilot-cta-arrow, fp-home-best-extraneous-moon, fp-copilot-responses-footnote
- fail: (none)
- notes: Live 5:166 text chars are “Open copilot  →” (instance name ↗ stale) — tiny proof restored after 5873ecf over-corrected to ↗; live 5:154 PB cream-only (no moon) + dest moon kept; live 5:256 in composer + 5:257 page foot via `.copilot-responses-foot`. Preserved Settings five-tab, ✦ Launch, Pause/End, LTR orbit, silent_mode true. No push.

## check: 2026-10-03T16:25:30-04:00
- reviewed: fp-home-footnote-rail, fp-home-copilot-shortcut-row; spot escalate:scrutinous (fp-session-at-launch-chrome, fp-welcome-signin-form, fp-end-confirm-modal-spec)
- pass: fp-home-footnote-rail, fp-home-copilot-shortcut-row
- fail: (none)
- notes: Live 2:34 metadata — 5:169 at x=969/y=750 + satellite 22:1311 under PB rail; 5:164 horizontal Talk-it-through + CTA. HEAD 43e68a5 `.mc-home-bottom` grid col2/row2 + `.mc-home-copilot-shortcut` row match. Scrutinous opens still accurate (next-step always-on / Google-only sign-in / window.confirm) — no rewrite. Preserved Settings five-tab, ✦ Launch, Pause/End, LTR orbit, silent_mode true, #app padding:0. No code edits; no push.

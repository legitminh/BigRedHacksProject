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


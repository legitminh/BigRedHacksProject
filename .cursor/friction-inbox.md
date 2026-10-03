# UX friction inbox

Scout appends open items. Fixer marks them `done` after a commit.

Format per item:

```
## open: <short-id>
- area: <path or UI surface>
- friction: <one sentence, tiny human issue>
- fix_hint: <smallest change>
```

---

## done: fr-s7k2
- area: src/main.ts:444-451 (refreshStatus on load)
- friction: If a lock-in is still running in the backend, reopening or refreshing the window lands on home with no timer or goals, even though coaching may still be active.
- fix_hint: After `get_status`, if `status.session?.active`, call `renderSession`, `startTimer`, and `show("view-session")`.
- commit: 9d1de77
- change: refreshStatus restores session view when lock-in is still active.

## done: fr-b4n8
- area: index.html:31,50,92 (back buttons)
- friction: The only control to leave Ask, lock-in setup, and summary is a bare “←” with no visible label, so screen-reader users hear an unlabeled button and sighted users may not know it always goes home.
- fix_hint: Add `aria-label="Back to home"` (and optional visible “Home” text) on each `[data-back]` button.
- commit: 9d1de77
- change: aria-label="Back to home" on all data-back buttons.

## done: fr-c3m1
- area: index.html:35 / view-chat
- friction: Opening Ask shows an empty chat log with no hint that you can type, use shortcuts below, or that Google is optional—just a blank panel.
- fix_hint: Seed `#chat-log` with one muted assistant bubble or a one-line empty-state under the header on first open.
- commit: 9d1de77
- change: muted #chat-empty bubble; restored after New chat.

## done: fr-e9p2
- area: index.html:76 / src/main.ts (#end-session)
- friction: “End” stops screen watching immediately with no confirmation, so a mis-click ends the session and you may miss the summary flow you expected.
- fix_hint: Wrap `stop_lock_in` in a native `confirm()` or inline “End lock-in?” two-step pattern.
- commit: 96ab893
- change: confirm() before stop_lock_in.

## done: fr-d2h5
- area: src/main.ts (study shortcut buttons)
- friction: Tapping “Explain simply”, “Quiz me”, or “Make a plan” only prefills the composer—it does not send—so nothing appears to happen unless you notice the textarea changed.
- fix_hint: After filling the prompt, call `sendChat()` or change button copy to “Fill prompt” / show a brief “Added to message — tap Send” hint.
- commit: 96ab893
- change: study shortcuts call sendChat() after prefilling.

## done: fr-k7m2
- area: index.html `#view-session` / Settings silent mode
- friction: Silent mode lives only under Settings, but the lock-in screen has no way to open Settings—so spoken coach lines cannot be turned off mid-session without ending it.
- fix_hint: Add a session-header control (e.g. “Mute voice”) that toggles `silent_mode` via `save_settings`, or a ghost Settings link that returns to `#view-session` after.
- commit: 6affa31
- change: Session header Mute voice toggles silent_mode via save_settings.

## done: fr-w3n9
- area: src/main.ts `renderSession` / `#view-session`
- friction: Coach nags appear briefly in the desktop overlay (~6.5s) and are intentionally not kept in-app, so if you miss the toast there is no scrollable coach history while the session runs.
- fix_hint: On `session-update`, render the latest `prompts` into a small `#session-coach-log` (last N lines) under the timer.
- commit: c7cf104
- change: #session-coach-log shows last 8 coach prompts on session-update.

## done: fr-o5t1
- area: overlay.html `#toast` / src/overlay.ts
- friction: Overlay coach text has no `aria-live`/`role` and the window ignores clicks, so VoiceOver users may never hear nags and nobody can select or copy a long line before it vanishes.
- fix_hint: Set `role="status"` and `aria-live="assertive"` on `#toast-text`; optionally mirror the latest line to the main session feed (see fr-w3n9).
- commit: 6affa31
- change: role=status and aria-live=assertive on overlay #toast.

## done: fr-h2v6
- area: index.html `#view-summary` / `renderSummary`
- friction: After a lock-in ends, the summary is stats-only with no labeled next step—only “←” back—so “start another block” vs “done for now” is unclear.
- fix_hint: Append a CTA row in `renderSummary`: primary “Lock in again” → `view-lockin`, secondary “Home”.
- commit: c7cf104
- change: Summary CTA row — Lock in again and Home buttons.

## done: fr-j4r8
- area: src/main.ts `startTimer` / `#session-timer`
- friction: When the countdown hits `00:00`, the UI keeps showing zero with no “Time’s up” or finishing state until the backend emits `session-ended`, which can feel like the app stalled.
- fix_hint: In the timer tick, when remaining ≤ 0, update `#session-status` (e.g. “Finishing up…”) and stop the interval until `session-ended`.
- commit: 6affa31
- change: Timer shows Finishing up… and stops interval at 00:00.

## open: fr-a8s3
- area: index.html `#view-settings` Account tab / `renderAccountSettings`
- friction: Settings → Account shows “Not signed in” but offers no sign-in control, so you must back out to home to find the login form.
- fix_hint: When `!status.signed_in`, render the same compact sign-in form (or a “Sign in on home” button that calls `show("view-home")`) inside `#settings-account`.
- deferred: Needs shared sign-in UI with home or duplicate form wiring; skipped this tick for a smaller diff.

## done: fr-n1d4
- area: index.html `#duration` / src/main.ts `#lockin-form` submit
- friction: Duration relies on HTML min/max only, so clearing the field or typing 0/500 sends `NaN` or out-of-range values to `start_lock_in` with a cryptic backend error.
- fix_hint: Clamp with `Math.min(180, Math.max(1, Number(...) || 45))` and mirror the value back into `#duration` before invoke; show `#lockin-error` for invalid input.
- commit: 27c6445
- change: Clamp duration 1–180, mirror #duration, show #lockin-error when adjusted.

## done: fr-p6c2
- area: src/main.ts `#new-chat`
- friction: “New chat” clears the whole thread immediately with no confirmation, so one mis-click loses context you may still need.
- fix_hint: If `#chat-log` has any `.bubble.user`, `confirm("Start a new chat? This clears the conversation.")` before `clear_chat`.
- commit: 27c6445
- change: confirm() before clear_chat when user messages exist.

## done: fr-q7w2
- area: src/main.ts `sendChat` catch block
- friction: After a failed reply, the error bubble stays in the log while the same message is copied back into the composer, which looks like you’re about to send a duplicate.
- fix_hint: On failure, leave the composer empty and add a “Retry” ghost button on the error bubble that resends the last user message (or drop the textarea restore).
- commit: 27c6445
- change: Composer stays empty; error bubble has Retry via retryChatAssistant.

## done: fr-u4k7
- area: src/main.ts `renderSummary` / `#view-summary`
- friction: On-task shows “Unverified” when `screen_checks === 0` with no explanation, so it reads like a bug rather than “we couldn’t verify focus yet.”
- fix_hint: Append a muted line under On task, e.g. “No screen checks this session — ratio unavailable,” or show “—” with that hint in smaller copy.
- commit: c7cf104
- change: Muted hint when screen_checks is 0 under On task.


## done: fr-g4n2
- area: index.html `#settings-general` / Settings → General tab
- friction: The General tab is clickable but only shows “More app preferences will land here,” so it feels like a broken or unfinished screen rather than an intentional empty state.
- fix_hint: Hide or disable the General tab until content exists, or replace the copy with one line (“Nothing here yet”) and `aria-disabled` on the tab.
- commit: fc584ff
- change: General settings tab hidden until content exists; panel copy clarified.

## done: fr-l8k5
- area: src/main.ts `#summary-lockin-again` / `#goals`
- friction: “Lock in again” opens setup with blank goals even though the summary you just read still lists them, so starting the next block means retyping or copy-paste.
- fix_hint: In the click handler (or `renderSummary`), set `#goals` from `summary.goals` and optionally restore last `#duration` from session storage.
- commit: fc584ff
- change: Lock in again prefills goals and last duration from sessionStorage.

## done: fr-t3m7
- area: src/main.ts `renderHome` Ask button / `#view-chat`
- friction: Opening Ask from home does not focus the message field, so you cannot start typing until you click or tab into `#chat-input`.
- fix_hint: After `show("view-chat")`, call `#chat-input`.focus()` (same pattern as after `sendChat`).
- commit: fc584ff
- change: show(view-chat) focuses #chat-input on next frame.

## open: fr-v6p1
- area: index.html `#view-lockin` / `[data-back]` from lock-in setup
- friction: Back to home from lock-in setup discards whatever you typed in Minutes and goals with no warning if you open Lock in again.
- fix_hint: On `#lockin-form` input, mirror values to `sessionStorage`; restore on `show("view-lockin")` or confirm when `#goals` is non-empty and `[data-back]` is pressed.

## open: fr-y7d3
- area: index.html `#view-summary` / `.summary-body` / src/styles.css
- friction: The summary view cannot scroll when stats and distractions are long, so content below the fold is clipped because the page uses `overflow: hidden` and `#summary-body` has no overflow region.
- fix_hint: Give `#summary-body` `flex: 1; min-height: 0; overflow-y: auto` inside the active summary `.view` flex column.

## open: fr-b2f8
- area: src/main.ts `renderHome` Lock in button / `#view-lockin` `#goals`
- friction: Opening lock-in setup from home does not focus the goals field, so you cannot start typing your task until you click or tab into the textarea.
- fix_hint: After `show("view-lockin")`, call `#goals`.focus()` (mirror fr-t3m7 for Ask).

## open: fr-h9t2
- area: index.html `#session-timer` / `#session-status` / src/main.ts `startTimer`
- friction: The countdown and “Finishing up…” status are visual-only with no `aria-live`, so VoiceOver users get no updates when time runs out or coaching state changes.
- fix_hint: Wrap timer + status in a `role="status"` region with `aria-live="polite"` and update `aria-label` or off-screen text when the chip changes.

## open: fr-m4c9
- area: src/main.ts `setChatControlsBusy` / `#chat-input`
- friction: While a reply is in flight, Send and shortcuts disable but the composer stays editable, so typing and pressing Enter feels broken because `sendChat` silently no-ops until the reply finishes.
- fix_hint: Set `#chat-input`.disabled = busy (or `readOnly`) inside `setChatControlsBusy`, matching the Send button.

## open: fr-z1w4
- area: index.html `#view-session` / src/styles.css `.view-session`
- friction: On shorter windows, goals, watch note, and the wellness panel can sit below the fold with no scroll because the session view is a fixed-height flex column and only `#session-coach-log` scrolls internally.
- fix_hint: Add a `.session-body` wrapper with `flex: 1; min-height: 0; overflow-y: auto` around everything below `.session-head`, or make `#view-session` scroll as a whole.

## open: fr-n8j1
- area: src/main.ts `renderLockinHints` / `#lockin-wellness`
- friction: Presage/local-model hints on lock-in setup refresh only on `refreshStatus()`, not when you tap “Lock in”, so the wellness line can stay stale until you sign in/out or reload.
- fix_hint: Call `renderLockinHints(await invoke("get_status"))` when entering `#view-lockin` (home Lock in click and summary “Lock in again”).

# Lock-in pain inbox

Scout appends open items (UX + technical). Fixer marks `done` after a commit.

```
## open: <short-id>
- area: <path or lock-in surface>
- kind: ux | tech
- pain: <one concrete sentence>
- fix_hint: <smallest safe change>
```

---

## open: li-end-race
- area: src-tauri/src/coach.rs `run_local_watch_loop` / src/main.ts `#end-session`
- kind: tech
- pain: Manual “End” sets `active=false` via `stop_coach`, but an in-flight local-watch tick can still hit `finish_session` and emit a second `session-ended` after the UI already showed the invoke summary.
- fix_hint: In `finish_session`, no-op (no emit) when `!session.active` or a `session_ended` flag is set; have `stop_lock_in` set that flag before summarizing.
- deferred: Needs coordinated Rust session flags; race fix is higher risk than this pass.

## done: li-timer-leak
- area: src/main.ts `startTimer` / `session-ended` / `#end-session`
- kind: tech
- pain: The 1s countdown interval is never cleared when a session ends, so the timer keeps ticking and can update `#session-timer` while you are on home or summary.
- fix_hint: Add `stopTimer()` that clears `timerHandle`; call it from the End handler and the `session-ended` listener (and before starting a new timer).
- commit: 2b8249b
- change: Added `stopTimer()`; call from End, `session-ended`, and at start of `startTimer`.

## done: li-coach-error-silent
- area: src-tauri/src/coach.rs (`coach-error` emit) / src/main.ts
- kind: ux
- pain: Presage upload/timeout failures emit `coach-error`, but the main window never listens, so wellness can fail silently during lock-in.
- fix_hint: `listen("coach-error", …)` and surface the message in `#session-watch-note` or a short inline alert on `#view-session`.
- commit: 2b8249b
- change: `listen("coach-error")` updates `#session-watch-note` with the payload.

## done: li-perm-accessibility
- area: index.html `#view-lockin` / coach `watching_note` copy
- kind: ux
- pain: Start copy only mentions Screen Recording and Camera, but tab/app coaching needs Accessibility + Automation — users only learn after focus scans fail.
- fix_hint: Extend the lock-in blurb with one line linking Accessibility and Automation (same wording as the runtime `watching_note` error).
- commit: 2b8249b
- change: Lock-in blurb mentions Accessibility + Automation for Waypoint in System Settings.

## open: li-stressed-skips-ticks
- area: src-tauri/src/coach.rs `mark_local_on_task`
- kind: tech
- pain: While session status is `Stressed` (Presage spike), on-task local ticks are skipped entirely, so summary “on task” can undercount focused work during stressful but legitimate study.
- fix_hint: Still increment `on_task_ticks`/`total_ticks` when focus is clear; keep `Stressed` as a separate chip without blocking tick accounting.
- deferred: Changes summary accounting semantics; needs careful validation.

## open: li-voice-truncate
- area: src-tauri/crates/waypoint-voice/src/lib.rs `speak` / overlay toast
- kind: ux
- pain: Spoken coach lines are hard-truncated at 220 characters while the overlay shows the full nag, so silent mode off still loses the end of long reminders.
- fix_hint: Truncate overlay text to match speech, or queue two `say` snippets for longer lines with a small delay.
- deferred: Voice crate + overlay behavior; larger UX surface.

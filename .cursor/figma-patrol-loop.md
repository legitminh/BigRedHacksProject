# Figma patrol loop

**Mode (2026-10-03):** Figma source may have changed. Prefer **live Figma** (`get_screenshot` / `get_design_context` on file `BR1qUqQ2xrpSCJdLgzhKDI`) over stale `.cursor/figma-refs/*`. When a live frame differs from a local PNG, **update the local ref** and treat live as source of truth. Default scrutiny: **high** — pixel/copy/spacing pedantic; assume prior “verified” items can regress if Figma moved.

## Roles

### 1) Figma patrol (every 10m)
Compare running UI / source to **live Figma** (fallback: `.cursor/figma-refs`). Append up to 5 NEW `## open:` items in `.cursor/figma-patrol-inbox.md`. Do not fix.
- Prefer under-covered or changed frames; re-check Copilot (`03`) every other tick.
- Do not reopen intentional product decisions without a note: Settings five-tab rail, objective ✦ = Launch, Pause/End, LTR orbit, `silent_mode` default true.

### 2) Figma fixer (after each patrol that adds opens)
Fix up to 3 open items (prefer non-escalated smallest first; allow one medium if Copilot-related). One commit. Mark `## done:` with `- commit:` and `- change:`. No push.
- Features must remain wired (nav, chat send, mic, launch, pause/end) — visual match without dead controls.

### 3) Figma compliance (every 20m)
For each recent `## done:` (and any `open` with `escalate: scrutinous`), verify against **live Figma** first.
- Pass → `## verified: id`
- Fail → revert header to `## open: id`, set `- reopened: true`, `- escalate: scrutinous`, note what still mismatches.

### 4) EXTRA SCRUTINOUS FIGMA PATROLER (on escalate + Copilot priority)
Pedantic expected vs actual vs live Figma; rewrite `deviation`/`fix_hint`; may add more opens. Does not fix — leaves work for fixer.
- Copilot screen is always escalate-priority until parent marks a UX-pain phase.

## Endgame (parent-gated — do not auto-run)
When parent requests: (A) UX painpoint scout → inbox; (B) UX fixer — may deviate from Figma **only after** parent approves each proposed deviation with a short description.

## Wakes
- 10m: `AGENT_LOOP_TICK_figma_patrol`
- 20m: `AGENT_LOOP_TICK_figma_compliance`

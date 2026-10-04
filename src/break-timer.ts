/**
 * Fullscreen 5-minute break window.
 * Rust owns window lifecycle + mission pause; this page shows countdown + Resume.
 */
import { invoke } from "@tauri-apps/api/core";
import { emit, listen } from "@tauri-apps/api/event";

const BREAK_DEFAULT_SECS = 5 * 60;

type BreakStartedPayload = {
  duration_secs?: number;
  durationSecs?: number;
  reason?: string;
};

type BreakTickPayload = {
  remaining_secs?: number;
  remainingSecs?: number;
  ends_at?: string;
  endsAt?: string;
};

declare global {
  interface Window {
    __waypointBreakStart?: (payload: BreakStartedPayload) => void;
  }
}

let endsAtMs: number | null = null;
let remainingMs = BREAK_DEFAULT_SECS * 1000;
let tickHandle: number | undefined;
let resuming = false;
let finishedEmitted = false;
let lastAnnouncedMinute = -1;

function $(id: string): HTMLElement | null {
  return document.getElementById(id);
}

function formatCountdownSecs(totalSecs: number): string {
  const total = Math.max(0, Math.floor(totalSecs));
  const m = Math.floor(total / 60).toString().padStart(2, "0");
  const s = (total % 60).toString().padStart(2, "0");
  return `${m}:${s}`;
}

function setStatus(message: string | null) {
  const el = $("break-status");
  if (!el) return;
  if (!message) {
    el.hidden = true;
    el.textContent = "";
    return;
  }
  el.hidden = false;
  el.textContent = message;
}

function remainingSecsLive(): number {
  if (endsAtMs != null) {
    return Math.max(0, Math.ceil((endsAtMs - Date.now()) / 1000));
  }
  return Math.max(0, Math.ceil(remainingMs / 1000));
}

function announceCountdown(secs: number, force = false) {
  const live = $("break-countdown-live");
  if (!live) return;
  if (secs <= 0) {
    if (force || lastAnnouncedMinute !== 0) {
      live.textContent = "Break finished. Resume mission when you are ready.";
      lastAnnouncedMinute = 0;
    }
    return;
  }
  // Announce at start / each whole minute / once when under a minute — not every tick.
  const minuteBucket = Math.max(1, Math.ceil(secs / 60));
  const onBoundary = secs === BREAK_DEFAULT_SECS || secs % 60 === 0 || secs === 60;
  if (!force && !onBoundary && lastAnnouncedMinute !== -1) return;
  if (!force && minuteBucket === lastAnnouncedMinute && secs > 60) return;
  lastAnnouncedMinute = minuteBucket;
  if (secs <= 60) {
    live.textContent = "One minute left in your break.";
  } else {
    live.textContent = `${minuteBucket} minutes remaining on your break.`;
  }
}

function renderCountdown() {
  const el = $("break-countdown");
  if (!el) return;
  const secs = remainingSecsLive();
  el.textContent = formatCountdownSecs(secs);
  el.classList.toggle("is-ending", secs > 0 && secs <= 60);
  el.classList.toggle("is-finished", secs <= 0);
  announceCountdown(secs);
}

function stopTicker() {
  if (tickHandle) {
    window.clearInterval(tickHandle);
    tickHandle = undefined;
  }
}

function markFinished() {
  stopTicker();
  renderCountdown();
  const btn = $("break-resume");
  btn?.classList.add("is-urgent");
  setStatus("Break’s up — tap Resume mission when you’re ready.");
  announceCountdown(0, true);
  if (finishedEmitted) return;
  finishedEmitted = true;
  void emit("break-timer-finished", { remaining_secs: 0 });
}

function startTicker() {
  stopTicker();
  renderCountdown();
  tickHandle = window.setInterval(() => {
    if (endsAtMs != null) {
      remainingMs = Math.max(0, endsAtMs - Date.now());
    }
    renderCountdown();
    if (remainingSecsLive() <= 0) {
      markFinished();
    }
  }, 250);
}

function applyRemainingSecs(secs: number) {
  const clamped = Math.max(0, secs);
  remainingMs = clamped * 1000;
  endsAtMs = Date.now() + remainingMs;
  finishedEmitted = false;
  lastAnnouncedMinute = -1;
  $("break-resume")?.classList.remove("is-urgent");
  if (clamped <= 0) {
    markFinished();
    return;
  }
  setStatus(null);
  startTicker();
}

function applyStartedPayload(payload: BreakStartedPayload | null | undefined) {
  if (!payload || typeof payload !== "object") return;
  const secs = payload.duration_secs ?? payload.durationSecs;
  if (typeof secs === "number" && Number.isFinite(secs) && secs > 0) {
    applyRemainingSecs(secs);
  }
}

function applyTickPayload(payload: BreakTickPayload | number | null | undefined) {
  if (payload == null) return;
  if (typeof payload === "number" && Number.isFinite(payload)) {
    applyRemainingSecs(payload);
    return;
  }
  if (typeof payload !== "object") return;
  const endsAt = payload.ends_at ?? payload.endsAt;
  if (typeof endsAt === "string" && endsAt.trim()) {
    const ms = new Date(endsAt).getTime();
    if (Number.isFinite(ms)) {
      endsAtMs = ms;
      remainingMs = Math.max(0, ms - Date.now());
      finishedEmitted = false;
      lastAnnouncedMinute = -1;
      startTicker();
      return;
    }
  }
  const secs = payload.remaining_secs ?? payload.remainingSecs;
  if (typeof secs === "number" && Number.isFinite(secs)) {
    applyRemainingSecs(secs);
  }
}

function focusResume() {
  window.requestAnimationFrame(() => {
    ($("break-resume") as HTMLButtonElement | null)?.focus({ preventScroll: true });
  });
}

async function resumeMission() {
  if (resuming) return;
  const btn = $("break-resume") as HTMLButtonElement | null;
  resuming = true;
  if (btn) {
    btn.disabled = true;
    btn.setAttribute("aria-busy", "true");
  }
  setStatus("Resuming your mission…");
  try {
    await invoke("end_break_timer");
  } catch (err) {
    const msg = String(err);
    console.error("end_break_timer failed:", err);
    const missing = /command.*not found|unknown command|not allowed/i.test(msg);
    setStatus(
      missing
        ? "Resume isn’t wired in this build yet. Use Resume mission in the main Waypoint window."
        : `Couldn’t resume: ${msg}`,
    );
    if (btn) {
      btn.disabled = false;
      btn.removeAttribute("aria-busy");
    }
    resuming = false;
    focusResume();
  }
}

async function boot() {
  applyRemainingSecs(BREAK_DEFAULT_SECS);
  focusResume();

  window.__waypointBreakStart = (payload) => {
    applyStartedPayload(payload);
    focusResume();
  };

  $("break-resume")?.addEventListener("click", () => {
    void resumeMission();
  });

  // Block Esc / Cmd+W chrome shortcuts in the webview (Rust also denies close).
  window.addEventListener(
    "keydown",
    (event) => {
      const key = event.key.toLowerCase();
      if (key === "escape" || ((event.metaKey || event.ctrlKey) && key === "w")) {
        event.preventDefault();
        event.stopPropagation();
      }
    },
    true,
  );

  try {
    await listen<BreakStartedPayload>("break-timer-started", (event) => {
      applyStartedPayload(event.payload);
      focusResume();
    });
  } catch {
    /* listen unavailable outside Tauri */
  }

  const tickEvents = [
    "break-timer-tick",
    "break-timer-update",
    "break-timer-sync",
  ] as const;
  for (const name of tickEvents) {
    try {
      await listen<BreakTickPayload | number>(name, (event) => {
        applyTickPayload(event.payload);
      });
    } catch {
      /* listen unavailable outside Tauri */
    }
  }

  try {
    await listen("break-timer-ended", () => {
      stopTicker();
      remainingMs = 0;
      endsAtMs = null;
      renderCountdown();
      setStatus("Break ended.");
    });
  } catch {
    /* ignore */
  }

  try {
    const status = await invoke<{
      active?: boolean;
      duration_secs?: number;
      durationSecs?: number;
    }>("get_break_timer_status");
    const secs = status?.duration_secs ?? status?.durationSecs;
    if (status?.active && typeof secs === "number" && secs > 0) {
      applyRemainingSecs(secs);
    }
  } catch {
    /* command unavailable outside Tauri / older builds */
  }

  focusResume();
}

if (document.readyState === "loading") {
  window.addEventListener("DOMContentLoaded", () => {
    void boot();
  }, { once: true });
} else {
  void boot();
}

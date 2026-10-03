import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";

type ViewId =
  | "view-home"
  | "view-chat"
  | "view-lockin"
  | "view-session"
  | "view-summary";

interface StatusPayload {
  google_connected: boolean;
  gemini_ready: boolean;
  presage_ready: boolean;
  session: LockInSession | null;
}

interface ChatMessage {
  role: string;
  content: string;
}

interface LockInSession {
  id: string;
  goals: string;
  duration_secs: number;
  started_at: string;
  ends_at: string;
  modality: string;
  status: string;
  active: boolean;
  prompts: CoachPrompt[];
  vitals: VitalsSnapshot;
}

interface CoachPrompt {
  id: string;
  at: string;
  text: string;
  kind: string;
}

interface VitalsSnapshot {
  heart_rate?: number | null;
  breathing_rate?: number | null;
  hrv_rmssd?: number | null;
  stress_index?: number | null;
  stressed: boolean;
  focus_ok: boolean;
  raw_summary: string;
  source: string;
}

interface SessionSummary {
  goals: string;
  duration_secs: number;
  modality: string;
  on_task_ratio: number;
  top_distractions: string[];
  stress_spikes: number;
  prompts: CoachPrompt[];
  closing_note: string;
}

const $ = <T extends HTMLElement>(sel: string) =>
  document.querySelector(sel) as T | null;

function show(view: ViewId) {
  document.querySelectorAll(".view").forEach((el) => el.classList.remove("active"));
  $(`#${view}`)?.classList.add("active");
}

function renderPills(status: StatusPayload) {
  const host = $("#connection-pills");
  if (!host) return;
  const items = [
    ["Gemini", status.gemini_ready],
    ["Google", status.google_connected],
    ["Presage", status.presage_ready],
  ] as const;
  host.innerHTML = items
    .map(
      ([label, on]) =>
        `<span class="pill ${on ? "on" : ""}">${label}${on ? " · live" : ""}</span>`,
    )
    .join("");

  const btn = $("#btn-google");
  if (btn) {
    btn.textContent = status.google_connected ? "Google connected" : "Connect Google";
  }
}

function appendChat(role: "user" | "assistant", content: string) {
  const log = $("#chat-log");
  if (!log) return;
  const bubble = document.createElement("div");
  bubble.className = `bubble ${role}`;
  bubble.textContent = content;
  log.appendChild(bubble);
  log.scrollTop = log.scrollHeight;
}

function formatRemaining(endsAt: string): string {
  const end = new Date(endsAt).getTime();
  const ms = Math.max(0, end - Date.now());
  const total = Math.floor(ms / 1000);
  const m = Math.floor(total / 60)
    .toString()
    .padStart(2, "0");
  const s = (total % 60).toString().padStart(2, "0");
  return `${m}:${s}`;
}

function statusLabel(status: string): string {
  return status.replace(/([A-Z])/g, "_$1").replace(/^_/, "").toLowerCase();
}

function renderSession(session: LockInSession) {
  const timer = $("#session-timer");
  const status = $("#session-status");
  const modality = $("#session-modality");
  const goals = $("#session-goals");
  const vitals = $("#vitals");
  if (timer) timer.textContent = formatRemaining(session.ends_at);
  if (status) {
    const label = statusLabel(String(session.status));
    status.textContent = label.replace(/_/g, " ");
    status.className = `status-chip ${label}`;
  }
  if (modality) modality.textContent = `Mode · ${session.modality}`;
  if (goals) goals.textContent = session.goals;
  if (vitals) {
    const v = session.vitals;
    vitals.innerHTML = [
      v.heart_rate != null ? `<span>HR ${Math.round(v.heart_rate)}</span>` : "",
      v.breathing_rate != null ? `<span>RR ${Math.round(v.breathing_rate)}</span>` : "",
      v.hrv_rmssd != null ? `<span>HRV ${Math.round(v.hrv_rmssd)}</span>` : "",
      `<span>${v.source}</span>`,
    ]
      .filter(Boolean)
      .join("");
  }
}

function appendPrompt(prompt: CoachPrompt) {
  const feed = $("#prompt-feed");
  if (!feed) return;
  const el = document.createElement("div");
  el.className = "prompt";
  el.textContent = prompt.text;
  feed.prepend(el);
}

function renderSummary(summary: SessionSummary) {
  const body = $("#summary-body");
  if (!body) return;
  const pct = Math.round(summary.on_task_ratio * 100);
  body.innerHTML = `
    <p>${summary.closing_note}</p>
    <h3>Goals</h3>
    <p>${summary.goals}</p>
    <h3>On task</h3>
    <p>${pct}% · modality ${summary.modality} · stress spikes ${summary.stress_spikes}</p>
    <h3>Distractions</h3>
    <p>${summary.top_distractions.length ? summary.top_distractions.join(", ") : "None logged"}</p>
  `;
}

let timerHandle: number | undefined;
let currentEndsAt: string | null = null;

function startTimer(endsAt: string) {
  currentEndsAt = endsAt;
  if (timerHandle) window.clearInterval(timerHandle);
  const tick = () => {
    const el = $("#session-timer");
    if (el && currentEndsAt) el.textContent = formatRemaining(currentEndsAt);
  };
  tick();
  timerHandle = window.setInterval(tick, 1000);
}

async function refreshStatus() {
  try {
    const status = await invoke<StatusPayload>("get_status");
    renderPills(status);
  } catch (e) {
    console.error(e);
  }
}

window.addEventListener("DOMContentLoaded", async () => {
  $("#goto-chat")?.addEventListener("click", () => show("view-chat"));
  $("#goto-lockin")?.addEventListener("click", () => show("view-lockin"));
  document.querySelectorAll("[data-back]").forEach((btn) => {
    btn.addEventListener("click", () => show("view-home"));
  });

  $("#btn-google")?.addEventListener("click", async () => {
    const btn = $("#btn-google");
    if (btn) btn.textContent = "Opening Google…";
    try {
      await invoke("connect_google");
      await refreshStatus();
    } catch (e) {
      alert(String(e));
      await refreshStatus();
    }
  });

  $("#clear-chat")?.addEventListener("click", async () => {
    await invoke("clear_chat");
    const log = $("#chat-log");
    if (log) log.innerHTML = "";
  });

  $("#chat-form")?.addEventListener("submit", async (e) => {
    e.preventDefault();
    const input = $("#chat-input") as HTMLInputElement | null;
    if (!input || !input.value.trim()) return;
    const message = input.value.trim();
    input.value = "";
    appendChat("user", message);
    appendChat("assistant", "Navigating…");
    const log = $("#chat-log");
    const pending = log?.lastElementChild as HTMLElement | null;
    try {
      const reply = await invoke<ChatMessage>("chat_send", { message });
      if (pending) pending.textContent = reply.content;
    } catch (err) {
      if (pending) pending.textContent = `Couldn’t reach Gemini: ${err}`;
    }
  });

  $("#lockin-form")?.addEventListener("submit", async (e) => {
    e.preventDefault();
    const goals = ($("#goals") as HTMLTextAreaElement | null)?.value ?? "";
    const duration = Number(($("#duration") as HTMLInputElement | null)?.value || 45);
    try {
      const session = await invoke<LockInSession>("start_lock_in", {
        goals,
        durationMins: duration,
      });
      const feed = $("#prompt-feed");
      if (feed) feed.innerHTML = "";
      renderSession(session);
      startTimer(session.ends_at);
      show("view-session");
    } catch (err) {
      alert(String(err));
    }
  });

  $("#end-session")?.addEventListener("click", async () => {
    try {
      const summary = await invoke<SessionSummary | null>("stop_lock_in");
      if (summary) {
        renderSummary(summary);
        show("view-summary");
      } else {
        show("view-home");
      }
    } catch (err) {
      alert(String(err));
    }
  });

  await listen<LockInSession>("session-update", (event) => {
    renderSession(event.payload);
    startTimer(event.payload.ends_at);
  });

  await listen<CoachPrompt>("coach-prompt", (event) => {
    appendPrompt(event.payload);
  });

  await listen<SessionSummary>("session-ended", (event) => {
    renderSummary(event.payload);
    show("view-summary");
  });

  await listen<string>("coach-error", (event) => {
    appendPrompt({
      id: crypto.randomUUID(),
      at: new Date().toISOString(),
      text: `Coach hiccup: ${event.payload}`,
      kind: "error",
    });
  });

  await refreshStatus();
});

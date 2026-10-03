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
  google_oauth_ready: boolean;
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
  ends_at: string;
  modality: string;
  status: string;
  active: boolean;
  prompts: CoachPrompt[];
}

interface CoachPrompt {
  id: string;
  at: string;
  text: string;
  kind: string;
}

interface SessionSummary {
  goals: string;
  modality: string;
  on_task_ratio: number;
  top_distractions: string[];
  stress_spikes: number;
  closing_note: string;
}

const $ = <T extends HTMLElement>(sel: string) =>
  document.querySelector(sel) as T | null;

function show(view: ViewId) {
  document.querySelectorAll(".view").forEach((el) => el.classList.remove("active"));
  $(`#${view}`)?.classList.add("active");
}

function renderHome(status: StatusPayload) {
  const host = $("#home-cta");
  if (!host) return;
  host.innerHTML = "";

  if (!status.google_connected) {
    const signIn = document.createElement("button");
    signIn.className = "primary";
    signIn.type = "button";
    signIn.textContent = status.google_oauth_ready
      ? "Sign in with Google"
      : "Google sign-in not configured";
    signIn.disabled = !status.google_oauth_ready;
    signIn.addEventListener("click", async () => {
      signIn.textContent = "Opening Google…";
      signIn.disabled = true;
      try {
        await invoke("connect_google");
        await refreshStatus();
      } catch (e) {
        alert(String(e));
        await refreshStatus();
      }
    });
    host.appendChild(signIn);
    return;
  }

  const chat = document.createElement("button");
  chat.className = "primary";
  chat.type = "button";
  chat.textContent = "Ask";
  chat.addEventListener("click", () => show("view-chat"));

  const lock = document.createElement("button");
  lock.className = "secondary";
  lock.type = "button";
  lock.textContent = "Lock in";
  lock.addEventListener("click", () => show("view-lockin"));

  const out = document.createElement("button");
  out.className = "ghost";
  out.type = "button";
  out.textContent = "Sign out";
  out.addEventListener("click", async () => {
    await invoke("disconnect_google");
    await refreshStatus();
  });

  host.append(chat, lock, out);
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
  const ms = Math.max(0, new Date(endsAt).getTime() - Date.now());
  const total = Math.floor(ms / 1000);
  const m = Math.floor(total / 60).toString().padStart(2, "0");
  const s = (total % 60).toString().padStart(2, "0");
  return `${m}:${s}`;
}

function statusLabel(status: string): string {
  return String(status)
    .replace(/([a-z])([A-Z])/g, "$1_$2")
    .toLowerCase();
}

function renderSession(session: LockInSession) {
  const timer = $("#session-timer");
  const status = $("#session-status");
  const goals = $("#session-goals");
  if (timer) timer.textContent = formatRemaining(session.ends_at);
  if (status) {
    const label = statusLabel(session.status);
    status.textContent = label.replace(/_/g, " ");
    status.className = `status-chip ${label}`;
  }
  if (goals) goals.textContent = session.goals;
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
    <p>${pct}%</p>
    <h3>Distractions</h3>
    <p>${summary.top_distractions.length ? summary.top_distractions.join(", ") : "None"}</p>
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
  const status = await invoke<StatusPayload>("get_status");
  renderHome(status);
  return status;
}

window.addEventListener("DOMContentLoaded", async () => {
  document.querySelectorAll("[data-back]").forEach((btn) => {
    btn.addEventListener("click", () => show("view-home"));
  });

  $("#chat-form")?.addEventListener("submit", async (e) => {
    e.preventDefault();
    const input = $("#chat-input") as HTMLInputElement | null;
    if (!input || !input.value.trim()) return;
    const message = input.value.trim();
    input.value = "";
    appendChat("user", message);
    appendChat("assistant", "…");
    const pending = $("#chat-log")?.lastElementChild as HTMLElement | null;
    try {
      const reply = await invoke<ChatMessage>("chat_send", { message });
      if (pending) pending.textContent = reply.content;
    } catch (err) {
      if (pending) pending.textContent = String(err);
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
    const summary = await invoke<SessionSummary | null>("stop_lock_in");
    if (summary) {
      renderSummary(summary);
      show("view-summary");
    } else {
      show("view-home");
    }
  });

  await listen<LockInSession>("session-update", (event) => {
    renderSession(event.payload);
    startTimer(event.payload.ends_at);
  });
  await listen<CoachPrompt>("coach-prompt", (event) => appendPrompt(event.payload));
  await listen<SessionSummary>("session-ended", (event) => {
    renderSummary(event.payload);
    show("view-summary");
  });
  await listen<string>("coach-error", (event) => {
    appendPrompt({
      id: crypto.randomUUID(),
      at: new Date().toISOString(),
      text: event.payload,
      kind: "error",
    });
  });

  await refreshStatus();
});

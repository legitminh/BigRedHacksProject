import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { openUrl } from "@tauri-apps/plugin-opener";
import { renderMarkdown } from "./markdown.ts";
import { retryChat } from "./chat-retry.ts";

type ViewId =
  | "view-home"
  | "view-chat"
  | "view-lockin"
  | "view-session"
  | "view-summary"
  | "view-settings";

interface UserSettings {
  silent_mode: boolean;
}

interface StatusPayload {
  google_connected: boolean;
  gemini_ready: boolean;
  google_oauth_ready: boolean;
  presage_ready: boolean;
  session: LockInSession | null;
}

interface ChatMessage {
  role: string;
  content: string;
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

interface LockInSession {
  id: string;
  goals: string;
  duration_secs: number;
  ends_at: string;
  modality: string;
  status: string;
  active: boolean;
  prompts: CoachPrompt[];
  vitals?: VitalsSnapshot;
  camera_ready?: boolean;
  presage_ready?: boolean;
  watching_note?: string;
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
  screen_checks?: number;
  top_distractions: string[];
  stress_spikes: number;
  closing_note: string;
  vitals_summary?: string;
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
      if (signIn.disabled) return;
      const label = status.google_oauth_ready
        ? "Sign in with Google"
        : "Google sign-in not configured";
      signIn.textContent = "Waiting for Google…";
      signIn.disabled = true;
      try {
        const connect = invoke("connect_google");
        // UI escape hatch if the browser tab is closed without finishing OAuth.
        const cancel = new Promise<never>((_, reject) => {
          window.setTimeout(() => {
            reject(new Error("Google sign-in timed out or was closed. Click Sign in with Google to try again."));
          }, 90_000);
        });
        await Promise.race([connect, cancel]);
      } catch (e) {
        console.error("connect_google failed:", e);
        alert(String(e));
      } finally {
        // Always rebuild the CTA so the button never stays stuck disabled.
        signIn.textContent = label;
        signIn.disabled = false;
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

  const settings = document.createElement("button");
  settings.className = "ghost";
  settings.type = "button";
  settings.textContent = "Settings";
  settings.addEventListener("click", () => {
    void openSettings();
  });

  const out = document.createElement("button");
  out.className = "ghost";
  out.type = "button";
  out.textContent = "Sign out";
  out.addEventListener("click", async () => {
    await invoke("disconnect_google");
    await refreshStatus();
  });

  host.append(chat, lock, settings, out);
}

function appendChat(role: "user" | "assistant", content: string) {
  const log = $("#chat-log");
  if (!log) return;
  const bubble = document.createElement("div");
  bubble.className = `bubble ${role}`;
  bubble.textContent = content;
  log.appendChild(bubble);
  log.scrollTop = log.scrollHeight;
  return bubble;
}

let chatBusy = false;
let hasChatReply = false;

async function sendChat() {
  const input = $<HTMLTextAreaElement>("#chat-input");
  if (chatBusy || !input?.value.trim()) return;
  const message = input.value;
  input.value = "";
  appendChat("user", message);
  const pending = appendChat("assistant", "Thinking…");
  chatBusy = true;
  $("#chat-log")?.setAttribute("aria-busy", "true");
  document.querySelectorAll<HTMLButtonElement>("#chat-send, [data-study], #new-chat").forEach((button) => button.disabled = true);
  try {
    const reply = await retryChat(message,
      original => invoke<ChatMessage>("chat_send", { message: original }),
      () => {
        if (pending) pending.textContent = "Sorry, there’s a slight delay. Still working on your reply…";
      },
    );
    if (pending) renderMarkdown(pending, reply.content);
    hasChatReply = true;
  } catch {
    if (pending) pending.textContent = "Sorry, I couldn’t get a reply right now. Please try again in a moment.";
    if (!input.value) input.value = message;
  } finally {
    chatBusy = false;
    $("#chat-log")?.setAttribute("aria-busy", "false");
    document.querySelectorAll<HTMLButtonElement>("#chat-send, [data-study], #new-chat").forEach((button) => button.disabled = false);
    const log = $("#chat-log");
    if (log) log.scrollTop = log.scrollHeight;
    if ($("#view-chat")?.classList.contains("active")) input.focus();
  }
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

function formatVitals(vitals?: VitalsSnapshot | null): string {
  if (!vitals || (!vitals.raw_summary && vitals.source !== "presage" && vitals.source !== "fallback")) {
    return "Running quietly in the background (not required to lock in)";
  }
  const bits: string[] = [];
  if (typeof vitals.heart_rate === "number") bits.push(`HR ${Math.round(vitals.heart_rate)}`);
  if (typeof vitals.breathing_rate === "number") bits.push(`RR ${vitals.breathing_rate.toFixed(1)}`);
  if (typeof vitals.stress_index === "number") bits.push(`stress ${Math.round(vitals.stress_index)}`);
  const state = vitals.stressed ? "elevated stress" : "steady";
  const source = vitals.source === "presage" ? "Presage" : vitals.source === "fallback" ? "vision estimate" : vitals.source || "—";
  if (bits.length) return `${bits.join(" · ")} · ${state} (${source})`;
  return vitals.raw_summary || `${state} (${source})`;
}

function renderVitals(vitals?: VitalsSnapshot | null) {
  const line = $("#vitals-line");
  const panel = $("#session-vitals");
  if (line) line.textContent = formatVitals(vitals);
  if (panel) panel.classList.toggle("stressed", Boolean(vitals?.stressed));
}

function renderSession(session: LockInSession) {
  const timer = $("#session-timer");
  const status = $("#session-status");
  const goals = $("#session-goals");
  const note = $("#session-watch-note");
  if (timer) timer.textContent = formatRemaining(session.ends_at);
  if (status) {
    const label = statusLabel(session.status);
    status.textContent = label.replace(/_/g, " ");
    status.className = `status-chip ${label}`;
  }
  if (goals) goals.textContent = session.goals;
  if (note) note.textContent = session.watching_note || "Watching your screen";
  renderVitals(session.vitals);
}

async function openSettings() {
  show("view-settings");
  try {
    const settings = await invoke<UserSettings>("get_settings");
    const silent = $("#setting-silent-mode") as HTMLInputElement | null;
    if (silent) silent.checked = Boolean(settings.silent_mode);
    const status = $("#settings-save-status");
    if (status) status.textContent = "";
  } catch (err) {
    console.error(err);
  }
}

function selectSettingsTab(tab: string) {
  document.querySelectorAll<HTMLButtonElement>("[data-settings-tab]").forEach((button) => {
    const active = button.dataset.settingsTab === tab;
    button.classList.toggle("active", active);
    button.setAttribute("aria-selected", active ? "true" : "false");
  });
  document.querySelectorAll<HTMLElement>(".settings-panel").forEach((panel) => {
    const active = panel.id === `settings-${tab}`;
    panel.classList.toggle("active", active);
    panel.hidden = !active;
  });
}

async function persistSilentMode() {
  const silent = $("#setting-silent-mode") as HTMLInputElement | null;
  const status = $("#settings-save-status");
  try {
    await invoke("save_settings", {
      settings: { silent_mode: Boolean(silent?.checked) },
    });
    if (status) status.textContent = "Saved.";
  } catch (err) {
    if (status) status.textContent = String(err);
  }
}

function renderSummary(summary: SessionSummary) {
  const body = $("#summary-body");
  if (!body) return;
  const checks = summary.screen_checks ?? 0;
  const pct = checks === 0 ? "Unverified" : `${Math.round(summary.on_task_ratio * 100)}%`;
  const escape = (value: string) =>
    value.replace(/&/g, "&amp;").replace(/</g, "&lt;").replace(/>/g, "&gt;");
  body.innerHTML = `
    <p>${escape(summary.closing_note)}</p>
    <h3>Goals</h3>
    <p>${escape(summary.goals)}</p>
    <h3>On task</h3>
    <p>${pct}</p>
    <h3>Screen checks</h3>
    <p>${checks}</p>
    <h3>Stress spikes</h3>
    <p>${summary.stress_spikes}</p>
    <h3>Wellness</h3>
    <p>${escape(summary.vitals_summary || "No wellness reading this session.")}</p>
    <h3>Distractions</h3>
    <p>${summary.top_distractions.length ? escape(summary.top_distractions.join(", ")) : "None"}</p>
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

function renderLockinHints(status: StatusPayload) {
  const hint = $("#lockin-wellness");
  if (!hint) return;
  if (status.presage_ready) {
    hint.textContent = "Presage key detected — wellness checks will use your webcam during the session.";
  } else {
    hint.textContent = "No Presage key yet — screen coaching still works; add presage_api_key in secrets.toml for stress readings.";
  }
}

async function refreshStatus() {
  const status = await invoke<StatusPayload>("get_status");
  renderHome(status);
  renderLockinHints(status);
  return status;
}

window.addEventListener("DOMContentLoaded", async () => {
  document.querySelectorAll("[data-back]").forEach((btn) => {
    btn.addEventListener("click", () => show("view-home"));
  });

  $("#chat-form")?.addEventListener("submit", (e) => {
    e.preventDefault();
    void sendChat();
  });

  $("#chat-input")?.addEventListener("keydown", (event) => {
    if (event.key === "Enter" && !event.shiftKey && !event.isComposing) {
      event.preventDefault();
      void sendChat();
    }
  });

  const studyPrompts: Record<string, [string, string]> = {
    explain: ["Explain this topic simply, with a worked example: ", "Explain the topic we’re discussing more simply, with a worked example."],
    quiz: ["Quiz me on this topic, one question at a time: ", "Quiz me on the topic we’re discussing. Ask one question, wait for my answer, then give feedback."],
    plan: ["Help me make a short study plan. My goal and available time are: ", "Turn what we’ve discussed into at most three concrete study steps with time estimates. Ask about my available time if needed."],
  };
  document.querySelectorAll<HTMLButtonElement>("[data-study]").forEach((button) => {
    button.addEventListener("click", () => {
      const input = $<HTMLTextAreaElement>("#chat-input");
      const prompts = studyPrompts[button.dataset.study || ""];
      if (!input || !prompts || chatBusy) return;
      input.value = input.value.trim()
        ? `${prompts[0]}${input.value.trim()}`
        : prompts[hasChatReply ? 1 : 0];
      input.focus();
      input.setSelectionRange(input.value.length, input.value.length);
    });
  });

  $("#new-chat")?.addEventListener("click", async () => {
    if (chatBusy) return;
    chatBusy = true;
    try {
      await invoke("clear_chat");
      $("#chat-log")?.replaceChildren();
      const input = $<HTMLTextAreaElement>("#chat-input");
      if (input) {
        input.value = "";
        input.focus();
      }
      hasChatReply = false;
    } catch (err) {
      alert(String(err));
    } finally {
      chatBusy = false;
    }
  });

  $("#chat-log")?.addEventListener("click", (event) => {
    const link = (event.target as Element).closest<HTMLAnchorElement>("a");
    if (!link) return;
    event.preventDefault();
    const href = link.getAttribute("href");
    if (href && /^https?:\/\//i.test(href)) {
      void openUrl(href).catch((err) => alert(`Couldn’t open link: ${String(err)}`));
    }
  });

  $("#lockin-form")?.addEventListener("submit", async (e) => {
    e.preventDefault();
    const goalsInput = $("#goals") as HTMLTextAreaElement | null;
    const goals = goalsInput?.value.trim() ?? "";
    const duration = Number(($("#duration") as HTMLInputElement | null)?.value || 45);
    const startBtn = $("#lockin-start") as HTMLButtonElement | null;
    const errEl = $("#lockin-error");
    if (errEl) {
      errEl.hidden = true;
      errEl.textContent = "";
    }
    if (!goals) {
      if (errEl) {
        errEl.hidden = false;
        errEl.textContent = "Add a goal first — what are you locking in on?";
      }
      goalsInput?.focus();
      return;
    }
    if (startBtn) {
      startBtn.disabled = true;
      startBtn.textContent = "Starting…";
    }
    try {
      const session = await invoke<LockInSession>("start_lock_in", {
        goals,
        durationMins: duration,
      });
      renderVitals(null);
      renderSession(session);
      startTimer(session.ends_at);
      show("view-session");
    } catch (err) {
      const message = String(err);
      console.error("start_lock_in failed:", err);
      if (errEl) {
        errEl.hidden = false;
        errEl.textContent = message;
      } else {
        alert(message);
      }
    } finally {
      if (startBtn) {
        startBtn.disabled = false;
        startBtn.textContent = "Start lock-in";
      }
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

  document.querySelectorAll<HTMLButtonElement>("[data-settings-tab]").forEach((button) => {
    button.addEventListener("click", () => selectSettingsTab(button.dataset.settingsTab || "lockin"));
  });
  $("#setting-silent-mode")?.addEventListener("change", () => {
    void persistSilentMode();
  });

  await listen<LockInSession>("session-update", (event) => {
    renderSession(event.payload);
    startTimer(event.payload.ends_at);
  });
  await listen<VitalsSnapshot>("vitals-update", (event) => renderVitals(event.payload));
  await listen<SessionSummary>("session-ended", (event) => {
    renderSummary(event.payload);
    show("view-summary");
  });

  await refreshStatus();
});

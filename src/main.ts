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
  signed_in: boolean;
  username?: string | null;
  google_connected: boolean;
  gemini_ready: boolean;
  google_oauth_ready: boolean;
  presage_ready: boolean;
  local_llm_model?: string;
  local_llm_enabled?: boolean;
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

  if (!status.signed_in) {
    const form = document.createElement("form");
    form.className = "signin-box";
    form.innerHTML = `
      <p class="signin-title">Waypoint sign in</p>
      <p class="signin-sub">Placeholder login — any username and password works for now.</p>
      <label>
        Username or email
        <input id="wp-username" name="username" type="text" autocomplete="username" required placeholder="you@school.edu" />
      </label>
      <label>
        Password
        <input id="wp-password" name="password" type="password" autocomplete="current-password" placeholder="anything" />
      </label>
      <p class="signin-error" id="wp-signin-error" hidden></p>
      <button class="primary wide" type="submit">Sign in</button>
    `;
    form.addEventListener("submit", async (event) => {
      event.preventDefault();
      const username = (form.querySelector("#wp-username") as HTMLInputElement | null)?.value ?? "";
      const password = (form.querySelector("#wp-password") as HTMLInputElement | null)?.value ?? "";
      const err = form.querySelector("#wp-signin-error") as HTMLElement | null;
      const btn = form.querySelector("button[type=submit]") as HTMLButtonElement | null;
      if (err) {
        err.hidden = true;
        err.textContent = "";
      }
      if (btn) {
        btn.disabled = true;
        btn.textContent = "Signing in…";
      }
      try {
        await invoke("sign_in_waypoint", { username, password });
        await refreshStatus();
      } catch (e) {
        if (err) {
          err.hidden = false;
          err.textContent = String(e);
        } else {
          alert(String(e));
        }
        if (btn) {
          btn.disabled = false;
          btn.textContent = "Sign in";
        }
      }
    });
    host.appendChild(form);
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
    await invoke("sign_out_waypoint");
    await refreshStatus();
  });

  host.append(chat, lock, settings, out);
}

function renderAccountSettings(status: StatusPayload) {
  const userEl = $("#account-waypoint-user");
  if (userEl) {
    userEl.textContent = status.signed_in
      ? status.username || "Signed in"
      : "Not signed in";
  }

  const googleStatus = $("#account-google-status");
  if (googleStatus) {
    googleStatus.textContent = status.google_connected
      ? "Connected"
      : status.google_oauth_ready
        ? "Not connected"
        : "Not configured";
  }

  const actions = $("#account-google-actions");
  if (!actions) return;
  actions.innerHTML = "";

  if (status.google_connected) {
    const disconnect = document.createElement("button");
    disconnect.className = "ghost";
    disconnect.type = "button";
    disconnect.textContent = "Disconnect Google";
    disconnect.addEventListener("click", async () => {
      disconnect.disabled = true;
      try {
        await invoke("disconnect_google");
        await refreshStatus();
      } catch (e) {
        alert(String(e));
        disconnect.disabled = false;
      }
    });
    actions.appendChild(disconnect);
    return;
  }

  const connect = document.createElement("button");
  connect.className = "secondary";
  connect.type = "button";
  connect.textContent = status.google_oauth_ready
    ? "Connect Google"
    : "Google not configured";
  connect.disabled = !status.google_oauth_ready;
  connect.addEventListener("click", async () => {
    if (connect.disabled) return;
    const label = connect.textContent || "Connect Google";
    connect.textContent = "Waiting for Google…";
    connect.disabled = true;
    try {
      const job = invoke("connect_google");
      const cancel = new Promise<never>((_, reject) => {
        window.setTimeout(() => {
          reject(new Error("Google connect timed out or was closed. Try again from Settings."));
        }, 90_000);
      });
      await Promise.race([job, cancel]);
    } catch (e) {
      console.error("connect_google failed:", e);
      alert(String(e));
    } finally {
      connect.textContent = label;
      connect.disabled = false;
      await refreshStatus();
    }
  });
  actions.appendChild(connect);
}

function renderChatEmptyState() {
  const log = $("#chat-log");
  if (!log || log.querySelector("#chat-empty")) return;
  const bubble = document.createElement("div");
  bubble.id = "chat-empty";
  bubble.className = "bubble assistant chat-empty";
  bubble.textContent =
    "Ask a question below, try the study shortcuts, or connect Google in Settings for Calendar and Drive — optional.";
  log.appendChild(bubble);
}

function appendChat(role: "user" | "assistant", content: string) {
  const log = $("#chat-log");
  if (!log) return;
  $("#chat-empty")?.remove();
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
    const appStatus = await invoke<StatusPayload>("get_status");
    renderAccountSettings(appStatus);
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

function stopTimer() {
  if (timerHandle) {
    window.clearInterval(timerHandle);
    timerHandle = undefined;
  }
  currentEndsAt = null;
}

function startTimer(endsAt: string) {
  stopTimer();
  currentEndsAt = endsAt;
  const tick = () => {
    const el = $("#session-timer");
    if (el && currentEndsAt) el.textContent = formatRemaining(currentEndsAt);
  };
  tick();
  timerHandle = window.setInterval(tick, 1000);
}

async function renderLockinHints(status: StatusPayload) {
  const hint = $("#lockin-wellness");
  if (!hint) return;
  const bits: string[] = [];
  if (status.local_llm_enabled !== false) {
    try {
      const local = await invoke<string>("local_llm_status");
      bits.push(local);
    } catch {
      bits.push(`Local model: ${status.local_llm_model || "qwen2.5:0.5b"} (checking…)`);
    }
  }
  if (status.presage_ready) {
    bits.push("Presage ready for webcam wellness.");
  } else {
    bits.push("No Presage key — wellness optional.");
  }
  hint.textContent = bits.join(" · ");
}

async function refreshStatus() {
  const status = await invoke<StatusPayload>("get_status");
  renderHome(status);
  await renderLockinHints(status);
  if ($("#view-settings")?.classList.contains("active")) {
    renderAccountSettings(status);
  }
  const session = status.session;
  if (session?.active) {
    renderSession(session);
    startTimer(session.ends_at);
    show("view-session");
  }
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
      void sendChat();
    });
  });

  $("#new-chat")?.addEventListener("click", async () => {
    if (chatBusy) return;
    chatBusy = true;
    try {
      await invoke("clear_chat");
      $("#chat-log")?.replaceChildren();
      renderChatEmptyState();
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
    if (!window.confirm("End this lock-in? Screen watching will stop.")) {
      return;
    }
    stopTimer();
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
  await listen<string>("coach-error", (event) => {
    const note = $("#session-watch-note");
    if (note) note.textContent = `Wellness check: ${event.payload}`;
  });
  await listen<SessionSummary>("session-ended", (event) => {
    stopTimer();
    renderSummary(event.payload);
    show("view-summary");
  });

  await refreshStatus();
});

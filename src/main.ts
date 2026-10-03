import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { openUrl } from "@tauri-apps/plugin-opener";
import { renderMarkdown } from "./markdown.ts";
import { retryChat } from "./chat-retry.ts";
import {
  initShipUI,
  onMissionCompleted,
  onMissionStarted,
  refreshAllShipViews,
  refreshSessionFlight,
  updateSessionFlight,
} from "./ship.ts";

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

interface SystemPermissions {
  screen_recording: boolean;
  camera: boolean;
  accessibility: boolean;
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
  duration_secs: number;
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

let lastSummaryGoals = "";

const LAUNCH_CELEBRATION_MS = 2200;

function updateSummaryCelebration(firstFlight: boolean): void {
  const block = $("#summary-celebration");
  if (!block) return;
  block.hidden = false;
  block.classList.remove(
    "mission-celebration--quest",
    "mission-celebration--first-flight",
    "mission-celebration--enter",
  );
  void block.offsetWidth;
  block.classList.add(
    firstFlight ? "mission-celebration--first-flight" : "mission-celebration--quest",
    "mission-celebration--enter",
  );
  const title = $("#summary-celebration-title");
  const sub = $("#summary-celebration-sub");
  if (firstFlight) {
    if (title) title.textContent = "First flight";
    if (sub) {
      sub.textContent =
        "You completed your first mission. Your flight log starts here — ready for the next orbit?";
    }
  } else {
    if (title) title.textContent = "Quest complete";
    if (sub) sub.textContent = "Mission ended — here's your flight summary.";
  }
}

function showSummaryWithCelebration(summary: SessionSummary): void {
  const firstFlight = onMissionCompleted(summary, summary.duration_secs);
  renderSummary(summary);
  updateSummaryCelebration(firstFlight);
  show("view-summary");
}

function playLaunchCelebration(then: () => void): void {
  const el = $("#celebration-launch");
  if (!el) {
    then();
    return;
  }
  el.hidden = false;
  el.classList.remove("mission-launch--active");
  void el.offsetWidth;
  el.classList.add("mission-launch--active");
  window.setTimeout(() => {
    el.hidden = true;
    el.classList.remove("mission-launch--active");
    then();
  }, LAUNCH_CELEBRATION_MS);
}
let welcomeSignInOpen = false;

const START_HERE_KEY = "waypoint-start-here-dismissed";

const DEMO_HOME_GOALS = [
  {
    title: "Finish Calc PSet 3",
    meta: "2 missions left · due Wed",
    planet: "lavender" as const,
  },
  {
    title: "Draft history essay outline",
    meta: "1 mission planned · due Fri",
    planet: "teal" as const,
  },
  {
    title: "Review orgo lab prep",
    meta: "Not scheduled yet",
    planet: "amber" as const,
  },
];

function show(view: ViewId) {
  document.querySelectorAll(".view").forEach((el) => el.classList.remove("active"));
  $(`#${view}`)?.classList.add("active");
  if (view === "view-chat") {
    requestAnimationFrame(() => {
      ($("#chat-input") as HTMLTextAreaElement | null)?.focus();
    });
  }
}

function syncDurationChips() {
  const durationInput = $("#duration") as HTMLInputElement | null;
  const mins = Number(durationInput?.value.trim() ?? "");
  document.querySelectorAll<HTMLButtonElement>(".duration-chip").forEach((chip) => {
    const preset = Number(chip.dataset.minutes);
    chip.classList.toggle("is-active", !Number.isNaN(mins) && preset === mins);
  });
}

function setMissionDuration(mins: number) {
  const durationInput = $("#duration") as HTMLInputElement | null;
  if (durationInput) {
    durationInput.value = String(mins);
  }
  syncDurationChips();
}

function restoreLockinFromLastSession() {
  const goalsInput = $("#goals") as HTMLTextAreaElement | null;
  if (goalsInput && lastSummaryGoals) {
    goalsInput.value = lastSummaryGoals;
  }
  try {
    const mins = sessionStorage.getItem("lockin-last-duration");
    const durationInput = $("#duration") as HTMLInputElement | null;
    if (durationInput && mins) {
      durationInput.value = mins;
    }
  } catch {
    // ignore private mode / quota
  }
  syncDurationChips();
}

function missionLaunchLabel(loading: boolean) {
  return loading ? "Launching…" : "Launch mission";
}

function setMissionLaunchButton(loading: boolean) {
  const startBtn = $("#lockin-start") as HTMLButtonElement | null;
  const labelEl = startBtn?.querySelector(".mission-launch-label");
  if (startBtn) startBtn.disabled = loading;
  if (labelEl) labelEl.textContent = missionLaunchLabel(loading);
}

function syncStartHerePanel(signedIn: boolean) {
  const panel = $("#start-here");
  if (!panel) return;
  if (signedIn || welcomeSignInOpen) {
    panel.hidden = true;
    return;
  }
  try {
    panel.hidden = localStorage.getItem(START_HERE_KEY) === "1";
  } catch {
    panel.hidden = false;
  }
}

function mountSignInForm(host: HTMLElement) {
  const form = document.createElement("form");
  form.className = "signin-box";
  form.innerHTML = `
    <button type="button" class="ghost signin-back" id="wp-signin-back">← Welcome</button>
    <p class="signin-title">Log in to Mission Control</p>
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
    <button class="primary wide pill" type="submit">Sign in</button>
  `;
  form.querySelector("#wp-signin-back")?.addEventListener("click", () => {
    welcomeSignInOpen = false;
    void refreshStatus();
  });
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
      welcomeSignInOpen = false;
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
  (form.querySelector("#wp-username") as HTMLInputElement | null)?.focus();
}

function openWelcomeSignIn() {
  welcomeSignInOpen = true;
  void refreshStatus();
}

function renderHomeGoals() {
  const list = $("#home-goals");
  const count = $("#home-goal-count");
  if (!list) return;
  const goals = DEMO_HOME_GOALS.map((g) => ({ ...g }));
  if (lastSummaryGoals.trim()) {
    const line = lastSummaryGoals.trim().split("\n")[0]?.trim() || lastSummaryGoals.trim();
    goals[0] = { ...goals[0], title: line, meta: "From your last mission" };
  }
  list.innerHTML = goals.map(
    (goal) => `
    <li class="mc-goal-card">
      <span class="mc-goal-planet mc-goal-planet--${goal.planet}" aria-hidden="true"></span>
      <div class="mc-goal-body">
        <p class="mc-goal-title">${escapeHtml(goal.title)}</p>
        <p class="mc-goal-meta">${escapeHtml(goal.meta)}</p>
      </div>
    </li>`,
  ).join("");
  if (count) count.textContent = String(goals.length);
}

function renderHomeNav(status: StatusPayload) {
  const nav = $("#home-nav-actions");
  if (!nav) return;
  nav.innerHTML = "";
  if (!status.signed_in) return;

  const who = status.username?.trim();
  if (who) {
    const label = document.createElement("span");
    label.className = "mc-nav-user";
    label.textContent = who;
    nav.appendChild(label);
  }

  const copilot = document.createElement("button");
  copilot.className = "ghost pill";
  copilot.type = "button";
  copilot.textContent = "Copilot";
  copilot.addEventListener("click", () => show("view-chat"));

  const settings = document.createElement("button");
  settings.className = "ghost pill";
  settings.type = "button";
  settings.textContent = "Settings";
  settings.addEventListener("click", () => {
    void openSettings();
  });

  nav.append(copilot, settings);
}

function renderHome(status: StatusPayload) {
  const home = $("#view-home");
  home?.classList.toggle("view-home--signed-in", status.signed_in);

  $("#home-guest")?.toggleAttribute("hidden", status.signed_in);
  $("#home-dashboard")?.toggleAttribute("hidden", !status.signed_in);

  renderHomeNav(status);
  syncStartHerePanel(status.signed_in);

  if (!status.signed_in) {
    const host = $("#home-guest-cta");
    if (!host) return;
    host.innerHTML = "";
    host.classList.toggle("mc-guest-cta--signin", welcomeSignInOpen);

    if (welcomeSignInOpen) {
      mountSignInForm(host);
      return;
    }

    const enter = document.createElement("button");
    enter.className = "primary pill";
    enter.type = "button";
    enter.textContent = "Enter Mission Control";
    enter.addEventListener("click", openWelcomeSignIn);

    const login = document.createElement("button");
    login.className = "secondary pill";
    login.type = "button";
    login.textContent = "Log in";
    login.addEventListener("click", openWelcomeSignIn);

    host.append(enter, login);
    return;
  }

  welcomeSignInOpen = false;
  renderHomeGoals();

  const host = $("#home-cta");
  if (!host) return;
  host.innerHTML = "";

  const start = document.createElement("button");
  start.className = "primary wide pill";
  start.type = "button";
  start.textContent = "Start mission";
  start.addEventListener("click", () => show("view-lockin"));

  const copilot = document.createElement("button");
  copilot.className = "secondary wide pill";
  copilot.type = "button";
  copilot.textContent = "Open Copilot";
  copilot.addEventListener("click", () => show("view-chat"));

  host.append(start, copilot);
  refreshAllShipViews();
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

function setPermissionBadge(
  id: string,
  granted: boolean,
  labels?: { on: string; off: string },
) {
  const el = $(`#${id}`);
  if (!el) return;
  const on = labels?.on ?? "Enabled";
  const off = labels?.off ?? "Grant access";
  el.textContent = granted ? on : off;
  el.setAttribute("data-state", granted ? "ok" : "needs");
}

async function renderPermissionsStatus() {
  ["perm-screen", "perm-camera", "perm-accessibility"].forEach((id) => {
    const el = $(`#${id}`);
    if (el) {
      el.textContent = "Checking…";
      el.setAttribute("data-state", "unknown");
    }
  });
  try {
    const perms = await invoke<SystemPermissions>("get_system_permissions");
    setPermissionBadge("perm-screen", perms.screen_recording, {
      on: "Enabled",
      off: "Required — grant access",
    });
    setPermissionBadge("perm-camera", perms.camera, {
      on: "Enabled",
      off: "Optional — grant for wellness",
    });
    setPermissionBadge("perm-accessibility", perms.accessibility, {
      on: "Enabled",
      off: "Grant for tab coaching",
    });
  } catch (err) {
    console.error(err);
    ["perm-screen", "perm-camera", "perm-accessibility"].forEach((id) => {
      const el = $(`#${id}`);
      if (el) {
        el.textContent = "Couldn’t check";
        el.setAttribute("data-state", "needs");
      }
    });
  }
}

function connectionRow(label: string, detail: string, ok: boolean): string {
  const state = ok ? "ok" : "warn";
  const status = ok ? "Connected" : "Offline";
  return `<li class="mc-conn-row">
    <div class="mc-conn-copy">
      <strong>${escapeHtml(label)}</strong>
      <span>${escapeHtml(detail)}</span>
    </div>
    <span class="mc-conn-dot" data-state="${state}" aria-hidden="true"></span>
    <span class="mc-conn-status">${status}</span>
  </li>`;
}

async function renderConnectionStatus(status: StatusPayload) {
  const list = $("#connection-status-list");
  if (!list) return;
  list.innerHTML = `<li class="mc-conn-row mc-conn-row--loading"><span class="muted">Checking links…</span></li>`;

  let localLine = "Local coach model";
  if (status.local_llm_enabled !== false) {
    try {
      localLine = await invoke<string>("local_llm_status");
    } catch {
      localLine = `Local model: ${status.local_llm_model || "qwen2.5:0.5b"} (checking…)`;
    }
  } else {
    localLine = "Local model disabled in config";
  }
  const localOk = /ready|online|running/i.test(localLine);

  const googleDetail = status.google_connected
    ? "Calendar and Drive linked for Copilot"
    : status.google_oauth_ready
      ? "Optional — connect for Calendar / Drive context"
      : "OAuth client not configured in this build";

  list.innerHTML = [
    connectionRow(
      "Gemini coach",
      status.gemini_ready ? "Cloud vision + chat API configured" : "Missing API key",
      status.gemini_ready,
    ),
    connectionRow("Google", googleDetail, status.google_connected),
    connectionRow(
      "Presage wellness",
      status.presage_ready ? "Webcam stress API key present" : "Optional — add Presage key for HR checks",
      status.presage_ready,
    ),
    connectionRow("Local LLM", localLine, localOk),
    connectionRow(
      "Waypoint account",
      status.signed_in
        ? `Signed in as ${status.username || "you"}`
        : "Sign in on home for synced Mission Control",
      status.signed_in,
    ),
  ].join("");
}

async function renderMissionControlSettings(status: StatusPayload) {
  await renderConnectionStatus(status);
  await renderPermissionsStatus();
  renderAccountSettings(status);
}

function renderChatEmptyState() {
  const log = $("#chat-log");
  if (!log || log.querySelector("#chat-empty")) return;
  const empty = document.createElement("div");
  empty.id = "chat-empty";
  empty.className = "copilot-empty";
  empty.innerHTML = `
    <div class="copilot-orbit" aria-hidden="true">
      <span class="copilot-orbit-ring copilot-orbit-ring--outer"></span>
      <span class="copilot-orbit-ring copilot-orbit-ring--inner"></span>
      <span class="copilot-planet"></span>
    </div>
    <h3 class="copilot-empty-heading">How can I help on this mission?</h3>
    <p class="copilot-empty-copy">
      Ask a question below, try the study shortcuts, or connect Google in Settings for Calendar and Drive — optional.
    </p>`;
  log.appendChild(empty);
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

const CHAT_FAIL_MSG =
  "Sorry, I couldn’t get a reply right now. Please try again in a moment.";

function setChatControlsBusy(busy: boolean) {
  $("#chat-log")?.setAttribute("aria-busy", busy ? "true" : "false");
  document
    .querySelectorAll<HTMLButtonElement>("#chat-send, [data-study], #new-chat")
    .forEach((button) => {
      button.disabled = busy;
    });
}

function showChatFailure(bubble: HTMLElement, userMessage: string) {
  bubble.replaceChildren();
  bubble.append(document.createTextNode(`${CHAT_FAIL_MSG} `));
  const retry = document.createElement("button");
  retry.type = "button";
  retry.className = "ghost chat-retry";
  retry.textContent = "Retry";
  retry.addEventListener("click", () => {
    void retryChatAssistant(userMessage, bubble);
  });
  bubble.appendChild(retry);
}

async function retryChatAssistant(userMessage: string, bubble: HTMLElement) {
  if (chatBusy) return;
  bubble.textContent = "Thinking…";
  chatBusy = true;
  setChatControlsBusy(true);
  try {
    const reply = await retryChat(
      userMessage,
      (original) => invoke<ChatMessage>("chat_send", { message: original }),
      () => {
        bubble.textContent =
          "Sorry, there’s a slight delay. Still working on your reply…";
      },
    );
    renderMarkdown(bubble, reply.content);
    hasChatReply = true;
  } catch {
    showChatFailure(bubble, userMessage);
  } finally {
    chatBusy = false;
    setChatControlsBusy(false);
    const log = $("#chat-log");
    if (log) log.scrollTop = log.scrollHeight;
    const input = $<HTMLTextAreaElement>("#chat-input");
    if ($("#view-chat")?.classList.contains("active")) input?.focus();
  }
}

async function sendChat() {
  const input = $<HTMLTextAreaElement>("#chat-input");
  if (chatBusy || !input?.value.trim()) return;
  const message = input.value;
  input.value = "";
  appendChat("user", message);
  const pending = appendChat("assistant", "Thinking…");
  chatBusy = true;
  setChatControlsBusy(true);
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
    if (pending) showChatFailure(pending, message);
  } finally {
    chatBusy = false;
    setChatControlsBusy(false);
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

function missionElapsedFraction(endsAt: string, durationSecs: number): number {
  const total = Math.max(1, durationSecs);
  const remaining = Math.max(0, new Date(endsAt).getTime() - Date.now()) / 1000;
  return Math.min(1, Math.max(0, (total - remaining) / total));
}

function updateSessionOrbit(endsAt: string, durationSecs: number) {
  const frac = missionElapsedFraction(endsAt, durationSecs);
  const progress = $("#session-orbit-progress");
  if (progress) {
    progress.style.strokeDashoffset = `${100 - frac * 100}`;
  }
  const shipWrap = $("#session-orbit-ship-wrap");
  if (shipWrap) {
    shipWrap.style.setProperty("--orbit-deg", `${frac * 360 - 90}deg`);
  }
}

function applySessionOrbitState(label: string) {
  const orbit = $("#session-orbit");
  if (orbit) orbit.setAttribute("data-state", label);
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

const SESSION_COACH_MAX = 8;

function escapeHtml(value: string): string {
  return value.replace(/&/g, "&amp;").replace(/</g, "&lt;").replace(/>/g, "&gt;");
}

function renderSessionCoachLog(prompts: CoachPrompt[]) {
  const log = $("#session-coach-log");
  if (!log) return;
  const recent = prompts.slice(-SESSION_COACH_MAX);
  if (!recent.length) {
    log.innerHTML =
      '<p class="muted session-coach-empty">Coach messages appear here if you miss the overlay.</p>';
    return;
  }
  log.innerHTML = recent.map((p) => `<div class="prompt">${escapeHtml(p.text)}</div>`).join("");
  log.scrollTop = log.scrollHeight;
}

function renderSession(session: LockInSession) {
  currentSessionDurationSecs = session.duration_secs;
  const timer = $("#session-timer");
  const status = $("#session-status");
  const goals = $("#session-goals");
  const note = $("#session-watch-note");
  if (timer) timer.textContent = formatRemaining(session.ends_at);
  if (status) {
    const label = statusLabel(session.status);
    status.textContent = label.replace(/_/g, " ");
    status.className = `status-chip session-status ${label}`;
    applySessionOrbitState(label);
  }
  if (goals) goals.textContent = session.goals;
  if (note) note.textContent = session.watching_note || "Watching your screen";
  renderVitals(session.vitals);
  renderSessionCoachLog(session.prompts);
  onMissionStarted(session.duration_secs);
  refreshSessionFlight();
  updateSessionOrbit(session.ends_at, session.duration_secs);
  updateSessionFlight(session.ends_at, session.duration_secs);
  void syncSessionMuteButton();
}

async function openSettings() {
  show("view-settings");
  try {
    const settings = await invoke<UserSettings>("get_settings");
    const silent = $("#setting-silent-mode") as HTMLInputElement | null;
    if (silent) silent.checked = Boolean(settings.silent_mode);
    const status = $("#settings-save-status");
    if (status) status.textContent = "";
    const invokeResult = $("#invoke-voice-result");
    if (invokeResult) invokeResult.textContent = "";
    const appStatus = await invoke<StatusPayload>("get_status");
    await renderMissionControlSettings(appStatus);
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
  if (tab === "permissions") {
    void renderPermissionsStatus();
  }
}

async function persistSilentMode() {
  const silent = $("#setting-silent-mode") as HTMLInputElement | null;
  const status = $("#settings-save-status");
  try {
    await invoke("save_settings", {
      settings: { silent_mode: Boolean(silent?.checked) },
    });
    if (status) status.textContent = "Saved.";
    await syncSessionMuteButton();
  } catch (err) {
    if (status) status.textContent = String(err);
  }
}

async function syncSessionMuteButton() {
  const btn = $("#session-mute-voice");
  if (!btn) return;
  try {
    const settings = await invoke<UserSettings>("get_settings");
    const on = Boolean(settings.silent_mode);
    btn.textContent = on ? "Unmute voice" : "Mute voice";
    btn.setAttribute("aria-pressed", on ? "true" : "false");
  } catch {
    // ignore — session UI still usable
  }
}

async function toggleSessionMute() {
  try {
    const settings = await invoke<UserSettings>("get_settings");
    const next = !settings.silent_mode;
    await invoke("save_settings", { settings: { silent_mode: next } });
    const silent = $("#setting-silent-mode") as HTMLInputElement | null;
    if (silent) silent.checked = next;
    await syncSessionMuteButton();
  } catch (err) {
    console.error(err);
  }
}

function formatFlightDuration(secs: number): string {
  const total = Math.max(0, Math.round(secs));
  const h = Math.floor(total / 3600);
  const m = Math.floor((total % 3600) / 60);
  if (h > 0 && m > 0) return `${h}h ${m}m`;
  if (h > 0) return `${h}h`;
  if (m > 0) return `${m}m`;
  return `${total}s`;
}

function renderSummary(summary: SessionSummary) {
  lastSummaryGoals = summary.goals;
  const closing = $("#summary-closing");
  const stats = $("#summary-stats");
  const body = $("#summary-body");
  if (!closing || !stats || !body) return;

  const checks = summary.screen_checks ?? 0;
  const onTaskValue = checks === 0 ? "—" : `${Math.round(summary.on_task_ratio * 100)}%`;
  const onTaskHint =
    checks === 0
      ? '<p class="flight-log-stat-hint muted">No screen checks — focus ratio unavailable.</p>'
      : "";

  closing.textContent = summary.closing_note;

  stats.innerHTML = `
    <article class="flight-log-stat">
      <span class="flight-log-stat-icon" aria-hidden="true">⏱</span>
      <p class="flight-log-stat-value">${escapeHtml(formatFlightDuration(summary.duration_secs))}</p>
      <p class="flight-log-stat-label">Flight time</p>
    </article>
    <article class="flight-log-stat">
      <span class="flight-log-stat-icon" aria-hidden="true">◎</span>
      <p class="flight-log-stat-value">${escapeHtml(onTaskValue)}</p>
      <p class="flight-log-stat-label">On task</p>
      ${onTaskHint}
    </article>
    <article class="flight-log-stat">
      <span class="flight-log-stat-icon" aria-hidden="true">📡</span>
      <p class="flight-log-stat-value">${checks}</p>
      <p class="flight-log-stat-label">Screen checks</p>
    </article>
    <article class="flight-log-stat">
      <span class="flight-log-stat-icon" aria-hidden="true">⚡</span>
      <p class="flight-log-stat-value">${summary.stress_spikes}</p>
      <p class="flight-log-stat-label">Stress spikes</p>
    </article>
  `;

  const distractions = summary.top_distractions.length
    ? summary.top_distractions.map((d) => `<li>${escapeHtml(d)}</li>`).join("")
    : '<li class="muted">None logged</li>';

  body.innerHTML = `
    <article class="flight-log-card">
      <h3 class="flight-log-card-title">Mission objectives</h3>
      <p class="flight-log-goals">${escapeHtml(summary.goals)}</p>
      <p class="flight-log-modality muted">${escapeHtml(summary.modality)}</p>
    </article>
    <article class="flight-log-card">
      <h3 class="flight-log-card-title">Wellness</h3>
      <p>${escapeHtml(summary.vitals_summary || "No wellness reading this session.")}</p>
    </article>
    <article class="flight-log-card">
      <h3 class="flight-log-card-title">Distractions</h3>
      <ul class="flight-log-distractions">${distractions}</ul>
    </article>
  `;
  refreshAllShipViews();
}

let timerHandle: number | undefined;
let currentEndsAt: string | null = null;
let currentSessionDurationSecs = 0;

function stopTimer() {
  if (timerHandle) {
    window.clearInterval(timerHandle);
    timerHandle = undefined;
  }
  currentEndsAt = null;
  currentSessionDurationSecs = 0;
}

function startTimer(endsAt: string) {
  if (timerHandle) {
    window.clearInterval(timerHandle);
    timerHandle = undefined;
  }
  currentEndsAt = endsAt;
  const tick = () => {
    if (!currentEndsAt) return;
    const el = $("#session-timer");
    const ms = new Date(currentEndsAt).getTime() - Date.now();
    if (el) el.textContent = formatRemaining(currentEndsAt);
    if (currentSessionDurationSecs > 0) {
      updateSessionOrbit(currentEndsAt, currentSessionDurationSecs);
      updateSessionFlight(currentEndsAt, currentSessionDurationSecs);
    }
    if (ms <= 0) {
      const statusEl = $("#session-status");
      if (statusEl) {
        statusEl.textContent = "Finishing up…";
        statusEl.className = "status-chip session-status finishing";
      }
      if (timerHandle) {
        window.clearInterval(timerHandle);
        timerHandle = undefined;
      }
    }
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
    await renderMissionControlSettings(status);
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

  $("#summary-lockin-again")?.addEventListener("click", () => {
    show("view-lockin");
    restoreLockinFromLastSession();
  });
  $("#summary-go-home")?.addEventListener("click", () => show("view-home"));

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
    const hasUserMessages = Boolean(
      $("#chat-log")?.querySelector(".bubble.user"),
    );
    if (
      hasUserMessages &&
      !window.confirm(
        "Start a new chat? This clears the conversation.",
      )
    ) {
      return;
    }
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
    const durationInput = $("#duration") as HTMLInputElement | null;
    const rawDuration = durationInput?.value.trim() ?? "";
    const parsedDuration = Number(rawDuration);
    const errEl = $("#lockin-error");
    if (errEl) {
      errEl.hidden = true;
      errEl.textContent = "";
    }
    let duration = Math.min(180, Math.max(1, parsedDuration || 45));
    let durationAdjusted = false;
    if (rawDuration === "" || Number.isNaN(parsedDuration)) {
      duration = 45;
      durationAdjusted = true;
      if (errEl) {
        errEl.hidden = false;
        errEl.textContent = "Enter 1–180 minutes; using 45.";
      }
    } else if (parsedDuration < 1 || parsedDuration > 180) {
      durationAdjusted = true;
      if (errEl) {
        errEl.hidden = false;
        errEl.textContent = `Duration must be 1–180 minutes; using ${duration}.`;
      }
    }
    if (durationInput && durationAdjusted) {
      durationInput.value = String(duration);
    }
    if (!goals) {
      if (errEl) {
        errEl.hidden = false;
        errEl.textContent = "Add your mission objectives before launch.";
      }
      goalsInput?.focus();
      return;
    }
    setMissionLaunchButton(true);
    try {
      const session = await invoke<LockInSession>("start_lock_in", {
        goals,
        durationMins: duration,
      });
      try {
        sessionStorage.setItem("lockin-last-duration", String(duration));
      } catch {
        // ignore
      }
      renderVitals(null);
      playLaunchCelebration(() => {
        renderSession(session);
        startTimer(session.ends_at);
        show("view-session");
      });
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
      setMissionLaunchButton(false);
    }
  });

  document.querySelectorAll<HTMLButtonElement>(".duration-chip").forEach((chip) => {
    chip.addEventListener("click", () => {
      const mins = Number(chip.dataset.minutes);
      if (!Number.isNaN(mins)) setMissionDuration(mins);
    });
  });
  $("#duration")?.addEventListener("input", () => syncDurationChips());
  syncDurationChips();

  $("#session-mute-voice")?.addEventListener("click", () => {
    void toggleSessionMute();
  });

  $("#end-session")?.addEventListener("click", async () => {
    if (!window.confirm("End this mission? Screen watching will stop.")) {
      return;
    }
    stopTimer();
    const summary = await invoke<SessionSummary | null>("stop_lock_in");
    if (summary) {
      showSummaryWithCelebration(summary);
    } else {
      show("view-home");
    }
  });

  document.querySelectorAll<HTMLButtonElement>("[data-settings-tab]").forEach((button) => {
    button.addEventListener("click", () =>
      selectSettingsTab(button.dataset.settingsTab || "connection"),
    );
  });
  $("#setting-silent-mode")?.addEventListener("change", () => {
    void persistSilentMode();
  });
  $("#connection-refresh")?.addEventListener("click", async () => {
    try {
      const status = await invoke<StatusPayload>("get_status");
      await renderConnectionStatus(status);
    } catch (err) {
      console.error(err);
    }
  });
  $("#permissions-refresh")?.addEventListener("click", () => {
    void renderPermissionsStatus();
  });
  $("#invoke-voice-speak")?.addEventListener("click", async () => {
    const out = $("#invoke-voice-result");
    const btn = $("#invoke-voice-speak") as HTMLButtonElement | null;
    if (btn) btn.disabled = true;
    if (out) out.textContent = "Speaking…";
    try {
      await invoke("voice_speak", {
        text: "Waypoint voice invoke test. Coaching audio is online.",
      });
      if (out) out.textContent = "Speak command finished.";
    } catch (err) {
      if (out) out.textContent = String(err);
    } finally {
      if (btn) btn.disabled = false;
    }
  });
  $("#invoke-voice-listen")?.addEventListener("click", async () => {
    const out = $("#invoke-voice-result");
    const btn = $("#invoke-voice-listen") as HTMLButtonElement | null;
    if (btn) btn.disabled = true;
    if (out) out.textContent = "Listening for 4 seconds…";
    try {
      const transcript = await invoke<VoiceTranscript>("voice_listen_test", {
        seconds: 4,
      });
      if (out) {
        out.textContent = transcript.text?.trim()
          ? `Heard: ${transcript.text}`
          : "Mic test finished (no transcript text).";
      }
    } catch (err) {
      if (out) out.textContent = String(err);
    } finally {
      if (btn) btn.disabled = false;
    }
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
    showSummaryWithCelebration(event.payload);
  });

  $("#start-here-dismiss")?.addEventListener("click", () => {
    try {
      localStorage.setItem(START_HERE_KEY, "1");
    } catch {
      // ignore private mode
    }
    const panel = $("#start-here");
    if (panel) panel.hidden = true;
  });

  initShipUI();
  await refreshStatus();
});

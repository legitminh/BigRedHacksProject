import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { openUrl } from "@tauri-apps/plugin-opener";
import { renderMarkdown } from "./markdown.ts";
import { retryChat } from "./chat-retry.ts";
import {
  initShipUI,
  onMissionCompleted,
  onMissionStarted,
  readShipProgress,
  refreshAllShipViews,
  refreshHomePersonalBest,
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

const PREF_CAMERA_SIGNALS = "wp-setting-camera-signals";
const PREF_SCREEN_SHARING = "wp-setting-screen-sharing";
const PREF_REDUCE_MOTION = "wp-setting-reduce-motion";

function readBoolPref(key: string, defaultValue: boolean): boolean {
  try {
    const raw = localStorage.getItem(key);
    if (raw === null) return defaultValue;
    return raw === "1";
  } catch {
    return defaultValue;
  }
}

function writeBoolPref(key: string, value: boolean) {
  try {
    localStorage.setItem(key, value ? "1" : "0");
  } catch {
    // private mode
  }
}

function syncSilentModeInputs(silentMode: boolean) {
  const silent = $("#setting-silent-mode") as HTMLInputElement | null;
  const audio = $("#setting-copilot-audio") as HTMLInputElement | null;
  if (silent) silent.checked = silentMode;
  if (audio) audio.checked = !silentMode;
}

function applyReduceMotionPref() {
  const on = readBoolPref(PREF_REDUCE_MOTION, false);
  document.documentElement.classList.toggle("wp-reduce-motion", on);
  const input = $("#setting-reduce-motion") as HTMLInputElement | null;
  if (input) input.checked = on;
}

function syncSessionPreferenceToggles() {
  const camera = $("#setting-camera-signals") as HTMLInputElement | null;
  const screen = $("#setting-screen-sharing") as HTMLInputElement | null;
  if (camera) camera.checked = readBoolPref(PREF_CAMERA_SIGNALS, false);
  if (screen) screen.checked = readBoolPref(PREF_SCREEN_SHARING, false);
  applyReduceMotionPref();
}

function renderNavAvatar(status: StatusPayload) {
  const name = status.username?.trim();
  const initials = !name
    ? "—"
    : (() => {
        const parts = name.split(/\s+/).filter(Boolean);
        return parts.length >= 2
          ? `${parts[0][0] ?? ""}${parts[1][0] ?? ""}`.toUpperCase()
          : name.slice(0, 2).toUpperCase();
      })();
  for (const id of ["settings-avatar", "session-avatar"]) {
    const el = $(`#${id}`);
    if (el) el.textContent = initials;
  }
}

function renderSettingsAvatar(status: StatusPayload) {
  renderNavAvatar(status);
}

interface SystemPermissions {
  screen_recording: boolean;
  camera: boolean;
  accessibility: boolean;
}

interface VoiceTranscript {
  text: string;
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
  on_task_ticks?: number;
  total_ticks?: number;
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
let lastSessionSummary: SessionSummary | null = null;
let lastSummaryPersonalBest: { previous: number; isNew: boolean; delta: number } | null = null;
let lastSummaryRelaunches = 0;

type ObjectiveOutcome = "finished" | "partly" | "not-yet";
let lastSummaryObjective: ObjectiveOutcome = "finished";

const LONGEST_FLIGHT_KEY = "waypoint-longest-flight-min";
const RELAUNCH_FLAG_KEY = "waypoint-summary-from-relaunch";

const LAUNCH_CELEBRATION_MS = 2200;

function flightMinutesFromSecs(secs: number): number {
  return Math.max(0, Math.round(secs / 60));
}

function recordLongestFlight(minutes: number): { previous: number; isNew: boolean; delta: number } {
  const previous = Number(localStorage.getItem(LONGEST_FLIGHT_KEY)) || 0;
  const isNew = minutes > previous && minutes > 0;
  if (isNew) localStorage.setItem(LONGEST_FLIGHT_KEY, String(minutes));
  return { previous, isNew, delta: isNew ? minutes - previous : 0 };
}

function consumeRelaunchFlag(): number {
  const fromRelaunch = sessionStorage.getItem(RELAUNCH_FLAG_KEY) === "1";
  sessionStorage.removeItem(RELAUNCH_FLAG_KEY);
  return fromRelaunch ? 1 : 0;
}

function objectivePhrase(outcome: ObjectiveOutcome): string {
  if (outcome === "finished") return "finished your objective";
  if (outcome === "partly") return "partly finished your objective";
  return "didn't finish your objective yet";
}

function buildCopilotNote(
  summary: SessionSummary,
  flightMinutes: number,
  relaunches: number,
  pb: { previous: number; isNew: boolean; delta: number },
  outcome: ObjectiveOutcome,
): string {
  const goalLine = summary.goals.trim().split("\n")[0]?.trim();
  let note = `You logged ${flightMinutes} flight minute${flightMinutes === 1 ? "" : "s"}`;
  if (relaunches > 0) {
    note += `, relaunched ${relaunches === 1 ? "once" : `${relaunches} times`}`;
  }
  note += `, and said you ${objectivePhrase(outcome)}`;
  if (goalLine) note += ` — ${goalLine}`;
  note += ".";
  if (pb.isNew && pb.delta > 0) {
    note += ` That's ${pb.delta} minute${pb.delta === 1 ? "" : "s"} beyond your previous longest flight.`;
  } else if (summary.closing_note) {
    note += ` ${summary.closing_note}`;
  }
  return note;
}

function resetObjectiveButtons(): void {
  document.querySelectorAll<HTMLButtonElement>(".quest-objective-btn").forEach((btn) => {
    btn.classList.toggle("is-selected", btn.dataset.objective === "finished");
  });
}

function refreshSummaryCopilotNote(): void {
  if (!lastSessionSummary || !lastSummaryPersonalBest) return;
  const closing = $("#summary-closing");
  if (!closing) return;
  closing.textContent = buildCopilotNote(
    lastSessionSummary,
    flightMinutesFromSecs(lastSessionSummary.duration_secs),
    lastSummaryRelaunches,
    lastSummaryPersonalBest,
    lastSummaryObjective,
  );
}

function updateSummaryCelebration(summary: SessionSummary, firstFlight: boolean): void {
  const block = $("#summary-celebration");
  if (!block) return;
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
  const badge = $("#summary-celebration-badge");
  const title = $("#summary-celebration-title");
  const sub = $("#summary-celebration-sub");
  const goalLine = summary.goals.trim().split("\n")[0]?.trim() || summary.goals.trim();
  if (firstFlight) {
    if (badge) badge.textContent = "✦  FIRST FLIGHT";
    if (title) title.textContent = "One mission. Well done.";
    if (sub) sub.textContent = goalLine || "Your first personal best is on the board.";
  } else {
    if (badge) badge.textContent = "✓  QUEST COMPLETE";
    if (title) title.textContent = "One mission. Well done.";
    if (sub) sub.textContent = goalLine || "Mission ended — your debrief is below.";
  }
}

function showSummaryWithCelebration(summary: SessionSummary): void {
  const firstFlight = onMissionCompleted(summary, summary.duration_secs);
  lastSummaryPersonalBest = recordLongestFlight(flightMinutesFromSecs(summary.duration_secs));
  lastSummaryRelaunches = consumeRelaunchFlag();
  lastSummaryObjective = "finished";
  resetObjectiveButtons();
  renderSummary(summary);
  updateSummaryCelebration(summary, firstFlight);
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
const START_HERE_KEY = "waypoint-start-here-dismissed";

function navInitials(username?: string | null): string {
  const who = username?.trim() || "You";
  const parts = who.split(/\s+/).filter(Boolean);
  if (parts.length >= 2) {
    return (parts[0].charAt(0) + parts[1].charAt(0)).toUpperCase();
  }
  return who.slice(0, 2).toUpperCase();
}

function show(view: ViewId) {
  document.querySelectorAll(".view").forEach((el) => el.classList.remove("active"));
  $(`#${view}`)?.classList.add("active");
  if (view === "view-chat") {
    requestAnimationFrame(() => {
      ($("#chat-input") as HTMLTextAreaElement | null)?.focus();
    });
  }
  if (view === "view-lockin") {
    syncDurationChips();
    void invoke<StatusPayload>("get_status")
      .then(renderLockinHints)
      .catch(() => {});
    requestAnimationFrame(() => {
      ($("#goals") as HTMLTextAreaElement | null)?.focus();
    });
  }
  if (view === "view-session") {
    requestAnimationFrame(() => {
      ($("#session-chat-input") as HTMLTextAreaElement | null)?.focus();
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
  return loading ? "Launching…" : "Launch mission →";
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
  if (signedIn) {
    panel.hidden = true;
    return;
  }
  try {
    panel.hidden = localStorage.getItem(START_HERE_KEY) === "1";
  } catch {
    panel.hidden = false;
  }
}

async function submitWelcomeSignIn(username: string, password: string) {
  const form = $("#welcome-signin-form");
  const err = $("#wp-signin-error");
  const btn = form?.querySelector("button[type=submit]") as HTMLButtonElement | null;
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
      btn.textContent = "Sign in →";
    }
  }
}

function wireWelcomeSignIn() {
  const form = $("#welcome-signin-form");
  form?.addEventListener("submit", (event) => {
    event.preventDefault();
    const username = ($("#wp-username") as HTMLInputElement | null)?.value ?? "";
    const password = ($("#wp-password") as HTMLInputElement | null)?.value ?? "";
    void submitWelcomeSignIn(username, password);
  });
  $("#welcome-continue-guest")?.addEventListener("click", () => {
    void submitWelcomeSignIn("Guest", "");
  });
}

function renderGuestNav() {
  const brand = $("#home-nav-brand");
  const nav = $("#home-nav-actions");
  const center = $("#home-nav-center");
  const header = document.querySelector(".welcome-nav");
  header?.classList.add("welcome-nav--guest");
  header?.classList.remove("welcome-nav--signed-in");
  center?.toggleAttribute("hidden", true);
  if (brand) {
    brand.innerHTML = '<span class="welcome-brand-star" aria-hidden="true">✦</span> Waypoint';
  }
  if (nav) {
    nav.innerHTML = "";
    const tag = document.createElement("span");
    tag.className = "welcome-nav-tag";
    tag.textContent = "Your space to make progress";
    nav.appendChild(tag);
  }
}

function renderHomeNav(status: StatusPayload) {
  const nav = $("#home-nav-actions");
  const center = $("#home-nav-center");
  const brand = $("#home-nav-brand");
  const header = document.querySelector(".welcome-nav");
  if (!nav) return;
  nav.innerHTML = "";
  if (center) {
    center.innerHTML = "";
    center.toggleAttribute("hidden", !status.signed_in);
  }
  header?.classList.toggle("welcome-nav--signed-in", status.signed_in);
  header?.classList.remove("welcome-nav--guest");
  if (!status.signed_in) return;

  if (brand) {
    brand.innerHTML = '<span class="mc-nav-star" aria-hidden="true">✦</span> Waypoint';
  }

  const links: { label: string; view: ViewId; active?: boolean }[] = [
    { label: "Home", view: "view-home", active: true },
    { label: "Copilot", view: "view-chat" },
    { label: "Lock in", view: "view-lockin" },
  ];
  for (const link of links) {
    const btn = document.createElement("button");
    btn.type = "button";
    btn.className = link.active ? "mc-nav-link mc-nav-link--active" : "mc-nav-link";
    btn.textContent = link.label;
    btn.addEventListener("click", () => show(link.view));
    center?.appendChild(btn);
  }

  const settings = document.createElement("button");
  settings.type = "button";
  settings.className = "mc-nav-link";
  settings.textContent = "Settings";
  settings.addEventListener("click", () => {
    void openSettings();
  });

  const avatar = document.createElement("span");
  avatar.className = "mc-nav-avatar";
  avatar.setAttribute("aria-hidden", "true");
  avatar.textContent = navInitials(status.username);

  nav.append(settings, avatar);
}

function renderHome(status: StatusPayload) {
  const home = $("#view-home");
  home?.classList.toggle("view-home--signed-in", status.signed_in);
  home?.classList.toggle("view-home--guest", !status.signed_in);

  $("#home-guest")?.toggleAttribute("hidden", status.signed_in);

  if (!status.signed_in) {
    $("#home-first-flight")?.toggleAttribute("hidden", true);
    $("#home-dashboard")?.toggleAttribute("hidden", true);
    renderGuestNav();
    syncStartHerePanel(status.signed_in);
    return;
  }

  const showFirstFlightHome = readShipProgress().completedMissions === 0;
  $("#home-first-flight")?.toggleAttribute("hidden", !showFirstFlightHome);
  $("#home-dashboard")?.toggleAttribute("hidden", showFirstFlightHome);

  renderHomeNav(status);
  syncStartHerePanel(status.signed_in);

  if (showFirstFlightHome) {
    refreshAllShipViews();
    return;
  }

  refreshHomePersonalBest();

  const host = $("#home-cta");
  if (host) {
    host.innerHTML = "";
    const lockIn = document.createElement("button");
    lockIn.className = "mc-home-btn-primary";
    lockIn.type = "button";
    lockIn.textContent = "Let's lock in →";
    lockIn.addEventListener("click", () => show("view-lockin"));
    host.appendChild(lockIn);
  }

  const copilotHost = $("#home-copilot-cta");
  if (copilotHost) {
    copilotHost.innerHTML = "";
    const copilot = document.createElement("button");
    copilot.className = "mc-home-btn-secondary";
    copilot.type = "button";
    copilot.textContent = "Open copilot →";
    copilot.addEventListener("click", () => show("view-chat"));
    copilotHost.appendChild(copilot);
  }

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
  renderSettingsAvatar(status);
  await renderConnectionStatus(status);
  await renderPermissionsStatus();
  renderAccountSettings(status);
}

function renderChatEmptyState() {
  const log = $("#chat-log");
  if (!log || log.querySelector("#chat-empty")) return;
  const empty = document.createElement("div");
  empty.id = "chat-empty";
  empty.className = "copilot-empty-card";
  empty.innerHTML = `
    <p class="copilot-card-kicker"><span aria-hidden="true">✦</span> COPILOT</p>
    <h3 class="copilot-empty-heading">What are you working on today?</h3>
    <p class="copilot-empty-copy">
      Tell me what feels tricky, and we’ll find one manageable place to start.
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
  $("#session-chat-log")?.setAttribute("aria-busy", busy ? "true" : "false");
  document
    .querySelectorAll<HTMLButtonElement>(
      "#chat-send, #chat-mic, #session-chat-send, #session-chat-mic, [data-study], #new-chat",
    )
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

function appendSessionChat(role: "user" | "assistant", content: string) {
  const log = $("#session-chat-log");
  if (!log) return;
  $("#session-chat-empty")?.remove();
  const bubble = document.createElement("div");
  bubble.className = `bubble ${role}`;
  bubble.textContent = content;
  log.appendChild(bubble);
  log.scrollTop = log.scrollHeight;
  return bubble;
}

async function dispatchChatMessage(
  message: string,
  appendUser: (text: string) => HTMLElement | undefined,
  appendAssistant: (text: string) => HTMLElement | undefined,
  focusInput?: HTMLTextAreaElement | null,
) {
  if (chatBusy || !message.trim()) return;
  appendUser(message);
  const pending = appendAssistant("Thinking…");
  chatBusy = true;
  setChatControlsBusy(true);
  try {
    const reply = await retryChat(message, (original) =>
      invoke<ChatMessage>("chat_send", { message: original }),
    () => {
      if (pending) pending.textContent = "Sorry, there’s a slight delay. Still working on your reply…";
    });
    if (pending) renderMarkdown(pending, reply.content);
    hasChatReply = true;
  } catch {
    if (pending) showChatFailure(pending, message);
  } finally {
    chatBusy = false;
    setChatControlsBusy(false);
    const log = $("#chat-log");
    if (log) log.scrollTop = log.scrollHeight;
    const sessionLog = $("#session-chat-log");
    if (sessionLog) sessionLog.scrollTop = sessionLog.scrollHeight;
    if (focusInput && $("#view-session")?.classList.contains("active")) focusInput.focus();
    else if ($("#view-chat")?.classList.contains("active")) {
      ($("#chat-input") as HTMLTextAreaElement | null)?.focus();
    }
  }
}

async function sendChat() {
  const input = $<HTMLTextAreaElement>("#chat-input");
  if (!input?.value.trim()) return;
  const message = input.value;
  input.value = "";
  await dispatchChatMessage(
    message,
    (text) => appendChat("user", text),
    (text) => appendChat("assistant", text),
    input,
  );
}

async function sendSessionChat() {
  const input = $<HTMLTextAreaElement>("#session-chat-input");
  if (!input?.value.trim()) return;
  const message = input.value;
  input.value = "";
  await dispatchChatMessage(
    message,
    (text) => appendSessionChat("user", text),
    (text) => appendSessionChat("assistant", text),
    input,
  );
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
    shipWrap.style.setProperty("--flight-pct", String(frac));
  }
}

let currentOnTaskTicks = 0;
let currentTotalTicks = 0;

function flightMinutesEarned(endsAt: string, durationSecs: number, onTask: number, total: number): number {
  const totalMins = Math.max(1, Math.round(durationSecs / 60));
  const remaining = Math.max(0, new Date(endsAt).getTime() - Date.now()) / 1000;
  const elapsedMins = Math.max(0, durationSecs - remaining) / 60;
  const ratio = total > 0 ? onTask / total : 0;
  return Math.min(totalMins, Math.floor(elapsedMins * ratio));
}

function updateFlightMinutesLine(endsAt: string, durationSecs: number) {
  const el = $("#session-flight-minutes");
  if (!el) return;
  const totalMins = Math.max(1, Math.round(durationSecs / 60));
  const earned = flightMinutesEarned(endsAt, durationSecs, currentOnTaskTicks, currentTotalTicks);
  el.textContent = `${earned} of ${totalMins} flight minutes earned`;
}

function applySessionOrbitState(label: string) {
  const orbit = $("#session-orbit");
  if (orbit) orbit.setAttribute("data-state", label);
}

function updateSessionOrbitPersonalBest() {
  const wrap = $("#session-orbit-personal-best");
  const label = $("#session-orbit-pb-label");
  if (!wrap || !label) return;
  const mins = readShipProgress().longestFlightMinutes || 0;
  if (mins <= 0) {
    wrap.hidden = true;
    return;
  }
  wrap.hidden = false;
  label.textContent = `Personal best ${mins} min`;
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

function syncSessionSignalPills() {
  const cameraOn = ($("#lockin-camera") as HTMLInputElement | null)?.checked ?? false;
  const screenOn = ($("#lockin-screen") as HTMLInputElement | null)?.checked ?? false;
  const camera = $("#session-pill-camera");
  const screen = $("#session-pill-screen");
  if (camera) {
    camera.textContent = cameraOn ? "Camera on" : "Camera off";
    camera.classList.toggle("is-on", cameraOn);
  }
  if (screen) {
    screen.textContent = screenOn ? "Screen on" : "Screen off";
    screen.classList.toggle("is-on", screenOn);
  }
}

function updateSessionProgressPill(label: string, finishing = false) {
  const pill = $("#session-progress-pill");
  const text = $("#session-progress-label");
  if (!pill || !text) return;
  pill.classList.toggle("is-finishing", finishing);
  text.textContent = finishing
    ? "Finishing up…"
    : label === "distracted"
      ? "Needs focus"
      : "Mission in progress";
}

function renderSession(session: LockInSession) {
  currentSessionDurationSecs = session.duration_secs;
  currentOnTaskTicks = session.on_task_ticks ?? 0;
  currentTotalTicks = session.total_ticks ?? 0;
  const timer = $("#session-timer");
  const status = $("#session-status");
  const goals = $("#session-goals");
  const note = $("#session-watch-note");
  if (timer) timer.textContent = formatRemaining(session.ends_at);
  if (status) {
    const label = statusLabel(session.status);
    status.textContent = `Coach status: ${label.replace(/_/g, " ")}`;
    applySessionOrbitState(label);
    updateSessionProgressPill(label);
  }
  if (goals) {
    goals.textContent = session.goals || "Your mission";
    goals.hidden = false;
  }
  if (note) note.textContent = session.watching_note || "Watching your screen";
  syncSessionSignalPills();
  renderVitals(session.vitals);
  renderSessionCoachLog(session.prompts);
  onMissionStarted(session.duration_secs);
  refreshSessionFlight();
  updateSessionOrbitPersonalBest();
  updateSessionOrbit(session.ends_at, session.duration_secs);
  updateSessionFlight(session.ends_at, session.duration_secs);
  updateFlightMinutesLine(session.ends_at, session.duration_secs);
  void syncSessionMuteButton();
  void invoke<StatusPayload>("get_status")
    .then(renderNavAvatar)
    .catch(() => {});
}

async function openSettings() {
  show("view-settings");
  try {
    const settings = await invoke<UserSettings>("get_settings");
    syncSilentModeInputs(Boolean(settings.silent_mode));
    syncSessionPreferenceToggles();
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
  const audio = $("#setting-copilot-audio") as HTMLInputElement | null;
  const silentMode = silent
    ? silent.checked
    : audio
      ? !audio.checked
      : false;
  syncSilentModeInputs(silentMode);
  const status = $("#settings-save-status");
  try {
    await invoke("save_settings", {
      settings: { silent_mode: silentMode },
    });
    if (status) status.textContent = "Saved.";
    await syncSessionMuteButton();
  } catch (err) {
    if (status) status.textContent = String(err);
  }
}

async function persistCopilotAudioFromToggle() {
  const audio = $("#setting-copilot-audio") as HTMLInputElement | null;
  const silent = $("#setting-silent-mode") as HTMLInputElement | null;
  if (silent && audio) silent.checked = !audio.checked;
  await persistSilentMode();
}

async function syncSessionMuteButton() {
  const btn = $("#session-mute-voice");
  if (!btn) return;
  try {
    const settings = await invoke<UserSettings>("get_settings");
    const silent = Boolean(settings.silent_mode);
    btn.textContent = silent ? "Audio off" : "Audio on";
    btn.setAttribute("aria-pressed", silent ? "true" : "false");
    btn.classList.toggle("is-on", !silent);
  } catch {
    // ignore — session UI still usable
  }
}

async function toggleSessionMute() {
  try {
    const settings = await invoke<UserSettings>("get_settings");
    const next = !settings.silent_mode;
    await invoke("save_settings", { settings: { silent_mode: next } });
    syncSilentModeInputs(next);
    await syncSessionMuteButton();
  } catch (err) {
    console.error(err);
  }
}

function renderSummary(summary: SessionSummary) {
  lastSummaryGoals = summary.goals;
  lastSessionSummary = summary;
  const closing = $("#summary-closing");
  const stats = $("#summary-stats");
  const body = $("#summary-body");
  const pbBanner = $("#summary-pb-banner");
  if (!closing || !stats || !body) return;

  const flightMinutes = flightMinutesFromSecs(summary.duration_secs);
  const relaunches = lastSummaryRelaunches;
  const pb = lastSummaryPersonalBest ?? recordLongestFlight(flightMinutes);
  const checks = summary.screen_checks ?? 0;
  const onTaskValue = checks === 0 ? "—" : `${Math.round(summary.on_task_ratio * 100)}%`;

  closing.textContent = buildCopilotNote(summary, flightMinutes, relaunches, pb, lastSummaryObjective);

  const pbStatValue =
    pb.isNew && pb.delta > 0 ? `+${pb.delta} min` : `${Math.max(flightMinutes, pb.previous)} min`;
  const pbStatLabel = pb.isNew && pb.delta > 0 ? "new personal best" : "longest flight";

  stats.innerHTML = `
    <article class="quest-stat">
      <p class="flight-log-stat-value">${flightMinutes}</p>
      <p class="flight-log-stat-label">flight minutes</p>
    </article>
    <article class="quest-stat">
      <p class="flight-log-stat-value">${relaunches}</p>
      <p class="flight-log-stat-label">relaunch</p>
    </article>
    <article class="quest-stat quest-stat--best">
      <p class="flight-log-stat-value">${escapeHtml(pbStatValue)}</p>
      <p class="flight-log-stat-label">${escapeHtml(pbStatLabel)}</p>
    </article>
  `;

  if (pbBanner) {
    if (pb.isNew && pb.previous > 0) {
      pbBanner.hidden = false;
      pbBanner.textContent = `New longest flight! ${pb.previous} → ${flightMinutes} min`;
    } else {
      pbBanner.hidden = true;
      pbBanner.textContent = "";
    }
  }

  const distractions = summary.top_distractions.length
    ? summary.top_distractions.map((d) => `<li>${escapeHtml(d)}</li>`).join("")
    : '<li class="muted">None logged</li>';

  body.innerHTML = `
    <article class="flight-log-card">
      <h3 class="flight-log-card-title">Focus</h3>
      <p><strong>On task:</strong> ${escapeHtml(onTaskValue)} · <strong>Screen checks:</strong> ${checks} · <strong>Stress spikes:</strong> ${summary.stress_spikes}</p>
    </article>
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
      updateFlightMinutesLine(currentEndsAt, currentSessionDurationSecs);
    }
    if (ms <= 0) {
      const statusEl = $("#session-status");
      if (statusEl) {
        statusEl.textContent = "Coach status: finishing";
      }
      updateSessionProgressPill("finishing", true);
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
  wireWelcomeSignIn();

  document.querySelectorAll("[data-back]").forEach((btn) => {
    btn.addEventListener("click", () => show("view-home"));
  });

  $("#summary-lockin-again")?.addEventListener("click", () => {
    sessionStorage.setItem(RELAUNCH_FLAG_KEY, "1");
    show("view-lockin");
    restoreLockinFromLastSession();
  });
  $("#summary-go-home")?.addEventListener("click", () => show("view-home"));

  document.querySelectorAll<HTMLButtonElement>(".quest-objective-btn").forEach((button) => {
    button.addEventListener("click", () => {
      document.querySelectorAll(".quest-objective-btn").forEach((btn) => btn.classList.remove("is-selected"));
      button.classList.add("is-selected");
      const outcome = button.dataset.objective as ObjectiveOutcome | undefined;
      if (outcome) lastSummaryObjective = outcome;
      refreshSummaryCopilotNote();
    });
  });
  $("#home-first-flight-cta")?.addEventListener("click", () => show("view-lockin"));

  $("#view-chat")?.querySelectorAll<HTMLButtonElement>("[data-copilot-nav]").forEach((button) => {
    button.addEventListener("click", () => {
      const dest = button.dataset.copilotNav;
      if (dest === "home") show("view-home");
      else if (dest === "lockin") show("view-lockin");
      else if (dest === "settings") void openSettings();
    });
  });
  $("#copilot-start-mission")?.addEventListener("click", () => show("view-lockin"));

  let chatMicListening = false;
  $("#chat-mic")?.addEventListener("click", async () => {
    if (chatBusy || chatMicListening) return;
    const input = $<HTMLTextAreaElement>("#chat-input");
    const mic = $("#chat-mic") as HTMLButtonElement | null;
    chatMicListening = true;
    if (mic) {
      mic.disabled = true;
      mic.textContent = "…";
    }
    try {
      const transcript = await invoke<VoiceTranscript>("voice_listen_test", { seconds: 4 });
      const text = transcript.text?.trim();
      if (text && input) {
        input.value = input.value.trim() ? `${input.value.trim()} ${text}` : text;
        input.focus();
      }
    } catch (err) {
      console.error(err);
    } finally {
      chatMicListening = false;
      if (mic) {
        mic.disabled = chatBusy;
        mic.textContent = "Mic";
      }
    }
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
    stuck: ["I'm stuck on this. Here's where I am: ", "I'm stuck on what we're discussing. Help me find one manageable next step without overwhelming me."],
    plan: ["Help me find one manageable next step for: ", "Based on our conversation, suggest one small next step I can take in the next 15 minutes."],
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
    let duration = Math.min(180, Math.max(1, parsedDuration || 25));
    let durationAdjusted = false;
    if (rawDuration === "" || Number.isNaN(parsedDuration)) {
      duration = 25;
      durationAdjusted = true;
      if (errEl) {
        errEl.hidden = false;
        errEl.textContent = "Enter 1–180 minutes; using 25.";
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

  $("#session-pause")?.addEventListener("click", () => {
    const btn = $("#session-pause") as HTMLButtonElement | null;
    if (!btn) return;
    const paused = btn.getAttribute("aria-pressed") === "true";
    const next = !paused;
    btn.setAttribute("aria-pressed", next ? "true" : "false");
    btn.textContent = next ? "Resume" : "Pause";
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
  document.querySelectorAll<HTMLButtonElement>("[data-settings-nav]").forEach((button) => {
    button.addEventListener("click", () => {
      const dest = button.dataset.settingsNav;
      if (dest === "copilot") show("view-chat");
      else if (dest === "lockin") show("view-lockin");
      else show("view-home");
    });
  });
  document.querySelectorAll<HTMLButtonElement>("[data-session-nav]").forEach((button) => {
    button.addEventListener("click", () => {
      const dest = button.dataset.sessionNav;
      if (dest === "copilot") show("view-chat");
      else if (dest === "settings") void openSettings();
      else show("view-home");
    });
  });
  $("#goals-copilot-affordance")?.addEventListener("click", () => show("view-chat"));
  $("#session-chat-form")?.addEventListener("submit", (e) => {
    e.preventDefault();
    void sendSessionChat();
  });
  $("#session-chat-mic")?.addEventListener("click", async () => {
    if (chatBusy || chatMicListening) return;
    const input = $<HTMLTextAreaElement>("#session-chat-input");
    const mic = $("#session-chat-mic") as HTMLButtonElement | null;
    chatMicListening = true;
    if (mic) {
      mic.disabled = true;
      mic.textContent = "…";
    }
    try {
      const transcript = await invoke<VoiceTranscript>("voice_listen_test", { seconds: 4 });
      const text = transcript.text?.trim();
      if (text && input) {
        input.value = input.value.trim() ? `${input.value.trim()} ${text}` : text;
        input.focus();
      }
    } catch (err) {
      console.error(err);
    } finally {
      chatMicListening = false;
      if (mic) {
        mic.disabled = false;
        mic.textContent = "Mic";
      }
    }
  });
  $("#settings-lock-in")?.addEventListener("click", () => show("view-lockin"));
  $("#setting-copilot-audio")?.addEventListener("change", () => {
    void persistCopilotAudioFromToggle();
  });
  $("#setting-silent-mode")?.addEventListener("change", () => {
    void persistSilentMode();
  });
  $("#setting-camera-signals")?.addEventListener("change", (event) => {
    const input = event.target as HTMLInputElement;
    writeBoolPref(PREF_CAMERA_SIGNALS, input.checked);
  });
  $("#setting-screen-sharing")?.addEventListener("change", (event) => {
    const input = event.target as HTMLInputElement;
    writeBoolPref(PREF_SCREEN_SHARING, input.checked);
  });
  $("#setting-reduce-motion")?.addEventListener("change", (event) => {
    const input = event.target as HTMLInputElement;
    writeBoolPref(PREF_REDUCE_MOTION, input.checked);
    applyReduceMotionPref();
  });
  applyReduceMotionPref();
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

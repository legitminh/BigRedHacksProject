import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { openUrl } from "@tauri-apps/plugin-opener";
import { renderMarkdown } from "./markdown.ts";
import { retryChat } from "./chat-retry.ts";
import {
  CompanionLiveSession,
  type CompanionContext,
  type CompanionPhase,
} from "./companion-live.ts";
import {
  hydrateShipFromStudyStats,
  initShipUI,
  onMissionCompleted,
  onMissionStarted,
  readShipProgress,
  recordLongestFlightMinutes,
  refreshAllShipViews,
  refreshHomePersonalBest,
  refreshSessionFlight,
  setShipAccountId,
  updateSessionFlight,
  wipeLegacyUnscopedShipKeys,
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
  for (const id of [
    "settings-avatar",
    "session-avatar",
    "setup-avatar",
    "summary-avatar",
    "home-avatar",
    "copilot-avatar",
  ]) {
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
  /** `authorized` | `denied` | `restricted` | `notDetermined` | `unknown` */
  microphone?: string;
}

interface VoiceTranscript {
  text: string;
  engine?: string;
  audio_path?: string | null;
  note?: string;
}

interface StatusPayload {
  /** Google OAuth completed (JWT stored). Required to use the app. */
  signed_in: boolean;
  /** Local-only Guest session — unlocks without cloud sync. */
  guest_mode?: boolean;
  username?: string | null;
  email?: string | null;
  user_id?: string | null;
  google_connected: boolean;
  gemini_ready: boolean;
  google_oauth_ready: boolean;
  presage_ready: boolean;
  local_llm_model?: string;
  local_llm_enabled?: boolean;
  session: LockInSession | null;
}

/** Unlocked after Google OAuth (+ Calendar/Drive) or local Guest mode. */
function isAppUnlocked(status: StatusPayload): boolean {
  return Boolean((status.signed_in && status.google_connected) || status.guest_mode);
}

interface StudySessionSuggestion {
  goals: string;
  duration_mins: number;
  reason: string;
  proposed_start: string;
  calendar_checked: boolean;
  calendar_clear: boolean;
  conflict_summary?: string | null;
}

interface ChatMessage {
  role: string;
  content: string;
  study_suggestion?: StudySessionSuggestion | null;
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
  paused?: boolean;
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
let lastSummaryObjective: ObjectiveOutcome | null = null;
let lastSummaryEndedEarly = false;
let lastSummaryFirstFlight = false;

const RELAUNCH_FLAG_KEY = "waypoint-summary-from-relaunch";

const LAUNCH_CELEBRATION_MS = 2200;

/** Tracks which account’s local ship/PB data is loaded (null = signed out). */
let activeShipAccountKey: string | null = null;
/** Last status used to paint home — reused when navigating home after a mission. */
let lastHomeStatus: StatusPayload | null = null;

function flightMinutesFromSecs(secs: number): number {
  return Math.max(0, Math.round(secs / 60));
}

function recordLongestFlight(minutes: number): { previous: number; isNew: boolean; delta: number } {
  return recordLongestFlightMinutes(minutes);
}

function accountKeyFromStatus(status: StatusPayload): string | null {
  if (status.signed_in && status.user_id) return status.user_id;
  if (status.guest_mode) return "guest";
  return null;
}

async function applyAccountLocalScope(status: StatusPayload): Promise<void> {
  const next = accountKeyFromStatus(status);
  const switched = next !== activeShipAccountKey;
  wipeLegacyUnscopedShipKeys();
  setShipAccountId(next);
  if (!switched) {
    activeShipAccountKey = next;
    refreshHomePersonalBest();
    return;
  }
  activeShipAccountKey = next;
  teardownCompanionLive();
  const chatLog = $("#chat-log");
  if (chatLog) {
    chatLog.innerHTML = "";
    // Restore empty-state card if present in markup elsewhere — home refresh handles ship.
  }
  hasChatReply = false;
  void invoke("clear_chat").catch(() => {});
  void invoke("companion_clear").catch(() => {});

  if (next && status.signed_in) {
    try {
      const stats = await invoke<{
        total_sessions?: number;
        total_on_task_minutes?: number;
        longest_flight_minutes?: number;
      }>("study_memory_stats");
      hydrateShipFromStudyStats(stats);
    } catch {
      hydrateShipFromStudyStats(null);
    }
  } else {
    hydrateShipFromStudyStats(null);
  }
  refreshAllShipViews();
}

function consumeRelaunchFlag(): number {
  const fromRelaunch = sessionStorage.getItem(RELAUNCH_FLAG_KEY) === "1";
  sessionStorage.removeItem(RELAUNCH_FLAG_KEY);
  return fromRelaunch ? 1 : 0;
}

function personalBestMinutes(
  flightMinutes: number,
  pb: { previous: number; isNew: boolean },
): number {
  return pb.isNew ? flightMinutes : Math.max(pb.previous, flightMinutes);
}

function relaunchPhrase(relaunches: number): string {
  return relaunches === 1 ? "once" : `${relaunches} times`;
}

/**
 * Figma 13/12/25: lavender card + teal kicker when relaunches>0 for
 * Partly / Finished(+new PB) / Not yet.
 */
function summaryUsesRelaunchNote(
  relaunches: number,
  outcome: ObjectiveOutcome | null,
  pbIsNew = false,
): boolean {
  if (relaunches <= 0 || !outcome) return false;
  if (outcome === "not-yet" || outcome === "partly") return true;
  return outcome === "finished" && pbIsNew;
}

function buildCopilotNote(
  summary: SessionSummary,
  flightMinutes: number,
  relaunches: number,
  pb: { previous: number; isNew: boolean; delta: number },
  outcome: ObjectiveOutcome,
): string {
  const goalLine = summary.goals.trim().split("\n")[0]?.trim();
  const unit = flightMinutes === 1 ? "minute" : "minutes";
  const best = personalBestMinutes(flightMinutes, pb);
  const relaunch = relaunchPhrase(relaunches);
  if (outcome === "finished") {
    const finishedWhat = goalLine
      ? goalLine.replace(/^(finish|complete)\s+/i, "").trim() || goalLine
      : "your objective";
    // Figma 12 — Finished + relaunch≥1 + new PB (live 6:1125)
    if (summaryUsesRelaunchNote(relaunches, outcome, pb.isNew)) {
      return `You logged ${flightMinutes} flight ${unit}, relaunched ${relaunch}, and said you finished ${finishedWhat}. That’s ${pb.delta} minutes beyond your previous longest flight.`;
    }
    return `You logged ${flightMinutes} flight ${unit} and said you finished ${finishedWhat}. Your personal best remains ${best} minutes.`;
  }
  if (outcome === "partly") {
    // Figma 13 — Partly + relaunch≥1 (live 6:1226)
    if (summaryUsesRelaunchNote(relaunches, outcome)) {
      return `You logged ${flightMinutes} flight ${unit} and relaunched ${relaunch}. You said there’s more to do—and your time still counts. Pick up with one small step next time.`;
    }
    return `You logged ${flightMinutes} flight ${unit} and said you partly finished. Your time counts. Choose one small next step when you return.`;
  }
  if (summaryUsesRelaunchNote(relaunches, outcome)) {
    return `You logged ${flightMinutes} flight ${unit} and relaunched ${relaunch}. You said your objective is not finished yet. Your time still counts; pick one small step for your next flight.`;
  }
  return `You logged ${flightMinutes} flight ${unit} and said your objective is not finished yet. Your time still counts, and your personal best remains ${best} minutes.`;
}

function resetObjectiveButtons(): void {
  document.querySelectorAll<HTMLButtonElement>(".quest-objective-btn").forEach((btn) => {
    btn.classList.remove("is-selected");
  });
}

function syncSummaryCopilotNoteChrome(
  visible: boolean,
  relaunches = lastSummaryRelaunches,
  outcome: ObjectiveOutcome | null = lastSummaryObjective,
): void {
  const note = document.querySelector<HTMLElement>(".quest-copilot-note");
  const kicker = document.querySelector<HTMLElement>(".quest-copilot-kicker");
  const helper = document.querySelector<HTMLElement>(".quest-objective-helper");
  const asCard =
    visible &&
    summaryUsesRelaunchNote(
      relaunches,
      outcome,
      lastSummaryPersonalBest?.isNew ?? false,
    );
  if (note) {
    note.hidden = !visible;
    note.classList.toggle("quest-copilot-note--card", asCard);
  }
  if (kicker) kicker.hidden = !asCard;
  if (helper) helper.hidden = visible;
}

function setSummaryNoteVisible(visible: boolean): void {
  syncSummaryCopilotNoteChrome(visible);
}

function refreshSummaryCopilotNote(): void {
  if (!lastSessionSummary || !lastSummaryPersonalBest || !lastSummaryObjective) {
    setSummaryNoteVisible(false);
    return;
  }
  const closing = $("#summary-closing");
  if (!closing) return;
  closing.textContent = buildCopilotNote(
    lastSessionSummary,
    flightMinutesFromSecs(lastSessionSummary.duration_secs),
    lastSummaryRelaunches,
    lastSummaryPersonalBest,
    lastSummaryObjective,
  );
  setSummaryNoteVisible(true);
}

function updateSummaryCelebration(
  summary: SessionSummary,
  firstFlight: boolean,
  endedEarly = false,
): void {
  const block = $("#summary-celebration");
  if (!block) return;
  block.classList.remove(
    "mission-celebration--quest",
    "mission-celebration--first-flight",
    "mission-celebration--flight-logged",
    "mission-celebration--enter",
  );
  void block.offsetWidth;
  const mode = firstFlight
    ? "first-flight"
    : endedEarly
      ? "flight-logged"
      : "quest";
  block.classList.add(`mission-celebration--${mode}`, "mission-celebration--enter");
  const badge = $("#summary-celebration-badge");
  const title = $("#summary-celebration-title");
  const sub = $("#summary-celebration-sub");
  const goalLine = summary.goals.trim().split("\n")[0]?.trim() || summary.goals.trim();
  if (firstFlight) {
    if (badge) badge.textContent = "✦  FIRST FLIGHT";
    if (title) title.textContent = "One mission. Well done.";
    if (sub) sub.textContent = goalLine || "Your first personal best is on the board.";
  } else if (endedEarly) {
    if (badge) badge.textContent = "✦  FLIGHT LOGGED";
    if (title) title.textContent = "Every flight moves you forward.";
    if (sub) sub.textContent = goalLine || "Mission ended — your debrief is below.";
  } else {
    if (badge) badge.textContent = "✓  QUEST COMPLETE";
    if (title) title.textContent = "One mission. Well done.";
    if (sub) sub.textContent = goalLine || "Mission ended — your debrief is below.";
  }
}

/** Early End starts as FLIGHT LOGGED; Finished upgrades to QUEST COMPLETE (live 22). */
function syncSummaryCelebrationFromOutcome(): void {
  if (!lastSessionSummary || lastSummaryFirstFlight || !lastSummaryObjective) return;
  if (!lastSummaryEndedEarly) return;
  const questComplete = lastSummaryObjective === "finished";
  updateSummaryCelebration(lastSessionSummary, false, !questComplete);
}

/** Guards against the same mission being credited twice (early End + timer race). */
let lastCreditedSummaryKey: string | null = null;
let lastCreditedSummaryAt = 0;

function showSummaryWithCelebration(summary: SessionSummary, endedEarly = false): void {
  const key = `${summary.goals}|${summary.modality}|${summary.screen_checks ?? 0}`;
  const now = Date.now();
  if (key === lastCreditedSummaryKey && now - lastCreditedSummaryAt < 15_000) {
    return;
  }
  lastCreditedSummaryKey = key;
  lastCreditedSummaryAt = now;
  // PB first so it compares against the pre-mission longest, then bump mission count.
  lastSummaryPersonalBest = recordLongestFlight(flightMinutesFromSecs(summary.duration_secs));
  const firstFlight = onMissionCompleted(summary, summary.duration_secs);
  lastSummaryRelaunches = consumeRelaunchFlag();
  lastSummaryObjective = null;
  lastSummaryEndedEarly = endedEarly;
  lastSummaryFirstFlight = firstFlight;
  resetObjectiveButtons();
  setSummaryNoteVisible(false);
  renderSummary(summary);
  updateSummaryCelebration(summary, firstFlight, endedEarly);
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
function navInitials(username?: string | null): string {
  const who = username?.trim() || "You";
  const parts = who.split(/\s+/).filter(Boolean);
  if (parts.length >= 2) {
    return (parts[0].charAt(0) + parts[1].charAt(0)).toUpperCase();
  }
  return who.slice(0, 2).toUpperCase();
}

let appUnlocked = false;

function show(view: ViewId) {
  // Gate: nothing past the welcome screen until Google sign-in + link completes.
  if (!appUnlocked && view !== "view-home") {
    view = "view-home";
  }
  document.querySelectorAll(".view").forEach((el) => el.classList.remove("active"));
  $(`#${view}`)?.classList.add("active");
  if (view === "view-home" && lastHomeStatus && appUnlocked) {
    // Re-paint first-flight vs dashboard from current ship progress (mission complete
    // updates localStorage but used to leave the stale first-flight panel visible).
    renderHome(lastHomeStatus);
  }
  if (view === "view-chat") {
    requestAnimationFrame(() => {
      ($("#chat-input") as HTMLTextAreaElement | null)?.focus();
    });
  }
  if (view === "view-lockin") {
    syncDurationChips();
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

const SETUP_FOOT_EMPTY =
  "Type an objective to enable launch.<br />Both optional inputs are off.";
const SETUP_FOOT_READY =
  "You can fly with both inputs off. Nothing starts capturing until you give permission.";

let missionLaunchLoading = false;

function goalsHasObjective(): boolean {
  const goalsInput = $("#goals") as HTMLTextAreaElement | null;
  return Boolean(goalsInput?.value.trim());
}

function syncMissionSetupLaunchUi(): void {
  const startBtn = $("#lockin-start") as HTMLButtonElement | null;
  const labelEl = startBtn?.querySelector(".mission-launch-label");
  const foot = $("#mission-setup-foot");
  const canLaunch = goalsHasObjective() && !missionLaunchLoading;

  if (labelEl) labelEl.textContent = missionLaunchLabel(missionLaunchLoading);
  if (startBtn) startBtn.disabled = !canLaunch;

  if (foot) {
    if (goalsHasObjective()) foot.textContent = SETUP_FOOT_READY;
    else foot.innerHTML = SETUP_FOOT_EMPTY;
  }
}

function setMissionLaunchButton(loading: boolean) {
  missionLaunchLoading = loading;
  syncMissionSetupLaunchUi();
}

let googleSignInInFlight = false;

async function submitWelcomeGoogle() {
  if (googleSignInInFlight) return;
  googleSignInInFlight = true;
  const err = $("#wp-signin-error");
  const btn = $("#welcome-google-signin") as HTMLButtonElement | null;
  if (err) {
    err.hidden = true;
    err.textContent = "";
  }
  if (btn) {
    btn.disabled = true;
    btn.textContent = "Waiting for Google…";
  }
  try {
    await invoke("sign_in_waypoint_google");
    await refreshStatus();
    if (!appUnlocked) {
      throw new Error(
        "Google signed in, but Calendar/Drive were not linked. Try Sign in with Google again and accept all permissions.",
      );
    }
  } catch (e) {
    const raw =
      typeof e === "string"
        ? e
        : e && typeof e === "object" && "message" in e
          ? String((e as { message: unknown }).message)
          : String(e);
    const nice = raw.replace(/^Error:\s*/i, "").trim() || "Google sign-in failed.";
    if (err) {
      err.hidden = false;
      err.textContent = nice;
    } else {
      alert(nice);
    }
    if (btn) {
      btn.disabled = false;
      btn.textContent = "Sign in with Google →";
    }
  } finally {
    if (!appUnlocked) googleSignInInFlight = false;
  }
}

async function submitWelcomeGuest() {
  const err = $("#wp-signin-error");
  const guestBtn = $("#welcome-continue-guest") as HTMLButtonElement | null;
  if (err) {
    err.hidden = true;
    err.textContent = "";
  }
  if (guestBtn) guestBtn.disabled = true;
  try {
    await invoke("sign_in_waypoint_guest");
    await refreshStatus();
  } catch (e) {
    const raw =
      typeof e === "string"
        ? e
        : e && typeof e === "object" && "message" in e
          ? String((e as { message: unknown }).message)
          : String(e);
    const nice = raw.replace(/^Error:\s*/i, "").trim() || "Guest sign-in failed.";
    if (err) {
      err.hidden = false;
      err.textContent = nice;
    } else {
      alert(nice);
    }
  } finally {
    if (guestBtn) guestBtn.disabled = false;
  }
}

function wireWelcomeSignIn() {
  const btn = $("#welcome-google-signin") as HTMLButtonElement | null;
  if (btn && btn.dataset.wired !== "1") {
    btn.dataset.wired = "1";
    // onclick (not addEventListener) so Vite HMR / double-init cannot stack handlers.
    btn.onclick = (event) => {
      event.preventDefault();
      event.stopPropagation();
      void submitWelcomeGoogle();
    };
  }
  const guest = $("#welcome-continue-guest") as HTMLButtonElement | null;
  if (guest && guest.dataset.wired !== "1") {
    guest.dataset.wired = "1";
    guest.onclick = (event) => {
      event.preventDefault();
      event.stopPropagation();
      void submitWelcomeGuest();
    };
  }
}

function renderGuestNav() {
  const brand = $("#home-nav-brand");
  const nav = $("#home-nav-actions");
  const center = $("#home-nav-center");
  const header = document.querySelector(".welcome-nav");
  header?.classList.add("mc-nav", "welcome-nav--guest");
  header?.classList.remove("welcome-nav--signed-in", "settings-top-bar");
  center?.classList.remove("settings-nav");
  center?.classList.add("mc-nav-center");
  center?.toggleAttribute("hidden", true);
  nav?.classList.remove("settings-nav-end");
  nav?.classList.add("mc-nav-actions");
  if (brand) {
    brand.className = "mc-nav-brand";
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
  const unlocked = isAppUnlocked(status);
  if (center) {
    center.innerHTML = "";
    center.toggleAttribute("hidden", !unlocked);
  }
  header?.classList.toggle("welcome-nav--signed-in", unlocked);
  header?.classList.remove("welcome-nav--guest");
  if (!unlocked) return;

  header?.classList.add("settings-top-bar");
  header?.classList.remove("mc-nav");
  if (brand) {
    brand.className = "settings-brand";
    brand.innerHTML =
      '<span class="settings-brand-star" aria-hidden="true">✦</span> Waypoint';
  }
  if (center) {
    center.className = "settings-nav";
    center.setAttribute("aria-label", "Primary");
  }
  nav.className = "settings-nav-end";

  const links: { label: string; view: ViewId; active?: boolean }[] = [
    { label: "Home", view: "view-home", active: true },
    { label: "Copilot", view: "view-chat" },
    { label: "Lock in", view: "view-lockin" },
  ];
  for (const link of links) {
    if (link.active) {
      const current = document.createElement("span");
      current.className = "settings-nav-link settings-nav-link--active";
      current.setAttribute("aria-current", "page");
      current.textContent = link.label;
      center?.appendChild(current);
      continue;
    }
    const btn = document.createElement("button");
    btn.type = "button";
    btn.className = "settings-nav-link";
    btn.textContent = link.label;
    btn.addEventListener("click", () => show(link.view));
    center?.appendChild(btn);
  }

  const settings = document.createElement("button");
  settings.type = "button";
  settings.className = "settings-nav-link";
  settings.textContent = "Settings";
  settings.addEventListener("click", () => {
    void openSettings();
  });

  const avatar = document.createElement("span");
  avatar.id = "home-avatar";
  avatar.className = "settings-avatar";
  avatar.setAttribute("aria-hidden", "true");
  avatar.textContent = navInitials(status.username);

  nav.append(settings, avatar);
}

function renderHome(status: StatusPayload) {
  lastHomeStatus = status;
  const unlocked = isAppUnlocked(status);
  appUnlocked = unlocked;

  const home = $("#view-home");
  home?.classList.toggle("view-home--signed-in", unlocked);
  home?.classList.toggle("view-home--guest", !unlocked);

  $("#home-guest")?.toggleAttribute("hidden", unlocked);

  if (!unlocked) {
    $("#home-first-flight")?.toggleAttribute("hidden", true);
    $("#home-dashboard")?.toggleAttribute("hidden", true);
    renderGuestNav();
    // Keep user on the Google sign-in screen.
    if (!$("#view-home")?.classList.contains("active")) {
      show("view-home");
    }
    const btn = $("#welcome-google-signin") as HTMLButtonElement | null;
    if (btn && !btn.disabled) btn.textContent = "Sign in with Google →";
    return;
  }

  const showFirstFlightHome = readShipProgress().completedMissions === 0;
  $("#home-first-flight")?.toggleAttribute("hidden", !showFirstFlightHome);
  $("#home-dashboard")?.toggleAttribute("hidden", showFirstFlightHome);

  renderHomeNav(status);
  renderNavAvatar(status);

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
    copilot.textContent = "Open copilot  →";
    copilot.addEventListener("click", () => show("view-chat"));
    copilotHost.appendChild(copilot);
  }

  refreshAllShipViews();
}

function renderAccountSettings(status: StatusPayload) {
  const userEl = $("#account-waypoint-user");
  if (userEl) {
    userEl.textContent = status.signed_in
      ? status.username || status.email || "Signed in with Google"
      : "Not signed in";
  }
  const emailEl = $("#account-waypoint-email");
  if (emailEl) {
    emailEl.textContent = status.email && status.email !== status.username ? status.email : "";
  }

  const accountActions = $("#account-waypoint-actions");
  if (accountActions) {
    accountActions.innerHTML = "";
    if (status.signed_in) {
      const signOut = document.createElement("button");
      signOut.className = "ghost pill";
      signOut.type = "button";
      signOut.textContent = "Sign out";
      signOut.addEventListener("click", async () => {
        signOut.disabled = true;
        try {
          await invoke("sign_out_waypoint");
          await refreshStatus();
        } catch (e) {
          alert(String(e));
          signOut.disabled = false;
        }
      });
      accountActions.appendChild(signOut);
    }
  }

  const googleStatus = $("#account-google-status");
  if (googleStatus) {
    googleStatus.textContent = status.google_connected
      ? "Linked"
      : status.signed_in
        ? "Not linked — re-sign in with Google"
        : "Sign in required";
  }

  const actions = $("#account-google-actions");
  if (!actions) return;
  actions.innerHTML = "";

  if (!status.signed_in) return;

  if (!status.google_connected) {
    const connect = document.createElement("button");
    connect.className = "secondary pill";
    connect.type = "button";
    connect.textContent = "Link Calendar & Drive";
    connect.addEventListener("click", async () => {
      const label = connect.textContent || "Link Calendar & Drive";
      connect.textContent = "Waiting for Google…";
      connect.disabled = true;
      try {
        await invoke("connect_google");
        await refreshStatus();
      } catch (e) {
        console.error("connect_google failed:", e);
        alert(String(e));
      } finally {
        connect.textContent = label;
        connect.disabled = false;
      }
    });
    actions.appendChild(connect);
  }
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

function setMicrophoneBadge(status: string | undefined) {
  const el = $("#perm-microphone");
  if (!el) return;
  const s = (status || "unknown").toLowerCase();
  if (s === "authorized") {
    el.textContent = "Enabled";
    el.setAttribute("data-state", "ok");
  } else if (s === "denied" || s === "restricted") {
    el.textContent = "Denied — System Settings";
    el.setAttribute("data-state", "needs");
  } else if (s === "notdetermined") {
    el.textContent = "Grant when testing voice";
    el.setAttribute("data-state", "optional");
  } else {
    el.textContent = "Grant when testing voice";
    el.setAttribute("data-state", "optional");
  }
}

async function renderPermissionsStatus() {
  ["perm-screen", "perm-camera", "perm-accessibility", "perm-microphone"].forEach((id) => {
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
    setMicrophoneBadge(perms.microphone);
  } catch (err) {
    console.error(err);
    ["perm-screen", "perm-camera", "perm-accessibility", "perm-microphone"].forEach((id) => {
      const el = $(`#${id}`);
      if (el) {
        el.textContent = "Couldn’t check";
        el.setAttribute("data-state", "needs");
      }
    });
  }
}

type ConnState = "ok" | "warn" | "err";

/** Backend `GET /v1/status` — Connection panel is a thin renderer of this payload. */
interface ServiceIndicator {
  id: string;
  label: string;
  state: ConnState | string;
  status: string;
  detail: string;
  optional: boolean;
}

interface ServiceStatusPayload {
  ok: boolean;
  checked_at: string;
  cache_ttl_seconds: number;
  services: ServiceIndicator[];
}

function connectionRow(
  label: string,
  detail: string,
  ok: boolean,
  opts?: { state?: ConnState; status?: string },
): string {
  const state = opts?.state ?? (ok ? "ok" : "warn");
  const status = opts?.status ?? (ok ? "Connected" : "Offline");
  return `<li class="mc-conn-row">
    <div class="mc-conn-copy">
      <strong>${escapeHtml(label)}</strong>
      <span>${escapeHtml(detail)}</span>
    </div>
    <span class="mc-conn-dot" data-state="${state}" aria-hidden="true"></span>
    <span class="mc-conn-status">${escapeHtml(status)}</span>
  </li>`;
}

function asConnState(value: string): ConnState {
  if (value === "ok" || value === "warn" || value === "err") return value;
  return "warn";
}

/** Map backend health rows to plain student-facing labels/details. */
function friendlyServiceCopy(s: ServiceIndicator): { label: string; detail: string } {
  const state = asConnState(String(s.state));
  const status = String(s.status ?? "");
  switch (s.id) {
    case "api":
      return {
        label: "Waypoint service",
        detail:
          state === "ok" ? "Online and reachable" : "Can't reach Waypoint right now",
      };
    case "gemini":
      return {
        label: "Cloud coach",
        detail:
          state === "ok"
            ? "Ready for coaching"
            : /quota/i.test(status)
              ? "Daily usage limit reached — try again later"
              : "Unavailable right now",
      };
    case "ollama":
      return {
        label: "Lock-in coach",
        detail:
          state === "ok"
            ? "Ready for lock-in coaching"
            : state === "warn"
              ? "Partly ready"
              : "Unavailable right now",
      };
    case "chat_provider":
      return {
        label: "Copilot chat",
        detail:
          state === "ok"
            ? "Ready"
            : state === "warn"
              ? "Working with limited capacity"
              : "Unavailable right now",
      };
    case "google_oauth":
      return {
        label: "Google sign-in",
        detail:
          state === "ok"
            ? "Sign-in is ready"
            : "Sign-in isn't available right now",
      };
    case "account":
      return {
        label: "Waypoint account",
        detail: s.detail?.trim() || (state === "ok" ? "Signed in" : "Sign in required"),
      };
    case "google":
      return {
        label: "Google",
        detail:
          s.detail?.trim() ||
          (state === "ok"
            ? "Calendar and Drive linked"
            : "Link Calendar and Drive to continue"),
      };
    default:
      return {
        label: s.label?.trim() || "Service",
        detail:
          state === "ok"
            ? "Connected"
            : state === "warn"
              ? "Needs attention"
              : "Unavailable right now",
      };
  }
}

async function renderConnectionStatus(_status?: StatusPayload) {
  const list = $("#connection-status-list");
  if (!list) return;
  list.innerHTML = `<li class="mc-conn-row mc-conn-row--loading"><span class="muted">Checking links…</span></li>`;

  try {
    const payload = await invoke<ServiceStatusPayload>("service_status");
    if (!payload.services?.length) {
      list.innerHTML = `<li class="mc-conn-row"><span class="muted">No connection details available right now.</span></li>`;
      return;
    }
    list.innerHTML = payload.services
      .map((s) => {
        const state = asConnState(String(s.state));
        const copy = friendlyServiceCopy(s);
        return connectionRow(copy.label, copy.detail, state === "ok", {
          state,
          status: s.status,
        });
      })
      .join("");
  } catch (e) {
    list.innerHTML = connectionRow(
      "Waypoint service",
      connectionErrorMessage(e),
      false,
      { state: "err", status: "Offline" },
    );
  }
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

/** True when the user is already inside an active lock-in mission UI. */
function isLockInSessionActive(): boolean {
  return Boolean($("#view-session")?.classList.contains("active"));
}

function parseStudySuggestion(
  raw: ChatMessage["study_suggestion"] | Record<string, unknown> | null | undefined,
): StudySessionSuggestion | null {
  if (!raw || typeof raw !== "object") return null;
  const o = raw as Record<string, unknown>;
  const calendarClear = o.calendar_clear ?? o.calendarClear;
  if (calendarClear !== true) return null;
  const durationRaw = o.duration_mins ?? o.durationMins ?? o.duration;
  const durationMins = Number(durationRaw);
  if (!Number.isFinite(durationMins) || durationMins <= 0) return null;
  return {
    goals: String(o.goals ?? ""),
    duration_mins: durationMins,
    reason: String(o.reason ?? ""),
    proposed_start: String(o.proposed_start ?? o.proposedStart ?? ""),
    calendar_checked: Boolean(o.calendar_checked ?? o.calendarChecked),
    calendar_clear: true,
    conflict_summary:
      o.conflict_summary == null && o.conflictSummary == null
        ? null
        : String(o.conflict_summary ?? o.conflictSummary ?? ""),
  };
}

function removeStudySuggestionCardNear(bubble: HTMLElement) {
  const anchor = bubble.closest(".session-chat-turn") ?? bubble;
  const next = anchor.nextElementSibling;
  if (next?.classList.contains("study-suggest-card")) next.remove();
}

function appendStudySuggestionCard(
  bubble: HTMLElement,
  suggestion: StudySessionSuggestion,
) {
  if (isLockInSessionActive()) return;

  removeStudySuggestionCardNear(bubble);

  const mins = Math.min(180, Math.max(1, Math.round(suggestion.duration_mins)));
  const goalsText = suggestion.goals.trim() || "General study session";
  const reasonText =
    suggestion.reason.trim() || "Looks like a good window for a focused session.";

  const card = document.createElement("div");
  card.className = "study-suggest-card";
  card.setAttribute("role", "group");
  card.setAttribute("aria-label", "Suggested study session");

  const kicker = document.createElement("p");
  kicker.className = "study-suggest-card__kicker";
  kicker.textContent = "Suggested lock-in";

  const reason = document.createElement("p");
  reason.className = "study-suggest-card__reason";
  reason.textContent = reasonText;

  const goals = document.createElement("p");
  goals.className = "study-suggest-card__goals";
  goals.textContent = goalsText;

  const meta = document.createElement("p");
  meta.className = "study-suggest-card__meta";
  meta.textContent = `${mins} min`;

  const error = document.createElement("p");
  error.className = "study-suggest-card__error";
  error.hidden = true;

  const actions = document.createElement("div");
  actions.className = "study-suggest-card__actions";

  const decline = document.createElement("button");
  decline.type = "button";
  decline.className = "ghost study-suggest-card__decline";
  decline.textContent = "Decline";

  const accept = document.createElement("button");
  accept.type = "button";
  accept.className = "primary study-suggest-card__accept";
  accept.textContent = "Accept";

  decline.addEventListener("click", () => {
    card.remove();
  });

  accept.addEventListener("click", () => {
    void (async () => {
      if (accept.disabled) return;
      if (isLockInSessionActive()) {
        card.remove();
        return;
      }
      accept.disabled = true;
      decline.disabled = true;
      error.hidden = true;
      error.textContent = "";
      try {
        const session = await invoke<LockInSession>("start_lock_in", {
          goals: goalsText,
          durationMins: mins,
        });
        try {
          sessionStorage.setItem("lockin-last-duration", String(mins));
        } catch {
          // ignore
        }
        card.remove();
        renderVitals(null);
        playLaunchCelebration(() => {
          clearNextStepTimer();
          renderSession(session);
          syncMissionTimer(session);
          show("view-session");
        });
      } catch (err) {
        const message = String(err);
        console.error("start_lock_in from study suggestion failed:", err);
        error.hidden = false;
        error.textContent = message || "Couldn’t start that session. Try again.";
        accept.disabled = false;
        decline.disabled = false;
      }
    })();
  });

  actions.append(decline, accept);
  card.append(kicker, reason, goals, meta, error, actions);

  const anchor = bubble.closest(".session-chat-turn") ?? bubble;
  anchor.insertAdjacentElement("afterend", card);

  const log = card.closest(".chat-log, #chat-log, #session-chat-log") as HTMLElement | null;
  if (log) log.scrollTop = log.scrollHeight;
}

function maybeShowStudySuggestion(
  bubble: HTMLElement | undefined | null,
  reply: ChatMessage,
) {
  if (!bubble) return;
  const suggestion = parseStudySuggestion(reply.study_suggestion);
  if (!suggestion) return;
  appendStudySuggestionCard(bubble, suggestion);
}

let chatBusy = false;
let hasChatReply = false;

/** Live companion — backend-mediated Gemini Live (Copilot tab or lock-in session). */
type LiveSurface = "copilot" | "session";
let companionLive: CompanionLiveSession | null = null;
let companionBusy = false;
let liveSurface: LiveSurface | null = null;

const CHAT_FAIL_MSG =
  "Sorry, I couldn’t get a reply right now. Please try again in a moment.";

/** Map backend errors to short UI copy — never dump raw JSON or vendor names. */
function chatErrorMessage(err: unknown): string {
  const raw = String(err ?? "").trim();
  if (!raw || raw === "undefined" || raw === "[object Object]") return CHAT_FAIL_MSG;
  // Prefer already-friendly strings that don't expose stack/vendor jargon.
  if (
    !/[{\[]/.test(raw) &&
    !/generativelanguage\.googleapis|error\":|\"status\"|HTTP\s*\d{3}|gemini|ollama|xai|grok|jwt|websocket|\/v1\//i.test(
      raw,
    ) &&
    raw.length <= 160 &&
    /coach|quota|busy|timed out|sign in|configured|try again|live voice|microphone/i.test(raw)
  ) {
    return raw;
  }
  if (/quota|free_tier|free limit|resource_exhausted|429/i.test(raw)) {
    return "Your coach hit today’s usage limit. Try again later — mission watching still works.";
  }
  if (/rate limit|rate-limited/i.test(raw)) {
    return "Your coach is getting too many requests. Please try again in a moment.";
  }
  if (/api[_ ]?key|invalid|permission|unauthorized|403|401/i.test(raw)) {
    return "Your coach couldn’t connect. Check Connection in Settings and try again.";
  }
  if (/timeout|timed out|unavailable|503|busy|high demand/i.test(raw)) {
    return "Your coach is busy right now. Please try again shortly.";
  }
  return CHAT_FAIL_MSG;
}

function connectionErrorMessage(err: unknown): string {
  return chatErrorMessage(err);
}

function setComposerMicHint(message: string, target: "chat" | "session" = "chat") {
  if (target === "session") {
    const hint = $("#session-chat-hint");
    if (hint) hint.textContent = message;
    return;
  }
  const hint = $("#chat-hint");
  if (hint) hint.textContent = message;
}

function setChatControlsBusy(busy: boolean) {
  $("#chat-log")?.setAttribute("aria-busy", busy ? "true" : "false");
  $("#session-chat-log")?.setAttribute("aria-busy", busy ? "true" : "false");
  document
    .querySelectorAll<HTMLButtonElement>(
      "#chat-send, #chat-mic, #session-chat-send, #session-chat-mic, [data-study], #new-chat",
    )
    .forEach((button) => {
      // Keep Talk/Live available so the user can end an active Live session.
      if (
        companionLive?.active &&
        (button.id === "chat-mic" || button.id === "session-chat-mic")
      ) {
        button.disabled = false;
        return;
      }
      button.disabled = busy;
    });
}

function showChatFailure(bubble: HTMLElement, userMessage: string, err?: unknown) {
  bubble.replaceChildren();
  bubble.append(document.createTextNode(`${chatErrorMessage(err)} `));
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
  removeStudySuggestionCardNear(bubble);
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
    maybeShowStudySuggestion(bubble, reply);
    hasChatReply = true;
  } catch (e) {
    removeStudySuggestionCardNear(bubble);
    showChatFailure(bubble, userMessage, e);
  } finally {
    chatBusy = false;
    setChatControlsBusy(false);
    const log = $("#chat-log");
    if (log) log.scrollTop = log.scrollHeight;
    const input = $<HTMLTextAreaElement>("#chat-input");
    if ($("#view-chat")?.classList.contains("active")) input?.focus();
  }
}

function appendSessionChat(
  role: "user" | "assistant" | "system",
  content: string,
  meta?: string,
) {
  const log = $("#session-chat-log");
  if (!log) return;
  $("#session-chat-empty")?.remove();
  const turn = document.createElement("div");
  turn.className = `session-chat-turn session-chat-turn--${role}`;
  const kicker = document.createElement("p");
  kicker.className = "session-chat-meta";
  kicker.textContent =
    meta ??
    (role === "user"
      ? "YOU · JUST NOW"
      : role === "system"
        ? "AT LAUNCH"
        : "COPILOT · JUST NOW");
  const bubble = document.createElement("div");
  bubble.className = `bubble ${role}`;
  bubble.textContent = content;
  turn.append(kicker, bubble);
  log.appendChild(turn);
  log.scrollTop = log.scrollHeight;
  return bubble;
}

let seededAtLaunchSessionId: string | null = null;

function formatAtLaunchMissionLine(goals: string, durationSecs: number): string {
  const goalLine =
    goals.trim().split("\n")[0]?.trim().replace(/\.$/, "") || "your objective";
  const missionGoal =
    goalLine.charAt(0).toLowerCase() + goalLine.slice(1);
  const mins = Math.max(1, Math.round(durationSecs / 60));
  return `Your mission is to ${missionGoal}. You have ${mins} minutes.`;
}

function ensureSessionAtLaunchSeed(session: LockInSession): void {
  if (seededAtLaunchSessionId === session.id) return;
  seededAtLaunchSessionId = session.id;
  const log = $("#session-chat-log");
  if (log) log.innerHTML = "";
  appendSessionChat(
    "system",
    formatAtLaunchMissionLine(session.goals, session.duration_secs),
    "AT LAUNCH",
  );
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
    if (pending) {
      renderMarkdown(pending, reply.content);
      maybeShowStudySuggestion(pending, reply);
    }
    hasChatReply = true;
  } catch (e) {
    if (pending) {
      removeStudySuggestionCardNear(pending);
      showChatFailure(pending, message, e);
    }
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
  if (chatBusy || !input?.value.trim()) return;
  const message = input.value.trim();
  input.value = "";
  // While Copilot Live is open, typed lines go over the Live socket (not STT→composer).
  if (companionLive?.active && liveSurface === "copilot") {
    companionLive.sendText(message);
    return;
  }
  await dispatchChatMessage(
    message,
    (text) => appendChat("user", text),
    (text) => appendChat("assistant", text),
    input,
  );
}

const COMPANION_PHASE_COPY: Record<CompanionPhase, string> = {
  idle: "Tap Talk to start a live voice turn",
  connecting: "Connecting companion…",
  listening: "Listening — talk or type",
  thinking: "Companion is answering…",
  speaking: "Companion speaking — talk to interrupt",
};

/** Live Figma 03 `5:256` — restore when Copilot Live returns to idle. */
const COPILOT_IDLE_HINT =
  "Enter to send · Click the microphone to start or stop a voice turn.";
const COPILOT_IDLE_PLACEHOLDER = "Message your copilot…";
const COPILOT_LISTENING_PLACEHOLDER = "Listening… click mic to stop";

function applyMicLiveUi(
  mic: HTMLButtonElement | null,
  phase: CompanionPhase,
  startLabel: string,
  opts?: { stickyLabel?: boolean },
) {
  if (!mic) return;
  const sticky = opts?.stickyLabel === true;
  mic.classList.toggle("is-live", phase === "listening");
  mic.classList.toggle("is-thinking", phase === "thinking");
  mic.classList.toggle("is-speaking", phase === "speaking");
  mic.classList.toggle("is-connecting", phase === "connecting");
  // Copilot keeps Figma “Mic”; session Talk may show Live / … while active.
  mic.textContent =
    sticky || phase === "idle"
      ? startLabel
      : phase === "connecting"
        ? "…"
        : "Live";
  mic.setAttribute(
    "aria-label",
    phase === "idle"
      ? sticky
        ? "Start live voice with Copilot"
        : `Start ${startLabel.toLowerCase()} live voice`
      : "End live voice",
  );
}

function setCopilotListeningPlaceholder(listening: boolean) {
  const input = $<HTMLTextAreaElement>("#chat-input");
  if (!input) return;
  input.placeholder = listening
    ? COPILOT_LISTENING_PLACEHOLDER
    : COPILOT_IDLE_PLACEHOLDER;
}

function setCompanionPhaseUi(phase: CompanionPhase) {
  const label = $("#session-companion-phase");
  if (label) {
    label.textContent =
      liveSurface === "session" || phase === "idle"
        ? (COMPANION_PHASE_COPY[phase] ?? phase)
        : COMPANION_PHASE_COPY.idle;
  }
  const sessionPhase =
    liveSurface === "session" || phase === "idle" ? phase : "idle";
  const copilotPhase =
    liveSurface === "copilot" || phase === "idle" ? phase : "idle";
  applyMicLiveUi(
    $("#session-chat-mic") as HTMLButtonElement | null,
    sessionPhase,
    "Talk",
  );
  applyMicLiveUi($("#chat-mic") as HTMLButtonElement | null, copilotPhase, "Mic", {
    stickyLabel: true,
  });
  setCopilotListeningPlaceholder(
    liveSurface === "copilot" &&
      (copilotPhase === "listening" ||
        copilotPhase === "connecting" ||
        copilotPhase === "thinking" ||
        copilotPhase === "speaking"),
  );
  setSessionListeningUi(
    liveSurface === "session" &&
      (phase === "listening" || phase === "connecting" || phase === "thinking" || phase === "speaking"),
  );
}

function upsertSessionLiveBubble(role: "user" | "assistant", text: string, isFinal: boolean) {
  const log = $("#session-chat-log");
  if (!log) return;
  $("#session-chat-empty")?.remove();
  const last = log.lastElementChild as HTMLElement | null;
  if (
    last?.classList.contains(`session-chat-turn--${role}`) &&
    last.dataset.sealed !== "true"
  ) {
    const bubble = last.querySelector(".bubble");
    if (bubble) bubble.textContent = text;
    if (isFinal) last.dataset.sealed = "true";
  } else {
    const bubble = appendSessionChat(role, text);
    const turn = bubble?.closest(".session-chat-turn") as HTMLElement | null;
    if (turn) turn.dataset.sealed = isFinal ? "true" : "false";
  }
  log.scrollTop = log.scrollHeight;
}

function upsertCopilotLiveBubble(role: "user" | "assistant", text: string, isFinal: boolean) {
  const log = $("#chat-log");
  if (!log) return;
  $("#chat-empty")?.remove();
  const last = log.lastElementChild as HTMLElement | null;
  if (
    last?.classList.contains("bubble") &&
    last.classList.contains(role) &&
    last.dataset.liveSealed !== "true"
  ) {
    last.textContent = text;
    if (isFinal) last.dataset.liveSealed = "true";
  } else {
    const bubble = appendChat(role, text);
    if (bubble) bubble.dataset.liveSealed = isFinal ? "true" : "false";
  }
  log.scrollTop = log.scrollHeight;
  if (isFinal && role === "assistant") hasChatReply = true;
}

function collectCompanionContext(): CompanionContext {
  const goals = ($("#session-goals")?.textContent ?? "").trim();
  const timer = $("#session-timer")?.textContent ?? "00:00";
  const [mm, ss] = timer.split(":").map((p) => Number(p));
  const remainingMins =
    Number.isFinite(mm) && Number.isFinite(ss) ? mm + ss / 60 : undefined;
  let nextStepSecs: number | undefined;
  if (nextStepEndsAtMs != null && !nextStepPaused) {
    nextStepSecs = Math.max(0, Math.ceil((nextStepEndsAtMs - Date.now()) / 1000));
  } else if (nextStepPaused) {
    nextStepSecs = Math.max(0, Math.ceil(nextStepRemainingMs / 1000));
  }
  const paused =
    ($("#session-pause") as HTMLButtonElement | null)?.getAttribute("aria-pressed") ===
    "true";
  const inSession = Boolean($("#view-session")?.classList.contains("active"));
  return {
    goals: goals || undefined,
    notes: inSession ? undefined : "Copilot tab live voice (no lock-in session).",
    remaining_mins: inSession ? remainingMins : undefined,
    duration_mins:
      inSession && currentSessionDurationSecs
        ? currentSessionDurationSecs / 60
        : undefined,
    next_step_secs: inSession ? nextStepSecs : undefined,
    paused: inSession ? paused : false,
  };
}

function teardownCompanionLive() {
  if (companionLive) {
    companionLive.end(true);
    companionLive = null;
  }
  companionBusy = false;
  liveSurface = null;
  setCompanionPhaseUi("idle");
  void invoke("voice_stop").catch(() => {});
  void invoke("companion_clear").catch(() => {});
}

async function ensureCompanionLive(surface: LiveSurface): Promise<CompanionLiveSession> {
  if (companionLive?.active && liveSurface === surface) return companionLive;
  if (companionLive?.active) teardownCompanionLive();
  liveSurface = surface;
  companionLive = new CompanionLiveSession({
    onPhase: (phase) => setCompanionPhaseUi(phase),
    onUser: (text, isFinal) => {
      if (liveSurface === "copilot") upsertCopilotLiveBubble("user", text, isFinal);
      else upsertSessionLiveBubble("user", text, isFinal);
    },
    onAssistant: (text, isFinal) => {
      if (liveSurface === "copilot") upsertCopilotLiveBubble("assistant", text, isFinal);
      else upsertSessionLiveBubble("assistant", text, isFinal);
    },
    onError: (message) => {
      setComposerMicHint(message, liveSurface === "copilot" ? "chat" : "session");
      liveSurface = null;
      setCompanionPhaseUi("idle");
    },
  });
  await companionLive.start(collectCompanionContext());
  return companionLive;
}

async function toggleCompanionLive(surface: LiveSurface): Promise<void> {
  if (chatBusy || companionBusy) return;
  const micId = surface === "copilot" ? "#chat-mic" : "#session-chat-mic";
  const hintTarget = surface === "copilot" ? "chat" : "session";
  if (companionLive?.active && liveSurface === surface) {
    teardownCompanionLive();
    setComposerMicHint(
      surface === "copilot"
        ? COPILOT_IDLE_HINT
        : "Live voice ended. Tap Talk to start again, or type a turn.",
      hintTarget,
    );
    return;
  }
  const mic = $(micId) as HTMLButtonElement | null;
  if (mic) mic.disabled = true;
  try {
    await ensureCompanionLive(surface);
    setComposerMicHint(
      surface === "copilot"
        ? "Live voice open — speak or type. Click the microphone to end."
        : "Live voice open — talk freely or type a line. Tap Live to end.",
      hintTarget,
    );
    void renderPermissionsStatus();
  } catch (err) {
    console.error(err);
    teardownCompanionLive();
    setComposerMicHint(chatErrorMessage(err), hintTarget);
  } finally {
    if (mic) mic.disabled = false;
  }
}

async function sendSessionChat() {
  const input = $<HTMLTextAreaElement>("#session-chat-input");
  if (companionBusy || chatBusy || !input?.value.trim()) return;
  const message = input.value.trim();
  input.value = "";

  // Prefer Live socket when already open (demo-style typed turn).
  if (companionLive?.active) {
    companionLive.sendText(message);
    return;
  }

  // Typed fallback via backend-mediated /v1/companion/chat (not main Copilot history).
  companionBusy = true;
  setChatControlsBusy(true);
  appendSessionChat("user", message);
  const pending = appendSessionChat("assistant", "Thinking…");
  try {
    const reply = await invoke<{ content: string }>("companion_send", {
      message,
      context: collectCompanionContext(),
    });
    if (pending) pending.textContent = reply.content;
  } catch (err) {
    if (pending) pending.textContent = chatErrorMessage(err);
  } finally {
    companionBusy = false;
    setChatControlsBusy(false);
    const sessionLog = $("#session-chat-log");
    if (sessionLog) sessionLog.scrollTop = sessionLog.scrollHeight;
    input.focus();
  }
}

function formatRemaining(endsAt: string): string {
  const ms = Math.max(0, new Date(endsAt).getTime() - Date.now());
  const total = Math.floor(ms / 1000);
  const m = Math.floor(total / 60).toString().padStart(2, "0");
  const s = (total % 60).toString().padStart(2, "0");
  return `${m}:${s}`;
}

function formatCountdownSecs(totalSecs: number): string {
  const total = Math.max(0, Math.floor(totalSecs));
  const m = Math.floor(total / 60).toString().padStart(2, "0");
  const s = (total % 60).toString().padStart(2, "0");
  return `${m}:${s}`;
}

const NEXT_STEP_DEFAULT_SECS = 5 * 60;
let nextStepEndsAtMs: number | null = null;
let nextStepRemainingMs = NEXT_STEP_DEFAULT_SECS * 1000;
let nextStepPaused = false;
let nextStepHandle: number | undefined;

function renderNextStepTimerDisplay() {
  const el = $("#session-next-step-timer");
  if (!el) return;
  if (nextStepEndsAtMs != null && !nextStepPaused) {
    const rem = Math.max(0, Math.ceil((nextStepEndsAtMs - Date.now()) / 1000));
    el.textContent = formatCountdownSecs(rem);
    return;
  }
  el.textContent = formatCountdownSecs(Math.ceil(nextStepRemainingMs / 1000));
}

function stopNextStepTicker() {
  if (nextStepHandle) {
    window.clearInterval(nextStepHandle);
    nextStepHandle = undefined;
  }
}

function clearNextStepTimer() {
  stopNextStepTicker();
  nextStepEndsAtMs = null;
  nextStepRemainingMs = NEXT_STEP_DEFAULT_SECS * 1000;
  nextStepPaused = false;
  renderNextStepTimerDisplay();
}

function tickNextStepTimer() {
  if (nextStepPaused || nextStepEndsAtMs == null) return;
  const remMs = Math.max(0, nextStepEndsAtMs - Date.now());
  nextStepRemainingMs = remMs;
  renderNextStepTimerDisplay();
  if (remMs <= 0) {
    stopNextStepTicker();
    nextStepEndsAtMs = null;
    nextStepRemainingMs = 0;
  }
}

function startNextStepTicker() {
  stopNextStepTicker();
  tickNextStepTimer();
  nextStepHandle = window.setInterval(tickNextStepTimer, 250);
}

function startNextStepTimer(durationSecs = NEXT_STEP_DEFAULT_SECS) {
  nextStepPaused = false;
  nextStepRemainingMs = durationSecs * 1000;
  nextStepEndsAtMs = Date.now() + nextStepRemainingMs;
  startNextStepTicker();
}

function nextStepRemainingSecsLive(): number {
  if (nextStepEndsAtMs != null && !nextStepPaused) {
    return Math.max(0, Math.ceil((nextStepEndsAtMs - Date.now()) / 1000));
  }
  return Math.max(0, Math.ceil(nextStepRemainingMs / 1000));
}

/** True while a five-minute next-step countdown is in flight (or paused mid-timer). */
function isNextStepTimerActive(): boolean {
  if (nextStepEndsAtMs != null && !nextStepPaused) {
    return nextStepEndsAtMs > Date.now();
  }
  return nextStepPaused && nextStepRemainingMs > 0;
}

/** Body label: m:ss without zero-padded minutes (live 15 sample “4:32”). */
function formatNextStepRemainLabel(totalSecs: number): string {
  const total = Math.max(0, Math.floor(totalSecs));
  const m = Math.floor(total / 60);
  const s = (total % 60).toString().padStart(2, "0");
  return `${m}:${s}`;
}

function openTimerReplaceModal() {
  const modal = $("#timer-replace-modal");
  const body = $("#timer-replace-modal-body");
  if (body) {
    const remain = formatNextStepRemainLabel(nextStepRemainingSecsLive());
    body.textContent =
      `Your current timer has ${remain} left. Replace it with a new five-minute timer? ` +
      `Your mission timer keeps running.`;
  }
  // Do not toggle is-session-ending — next-step + suggest must stay visible under scrim.
  if (modal) modal.hidden = false;
}

function closeTimerReplaceModal() {
  const modal = $("#timer-replace-modal");
  if (modal) modal.hidden = true;
}

function requestOrStartNextStepTimer() {
  if (isNextStepTimerActive()) {
    openTimerReplaceModal();
    return;
  }
  startNextStepTimer(NEXT_STEP_DEFAULT_SECS);
  const hint = $("#session-chat-hint");
  if (hint) {
    hint.textContent = "Five-minute next-step timer started. One small step at a time.";
  }
}

function pauseNextStepTimer() {
  if (nextStepPaused) return;
  if (nextStepEndsAtMs != null) {
    nextStepRemainingMs = Math.max(0, nextStepEndsAtMs - Date.now());
    nextStepEndsAtMs = null;
  }
  nextStepPaused = true;
  stopNextStepTicker();
  renderNextStepTimerDisplay();
}

function resumeNextStepTimer() {
  if (!nextStepPaused) return;
  nextStepPaused = false;
  if (nextStepRemainingMs > 0) {
    nextStepEndsAtMs = Date.now() + nextStepRemainingMs;
    startNextStepTicker();
  } else {
    renderNextStepTimerDisplay();
  }
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
  const progress = document.getElementById(
    "session-orbit-progress",
  ) as SVGPathElement | null;
  if (progress) {
    // Use the real path length — a fixed dash of 100 on a longer path drew a
    // phantom segment near Destination while Launch was only barely filled.
    const len = typeof progress.getTotalLength === "function" ? progress.getTotalLength() : 100;
    const safeLen = len > 0 ? len : 100;
    progress.style.strokeDasharray = `${safeLen}`;
    progress.style.strokeDashoffset = `${safeLen * (1 - frac)}`;
  }
  const shipWrap = $("#session-orbit-ship-wrap");
  if (shipWrap) {
    shipWrap.style.setProperty("--flight-pct", String(Math.min(1, Math.max(0, frac))));
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
  label.textContent = `${mins} min`;
}

function setSessionListeningUi(listening: boolean) {
  $("#view-session")?.classList.toggle("is-session-listening", listening);
}

/** Match overlay.ts toast: 6500ms visible + 280ms leave animation. */
const SESSION_CHECKIN_HIDE_MS = 6780;
let sessionCheckinHideTimer: number | undefined;

function syncCopilotPanelAriaLabel() {
  const panel = $(".session-copilot-panel");
  if (!panel) return;
  const view = $("#view-session");
  if (view?.classList.contains("is-session-break")) {
    panel.setAttribute("aria-label", "On a break");
  } else if (view?.classList.contains("is-session-checkin")) {
    panel.setAttribute("aria-label", "Quick check-in");
  } else {
    panel.setAttribute("aria-label", "Mission copilot");
  }
}

function setSessionCheckinUi(active: boolean) {
  const view = $("#view-session");
  view?.classList.toggle("is-session-checkin", active);
  const checkinCard = $("#session-checkin-card");
  if (checkinCard) {
    checkinCard.hidden = !active;
    checkinCard.setAttribute("aria-hidden", active ? "false" : "true");
  }
  syncCopilotPanelAriaLabel();
  if (sessionCheckinHideTimer) {
    window.clearTimeout(sessionCheckinHideTimer);
    sessionCheckinHideTimer = undefined;
  }
  if (active) {
    sessionCheckinHideTimer = window.setTimeout(() => {
      setSessionCheckinUi(false);
    }, SESSION_CHECKIN_HIDE_MS);
  }
}

function setSessionEndingUi(ending: boolean) {
  $("#view-session")?.classList.toggle("is-session-ending", ending);
}

function currentEarnedFlightMinutes(): number {
  if (!currentEndsAt || currentSessionDurationSecs <= 0) return 0;
  return flightMinutesEarned(
    currentEndsAt,
    currentSessionDurationSecs,
    currentOnTaskTicks,
    currentTotalTicks,
  );
}

function openEndSessionModal() {
  const modal = $("#end-session-modal");
  const body = $("#end-session-modal-body");
  if (body) {
    const earned = currentEarnedFlightMinutes();
    body.textContent = `Your ${earned} earned flight minutes will be saved. You can reflect on your objective next.`;
  }
  setSessionEndingUi(true);
  if (modal) modal.hidden = false;
}

function closeEndSessionModal() {
  const modal = $("#end-session-modal");
  if (modal) modal.hidden = true;
  setSessionEndingUi(false);
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
  const source =
    vitals.source === "presage"
      ? "camera"
      : vitals.source === "fallback"
        ? "camera estimate"
        : vitals.source || "—";
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
  pill.classList.toggle("is-paused", label === "paused");
  text.textContent = finishing
    ? "Finishing up…"
    : label === "paused"
      ? "Ⅱ  On a break"
      : label === "distracted"
        ? "Needs focus"
        : "Mission in progress";
}

function syncPauseControls(paused: boolean) {
  const btn = $("#session-pause") as HTMLButtonElement | null;
  const note = $("#session-pause-note");
  const endBtn = $("#end-session") as HTMLButtonElement | null;
  const caption = $("#session-timer-caption");
  const breakCard = $("#session-break-card");
  $("#view-session")?.classList.toggle("is-session-break", paused);
  if (btn) {
    btn.disabled = false;
    btn.setAttribute("aria-pressed", paused ? "true" : "false");
    btn.textContent = paused ? "Resume mission" : "Pause";
    btn.classList.toggle("is-paused", paused);
  }
  if (endBtn) endBtn.disabled = false;
  // Live 10: break copy lives in the right card only — never show left pause note.
  if (note) note.hidden = true;
  if (breakCard) {
    breakCard.hidden = !paused;
    breakCard.setAttribute("aria-hidden", paused ? "false" : "true");
  }
  syncCopilotPanelAriaLabel();
  if (caption) {
    caption.textContent = paused ? "REMAINING · TIMER PAUSED" : "REMAINING IN YOUR FLIGHT";
  }
}

/** Captured MM:SS while paused so session-update cannot thaw the display. */
let missionTimerFrozenDisplay: string | null = null;

function renderSession(session: LockInSession) {
  currentSessionDurationSecs = session.duration_secs;
  currentOnTaskTicks = session.on_task_ticks ?? 0;
  currentTotalTicks = session.total_ticks ?? 0;
  const timer = $("#session-timer");
  const status = $("#session-status");
  const goals = $("#session-goals");
  const note = $("#session-watch-note");
  if (timer) {
    // While paused, keep the captured freeze (session-update must not thaw countdown).
    timer.textContent =
      session.paused && missionTimerFrozenDisplay != null
        ? missionTimerFrozenDisplay
        : formatRemaining(session.ends_at);
  }
  if (status) {
    const label = session.paused ? "paused" : statusLabel(session.status);
    status.textContent = session.paused
      ? "Coach status: on a break"
      : `Coach status: ${label.replace(/_/g, " ")}`;
    applySessionOrbitState(session.paused ? "on_task" : label);
    updateSessionProgressPill(label);
  }
  if (goals) {
    goals.textContent = session.goals || "Your mission";
    goals.hidden = false;
  }
  if (note) note.textContent = session.watching_note || "Watching your screen";
  syncPauseControls(Boolean(session.paused));
  if (session.paused) pauseNextStepTimer();
  else if (nextStepPaused) resumeNextStepTimer();
  syncSessionSignalPills();
  renderVitals(session.vitals);
  renderSessionCoachLog(session.prompts);
  ensureSessionAtLaunchSeed(session);
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

async function openSettings(tab = "lockin") {
  if (!appUnlocked) {
    show("view-home");
    return;
  }
  show("view-settings");
  selectSettingsTab(tab);
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

type CameraHandoffSource = "setup" | "settings";

function setCameraToggleChecked(on: boolean) {
  const setup = $("#lockin-camera") as HTMLInputElement | null;
  const settings = $("#setting-camera-signals") as HTMLInputElement | null;
  if (setup) setup.checked = on;
  if (settings) settings.checked = on;
  writeBoolPref(PREF_CAMERA_SIGNALS, on);
  syncSessionSignalPills();
}

function showPermissionHandoffModal(_source: CameraHandoffSource) {
  const modal = $("#permission-handoff-modal");
  if (modal) modal.hidden = false;
}

function hidePermissionHandoffModal() {
  const modal = $("#permission-handoff-modal");
  if (modal) modal.hidden = true;
}

function showPermissionDeniedModal() {
  const modal = $("#permission-denied-modal");
  if (modal) modal.hidden = false;
}

function hidePermissionDeniedModal() {
  const modal = $("#permission-denied-modal");
  if (modal) modal.hidden = true;
}

async function continueCameraHandoff() {
  const continueBtn = $("#permission-handoff-continue") as HTMLButtonElement | null;
  if (continueBtn) continueBtn.disabled = true;
  try {
    const granted = await invoke<boolean>("request_camera_permission");
    hidePermissionHandoffModal();
    if (granted) {
      setCameraToggleChecked(true);
      void renderPermissionsStatus();
    } else {
      setCameraToggleChecked(false);
      showPermissionDeniedModal();
    }
  } catch (err) {
    console.error(err);
    hidePermissionHandoffModal();
    setCameraToggleChecked(false);
    showPermissionDeniedModal();
  } finally {
    if (continueBtn) continueBtn.disabled = false;
  }
}

function cancelCameraHandoff() {
  hidePermissionHandoffModal();
  setCameraToggleChecked(false);
}

function wireCameraPermissionHandoff(inputId: string, source: CameraHandoffSource) {
  $(`#${inputId}`)?.addEventListener("change", (event) => {
    const input = event.target as HTMLInputElement;
    if (!input.checked) {
      writeBoolPref(PREF_CAMERA_SIGNALS, false);
      const other =
        source === "setup"
          ? ($("#setting-camera-signals") as HTMLInputElement | null)
          : ($("#lockin-camera") as HTMLInputElement | null);
      if (other) other.checked = false;
      return;
    }
    // Defer OS prompt until Continue — leave toggle off until granted.
    input.checked = false;
    showPermissionHandoffModal(source);
  });
}

function selectSettingsTab(tab: string) {
  const next = tab || "lockin";
  document.querySelectorAll<HTMLButtonElement>("[data-settings-tab]").forEach((button) => {
    const active = button.dataset.settingsTab === next;
    button.classList.toggle("active", active);
    button.setAttribute("aria-selected", active ? "true" : "false");
  });
  document.querySelectorAll<HTMLElement>(".settings-panel").forEach((panel) => {
    const active = panel.id === `settings-${next}`;
    panel.classList.toggle("active", active);
    panel.hidden = !active;
  });
  if (next === "permissions") {
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

  if (lastSummaryObjective) {
    closing.textContent = buildCopilotNote(
      summary,
      flightMinutes,
      relaunches,
      pb,
      lastSummaryObjective,
    );
    setSummaryNoteVisible(true);
  } else {
    closing.textContent = "";
    setSummaryNoteVisible(false);
  }

  const pbStatValue =
    pb.isNew && pb.delta > 0 ? `+${pb.delta} min` : `${Math.max(flightMinutes, pb.previous)} min`;
  const pbStatLabel = pb.isNew && pb.delta > 0 ? "new personal best" : "personal best";

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
    pbBanner.hidden = false;
    if (pb.isNew && pb.previous > 0) {
      pbBanner.innerHTML = `
        <svg class="quest-complete-pb-banner-flag" width="12" height="12" viewBox="0 0 12 12" aria-hidden="true">
          <path
            fill="currentColor"
            d="M2 1.5v9M2 2.5h6.2c.3 0 .5.2.5.5v2.8c0 .3-.2.5-.5.5H2"
          />
        </svg>
        <span>New longest flight! ${escapeHtml(String(pb.previous))} → ${escapeHtml(String(flightMinutes))} min</span>
      `;
    } else {
      const best = personalBestMinutes(flightMinutes, pb);
      pbBanner.textContent = `${flightMinutes} minutes logged · your best is still ${best} min`;
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
  missionTimerFrozenDisplay = null;
  clearNextStepTimer();
}

function startTimer(endsAt: string) {
  if (timerHandle) {
    window.clearInterval(timerHandle);
    timerHandle = undefined;
  }
  currentEndsAt = endsAt;
  missionTimerFrozenDisplay = null;
  const tick = () => {
    if (!currentEndsAt || missionTimerFrozenDisplay != null) return;
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
  // Schedule first so a throw inside tick cannot leave the mission without an interval.
  timerHandle = window.setInterval(tick, 1000);
  tick();
}

/**
 * Keep the mission countdown ticking while active; freeze while paused.
 * Must not thrash-restart on every session-update (coach emits ~1Hz).
 */
function syncMissionTimer(session: Pick<LockInSession, "paused" | "ends_at">) {
  if (session.paused) {
    if (timerHandle) {
      window.clearInterval(timerHandle);
      timerHandle = undefined;
    }
    if (session.ends_at) currentEndsAt = session.ends_at;
    if (missionTimerFrozenDisplay == null) {
      missionTimerFrozenDisplay = formatRemaining(
        session.ends_at || currentEndsAt || new Date().toISOString(),
      );
    }
    const el = $("#session-timer");
    if (el) el.textContent = missionTimerFrozenDisplay;
    return;
  }

  missionTimerFrozenDisplay = null;
  if (!session.ends_at) return;

  // Same deadline already scheduled — leave the 1s interval alone.
  if (timerHandle && currentEndsAt === session.ends_at) {
    return;
  }
  startTimer(session.ends_at);
}

async function refreshStatus() {
  const status = await invoke<StatusPayload>("get_status");
  await applyAccountLocalScope(status);
  renderHome(status);
  if (!isAppUnlocked(status)) {
    show("view-home");
    return status;
  }
  if ($("#view-settings")?.classList.contains("active")) {
    await renderMissionControlSettings(status);
  }
  const session = status.session;
  if (session?.active) {
    renderSession(session);
    syncMissionTimer(session);
    show("view-session");
  }
  return status;
}

async function bootApp() {
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
      syncSummaryCelebrationFromOutcome();
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

  const formatVoiceResult = (transcript: VoiceTranscript): string => {
    const text = transcript.text?.trim();
    const note = transcript.note?.trim();
    if (text) {
      return note ? `Heard: “${text}” — ${note}` : `Heard: “${text}”`;
    }
    if (note) return `Mic test finished — no speech detected. ${note}`;
    return "Mic test finished — no speech detected. Try speaking clearly for the full 4 seconds.";
  };

  /** Fallback STT when the Rust/macOS Speech helper is unavailable. */
  function listenWithWebSpeech(
    seconds: number,
    signal?: AbortSignal,
  ): Promise<VoiceTranscript> {
    interface WebSpeechRecognition extends EventTarget {
      lang: string;
      interimResults: boolean;
      continuous: boolean;
      start(): void;
      stop(): void;
      abort(): void;
      onresult: ((event: {
        resultIndex: number;
        results: ArrayLike<{ isFinal: boolean; 0?: { transcript?: string } }>;
      }) => void) | null;
      onerror: ((event: { error: string }) => void) | null;
      onend: (() => void) | null;
    }
    type SRCtor = new () => WebSpeechRecognition;
    const Ctor =
      (window as unknown as { SpeechRecognition?: SRCtor; webkitSpeechRecognition?: SRCtor })
        .SpeechRecognition ||
      (window as unknown as { webkitSpeechRecognition?: SRCtor }).webkitSpeechRecognition;
    if (!Ctor) {
      return Promise.reject(
        new Error("Speech recognition isn’t available in this window."),
      );
    }
    return new Promise((resolve, reject) => {
      if (signal?.aborted) {
        resolve({ text: "", engine: "webkit-speech", note: "Listening stopped." });
        return;
      }
      const recognition = new Ctor();
      recognition.lang = "en-US";
      recognition.interimResults = true;
      recognition.continuous = true;
      let finalText = "";
      let settled = false;
      const finish = (text: string, note: string) => {
        if (settled) return;
        settled = true;
        signal?.removeEventListener("abort", onAbort);
        try {
          recognition.stop();
        } catch {
          /* ignore */
        }
        resolve({
          text: text.trim(),
          engine: "webkit-speech",
          note,
        });
      };
      const onAbort = () => {
        window.clearTimeout(timer);
        try {
          recognition.abort();
        } catch {
          try {
            recognition.stop();
          } catch {
            /* ignore */
          }
        }
        finish(finalText, "Listening stopped.");
      };
      const timer = window.setTimeout(() => {
        finish(
          finalText,
          finalText ? "Listening finished." : "No speech detected.",
        );
      }, Math.max(2, seconds) * 1000);
      signal?.addEventListener("abort", onAbort, { once: true });
      recognition.onresult = (event) => {
        let interim = "";
        for (let i = event.resultIndex; i < event.results.length; i++) {
          const piece = event.results[i][0]?.transcript ?? "";
          if (event.results[i].isFinal) finalText += `${piece} `;
          else interim += piece;
        }
        if (!finalText && interim) finalText = interim;
      };
      recognition.onerror = (event) => {
        window.clearTimeout(timer);
        signal?.removeEventListener("abort", onAbort);
        if (settled) return;
        if (event.error === "aborted") {
          finish(finalText, "Listening stopped.");
          return;
        }
        settled = true;
        reject(new Error(`Mic / speech error: ${event.error}`));
      };
      recognition.onend = () => {
        window.clearTimeout(timer);
        finish(finalText, "Listening ended.");
      };
      try {
        recognition.start();
      } catch (err) {
        window.clearTimeout(timer);
        signal?.removeEventListener("abort", onAbort);
        reject(err);
      }
    });
  }

  async function listenForTranscript(
    seconds = 4,
    signal?: AbortSignal,
  ): Promise<VoiceTranscript> {
    if (signal?.aborted) {
      return { text: "", note: "Listening stopped." };
    }
    try {
      const job = invoke<VoiceTranscript>("voice_listen_test", { seconds });
      if (!signal) return await job;
      return await Promise.race([
        job,
        new Promise<VoiceTranscript>((resolve) => {
          const onAbort = () => {
            resolve({ text: "", note: "Listening stopped." });
          };
          if (signal.aborted) onAbort();
          else signal.addEventListener("abort", onAbort, { once: true });
        }),
      ]);
    } catch (primary) {
      try {
        return await listenWithWebSpeech(seconds, signal);
      } catch {
        throw primary;
      }
    }
  }

  $("#chat-mic")?.addEventListener("click", () => {
    void toggleCompanionLive("copilot");
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

  $("#session-chat-input")?.addEventListener("keydown", (event) => {
    if (event.key === "Enter" && !event.shiftKey && !event.isComposing) {
      event.preventDefault();
      void sendSessionChat();
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
    if (missionLaunchLoading) return;
    const goalsInput = $("#goals") as HTMLTextAreaElement | null;
    const goals = goalsInput?.value.trim() ?? "";
    if (!goals) {
      syncMissionSetupLaunchUi();
      return;
    }
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
    const missionGoals = goals;
    setMissionLaunchButton(true);
    try {
      const session = await invoke<LockInSession>("start_lock_in", {
        goals: missionGoals,
        durationMins: duration,
      });
      try {
        sessionStorage.setItem("lockin-last-duration", String(duration));
      } catch {
        // ignore
      }
      renderVitals(null);
      playLaunchCelebration(() => {
        clearNextStepTimer();
        renderSession(session);
        syncMissionTimer(session);
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

  $("#session-pause")?.addEventListener("click", async () => {
    const btn = $("#session-pause") as HTMLButtonElement | null;
    if (!btn || btn.disabled) return;
    const currentlyPaused = btn.getAttribute("aria-pressed") === "true";
    const next = !currentlyPaused;
    btn.disabled = true;
    try {
      const session = await invoke<LockInSession>("set_lock_in_paused", { paused: next });
      renderSession(session);
      syncMissionTimer(session);
      // Belt-and-suspenders: renderSession also syncs next-step; keep explicit for Pause/Resume.
      if (session.paused) pauseNextStepTimer();
      else resumeNextStepTimer();
    } catch (err) {
      const msg = String(err);
      alert(/no active mission/i.test(msg) ? msg : "Couldn’t pause right now. Try again.");
      syncPauseControls(currentlyPaused);
    } finally {
      if (btn) btn.disabled = false;
    }
  });

  $("#session-break-resume")?.addEventListener("click", () => {
    const pauseBtn = $("#session-pause") as HTMLButtonElement | null;
    if (!pauseBtn || pauseBtn.disabled) return;
    if (pauseBtn.getAttribute("aria-pressed") !== "true") return;
    pauseBtn.click();
  });

  $("#session-checkin-on-task")?.addEventListener("click", () => {
    setSessionCheckinUi(false);
  });

  $("#session-checkin-distracted")?.addEventListener("click", () => {
    setSessionCheckinUi(false);
    // Soft local acknowledge — backend still owns coach status ticks.
    applySessionOrbitState("distracted");
    updateSessionProgressPill("distracted");
  });

  $("#session-checkin-break")?.addEventListener("click", () => {
    setSessionCheckinUi(false);
    const pauseBtn = $("#session-pause") as HTMLButtonElement | null;
    if (!pauseBtn || pauseBtn.disabled) return;
    if (pauseBtn.getAttribute("aria-pressed") === "true") return;
    pauseBtn.click();
  });

  const confirmEndSession = async () => {
    closeEndSessionModal();
    const btn = $("#end-session") as HTMLButtonElement | null;
    if (btn) btn.disabled = true;
    const pauseBtn = $("#session-pause") as HTMLButtonElement | null;
    if (pauseBtn) pauseBtn.disabled = true;
    try {
      teardownCompanionLive();
      stopTimer();
      const summary = await invoke<SessionSummary | null>("stop_lock_in");
      syncPauseControls(false);
      setSessionCheckinUi(false);
      if (summary) {
        showSummaryWithCelebration(summary, true);
      } else {
        show("view-home");
      }
    } catch (err) {
      console.error(err);
      alert("Couldn’t end the mission. Please try again.");
      if (btn) btn.disabled = false;
      if (pauseBtn) pauseBtn.disabled = false;
    }
  };

  $("#end-session")?.addEventListener("click", () => {
    const btn = $("#end-session") as HTMLButtonElement | null;
    if (btn?.disabled) return;
    openEndSessionModal();
  });

  $("#end-session-modal")
    ?.querySelectorAll("[data-end-session-dismiss]")
    .forEach((el) => {
      el.addEventListener("click", () => closeEndSessionModal());
    });

  $("#end-session-confirm")?.addEventListener("click", () => {
    void confirmEndSession();
  });

  document.querySelectorAll<HTMLButtonElement>("[data-settings-tab]").forEach((button) => {
    button.addEventListener("click", () =>
      selectSettingsTab(button.dataset.settingsTab || "lockin"),
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
  $("#goals")?.addEventListener("input", () => {
    syncMissionSetupLaunchUi();
  });
  syncMissionSetupLaunchUi();

  $("#session-chat-form")?.addEventListener("submit", (e) => {
    e.preventDefault();
    void sendSessionChat();
  });
  $("#session-copilot-suggest")?.addEventListener("click", () => {
    requestOrStartNextStepTimer();
  });

  $("#timer-replace-modal")
    ?.querySelectorAll("[data-timer-replace-dismiss]")
    .forEach((el) => {
      el.addEventListener("click", () => closeTimerReplaceModal());
    });

  $("#timer-replace-confirm")?.addEventListener("click", () => {
    closeTimerReplaceModal();
    startNextStepTimer(NEXT_STEP_DEFAULT_SECS);
    const hint = $("#session-chat-hint");
    if (hint) {
      hint.textContent = "Five-minute next-step timer started. One small step at a time.";
    }
  });
  $("#session-chat-mic")?.addEventListener("click", () => {
    void toggleCompanionLive("session");
  });
  $("#setting-copilot-audio")?.addEventListener("change", () => {
    void persistCopilotAudioFromToggle();
  });
  $("#setting-silent-mode")?.addEventListener("change", () => {
    void persistSilentMode();
  });
  wireCameraPermissionHandoff("lockin-camera", "setup");
  wireCameraPermissionHandoff("setting-camera-signals", "settings");
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
  $("#connection-refresh")?.addEventListener("click", () => {
    void renderConnectionStatus();
  });
  $("#permissions-refresh")?.addEventListener("click", () => {
    void renderPermissionsStatus();
  });
  $("#permission-handoff-modal")
    ?.querySelectorAll("[data-perm-handoff-dismiss]")
    .forEach((el) => {
      el.addEventListener("click", () => cancelCameraHandoff());
    });
  $("#permission-handoff-continue")?.addEventListener("click", () => {
    void continueCameraHandoff();
  });
  $("#permission-denied-setup")?.addEventListener("click", () => {
    hidePermissionDeniedModal();
    show("view-lockin");
  });
  $("#permission-denied-settings")?.addEventListener("click", () => {
    hidePermissionDeniedModal();
    void openSettings("permissions");
  });
  $("#permission-denied-modal")
    ?.querySelectorAll("[data-perm-denied-dismiss]")
    .forEach((el) => {
      el.addEventListener("click", () => hidePermissionDeniedModal());
    });
  const deleteModal = $("#delete-data-modal");
  const deleteResult = $("#delete-data-result");
  const closeDeleteModal = () => {
    if (deleteModal) deleteModal.hidden = true;
  };
  const openDeleteModal = () => {
    if (deleteResult) deleteResult.textContent = "";
    if (deleteModal) deleteModal.hidden = false;
  };
  $("#delete-data-start")?.addEventListener("click", () => openDeleteModal());
  deleteModal?.querySelectorAll("[data-delete-dismiss]").forEach((el) => {
    el.addEventListener("click", () => closeDeleteModal());
  });
  $("#delete-data-confirm-btn")?.addEventListener("click", async () => {
    const btn = $("#delete-data-confirm-btn") as HTMLButtonElement | null;
    if (btn) btn.disabled = true;
    if (deleteResult) deleteResult.textContent = "Deleting…";
    try {
      const result = await invoke<{
        removed: string[];
        cleared_local_keys: string[];
      }>("delete_all_user_data");
      for (const key of result.cleared_local_keys) {
        try {
          localStorage.removeItem(key);
        } catch {
          // ignore
        }
      }
      try {
        localStorage.clear();
      } catch {
        // ignore
      }
      const chatLog = $("#chat-log");
      if (chatLog) chatLog.innerHTML = "";
      closeDeleteModal();
      appUnlocked = false;
      await refreshStatus();
      refreshAllShipViews();
      show("view-home");
      if (deleteResult) {
        deleteResult.textContent =
          result.removed.length > 0
            ? `Deleted and signed out. Removed: ${result.removed.join(", ")}.`
            : "Deleted and signed out.";
      }
    } catch (err) {
      if (deleteResult) deleteResult.textContent = `Delete failed: ${String(err)}`;
    } finally {
      if (btn) btn.disabled = false;
    }
  });

  $("#invoke-voice-speak")?.addEventListener("click", async () => {
    const out = $("#invoke-voice-result");
    const btn = $("#invoke-voice-speak") as HTMLButtonElement | null;
    if (btn) btn.disabled = true;
    if (out) out.textContent = "Speaking…";
    try {
      await invoke("voice_speak", {
        text: "Waypoint voice test. Spoken coaching is ready.",
      });
      if (out) out.textContent = "Speak OK — you should have heard a short test phrase.";
    } catch (err) {
      if (out) out.textContent = `Speak failed: ${String(err)}`;
    } finally {
      if (btn) btn.disabled = false;
    }
  });
  $("#invoke-voice-listen")?.addEventListener("click", async () => {
    const out = $("#invoke-voice-result");
    const btn = $("#invoke-voice-listen") as HTMLButtonElement | null;
    if (btn) btn.disabled = true;
    if (out) {
      out.textContent = "Listening for 4 seconds… Speak clearly.";
    }
    try {
      const transcript = await listenForTranscript(4);
      if (out) out.textContent = formatVoiceResult(transcript);
      void renderPermissionsStatus();
    } catch (err) {
      if (out) out.textContent = `Mic test failed: ${String(err)}`;
    } finally {
      if (btn) btn.disabled = false;
    }
  });

  await listen<LockInSession>("session-update", (event) => {
    renderSession(event.payload);
    syncMissionTimer(event.payload);
  });
  await listen<VitalsSnapshot>("vitals-update", (event) => renderVitals(event.payload));
  await listen("overlay-prompt", () => setSessionCheckinUi(true));
  await listen("overlay-clear", () => setSessionCheckinUi(false));
  await listen<string>("coach-error", (event) => {
    const note = $("#session-watch-note");
    if (note) note.textContent = `Wellness check: ${event.payload}`;
  });
  await listen<SessionSummary>("session-ended", (event) => {
    teardownCompanionLive();
    stopTimer();
    setSessionCheckinUi(false);
    setSessionEndingUi(false);
    showSummaryWithCelebration(event.payload);
  });

  initShipUI();
  await refreshStatus();
}

// Module scripts often run after DOMContentLoaded — only boot once either way.
if (document.readyState === "loading") {
  window.addEventListener("DOMContentLoaded", () => {
    void bootApp();
  }, { once: true });
} else {
  void bootApp();
}

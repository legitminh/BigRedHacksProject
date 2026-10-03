/** Lightweight ship gamification — progress scoped per signed-in account. */

import shipAssetUrl from "./assets/figma/ship.svg?url";

export interface ShipProgress {
  completedMissions: number;
  onTaskMinutes: number;
  longestFlightMinutes: number;
  firstFlightCelebrated: boolean;
}

export interface MissionCompleteInput {
  on_task_ratio: number;
  screen_checks?: number;
}

export type StudyStatsLite = {
  total_sessions?: number;
  total_on_task_minutes?: number;
  longest_flight_minutes?: number;
};

const STORAGE_PREFIX = "waypoint-ship-progress";
/** @deprecated device-global key — removed on account switch so PB cannot leak. */
const LEGACY_STORAGE_KEY = "waypoint-ship-progress";
const LEGACY_LONGEST_KEY = "waypoint-longest-flight-min";

const LEVEL_THRESHOLDS = [0, 12, 36, 90] as const; // cumulative "flight points"
const POINTS_PER_MISSION = 8;

/** Current account scope (`user_id`, `"guest"`, or signed-out). */
let accountId: string | null = null;

function storageKey(): string {
  return accountId ? `${STORAGE_PREFIX}:${accountId}` : `${STORAGE_PREFIX}:signed-out`;
}

/** Bind ship/PB storage to the signed-in user (or guest / signed-out). */
export function setShipAccountId(userId: string | null) {
  accountId = userId && userId.trim() ? userId.trim() : null;
}

export function currentShipAccountId(): string | null {
  return accountId;
}

/** Drop device-global keys that used to leak across Google accounts. */
export function wipeLegacyUnscopedShipKeys() {
  try {
    localStorage.removeItem(LEGACY_STORAGE_KEY);
    localStorage.removeItem(LEGACY_LONGEST_KEY);
  } catch {
    // private mode / quota
  }
}

export function readShipProgress(): ShipProgress {
  return loadProgress();
}

function emptyProgress(): ShipProgress {
  return {
    completedMissions: 0,
    onTaskMinutes: 0,
    longestFlightMinutes: 0,
    firstFlightCelebrated: false,
  };
}

function loadProgress(): ShipProgress {
  try {
    const raw = localStorage.getItem(storageKey());
    if (!raw) return emptyProgress();
    const parsed = JSON.parse(raw) as Partial<ShipProgress>;
    return {
      completedMissions: Number(parsed.completedMissions) || 0,
      onTaskMinutes: Number(parsed.onTaskMinutes) || 0,
      longestFlightMinutes: Number(parsed.longestFlightMinutes) || 0,
      firstFlightCelebrated: Boolean(parsed.firstFlightCelebrated),
    };
  } catch {
    return emptyProgress();
  }
}

function saveProgress(p: ShipProgress) {
  try {
    localStorage.setItem(storageKey(), JSON.stringify(p));
  } catch {
    // private mode / quota
  }
}

/** Replace local ship stats from server study memory after account switch. */
export function hydrateShipFromStudyStats(stats: StudyStatsLite | null | undefined) {
  const p = emptyProgress();
  if (stats) {
    p.completedMissions = Math.max(0, Number(stats.total_sessions) || 0);
    p.onTaskMinutes = Math.max(0, Number(stats.total_on_task_minutes) || 0);
    p.longestFlightMinutes = Math.max(0, Number(stats.longest_flight_minutes) || 0);
    p.firstFlightCelebrated = p.completedMissions > 0;
  }
  saveProgress(p);
  refreshAllShipViews();
}

/** Record personal-best flight minutes for the current account only. */
export function recordLongestFlightMinutes(minutes: number): {
  previous: number;
  isNew: boolean;
  delta: number;
} {
  const p = loadProgress();
  const previous = p.longestFlightMinutes || 0;
  const isNew = minutes > previous && minutes > 0;
  if (isNew) {
    p.longestFlightMinutes = minutes;
    saveProgress(p);
  }
  return { previous, isNew, delta: isNew ? minutes - previous : 0 };
}

export function flightPoints(p: ShipProgress): number {
  return p.completedMissions * POINTS_PER_MISSION + Math.floor(p.onTaskMinutes);
}

export function shipLevel(p: ShipProgress): 1 | 2 | 3 | 4 {
  const pts = flightPoints(p);
  if (pts >= LEVEL_THRESHOLDS[3]) return 4;
  if (pts >= LEVEL_THRESHOLDS[2]) return 3;
  if (pts >= LEVEL_THRESHOLDS[1]) return 2;
  return 1;
}

/** 0–1 progress within current level toward the next hull. */
export function levelProgress(p: ShipProgress): number {
  const pts = flightPoints(p);
  const lvl = shipLevel(p);
  const floor = LEVEL_THRESHOLDS[lvl - 1] ?? 0;
  const nextThreshold: Record<1 | 2 | 3 | 4, number> = {
    1: LEVEL_THRESHOLDS[1],
    2: LEVEL_THRESHOLDS[2],
    3: LEVEL_THRESHOLDS[3],
    4: LEVEL_THRESHOLDS[3] + 60,
  };
  const ceiling = nextThreshold[lvl];
  if (ceiling <= floor) return 1;
  return Math.min(1, Math.max(0, (pts - floor) / (ceiling - floor)));
}

export function shipLevelLabel(level: number): string {
  const names = ["Scout pod", "Runabout", "Cruiser", "Flagship"];
  return names[Math.min(3, Math.max(0, level - 1))] ?? "Scout pod";
}

/** Figma Waypoint rocket (`src/assets/figma/ship.svg`). Level scales size only. */
function shipImg(level: 1 | 2 | 3 | 4, className = ""): string {
  const extra = className ? ` ${className}` : "";
  return `<img src="${shipAssetUrl}" class="ship-figma ship-figma--level-${level}${extra}" alt="" aria-hidden="true" decoding="async" />`;
}

function formatFlightStat(p: ShipProgress): string {
  const lvl = shipLevel(p);
  const pct = Math.round(levelProgress(p) * 100);
  return `${shipLevelLabel(lvl)} · ${p.completedMissions} flight${p.completedMissions === 1 ? "" : "s"} · ${Math.round(p.onTaskMinutes)}m on-task · ${pct}% to next hull`;
}

function renderHangarInner(p: ShipProgress): string {
  const lvl = shipLevel(p);
  const prog = levelProgress(p);
  return `
    <div class="ship-hangar-card">
      <p class="ship-hangar-kicker">✦ YOUR SHIP</p>
      <div class="ship-sprite-wrap">${shipImg(lvl, "ship-sprite")}</div>
      <p class="ship-hangar-stat">${formatFlightStat(p)}</p>
      <div class="ship-orbit-track" role="progressbar" aria-valuenow="${Math.round(prog * 100)}" aria-valuemin="0" aria-valuemax="100" aria-label="Progress to next ship hull">
        <div class="ship-orbit-fill" style="width:${Math.round(prog * 100)}%"></div>
        <span class="ship-orbit-ship" style="left:${Math.round(prog * 100)}%">${shipImg(lvl, "ship-orbit-icon")}</span>
      </div>
    </div>`;
}

let sessionTotalMs = 0;

export function onMissionStarted(durationSecs: number) {
  sessionTotalMs = Math.max(1, durationSecs) * 1000;
}

export function onMissionCompleted(summary: MissionCompleteInput, elapsedSecs: number) {
  const p = loadProgress();
  const mins = Math.max(0, elapsedSecs) / 60;
  const checks = summary.screen_checks ?? 0;
  const ratio = checks === 0 ? 0.35 : Math.min(1, Math.max(0, summary.on_task_ratio));
  p.onTaskMinutes += mins * ratio;
  const flightMins = Math.max(0, Math.round(Math.max(0, elapsedSecs) / 60));
  p.longestFlightMinutes = Math.max(p.longestFlightMinutes || 0, flightMins);
  p.completedMissions += 1;
  saveProgress(p);

  const firstFlight = p.completedMissions === 1 && !p.firstFlightCelebrated;
  if (firstFlight) {
    p.firstFlightCelebrated = true;
    saveProgress(p);
  }

  refreshAllShipViews();
  return firstFlight;
}

function elapsedFromSession(endsAt: string, durationSecs: number): number {
  const total = Math.max(1, durationSecs);
  const remaining = Math.max(0, new Date(endsAt).getTime() - Date.now()) / 1000;
  return Math.min(total, total - remaining);
}

export function updateSessionFlight(endsAt: string, durationSecs: number) {
  if (!sessionTotalMs) sessionTotalMs = Math.max(1, durationSecs) * 1000;
  const elapsed = elapsedFromSession(endsAt, durationSecs);
  const frac = Math.min(1, elapsed / Math.max(1, durationSecs));
  const track = document.querySelector<HTMLElement>(".ship-flight-track");
  const marker = document.querySelector<HTMLElement>(".ship-flight-marker");
  if (track) {
    track.setAttribute("aria-valuenow", String(Math.round(frac * 100)));
  }
  if (marker) {
    marker.style.left = `${Math.round(frac * 100)}%`;
  }
}

function refreshHomeHangar() {
  const host = document.getElementById("home-ship-hangar");
  if (!host) return;
  host.innerHTML = renderHangarInner(loadProgress());
}

export function refreshHomePersonalBest() {
  const el = document.getElementById("home-longest-minutes");
  if (!el) return;
  const mins = loadProgress().longestFlightMinutes || 0;
  el.textContent = mins > 0 ? String(mins) : "0";
}

export function refreshSessionFlight() {
  const orbitShip = document.getElementById("session-orbit-ship");
  const p = loadProgress();
  const lvl = shipLevel(p);
  if (orbitShip?.querySelector(".session-figma-ship")) {
    const orbit = document.getElementById("session-orbit");
    orbit?.setAttribute("aria-label", `Mission timer, ${shipLevelLabel(lvl)} in flight`);
    return;
  }
  if (orbitShip) {
    orbitShip.innerHTML = shipImg(lvl, "ship-orbit-flight-icon");
    const orbit = document.getElementById("session-orbit");
    orbit?.setAttribute("aria-label", `Mission timer, ${shipLevelLabel(lvl)} in orbit`);
    return;
  }
  const host = document.getElementById("session-ship-flight");
  if (!host) return;
  host.innerHTML = `
    <div class="ship-flight-panel">
      <p class="ship-flight-label">In flight · ${shipLevelLabel(lvl)}</p>
      <div class="ship-flight-track" role="progressbar" aria-valuenow="0" aria-valuemin="0" aria-valuemax="100" aria-label="Mission flight progress">
        <div class="ship-flight-stars"></div>
        <span class="ship-flight-marker" style="left:0%">${shipImg(lvl, "ship-flight-icon")}</span>
      </div>
    </div>`;
}

function refreshSummaryShip() {
  const host = document.getElementById("summary-ship");
  if (!host) return;
  const p = loadProgress();
  const lvl = shipLevel(p);
  host.innerHTML = `
    <div class="ship-summary-card">
      ${shipImg(lvl, "ship-sprite ship-summary-sprite")}
      <div>
        <p class="ship-summary-title">Flight logged</p>
        <p class="ship-summary-stat">${formatFlightStat(p)}</p>
      </div>
    </div>`;
}

export function refreshAllShipViews() {
  refreshHomeHangar();
  refreshHomePersonalBest();
  refreshSessionFlight();
  refreshSummaryShip();
}

export function initShipUI() {
  refreshAllShipViews();
}

import { listen } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";

interface CoachPrompt {
  id: string;
  at: string;
  text: string;
  kind: string;
}

const toast = () => document.getElementById("toast");
const textEl = () => document.getElementById("toast-text");

let hideTimer: number | undefined;

async function passClicksThrough() {
  try {
    const win = getCurrentWindow();
    await win.setIgnoreCursorEvents(true);
  } catch {
    // ignore — main process also forces this
  }
}

async function hideOverlayWindow() {
  try {
    const win = getCurrentWindow();
    await win.setIgnoreCursorEvents(true);
    await win.hide();
  } catch {
    // ignore
  }
}

function showPrompt(prompt: CoachPrompt) {
  const root = toast();
  const line = textEl();
  if (!root || !line) return;
  line.textContent = prompt.text;
  root.hidden = false;
  root.classList.remove("leaving");
  root.style.animation = "none";
  void root.offsetWidth;
  root.style.animation = "";
  void passClicksThrough();

  if (hideTimer) window.clearTimeout(hideTimer);
  hideTimer = window.setTimeout(() => {
    root.classList.add("leaving");
    window.setTimeout(() => {
      root.hidden = true;
      root.classList.remove("leaving");
      void hideOverlayWindow();
    }, 280);
  }, 7000);
}

function clearPrompt() {
  const root = toast();
  if (!root) return;
  if (hideTimer) window.clearTimeout(hideTimer);
  root.hidden = true;
  root.classList.remove("leaving");
  void hideOverlayWindow();
}

declare global {
  interface Window {
    __waypointShow?: (prompt: CoachPrompt) => void;
    __waypointClear?: () => void;
  }
}

window.addEventListener("DOMContentLoaded", async () => {
  window.__waypointShow = showPrompt;
  window.__waypointClear = clearPrompt;
  await passClicksThrough();
  await listen<CoachPrompt>("overlay-prompt", (event) => showPrompt(event.payload));
  await listen("overlay-clear", () => clearPrompt());
});

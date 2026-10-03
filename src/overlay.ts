import { listen } from "@tauri-apps/api/event";

interface CoachPrompt {
  id: string;
  at: string;
  text: string;
  kind: string;
}

const toast = () => document.getElementById("toast");
const textEl = () => document.getElementById("toast-text");

let hideTimer: number | undefined;

function showPrompt(prompt: CoachPrompt) {
  const root = toast();
  const line = textEl();
  if (!root || !line) return;
  line.textContent = prompt.text;
  root.hidden = false;
  root.classList.remove("leaving");
  // restart animation
  root.style.animation = "none";
  void root.offsetWidth;
  root.style.animation = "";

  if (hideTimer) window.clearTimeout(hideTimer);
  hideTimer = window.setTimeout(() => {
    root.classList.add("leaving");
    window.setTimeout(() => {
      root.hidden = true;
      root.classList.remove("leaving");
    }, 280);
  }, 7000);
}

function clearPrompt() {
  const root = toast();
  if (!root) return;
  if (hideTimer) window.clearTimeout(hideTimer);
  root.hidden = true;
  root.classList.remove("leaving");
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
  await listen<CoachPrompt>("overlay-prompt", (event) => showPrompt(event.payload));
  await listen("overlay-clear", () => clearPrompt());
});

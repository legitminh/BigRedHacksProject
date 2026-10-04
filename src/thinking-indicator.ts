/** Animated “Thinking…” / “Still thinking…” indicator for pending AI replies. */

const timers = new WeakMap<HTMLElement, number>();
const DEFAULT_STILL_AFTER_MS = 9_000;

export function showThinkingIndicator(
  host: HTMLElement,
  opts?: { stillAfterMs?: number },
): void {
  clearThinkingTimer(host);
  host.classList.add("is-thinking");
  host.replaceChildren();

  const wrap = document.createElement("span");
  wrap.className = "thinking-indicator";
  wrap.setAttribute("role", "status");

  const label = document.createElement("span");
  label.className = "thinking-indicator__label";
  label.textContent = "Thinking";

  const dots = document.createElement("span");
  dots.className = "thinking-indicator__dots";
  dots.setAttribute("aria-hidden", "true");
  for (let i = 0; i < 3; i++) {
    const dot = document.createElement("span");
    dot.className = "thinking-indicator__dot";
    dots.appendChild(dot);
  }

  wrap.append(label, dots);
  host.appendChild(wrap);

  const stillAfter = opts?.stillAfterMs ?? DEFAULT_STILL_AFTER_MS;
  if (stillAfter <= 0) {
    markThinkingProlonged(host);
    return;
  }

  const id = window.setTimeout(() => {
    if (!host.classList.contains("is-thinking")) return;
    markThinkingProlonged(host);
  }, stillAfter);
  timers.set(host, id);
}

export function markThinkingProlonged(
  host: HTMLElement,
  message = "Still thinking",
): void {
  if (!host.classList.contains("is-thinking")) {
    showThinkingIndicator(host, { stillAfterMs: 0 });
  }
  const label = host.querySelector(".thinking-indicator__label");
  const wrap = host.querySelector(".thinking-indicator");
  if (label) label.textContent = message;
  wrap?.classList.add("is-prolonged");
}

export function clearThinkingIndicator(host: HTMLElement): void {
  clearThinkingTimer(host);
  host.classList.remove("is-thinking");
}

function clearThinkingTimer(host: HTMLElement): void {
  const id = timers.get(host);
  if (id != null) {
    clearTimeout(id);
    timers.delete(host);
  }
}

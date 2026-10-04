import { registerHooks } from 'node:module';
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { JSDOM } from 'jsdom';

// Vite `*.png?url` / `*.svg?url` imports are not understood by Node's test runner.
registerHooks({
  resolve(specifier, context, nextResolve) {
    if (/\.(svg|png)(\?|$)/.test(specifier)) {
      return {
        shortCircuit: true,
        url: 'data:text/javascript,export default "";',
      };
    }
    return nextResolve(specifier, context);
  },
});

const dom = new JSDOM(readFileSync(new URL('../index.html', import.meta.url), 'utf8'), {
  url: 'http://localhost/',
  pretendToBeVisual: true,
});
globalThis.window = dom.window;
globalThis.document = dom.window.document;
globalThis.requestAnimationFrame = (cb) => setTimeout(() => cb(Date.now()), 0);
globalThis.cancelAnimationFrame = (id) => clearTimeout(id);
window.requestAnimationFrame = globalThis.requestAnimationFrame;
window.cancelAnimationFrame = globalThis.cancelAnimationFrame;
window.confirm = () => true;
window.__TAURI_INTERNALS__ = {
  transformCallback: () => 1,
  invoke: async () => ({}),
};

const {
  adoptMissionSession,
  applySessionUpdate,
  clearMissionRunningState,
  isMissionRunning,
} = await import('../src/main.ts');

function sampleSession(overrides = {}) {
  return {
    id: 'sess-end-1',
    goals: 'Finish ENGL 1140 outline',
    duration_secs: 1800,
    ends_at: new Date(Date.now() + 30 * 60 * 1000).toISOString(),
    modality: 'mixed',
    status: 'running',
    active: true,
    prompts: [],
    camera_enabled: false,
    screen_enabled: true,
    ...overrides,
  };
}

test('ending a mission clears running state and ignores late session-update', () => {
  adoptMissionSession(sampleSession());
  assert.equal(isMissionRunning(), true);
  assert.match(document.querySelector('#session-timer')?.textContent ?? '', /\d/);

  // First End Session: seal FE chrome immediately (same as confirmEndSession).
  clearMissionRunningState();
  assert.equal(isMissionRunning(), false);

  // Stale coach tick that used to resurrect the bottom mission bar / timer.
  applySessionUpdate(
    sampleSession({
      ends_at: new Date(Date.now() + 45 * 60 * 1000).toISOString(),
      watching_note: 'Late update after end',
    }),
  );
  assert.equal(isMissionRunning(), false);

  // Inactive payloads are ignored even if intake were somehow re-enabled.
  applySessionUpdate(sampleSession({ active: false }));
  assert.equal(isMissionRunning(), false);
});

test('second end stays idle — adopt then clear leaves no zombie running mission', () => {
  adoptMissionSession(sampleSession({ id: 'sess-end-2' }));
  assert.equal(isMissionRunning(), true);
  clearMissionRunningState();
  clearMissionRunningState();
  assert.equal(isMissionRunning(), false);
  const banner = document.querySelector('#mission-running-banner');
  if (banner) assert.equal(banner.hidden, true);
});

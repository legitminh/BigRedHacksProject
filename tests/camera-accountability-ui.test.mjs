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
  classifyCameraAccountabilityUi,
  cameraAccountabilityHonestNote,
} = await import('../src/main.ts');

test('server checking note → checking; present/away/obstructed → healthy', () => {
  assert.equal(
    classifyCameraAccountabilityUi('Camera accountability · checking', null),
    'checking',
  );
  assert.equal(
    classifyCameraAccountabilityUi('Camera accountability · present', null),
    'healthy',
  );
  assert.equal(
    classifyCameraAccountabilityUi('Camera accountability · away from desk', null),
    'healthy',
  );
  assert.equal(
    classifyCameraAccountabilityUi('Camera accountability · camera unclear', null),
    'healthy',
  );
});

test('JPEG skip / unavailable → failed; vitals presence=uncertain → checking', () => {
  assert.equal(
    classifyCameraAccountabilityUi(
      'Camera accountability · checking · JPEG observe skips Presage — leave/stress detection requires video/mp4 or video/webm',
      null,
    ),
    'failed',
  );
  assert.equal(
    classifyCameraAccountabilityUi('Watching your screen · camera accountability unavailable', null),
    'failed',
  );
  assert.equal(
    classifyCameraAccountabilityUi('Watching with you…', {
      stressed: false,
      focus_ok: true,
      raw_summary: 'presence=uncertain',
      source: 'presence',
    }),
    'checking',
  );
  assert.equal(
    classifyCameraAccountabilityUi('On task', {
      stressed: false,
      focus_ok: true,
      raw_summary: 'presence=left_frame',
      source: 'presence',
    }),
    'healthy',
  );
});

test('honest note escalates from checking to unavailable', () => {
  assert.equal(
    cameraAccountabilityHonestNote('checking', false),
    'Accountability checking…',
  );
  assert.equal(
    cameraAccountabilityHonestNote('checking', true),
    'Camera accountability unavailable',
  );
  assert.equal(
    cameraAccountabilityHonestNote('failed', false),
    'Camera accountability unavailable',
  );
  assert.equal(cameraAccountabilityHonestNote('healthy', false), null);
});

import { registerHooks } from 'node:module';
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { JSDOM } from 'jsdom';

// Vite `*.svg?url` imports are not understood by Node's test runner.
registerHooks({
  resolve(specifier, context, nextResolve) {
    if (/\.svg(\?|$)/.test(specifier)) {
      return {
        shortCircuit: true,
        url: 'data:text/javascript,export default "data:image/svg+xml,%3Csvg xmlns=%27http://www.w3.org/2000/svg%27/%3E";',
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
const { renderMarkdown } = await import('../src/markdown.ts');
const tick = () => new Promise(resolve => setTimeout(resolve, 0));

let resolveReply;
let rejectReply;
const calls = [];
const defaultInvoke = async (command, args) => {
  calls.push({ command, args });
  if (command === 'get_status') return { google_connected: true };
  if (command === 'chat_send') return new Promise((resolve, reject) => { resolveReply = resolve; rejectReply = reject; });
  return 1;
};
window.__TAURI_INTERNALS__ = {
  transformCallback: () => 1,
  invoke: defaultInvoke,
};
await import('../src/main.ts');
window.dispatchEvent(new window.Event('DOMContentLoaded'));
await tick();

test('renders study notes, nested lists, code, tables, and links', () => {
  const host = document.createElement('div');
  renderMarkdown(host, '# Review\n\n**Key idea** and *example*\n\n1. First\n   - Hint\n\n```js\nconst x = 1 < 2;\n```\n\n| Topic | Time |\n| --- | --- |\n| Math | 20 min |\n\n[Source](https://example.com/notes)');
  assert.equal(host.querySelector('h1').textContent, 'Review');
  assert.equal(host.querySelector('strong').textContent, 'Key idea');
  assert.ok(host.querySelector('ol ul li'));
  assert.match(host.querySelector('pre code').textContent, /1 < 2/);
  assert.equal(host.querySelector('.table-scroll td').textContent, 'Math');
  assert.equal(host.querySelector('a').href, 'https://example.com/notes');
});

test('untrusted replies cannot inject scripts, events, embeds, or unsafe links', () => {
  const host = document.createElement('div');
  renderMarkdown(host, '<script>alert(1)</script><img src="https://example.com/pixel" onerror="alert(1)"><iframe src="https://example.com"></iframe><p id="chat-input" style="position:fixed" onclick="alert(1)">Safe</p>\n\n[Bad](javascript:alert%281%29) [Local](file:///etc/passwd) [Relative](/other)');
  assert.equal(host.querySelector('script,img,iframe,[onclick],[onerror],[style],[id]'), null);
  assert.equal(host.querySelector('a[href]'), null);
  assert.match(host.textContent, /Safe/);
});

test('chat keeps turns in order, renders replies, supports study actions and clears history', async () => {
  const input = document.querySelector('#chat-input');
  const form = document.querySelector('#chat-form');
  // Chips auto-send the prompt.
  document.querySelector('[data-study="explain"]').click();
  assert.equal(input.value, '');
  assert.match(document.querySelector('.bubble.user').textContent, /Explain this topic simply/);
  assert.equal(document.querySelector('#chat-send').disabled, true);
  input.value = 'next draft';
  form.dispatchEvent(new window.Event('submit', { cancelable: true }));
  assert.equal(calls.filter(c => c.command === 'chat_send').length, 1);
  resolveReply({ role: 'assistant', content: '## Photosynthesis\n\nPlants use **light**.' });
  await tick();
  assert.equal(document.querySelector('.bubble.assistant h2').textContent, 'Photosynthesis');
  assert.equal(input.value, 'next draft');
  assert.equal(document.querySelector('#chat-send').disabled, false);

  input.value = '<b>Explain photosynthesis</b>\nWith an example';
  form.dispatchEvent(new window.Event('submit', { cancelable: true }));
  assert.equal(document.querySelector('.bubble.user b'), null);
  assert.equal(calls.filter(c => c.command === 'chat_send').length, 2);
  resolveReply({ role: 'assistant', content: 'Still **light**.' });
  await tick();

  document.querySelector('[data-study="stuck"]').click();
  const userBubbles = [...document.querySelectorAll('.bubble.user')];
  assert.match(userBubbles.at(-1)?.textContent ?? '', /we're discussing|stuck/i);
  assert.equal(calls.filter(c => c.command === 'chat_send').length, 3);
  resolveReply({ role: 'assistant', content: 'One small step.' });
  await tick();

  const shiftEnter = new window.KeyboardEvent('keydown', { key: 'Enter', shiftKey: true, cancelable: true });
  input.dispatchEvent(shiftEnter);
  assert.equal(shiftEnter.defaultPrevented, false);
  document.querySelector('#new-chat').click();
  await tick();
  assert.ok(calls.some(c => c.command === 'clear_chat'));
  assert.ok(document.querySelector('#chat-empty'));
});

test('renders LaTeX delimiters while leaving code and currency alone', () => {
  const host = document.createElement('div');
  renderMarkdown(host, String.raw`Inline $x^2$ and \(\frac{1}{2}\).

$$\sum_{i=1}^n i$$

\[\begin{pmatrix}1 & 2 \\ 3 & 4\end{pmatrix}\]

Code: \`$x$\`.

Costs $5 and $10.`.replaceAll('\\`', '`'));
  assert.equal(host.querySelectorAll('.katex').length, 4);
  assert.equal(host.querySelectorAll('.katex-display').length, 2);
  assert.equal(host.querySelector('code').textContent, '$x$');
  assert.match(host.textContent, /Costs \$5 and \$10/);
});

test('invalid math stays readable and untrusted math cannot load resources', () => {
  const host = document.createElement('div');
  renderMarkdown(host, String.raw`$\frac{$ and $\includegraphics{https://example.com/pixel}$ and $\href{javascript:alert(1)}{click}$`);
  assert.ok(host.querySelector('.katex-error'));
  assert.equal(host.querySelector('img,script,iframe,a[href]'), null);
});


test('study suggestion card accepts into lock-in and decline dismisses', async () => {
  const input = document.querySelector('#chat-input');
  const form = document.querySelector('#chat-form');
  document.querySelector('#view-chat')?.classList.add('active');
  document.querySelector('#view-session')?.classList.remove('active');
  // Skip launch animation so Accept resolves synchronously in jsdom.
  document.querySelector('#celebration-launch')?.remove();

  input.value = 'Help me make a study plan for chemistry';
  form.dispatchEvent(new window.Event('submit', { cancelable: true }));
  resolveReply({
    role: 'assistant',
    content: 'Let’s lock in on chemistry for a bit.',
    study_suggestion: {
      goals: '',
      duration_mins: 25,
      reason: 'You asked for a plan and have a clear window.',
      proposed_start: '2026-10-03T19:00:00Z',
      calendar_checked: false,
      calendar_clear: true,
      conflict_summary: null,
    },
  });
  await tick();

  const card = document.querySelector('.study-suggest-card');
  assert.ok(card);
  assert.match(card.textContent, /General study session/);
  assert.match(card.textContent, /25 min/);
  assert.equal(document.querySelector('#chat-send').disabled, false);

  // Conflict / unclear calendar must not render a card.
  input.value = 'another question';
  form.dispatchEvent(new window.Event('submit', { cancelable: true }));
  resolveReply({
    role: 'assistant',
    content: 'Sure.',
    study_suggestion: {
      goals: 'Chem',
      duration_mins: 20,
      reason: 'nope',
      proposed_start: '2026-10-03T19:00:00Z',
      calendar_checked: true,
      calendar_clear: false,
      conflict_summary: 'Conflicts with: Lecture',
    },
  });
  await tick();
  assert.equal(document.querySelectorAll('.study-suggest-card').length, 1);

  document.querySelector('.study-suggest-card__decline').click();
  assert.equal(document.querySelector('.study-suggest-card'), null);

  // Fresh clear suggestion → Accept wires start_lock_in with empty-goals fallback.
  input.value = 'ready to focus';
  form.dispatchEvent(new window.Event('submit', { cancelable: true }));
  resolveReply({
    role: 'assistant',
    content: 'Go.',
    study_suggestion: {
      goals: '   ',
      duration_mins: 30,
      reason: 'Now is good.',
      proposed_start: '2026-10-03T19:30:00Z',
      calendar_checked: true,
      calendar_clear: true,
      conflict_summary: null,
    },
  });
  await tick();
  const acceptCard = document.querySelector('.study-suggest-card');
  assert.ok(acceptCard);

  const beforeStart = calls.filter(c => c.command === 'start_lock_in').length;
  window.__TAURI_INTERNALS__.invoke = async (command, args) => {
    calls.push({ command, args });
    if (command === 'get_status') return { google_connected: true };
    if (command === 'chat_send') return new Promise((resolve, reject) => { resolveReply = resolve; rejectReply = reject; });
    if (command === 'start_lock_in') {
      return {
        id: 'sess-suggest',
        goals: args.goals,
        duration_secs: args.durationMins * 60,
        ends_at: new Date(Date.now() + args.durationMins * 60000).toISOString(),
        modality: 'mixed',
        status: 'running',
        active: true,
        prompts: [],
      };
    }
    return 1;
  };
  document.querySelector('.study-suggest-card__accept').click();
  await tick();
  await tick();
  const startCalls = calls.filter(c => c.command === 'start_lock_in').slice(beforeStart);
  assert.equal(startCalls.length, 1);
  assert.equal(startCalls[0].args.goals, 'General study session');
  assert.equal(startCalls[0].args.durationMins, 30);
  assert.equal(document.querySelector('.study-suggest-card'), null);

  // End mission so timers/view state do not leak into later tests.
  window.__TAURI_INTERNALS__.invoke = async (command, args) => {
    calls.push({ command, args });
    if (command === 'stop_lock_in') return null;
    return defaultInvoke(command, args);
  };
  document.querySelector('#end-session')?.click();
  document.querySelector('#end-session-confirm')?.click();
  await tick();
  await tick();
  window.__TAURI_INTERNALS__.invoke = defaultInvoke;
  document.querySelector('#view-session')?.classList.remove('active');
  document.querySelector('#view-chat')?.classList.add('active');
  document.querySelector('#new-chat')?.click();
  await tick();
});

test('overload keeps one thinking bubble and silently retries the original message', async () => {
  const input = document.querySelector('#chat-input');
  const before = calls.filter(c => c.command === 'chat_send').length;
  input.value = '  Explain $x$\nplease  ';
  document.querySelector('#chat-form').dispatchEvent(new window.Event('submit', { cancelable: true }));
  rejectReply('Gemini error 503 Service Unavailable: overload');
  await tick();
  assert.match(document.querySelector('.bubble.assistant').textContent, /slight delay/);
  assert.doesNotMatch(document.querySelector('#chat-log').textContent, /503|overload/);
  input.value = 'a new draft';
  await new Promise(resolve => setTimeout(resolve, 2100));
  const attempts = calls.filter(c => c.command === 'chat_send').slice(before);
  assert.equal(attempts.length, 2);
  assert.equal(attempts[0].args.message, 'Explain $x$\nplease');
  assert.equal(attempts[1].args.message, attempts[0].args.message);
  assert.equal(document.querySelectorAll('.bubble.user').length, 1);
  assert.equal(document.querySelectorAll('.bubble.assistant').length, 1);
  resolveReply({ role: 'assistant', content: '$x$ is a variable.' });
  await tick();
  assert.ok(document.querySelector('.bubble.assistant .katex'));
  assert.equal(input.value, 'a new draft');
  assert.equal(document.querySelector('#chat-send').disabled, false);
});

test('lock-in companion renders assistant markdown and keeps the user turn plain', async () => {
  const input = document.querySelector('#session-chat-input');
  const form = document.querySelector('#session-chat-form');
  let resolveCompanion;
  const prev = window.__TAURI_INTERNALS__.invoke;
  window.__TAURI_INTERNALS__.invoke = async (command, args) => {
    if (command === 'companion_send') {
      return new Promise((resolve) => {
        resolveCompanion = resolve;
      });
    }
    return prev(command, args);
  };
  input.value = 'Explain **this**';
  form.dispatchEvent(new window.Event('submit', { cancelable: true }));
  const user = document.querySelector('#session-chat-log .bubble.user');
  assert.match(user.textContent, /Explain \*\*this\*\*/);
  assert.equal(user.querySelector('strong'), null);
  await tick();
  resolveCompanion({ content: '## Photosynthesis\n\nPlants use **light**.' });
  await tick();
  const assistant = [...document.querySelectorAll('#session-chat-log .bubble.assistant')].at(-1);
  assert.equal(assistant.querySelector('h2').textContent, 'Photosynthesis');
  assert.equal(assistant.querySelector('strong').textContent, 'light');
  window.__TAURI_INTERNALS__.invoke = prev;
});

test('tools tab connects each permission separately from account sign-in', async () => {
  const toolsTab = document.querySelector('[data-settings-tab="tools"]');
  const permissionsTab = document.querySelector('[data-settings-tab="permissions"]');
  const accountTab = document.querySelector('[data-settings-tab="account"]');
  assert.equal(toolsTab?.getAttribute('aria-controls'), 'settings-tools');
  assert.ok(
    permissionsTab.compareDocumentPosition(toolsTab) & window.Node.DOCUMENT_POSITION_FOLLOWING,
  );
  assert.ok(
    toolsTab.compareDocumentPosition(accountTab) & window.Node.DOCUMENT_POSITION_FOLLOWING,
  );

  const accountPanel = document.querySelector('#settings-account');
  assert.equal(accountPanel.querySelector('#account-google-status'), null);
  assert.equal(accountPanel.querySelector('#account-google-actions'), null);
  assert.match(accountPanel.textContent, /Waypoint identity/);
  assert.match(accountPanel.textContent, /Settings → Tools/);
  assert.doesNotMatch(accountPanel.textContent, /Calendar & Drive|Link Calendar|Required for deadlines/);
  assert.match(document.querySelector('.welcome-signin-lead').textContent, /optional/);
  assert.doesNotMatch(document.querySelector('.welcome-signin-lead').textContent, /link your account, Calendar/);

  const panel = document.querySelector('#settings-tools');
  assert.equal(panel.hidden, true);
  toolsTab.click();
  await tick();
  await tick();
  assert.equal(panel.hidden, false);
  assert.equal(document.querySelector('#settings-permissions').hidden, true);
  assert.equal(document.querySelector('#settings-account').hidden, true);
  assert.equal(document.querySelector('#settings-lockin').hidden, true);
  assert.match(panel.textContent, /Each tool has its own permission/);
  assert.match(panel.textContent, /could not be loaded/i);
  assert.match(panel.textContent, /Google Calendar/);
  assert.match(panel.textContent, /Google Drive/);
  assert.equal(
    [...panel.querySelectorAll('button')].map((button) => button.textContent).join(','),
    'Try again',
  );

  accountTab.click();
  assert.equal(accountPanel.hidden, false);
  assert.equal(panel.hidden, true);
  document.querySelector('[data-settings-tab="lockin"]').click();
  assert.equal(document.querySelector('#settings-lockin').hidden, false);
  assert.equal(panel.hidden, true);
});

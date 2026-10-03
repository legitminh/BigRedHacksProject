import { test } from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { JSDOM } from 'jsdom';

const dom = new JSDOM(readFileSync(new URL('../index.html', import.meta.url), 'utf8'), { url: 'http://localhost/' });
globalThis.window = dom.window;
globalThis.document = dom.window.document;
const { renderMarkdown } = await import('../src/markdown.ts');
const tick = () => new Promise(resolve => setTimeout(resolve, 0));

let resolveReply;
let rejectReply;
const calls = [];
window.__TAURI_INTERNALS__ = {
  transformCallback: () => 1,
  invoke: async (command, args) => {
    calls.push({ command, args });
    if (command === 'get_status') return { google_connected: true };
    if (command === 'chat_send') return new Promise((resolve, reject) => { resolveReply = resolve; rejectReply = reject; });
    return 1;
  },
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
  document.querySelector('[data-study="quiz"]').click();
  assert.match(input.value, /one question at a time/);
  input.value = '<b>Explain photosynthesis</b>\nWith an example';
  form.dispatchEvent(new window.Event('submit', { cancelable: true }));
  assert.equal(document.querySelector('.bubble.user b'), null);
  assert.equal(document.querySelector('#chat-send').disabled, true);
  input.value = 'next draft';
  form.dispatchEvent(new window.Event('submit', { cancelable: true }));
  assert.equal(calls.filter(c => c.command === 'chat_send').length, 1);
  resolveReply({ role: 'assistant', content: '## Photosynthesis\n\nPlants use **light**.' });
  await tick();
  assert.equal(document.querySelector('.bubble.assistant h2').textContent, 'Photosynthesis');
  assert.equal(input.value, 'next draft');
  assert.equal(document.querySelector('#chat-send').disabled, false);
  input.value = '';
  document.querySelector('[data-study="quiz"]').click();
  assert.match(input.value, /we’re discussing/);
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
  assert.equal(attempts[0].args.message, '  Explain $x$\nplease  ');
  assert.equal(attempts[1].args.message, attempts[0].args.message);
  assert.equal(document.querySelectorAll('.bubble.user').length, 1);
  assert.equal(document.querySelectorAll('.bubble.assistant').length, 1);
  resolveReply({ role: 'assistant', content: '$x$ is a variable.' });
  await tick();
  assert.ok(document.querySelector('.bubble.assistant .katex'));
  assert.equal(input.value, 'a new draft');
  assert.equal(document.querySelector('#chat-send').disabled, false);
});

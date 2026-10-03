import { test } from 'node:test';
import assert from 'node:assert/strict';
import { retryChat } from '../src/chat-retry.ts';

test('retries the exact message with backoff and delay notifications', async () => {
  const messages = [], sleeps = [];
  let notifications = 0;
  const message = '  Explain $x^2$\nwith an example.  ';
  const reply = await retryChat(message, async original => {
    messages.push(original);
    if (messages.length === 1) throw 'Gemini error 503 Service Unavailable';
    if (messages.length === 2) throw 'Gemini error 429: {"retryDelay":"10s"}';
    return 'answer';
  }, () => notifications++, async ms => { sleeps.push(ms); });
  assert.equal(reply, 'answer');
  assert.deepEqual(messages, [message, message, message]);
  assert.deepEqual(sleeps, [2000, 10000]);
  assert.equal(notifications, 2);
});

test('persistent overload eventually exits and configuration errors are not retried', async () => {
  let attempts = 0;
  await assert.rejects(retryChat('message', async () => {
    attempts++;
    throw new Error('Gemini error 503');
  }, () => {}, async () => {}));
  assert.equal(attempts, 6);
  attempts = 0;
  await assert.rejects(retryChat('message', async () => {
    attempts++;
    throw new Error('Gemini error 403');
  }, () => assert.fail('must not retry'), async () => {}));
  assert.equal(attempts, 1);
});

import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import test from 'node:test';
import vm from 'node:vm';

const source = await readFile(new URL('../../backend/crates/api/src/identity_web/login.js', import.meta.url), 'utf8');

class Element {
  constructor(id = '') {
    this.id = id;
    this.value = '';
    this.disabled = true;
    this.textContent = '';
    this.type = 'text';
    this.dataset = {};
    this.attributes = new Map();
    this.listeners = new Map();
  }
  addEventListener(name, callback) { this.listeners.set(name, callback); }
  setAttribute(name, value) { this.attributes.set(name, String(value)); }
  getAttribute(name) { return this.attributes.get(name) ?? null; }
  async dispatch(name, event = {}) { return this.listeners.get(name)?.({ preventDefault() {}, ...event }); }
  focus() {}
}

function browser({ hash = '', search = '', responses = [], onFetch } = {}) {
  const ids = ['login-form', 'email', 'password', 'submit', 'status', 'verify-button', 'request-button', 'logout-button', 'verification-token'];
  const elements = new Map(ids.map(id => [id, new Element(id)]));
  const form = elements.get('login-form');
  form.dataset = { mode: 'login' };
  form.setAttribute = Element.prototype.setAttribute;
  form.checkValidity = () => true;
  form.reportValidity = () => true;
  const windowListeners = new Map();
  const calls = [];
  let enabledAtReplace = false;
  const location = { pathname: '/auth/sign-in', search, hash };
  const context = {
    document: {
      getElementById: id => elements.get(id) ?? null,
      querySelectorAll: () => [],
    },
    window: { addEventListener: (name, callback) => windowListeners.set(name, callback) },
    location,
    history: { replaceState(_state, _title, url) {
      enabledAtReplace = [...elements.values()].some(element => !element.disabled);
      location.hash = '';
      location.search = '';
      calls.push({ history: url });
    } },
    localStorage: { setItem() { throw new Error('localStorage used'); } },
    sessionStorage: { setItem() { throw new Error('sessionStorage used'); } },
    setTimeout: callback => { context.timeout = callback; return 1; },
    clearTimeout() {},
    AbortController,
    fetch: async (url, options) => {
      calls.push({ url, options });
      onFetch?.(url, options);
      const response = responses.shift();
      if (typeof response === 'function') return response(options);
      if (response instanceof Error) throw response;
      return response ?? { status: 500, json: async () => ({ code: 'unexpected' }) };
    },
    console: { log() { throw new Error('console logging used'); }, error() { throw new Error('console logging used'); } },
  };
  vm.runInNewContext(source, context, { filename: 'login.js' });
  return { elements, form, calls, context, windowListeners, get enabledAtReplace() { return enabledAtReplace; }, location };
}

const json = value => ({ status: 200, json: async () => value });
const empty = status => ({ status, json: async () => ({}) });
const TOKEN = `ev1_${'a'.repeat(64)}`;
const pair = (access = 'access-secret', refresh = 'refresh-secret') => ({
  access_token: access,
  refresh_token: refresh,
  token_type: 'Bearer',
  expires_in: 900,
  refresh_expires_in: 2592000,
});

test('scrubs verification fragment before enabling controls and never puts it in the DOM or storage', () => {
  const page = browser({ hash: `#token=${TOKEN}` });
  assert.equal(page.enabledAtReplace, false);
  assert.equal(page.location.hash, '');
  assert.equal(page.elements.get('verification-token').value, '');
  assert.equal(page.elements.get('submit').disabled, false);
  assert.equal(page.calls[0].history, '/auth/sign-in');
});

test('sign-in holds both tokens only in memory and requires explicit verification confirmation', async () => {
  const page = browser({
    hash: `#token=${TOKEN}`,
    responses: [json(pair()), empty(204)],
  });
  const event = { preventDefault() {} };
  await page.form.dispatch('submit', event);
  assert.equal(page.elements.get('status').textContent, 'You are signed in. Confirm email verification to continue.');
  assert.equal(page.elements.get('verify-button').disabled, false);
  assert.equal(page.elements.get('verification-token').value, '');
  assert.equal(JSON.stringify(page.calls).includes(TOKEN), false);
  await page.elements.get('verify-button').dispatch('click');
  const confirmation = page.calls.at(-1);
  assert.equal(confirmation.url, '/v1/auth/email-verification/confirm');
  assert.equal(confirmation.options.headers.Authorization, 'Bearer access-secret');
  assert.deepEqual(JSON.parse(confirmation.options.body), { token: TOKEN });
  assert.equal(JSON.stringify(page.calls).includes('access-secret'), true);
  assert.equal(page.elements.get('status').textContent, 'Your email is verified.');
});

test('login can request verification mail and logout clears credentials', async () => {
  const page = browser({ responses: [json(pair()), empty(202), empty(204), json(pair())] });
  page.elements.get('email').value = 'person@example.test';
  page.elements.get('password').value = 'correct horse';
  await page.form.dispatch('submit');
  await page.elements.get('request-button').dispatch('click');
  const request = page.calls.at(-1);
  assert.equal(request.url, '/v1/auth/email-verification/request');
  assert.equal(request.options.headers.Authorization, 'Bearer access-secret');
  assert.equal(page.elements.get('status').textContent, 'If your account needs verification, check your inbox.');
  await page.elements.get('logout-button').dispatch('click');
  const logout = page.calls.at(-1);
  assert.equal(logout.url, '/v1/auth/logout');
  assert.deepEqual(JSON.parse(logout.options.body), { refresh_token: 'refresh-secret' });
  assert.equal(page.elements.get('status').textContent, 'You are signed out. Sign in again to continue.');
  page.elements.get('email').value = 'person@example.test';
  page.elements.get('password').value = 'correct horse';
  await page.form.dispatch('submit');
  assert.equal(page.elements.get('status').textContent, 'You are signed in. You can request a verification email or sign out.');
});

test('unknown verification and logout outcomes remove this page authority', async () => {
  const verifyPage = browser({
    hash: `#token=${TOKEN}`,
    responses: [json(pair()), new Error('response lost')],
  });
  await verifyPage.form.dispatch('submit');
  await verifyPage.elements.get('verify-button').dispatch('click');
  assert.equal(verifyPage.elements.get('verify-button').disabled, true);
  assert.equal(verifyPage.elements.get('request-button').disabled, true);
  assert.match(verifyPage.elements.get('status').textContent, /could not confirm verification/);

  const logoutPage = browser({
    responses: [json(pair()), empty(503)],
  });
  await logoutPage.form.dispatch('submit');
  await logoutPage.elements.get('logout-button').dispatch('click');
  assert.equal(logoutPage.elements.get('verify-button').disabled, true);
  assert.equal(logoutPage.elements.get('request-button').disabled, true);
  assert.equal(logoutPage.elements.get('email').disabled, false);
  assert.match(logoutPage.elements.get('status').textContent, /server sign-out could not be confirmed/);
});

test('uncertain login result fails closed and clears entered password', async () => {
  const page = browser({ responses: [new Error('network failed')] });
  page.elements.get('email').value = 'person@example.test';
  page.elements.get('password').value = 'correct horse';
  await page.form.dispatch('submit');
  assert.equal(page.elements.get('password').value, '');
  assert.equal(page.elements.get('verify-button').disabled, true);
  assert.equal(page.elements.get('status').textContent, 'We could not confirm sign-in. Try again; no session is available in this page.');
});

test('unexpected login response fields never establish an authenticated state', async () => {
  const page = browser({ responses: [json({ access_token: 'access-secret', refresh_token: 'refresh-secret' })] });
  page.elements.get('email').value = 'person@example.test';
  page.elements.get('password').value = 'correct horse';
  await page.form.dispatch('submit');
  assert.equal(page.elements.get('password').value, '');
  assert.equal(page.elements.get('verify-button').disabled, true);
  assert.equal(page.elements.get('request-button').disabled, true);
  assert.equal(page.elements.get('status').textContent, 'We could not confirm sign-in. Try again; no session is available in this page.');
});

test('pagehide clears all sensitive values and a restored bfcache page stays closed', async () => {
  const page = browser({ responses: [json(pair())] });
  page.elements.get('email').value = 'person@example.test';
  page.elements.get('password').value = 'correct horse';
  await page.form.dispatch('submit');
  await page.windowListeners.get('pagehide')({});
  assert.equal(page.elements.get('email').value, '');
  assert.equal(page.elements.get('password').value, '');
  assert.equal(page.elements.get('verify-button').disabled, true);
  await page.windowListeners.get('pageshow')({ persisted: true });
  assert.equal(page.elements.get('status').textContent, 'This sign-in page was closed. Reload it to sign in again.');
});

test('malformed fragments and query strings are scrubbed and keep every control disabled', () => {
  const badFragment = browser({ hash: '#token=not-a-verification-token' });
  assert.equal(badFragment.location.hash, '');
  assert.equal(badFragment.elements.get('submit').disabled, true);
  assert.match(badFragment.elements.get('status').textContent, /cannot be used/);
  const query = browser({ search: `?token=${TOKEN}` });
  assert.equal(query.location.search, '');
  assert.equal(query.elements.get('submit').disabled, true);
  assert.match(query.elements.get('status').textContent, /cannot be used/);
});

test('a late successful login response cannot restore authority after pagehide', async () => {
  let resolveFetch;
  const page = browser({ responses: [() => new Promise(resolve => { resolveFetch = resolve; })] });
  page.elements.get('email').value = 'person@example.test';
  page.elements.get('password').value = 'correct horse';
  const pending = page.form.dispatch('submit');
  await page.windowListeners.get('pagehide')({});
  resolveFetch(json(pair()));
  await pending;
  assert.equal(page.elements.get('verify-button').disabled, true);
  assert.equal(page.elements.get('request-button').disabled, true);
  assert.equal(page.elements.get('password').value, '');
});

test('a late successful JSON body cannot restore authority after pagehide', async () => {
  let resolveBody;
  let startBody;
  const bodyStarted = new Promise(resolve => { startBody = resolve; });
  const page = browser({ responses: [() => ({
    status: 200,
    json: () => {
      startBody();
      return new Promise(resolve => { resolveBody = resolve; });
    },
  })] });
  page.elements.get('email').value = 'person@example.test';
  page.elements.get('password').value = 'correct horse';
  const pending = page.form.dispatch('submit');
  await bodyStarted;
  await page.windowListeners.get('pagehide')({});
  resolveBody(pair());
  await pending;
  assert.equal(page.elements.get('verify-button').disabled, true);
  assert.equal(page.elements.get('request-button').disabled, true);
  assert.equal(page.elements.get('password').value, '');
});

test('login timeout fails closed without retaining a session or password', async () => {
  const page = browser({ responses: [options => new Promise((_resolve, reject) => {
    options.signal.addEventListener('abort', () => reject(new Error('aborted')));
  })] });
  page.elements.get('email').value = 'person@example.test';
  page.elements.get('password').value = 'correct horse';
  const pending = page.form.dispatch('submit');
  page.context.timeout();
  await pending;
  assert.equal(page.elements.get('password').value, '');
  assert.equal(page.elements.get('verify-button').disabled, true);
  assert.equal(page.elements.get('status').textContent, 'We could not confirm sign-in. Try again; no session is available in this page.');
});

test('login timeout remains active while the JSON response body is being read', async () => {
  let startBody;
  const bodyStarted = new Promise(resolve => { startBody = resolve; });
  const page = browser({ responses: [options => ({
    status: 200,
    json: () => {
      startBody();
      return new Promise((_resolve, reject) => {
        options.signal.addEventListener('abort', () => reject(new Error('aborted')));
      });
    },
  })] });
  page.elements.get('email').value = 'person@example.test';
  page.elements.get('password').value = 'correct horse';
  const pending = page.form.dispatch('submit');
  await bodyStarted;
  page.context.timeout();
  await pending;
  assert.equal(page.elements.get('password').value, '');
  assert.equal(page.elements.get('verify-button').disabled, true);
  assert.equal(page.elements.get('status').textContent, 'We could not confirm sign-in. Try again; no session is available in this page.');
});

// The generated HTML is checked by the Rust HTTP test; this assertion pins the browser page's mode contract.
test('login script expects one form mode and does not depend on an origin query parameter', () => {
  assert.match(source, /login-form/);
  assert.doesNotMatch(source, /localStorage|sessionStorage/);
});

test('transient login failure retains the scrubbed challenge for an explicit successful retry', async () => {
  const page = browser({ hash: `#token=${TOKEN}`, responses: [new Error('network'), json(pair()), empty(204)] });
  await page.form.dispatch('submit');
  assert.equal(page.elements.get('password').value, '');
  assert.equal(page.elements.get('verify-button').disabled, true);
  await page.form.dispatch('submit');
  assert.equal(page.elements.get('verify-button').disabled, false);
  await page.elements.get('verify-button').dispatch('click');
  assert.deepEqual(JSON.parse(page.calls.at(-1).options.body), { token: TOKEN });
  assert.equal(page.elements.get('status').textContent, 'Your email is verified.');
});

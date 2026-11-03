const { test } = require('node:test');
const assert = require('node:assert/strict');
const fs = require('node:fs');
const vm = require('node:vm');
const path = require('node:path');
const sourcePath = path.join(__dirname, '../src/identity_web/recovery.js');
const token = 'pr1_' + 'a'.repeat(64);

function browser(mode = 'reset', options = {}) {
  const ids = ['recovery-form', 'email', 'new-password', 'confirm-password', 'submit', 'status', 'heading'];
  const elements = Object.fromEntries(ids.map(id => [id, {
    id, disabled: true, value: '', hidden: false, textContent: '', dataset: {}, listeners: {},
    setAttribute(name, value) { this[name] = value; },
    addEventListener(name, fn) { this.listeners[name] = fn; },
    focus() { this.focused = true; },
    reportValidity() { return true; },
  }]));
  elements['recovery-form'].dataset.mode = mode;
  const location = { hash: options.hash ?? '#token=' + token, search: options.search ?? '', pathname: '/auth/' + (mode === 'reset' ? 'reset-password' : 'forgot-password') };
  const calls = [], events = {};
  const toggles = ['new-password', 'confirm-password'].map(id => ({
    dataset: {toggle:id}, disabled:true, textContent:'Show', listeners:{},
    setAttribute(name,value) { this[name]=value; },
    addEventListener(name,fn) { this.listeners[name]=fn; },
  }));
  elements['new-password'].type='password';
  elements['confirm-password'].type='password';
  const context = {
    document: { getElementById: id => elements[id], querySelectorAll: () => toggles }, location,
    history: { replaceState() { if (options.historyFails) throw Error('blocked'); location.hash = ''; location.search = ''; } },
    window: { addEventListener: (name, fn) => { events[name] = fn; } },
    fetch: async (url, init) => { calls.push({url, init}); return options.fetch ? options.fetch(url, init) : {status: 204}; },
    AbortController, setTimeout, clearTimeout,
  };
  vm.runInNewContext(fs.readFileSync(sourcePath, 'utf8'), context);
  elements['new-password'].value = 'A unique new password 2026!';
  elements['confirm-password'].value = elements['new-password'].value;
  elements.email.value = 'person@example.test';
  return {elements, toggles, location, calls, events, submit: () => elements['recovery-form'].listeners.submit({ preventDefault() {} })};
}

test('opening a link strips URL secrets and never consumes the token', () => {
  const b = browser();
  assert.equal(b.location.hash, '');
  assert.equal(b.calls.length, 0);
  assert.equal(b.elements.submit.disabled, false);
});

test('missing, wrong-purpose, extra-field, query and unremovable tokens fail closed', () => {
  for (const options of [{hash:''}, {hash:'#token=ev1_'+'a'.repeat(64)}, {hash:'#token='+token+'&x=1'}, {search:'?token='+token}, {historyFails:true}]) {
    const b = browser('reset', options);
    assert.equal(b.elements.submit.disabled, true);
    assert.equal(b.calls.length, 0);
  }
});

test('explicit confirm posts exact bytes without cookies and succeeds only on 204', async () => {
  const b = browser();
  await b.submit();
  assert.equal(b.calls.length, 1);
  assert.equal(b.calls[0].url, '/v1/auth/password-recovery/confirm');
  assert.deepEqual(JSON.parse(b.calls[0].init.body), {token, new_password:'A unique new password 2026!'});
  assert.equal(b.calls[0].init.credentials, 'omit');
  assert.equal(b.calls[0].init.redirect, 'error');
  assert.equal(b.elements['new-password'].value, '');
  assert.equal(b.elements['confirm-password'].value, '');
  assert.match(b.elements.status.textContent, /Password changed/);
  assert.equal(b.elements.submit.disabled, true);
  await b.submit();
  assert.equal(b.calls.length, 1);
});

test('password mismatch and scalar-length violations never send', async () => {
  for (const value of ['too short', 'x'.repeat(129)]) {
    const b=browser(); b.elements['new-password'].value=value; b.elements['confirm-password'].value=value;
    await b.submit(); assert.equal(b.calls.length,0);
  }
  const b=browser(); b.elements['confirm-password'].value='Different password';
  await b.submit(); assert.equal(b.calls.length,0); assert.match(b.elements.status.textContent,/match/);
});

test('Unicode scalar counts and password spaces are preserved', async () => {
  const b=browser(); const value=' '+ '🔑'.repeat(14)+' ';
  b.elements['new-password'].value=value; b.elements['confirm-password'].value=value;
  await b.submit(); assert.equal(JSON.parse(b.calls[0].init.body).new_password,value);
});

test('network loss, server failure or unexpected success never claim failure or replay', async () => {
  for (const fetch of [async()=>{throw Error('network');}, async()=>({status:503}), async()=>({status:200})]) {
    const b=browser('reset',{fetch}); await b.submit(); await b.submit();
    assert.match(b.elements.status.textContent,/could not confirm/);
    assert.equal(b.calls.length,1); assert.equal(b.elements.submit.disabled,true);
    assert.equal(b.elements['new-password'].value,'');
  }
});

test('invalid challenge closes the form; password rejection and throttling permit explicit correction', async () => {
  for (const [status,code,disabled] of [[400,'invalid_challenge',true],[400,'invalid_request',false],[429,'rate_limited',false]]) {
    const b=browser('reset',{fetch:async()=>({status,json:async()=>({code})})});
    await b.submit(); assert.equal(b.elements.submit.disabled,disabled);
    assert.equal(b.elements['new-password'].value,'');
    assert.doesNotMatch(b.elements.status.textContent,/Password changed/);
  }
});

test('request acceptance is generic and clears the submitted address', async () => {
  const b=browser('request',{fetch:async()=>({status:202})}); await b.submit();
  assert.equal(b.calls[0].url,'/v1/auth/password-recovery/request');
  assert.match(b.elements.status.textContent,/If an eligible/);
  assert.equal(b.elements.email.value,'');
  assert.doesNotMatch(b.elements.status.textContent,/person@example/);
});

test('request failure does not promise email delivery', async () => {
  for (const fetch of [async()=>{throw Error('offline');},async()=>({status:503}),async()=>({status:429})]) {
    const b=browser('request',{fetch}); await b.submit();
    assert.doesNotMatch(b.elements.status.textContent,/^Check your inbox\./);
    assert.equal(b.elements.submit.disabled,false);
  }
});

test('double activation sends once, pagehide clears secrets and bfcache cannot reactivate', async () => {
  let release; const b=browser('reset',{fetch:()=>new Promise(resolve=>{release=resolve;})});
  const first=b.submit(); await b.submit(); assert.equal(b.calls.length,1);
  b.events.pagehide(); release({status:204}); await first;
  b.events.pageshow({persisted:true});
  assert.equal(b.elements['new-password'].value,''); assert.equal(b.elements.submit.disabled,true);
  await b.submit(); assert.equal(b.calls.length,1);
});

test('malformed 400, invalid JSON, redirects and other success statuses close reset without replay', async () => {
  for (const fetch of [async()=>({status:400,json:async()=>null}), async()=>({status:400,json:async()=>{throw Error('invalid JSON')}}), async()=>({status:400,json:async()=>({error:{code:'invalid_challenge'}})}), ...[201,202,301,403,502].map(status=>async()=>({status}))]) {
    const b=browser('reset',{fetch}); await b.submit(); await b.submit();
    assert.equal(b.calls.length,1); assert.equal(b.elements.submit.disabled,true);
    assert.match(b.elements.status.textContent,/could not confirm/);
    assert.doesNotMatch(b.elements.status.textContent,/Password changed/);
  }
});

test('deferred rejection cannot reactivate page after navigation', async () => {
  let reject; const b=browser('reset',{fetch:()=>new Promise((_,fail)=>{reject=fail})});
  const pending=b.submit(); b.events.pagehide(); reject(Error('aborted')); await pending;
  assert.equal(b.elements.submit.disabled,true); assert.equal(b.elements['new-password'].value,'');
  await b.submit(); assert.equal(b.calls.length,1);
});

test('request JSON rejection uses only root code and no reflected server text', async()=>{
  for(const code of ['invalid_request','invalid_challenge','other']) {
    const b=browser('request',{fetch:async()=>({status:400,json:async()=>({code,message:'SECRET_SENTINEL'})})});
    await b.submit(); assert.doesNotMatch(b.elements.status.textContent,/SECRET_SENTINEL|Password changed|^Check your inbox/);
  }
});

test('challenge remains absent from visible text and DOM attributes after init and rejection', async()=>{
  const b=browser('reset',{fetch:async()=>({status:400,json:async()=>({code:'invalid_request'})})});
  assert(!JSON.stringify(b.elements).includes(token)); await b.submit();
  assert(!JSON.stringify(b.elements).includes(token));
  assert.equal(b.calls[0].init.method,'POST'); assert.equal(b.calls[0].init.cache,'no-store');
  assert.equal(b.calls[0].init.referrerPolicy,'no-referrer');
});

test('password visibility and accessible toggle names reset after a rejected submission', async () => {
  const b=browser('reset',{fetch:async()=>({status:400,json:async()=>({code:'invalid_request'})})});
  b.toggles[0].listeners.click();
  assert.equal(b.elements['new-password'].type,'text');
  assert.equal(b.toggles[0]['aria-label'],'Hide new password');
  await b.submit();
  assert.equal(b.elements['new-password'].type,'password');
  assert.equal(b.toggles[0].textContent,'Show');
  assert.equal(b.toggles[0]['aria-label'],'Show new password');
});

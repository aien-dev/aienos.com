'use strict';
/* Behaviour test for public/js/allen-voice.js in a scripted DOM (node vm).
 * Fakes stand in for SpeechRecognition, speechSynthesis, storage and the
 * window.allen transport, so no microphone permission is ever requested or
 * granted: getUserMedia and permissions.query throw if anything calls them.
 * This is a unit harness, not a browser or device claim.
 * Usage: node test-voice.cjs path/to/allen-voice.js */
const fs = require('node:fs');
const vm = require('node:vm');
const assert = require('node:assert/strict');

const file = process.argv[2] || `${__dirname}/../../public/js/allen-voice.js`;
const code = fs.readFileSync(file, 'utf8');
const COPY_OF = key => {
  const match = new RegExp(`\\n    ${key}: "([^"]*)",`).exec(code);
  assert.ok(match, `copy ${key} present`);
  return match[1];
};

class Event {
  constructor(type, init) {
    this.type = type;
    this.bubbles = !!(init && init.bubbles);
    this.defaultPrevented = false;
    this.stopped = false;
  }
  preventDefault() {
    this.defaultPrevented = true;
  }
  stopPropagation() {
    this.stopped = true;
  }
}
class Node {
  constructor() {
    this.listeners = new Map();
  }
  addEventListener(type, fn) {
    if (!this.listeners.has(type)) this.listeners.set(type, []);
    this.listeners.get(type).push(fn);
  }
  removeEventListener(type, fn) {
    const list = this.listeners.get(type) || [];
    const at = list.indexOf(fn);
    if (at >= 0) list.splice(at, 1);
  }
  dispatchEvent(event) {
    event.target = event.target || this;
    for (const fn of [...(this.listeners.get(event.type) || [])]) fn(event);
    return !event.defaultPrevented;
  }
}
class Element extends Node {
  constructor(tag, id) {
    super();
    this.tagName = tag.toUpperCase();
    this.id = id || '';
    this.attrs = new Map();
    this.dataset = {};
    this.textContent = '';
    this.title = '';
    this.disabled = false;
    this.children = [];
    this.parentNode = null;
    this.nextSibling = null;
  }
  setAttribute(name, value) {
    this.attrs.set(name, String(value));
  }
  getAttribute(name) {
    return this.attrs.has(name) ? this.attrs.get(name) : null;
  }
  insertBefore(child, before) {
    child.parentNode = this;
    const at = before ? this.children.indexOf(before) : -1;
    if (at < 0) this.children.push(child);
    else this.children.splice(at, 0, child);
    return child;
  }
  click() {
    const event = new Event('click');
    this.dispatchEvent(event);
    if (this.tagName === 'INPUT' && this.type === 'checkbox' && !this.disabled) {
      this.checked = !this.checked;
      this.dispatchEvent(new Event('change'));
    }
  }
}
class Field extends Element {
  constructor(id) {
    super('textarea', id);
    this.value = '';
    this.maxLength = -1;
    this.caret = null;
  }
  setSelectionRange(a, b) {
    this.caret = [a, b];
  }
}

function result(text, final) {
  return Object.assign([{ transcript: text, confidence: 0.9 }], { isFinal: final });
}
function results(...list) {
  return { results: list };
}
function tick() {
  return new Promise(done => setImmediate(done));
}

/* One page load. opts: recognition 'none'|'webkit'|'standard', available
   (string or undefined: no static available), secure, synth, voices,
   stored, toggle 'button'|'checkbox'|'none', mic false, allen false. */
function load(opts = {}) {
  const ids = new Map();
  const page = new Element('div', 'allen');
  const add = el => {
    ids.set(el.id, el);
    page.insertBefore(el, null);
    return el;
  };
  const input = add(new Field('allen-input'));
  const form = add(new Element('form', 'allen-form'));
  form.submits = 0;
  form.submit = () => form.submits++;
  const mic = opts.mic === false ? null : add(new Element('button', 'allen-mic'));
  const privacy = add(new Element('p', 'allen-voice-privacy'));
  let toggle = null;
  if (opts.toggle !== 'none') {
    toggle = new Element(opts.toggle === 'checkbox' ? 'input' : 'button', 'allen-speak');
    if (opts.toggle === 'checkbox') {
      toggle.type = 'checkbox';
      toggle.checked = false;
    }
    add(toggle);
  }
  const document = new Node();
  document.hidden = false;
  document.documentElement = { lang: opts.lang || 'en-US' };
  document.getElementById = id => ids.get(id) || null;
  document.createElement = tag => new Element(tag);
  const guard = { media: 0, permission: 0 };
  const store = new Map(opts.stored ? [['allen-voice-speak', opts.stored]] : []);
  const writes = [];
  const sandbox = {
    document,
    Event,
    console,
    isSecureContext: opts.secure !== false,
    navigator: {
      language: 'en-US',
      mediaDevices: { getUserMedia: () => { guard.media++; throw new Error('getUserMedia called'); } },
      permissions: { query: () => { guard.permission++; throw new Error('permissions.query called'); } },
    },
    localStorage: {
      getItem: key => (store.has(key) ? store.get(key) : null),
      setItem: (key, value) => { writes.push([key, String(value)]); store.set(key, String(value)); },
      removeItem: key => { writes.push([key, null]); store.delete(key); },
    },
  };
  const win = new Node();
  Object.assign(win, sandbox);
  win.window = win;
  const rec = { instances: [], availableCalls: [] };
  if (opts.recognition && opts.recognition !== 'none') {
    class Recognition {
      constructor() {
        this.calls = [];
        rec.instances.push(this);
      }
      start() {
        this.calls.push('start');
        if (opts.startThrows) throw new Error('InvalidStateError');
      }
      stop() {
        this.calls.push('stop');
      }
      abort() {
        this.calls.push('abort');
      }
    }
    if (opts.recognition === 'standard') {
      Recognition.prototype.processLocally = false;
      if (opts.available !== undefined) {
        Recognition.available = async options => {
          rec.availableCalls.push(options);
          return opts.available;
        };
      }
      win.SpeechRecognition = Recognition;
    } else {
      win.webkitSpeechRecognition = Recognition;
    }
  }
  const synth = { spoken: [], cancels: 0 };
  if (opts.synth !== false) {
    synth.speak = u => synth.spoken.push(u);
    synth.cancel = () => synth.cancels++;
    synth.getVoices = () => opts.voices || [];
    win.speechSynthesis = synth;
    win.SpeechSynthesisUtterance = class {
      constructor(text) {
        this.text = text;
      }
    };
  }
  const transport = { handlers: new Map(), sends: 0 };
  const makeAllen = () => Object.freeze({
    version: 1,
    on: (type, fn) => {
      if (!transport.handlers.has(type)) transport.handlers.set(type, []);
      transport.handlers.get(type).push(fn);
    },
    send: () => transport.sends++,
    regenerate: () => false,
    stop: () => {},
  });
  if (opts.allen !== false) win.allen = makeAllen();
  const emit = (type, detail) => {
    for (const fn of transport.handlers.get(type) || []) fn(Object.freeze({ ...detail }));
  };
  let inputs = 0;
  input.addEventListener('input', () => inputs++);
  const context = vm.createContext(win);
  vm.runInContext(code, context);
  const status = ids.get('allen-voice-status') || page.children.find(el => el.id === 'allen-voice-status');
  return {
    win, document, input, form, mic, privacy, toggle, status, rec, synth, transport, guard, store, writes, emit, makeAllen,
    inputs: () => inputs,
    last: () => rec.instances[rec.instances.length - 1],
    escape() {
      const event = new Event('keydown');
      event.key = 'Escape';
      document.dispatchEvent(event);
      return event;
    },
    type(value) {
      input.value = value;
      input.dispatchEvent(new Event('input', { bubbles: true }));
    },
  };
}

const C = {};
for (const key of ['cloud', 'local', 'none', 'insecure', 'start', 'stop', 'starting', 'listening', 'done', 'nothing', 'cancelled', 'blocked', 'nomic', 'network', 'lang', 'failed', 'fallback', 'hidden', 'typed', 'noSpeak', 'speakOn', 'speakOff', 'remote', 'drake', 'code', 'link', 'rest']) C[key] = COPY_OF(key);

const tests = [];
const test = (name, fn) => tests.push([name, fn]);

test('copy: no em or en dash anywhere, cloud note names Google, on-device note only claims on device', () => {
  assert.ok(!/[\u2013\u2014]/.test(code), 'no em/en dash in the script');
  assert.match(C.cloud, /Google/);
  assert.match(C.cloud, /online/);
  assert.doesNotMatch(C.cloud, /on this device|on-device/i);
  assert.match(C.local, /on this device/);
  assert.match(C.drake, /^Drake/);
});

test('unsupported browser: clear unavailable text, mic disabled, a tap does nothing', async () => {
  const env = load({ recognition: 'none' });
  await tick();
  assert.equal(env.privacy.textContent, C.none);
  assert.equal(env.status.textContent, C.none);
  assert.equal(env.mic.disabled, true);
  assert.equal(env.mic.getAttribute('aria-disabled'), 'true');
  env.mic.click();
  assert.equal(env.rec.instances.length, 0);
  assert.equal(env.win.AllenVoice.mode(), 'none');
});

test('insecure page: recognition present but unusable, says so', async () => {
  const env = load({ recognition: 'webkit', secure: false });
  await tick();
  assert.equal(env.privacy.textContent, C.insecure);
  assert.equal(env.mic.disabled, true);
  env.mic.click();
  assert.equal(env.rec.instances.length, 0);
});

test('online recognition: visible cloud disclosure before any tap, no permission or recognizer at load', async () => {
  const env = load({ recognition: 'webkit' });
  await tick();
  assert.equal(env.privacy.textContent, C.cloud);
  assert.equal(env.win.AllenVoice.mode(), 'cloud');
  assert.equal(env.rec.instances.length, 0, 'no recognizer before the tap');
  assert.deepEqual(env.guard, { media: 0, permission: 0 });
  assert.equal(env.mic.disabled, false);
  assert.equal(env.mic.getAttribute('aria-pressed'), 'false');
  assert.equal(env.mic.getAttribute('aria-label'), C.start);
  assert.equal(env.mic.getAttribute('aria-describedby'), 'allen-voice-privacy allen-voice-status');
  assert.equal(env.status.getAttribute('role'), 'status');
  assert.equal(env.status.getAttribute('aria-live'), 'polite');
});

test('installed on-device recognizer: local disclosure and processLocally on the tap', async () => {
  const env = load({ recognition: 'standard', available: 'available' });
  assert.equal(env.privacy.textContent, C.cloud, 'online until the browser confirms');
  await tick();
  assert.deepEqual(JSON.parse(JSON.stringify(env.rec.availableCalls)), [{ langs: ['en-US'], processLocally: true }]);
  assert.equal(env.privacy.textContent, C.local);
  assert.equal(env.rec.instances.length, 0);
  env.mic.click();
  assert.equal(env.last().processLocally, true);
  assert.deepEqual(env.guard, { media: 0, permission: 0 });
});

test('downloadable or missing on-device pack, or no available(): stays online, never claims on device', async () => {
  for (const available of ['downloadable', 'downloading', 'unavailable', undefined]) {
    const env = load({ recognition: 'standard', available });
    await tick();
    assert.equal(env.privacy.textContent, C.cloud, String(available));
    env.mic.click();
    assert.notEqual(env.last().processLocally, true, String(available));
  }
});

test('tap starts, results fill the editable box with input events, tap stops, nothing is sent', async () => {
  const env = load({ recognition: 'webkit' });
  env.input.value = 'Hi';
  env.mic.click();
  const r = env.last();
  assert.deepEqual(r.calls, ['start']);
  assert.equal(r.lang, 'en-US');
  assert.equal(r.interimResults, true);
  assert.equal(env.win.AllenVoice.state(), 'starting');
  assert.equal(env.status.textContent, C.starting);
  assert.equal(env.mic.getAttribute('aria-pressed'), 'true');
  assert.equal(env.mic.getAttribute('aria-label'), C.stop);
  r.onaudiostart();
  assert.equal(env.win.AllenVoice.state(), 'listening');
  assert.equal(env.status.textContent, C.listening);
  r.onresult(results(result('what is', false)));
  assert.equal(env.input.value, 'Hi what is');
  assert.equal(env.inputs(), 1);
  r.onresult(results(result('what is Omega', true)));
  assert.equal(env.input.value, 'Hi what is Omega');
  assert.equal(env.inputs(), 2);
  assert.deepEqual(env.input.caret, [16, 16]);
  env.mic.click();
  assert.deepEqual(r.calls, ['start', 'stop']);
  assert.equal(env.win.AllenVoice.state(), 'stopping');
  r.onend();
  assert.equal(env.win.AllenVoice.state(), 'idle');
  assert.equal(env.status.textContent, C.done);
  assert.equal(env.input.value, 'Hi what is Omega');
  assert.equal(env.mic.getAttribute('aria-pressed'), 'false');
  assert.equal(env.transport.sends, 0, 'never sends');
  assert.equal(env.form.submits, 0, 'never submits');
  assert.deepEqual(env.guard, { media: 0, permission: 0 });
  assert.equal(env.writes.length, 0, 'no transcript stored');
});

test('Escape cancels: recognizer aborted, the box restored, Escape consumed', async () => {
  const env = load({ recognition: 'webkit' });
  env.input.value = 'x';
  env.mic.click();
  const r = env.last();
  r.onresult(results(result('hello', false)));
  assert.equal(env.input.value, 'x hello');
  const key = env.escape();
  assert.equal(key.defaultPrevented, true);
  assert.deepEqual(r.calls, ['start', 'abort']);
  r.onerror({ error: 'aborted' });
  r.onresult(results(result('hello there', true)));
  r.onend();
  assert.equal(env.input.value, 'x');
  assert.equal(env.status.textContent, C.cancelled);
  assert.equal(env.transport.sends, 0);
  const idle = env.escape();
  assert.equal(idle.defaultPrevented, false, 'Escape is left alone when not listening');
});

test('errors are announced and leave the mic ready', async () => {
  const cases = [['not-allowed', C.blocked], ['service-not-allowed', C.blocked], ['audio-capture', C.nomic], ['network', C.network], ['no-speech', C.nothing], ['language-not-supported', C.lang], ['bad-grammar', C.failed]];
  for (const [error, said] of cases) {
    const env = load({ recognition: 'webkit' });
    env.mic.click();
    const r = env.last();
    r.onerror({ error });
    r.onend();
    assert.equal(env.status.textContent, said, error);
    assert.equal(env.win.AllenVoice.state(), 'idle', error);
    assert.equal(env.mic.getAttribute('aria-pressed'), 'false', error);
    env.mic.click();
    assert.equal(env.rec.instances.length, 2, `${error}: a second tap starts again`);
  }
  const env = load({ recognition: 'webkit', startThrows: true });
  env.mic.click();
  assert.equal(env.status.textContent, C.failed);
  assert.equal(env.win.AllenVoice.state(), 'idle');
});

test('ending with no words says so', async () => {
  const env = load({ recognition: 'webkit' });
  env.mic.click();
  env.last().onend();
  assert.equal(env.status.textContent, C.nothing);
});

test('on-device refusal falls back to the online note and waits for another tap', async () => {
  const env = load({ recognition: 'standard', available: 'available' });
  await tick();
  env.mic.click();
  const r = env.last();
  r.onerror({ error: 'language-not-supported' });
  r.onend();
  assert.equal(env.win.AllenVoice.mode(), 'cloud');
  assert.equal(env.privacy.textContent, C.cloud);
  assert.equal(env.status.textContent, C.fallback);
  assert.equal(env.rec.instances.length, 1, 'no automatic online retry');
  env.mic.click();
  assert.notEqual(env.last().processLocally, true);
});

test('typing while listening hands the box back to the visitor', async () => {
  const env = load({ recognition: 'webkit' });
  env.mic.click();
  const r = env.last();
  r.onresult(results(result('one', false)));
  env.type('my own words');
  assert.deepEqual(r.calls, ['start', 'abort']);
  r.onresult(results(result('one two', true)));
  r.onend();
  assert.equal(env.input.value, 'my own words');
  assert.equal(env.status.textContent, C.typed);
});

test('the box limit is respected', async () => {
  const env = load({ recognition: 'webkit' });
  env.input.maxLength = 10;
  env.mic.click();
  env.last().onresult(results(result('abcdefghijklmnop', true)));
  assert.equal(env.input.value, 'abcdefghij');
});

test('hiding the page stops listening and keeps the words', async () => {
  const env = load({ recognition: 'webkit' });
  env.mic.click();
  const r = env.last();
  r.onresult(results(result('keep me', true)));
  env.document.hidden = true;
  env.document.dispatchEvent(new Event('visibilitychange'));
  assert.deepEqual(r.calls, ['start', 'abort']);
  r.onend();
  assert.equal(env.input.value, 'keep me');
  assert.equal(env.status.textContent, C.hidden);
});

const LOCAL = { lang: 'en-US', localService: true, name: 'Local' };
const REMOTE = { lang: 'en-US', localService: false, name: 'Online' };

test('replies read aloud: off by default, nothing spoken, nothing stored', async () => {
  const env = load({ recognition: 'webkit', voices: [LOCAL] });
  assert.equal(env.toggle.getAttribute('aria-pressed'), 'false');
  assert.equal(env.win.AllenVoice.speaking(), false);
  env.emit('send', { text: 'hi', silent: false });
  env.emit('reply', { text: 'Hello there.', outcome: '' });
  assert.equal(env.synth.spoken.length, 0);
  assert.equal(env.writes.length, 0);
});

test('turned on: reads replies with an on-device voice, Drake labelled, stops on send, hide and off', async () => {
  const env = load({ recognition: 'webkit', voices: [REMOTE, { lang: 'en-GB', localService: true }, LOCAL] });
  env.toggle.click();
  assert.equal(env.toggle.getAttribute('aria-pressed'), 'true');
  assert.deepEqual(env.writes, [['allen-voice-speak', 'on']]);
  assert.equal(env.status.textContent, C.speakOn);
  assert.equal(env.synth.spoken.length, 1, 'one silent unlock inside the tap');
  assert.equal(env.synth.spoken[0].text, '');
  assert.equal(env.synth.spoken[0].volume, 0);
  env.synth.spoken.length = 0;
  env.emit('reply', { text: 'Hello **there**. See https://example.org/a\n```js\nsecret()\n```\nBye.', outcome: '' });
  const said = env.synth.spoken.map(u => u.text).join(' ');
  assert.match(said, /^Hello there\. See link/);
  assert.ok(said.includes(C.code));
  assert.ok(!said.includes('secret'));
  assert.ok(env.synth.spoken.every(u => u.voice === LOCAL && u.lang === 'en-US'));
  const before = env.synth.cancels;
  env.emit('send', { text: 'next', silent: false });
  assert.equal(env.synth.cancels, before + 1, 'a new request stops speech');
  env.synth.spoken.length = 0;
  env.emit('human', { text: 'I am here.' });
  assert.equal(env.synth.spoken[0].text, `${C.drake} I am here.`);
  env.synth.spoken.length = 0;
  env.emit('reply', { text: 'Partial', outcome: 'Stopped.' });
  assert.equal(env.synth.spoken.length, 0, 'a stopped reply is not read');
  const hidden = env.synth.cancels;
  env.document.hidden = true;
  env.document.dispatchEvent(new Event('visibilitychange'));
  assert.equal(env.synth.cancels, hidden + 1);
  env.emit('reply', { text: 'Not while hidden.', outcome: '' });
  assert.equal(env.synth.spoken.length, 0);
  env.document.hidden = false;
  const off = env.synth.cancels;
  env.toggle.click();
  assert.equal(env.synth.cancels, off + 1);
  assert.equal(env.status.textContent, C.speakOff);
  assert.deepEqual(env.writes, [['allen-voice-speak', 'on'], ['allen-voice-speak', null]]);
  assert.ok(env.writes.every(([key, value]) => key === 'allen-voice-speak' && (value === 'on' || value === null)), 'only the choice is stored');
});

test('long replies are cut into short utterances and capped', async () => {
  const env = load({ recognition: 'webkit', voices: [LOCAL], stored: 'on' });
  const long = 'This sentence is about forty characters. '.repeat(200) + 'x'.repeat(500);
  env.emit('reply', { text: long, outcome: '' });
  assert.ok(env.synth.spoken.length > 1);
  assert.ok(env.synth.spoken.every(u => u.text.length > 0 && u.text.length <= 200));
  const said = env.synth.spoken.map(u => u.text).join(' ');
  assert.ok(said.endsWith(C.rest));
  assert.ok(said.length <= 4000 + C.rest.length + env.synth.spoken.length + 1);
});

test('a stored choice restores; a checkbox toggle works; only an online voice is disclosed', async () => {
  const restored = load({ recognition: 'webkit', voices: [LOCAL], stored: 'on' });
  assert.equal(restored.toggle.getAttribute('aria-pressed'), 'true');
  restored.emit('reply', { text: 'Back again.', outcome: '' });
  assert.equal(restored.synth.spoken.length, 1);
  const box = load({ recognition: 'webkit', voices: [REMOTE], toggle: 'checkbox' });
  assert.equal(box.toggle.checked, false);
  box.toggle.click();
  assert.equal(box.toggle.checked, true);
  assert.equal(box.status.textContent, `${C.speakOn} ${C.remote}`);
  box.emit('reply', { text: 'Online voice.', outcome: '' });
  assert.equal(box.synth.spoken[box.synth.spoken.length - 1].voice, undefined, 'no online voice picked by this file');
});

test('no speech synthesis: the toggle is disabled with a reason', async () => {
  const env = load({ recognition: 'webkit', synth: false, stored: 'on' });
  assert.equal(env.toggle.disabled, true);
  assert.equal(env.toggle.title, C.noSpeak);
  assert.equal(env.toggle.getAttribute('aria-pressed'), 'false');
  env.toggle.click();
  assert.equal(env.win.AllenVoice.speaking(), false);
});

test('a mic tap silences speech so the recognizer never hears it', async () => {
  const env = load({ recognition: 'webkit', voices: [LOCAL], stored: 'on' });
  const before = env.synth.cancels;
  env.mic.click();
  assert.equal(env.synth.cancels, before + 1);
});

test('a transport that arrives later is hooked once, on allen-ready', async () => {
  const env = load({ recognition: 'webkit', voices: [LOCAL], stored: 'on', allen: false });
  env.win.allen = env.makeAllen();
  env.document.dispatchEvent(new Event('allen-ready'));
  env.document.dispatchEvent(new Event('allen-ready'));
  assert.equal(env.transport.handlers.get('reply').length, 1);
  env.emit('reply', { text: 'Late hello.', outcome: '' });
  assert.equal(env.synth.spoken.length, 1);
});

test('a page without the mic still reads aloud; a page without the box does nothing', async () => {
  const env = load({ recognition: 'webkit', mic: false, voices: [LOCAL], stored: 'on' });
  env.emit('reply', { text: 'Still here.', outcome: '' });
  assert.equal(env.synth.spoken.length, 1);
  assert.equal(env.rec.instances.length, 0);
});

(async () => {
  let failed = 0;
  for (const [name, fn] of tests) {
    try {
      await fn();
      console.log(`ok   ${name}`);
    } catch (error) {
      failed += 1;
      console.log(`FAIL ${name}\n     ${error && error.stack ? error.stack.split('\n').slice(0, 3).join('\n     ') : error}`);
    }
  }
  console.log(`${failed ? 'VOICE TEST FAIL' : 'VOICE TEST PASS'} tests=${tests.length} pass=${tests.length - failed} fail=${failed}`);
  process.exit(failed ? 1 : 0);
})();

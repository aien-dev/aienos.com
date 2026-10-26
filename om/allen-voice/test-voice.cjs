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
  /* Allen's server: fetch, getUserMedia, MediaRecorder, AudioContext, timers
     and the clock, all scripted. opts.server = { presence, opus, audio,
     decoded, media, respond, hold }. */
  const srv = { fetches: [], gum: 0, recorders: [], streams: [], timers: [], clock: 0, held: [] };
  if (opts.server) {
    const S = opts.server;
    win.Blob = Blob;
    win.AbortController = AbortController;
    win.performance = { now: () => srv.clock };
    win.setTimeout = (fn, ms) => {
      srv.timers.push({ fn, ms, live: true });
      return srv.timers.length;
    };
    win.clearTimeout = id => {
      if (srv.timers[id - 1]) srv.timers[id - 1].live = false;
    };
    win.fetch = (url, init = {}) => {
      const call = { url, init };
      srv.fetches.push(call);
      if (url === '/allen/api/presence') {
        if (S.presence === 'throw') return Promise.reject(new TypeError('offline'));
        if (S.presence === 'error') return Promise.resolve({ ok: false, status: 500, json: async () => ({}) });
        return Promise.resolve({ ok: true, status: 200, json: async () => S.presence });
      }
      return new Promise((resolve, reject) => {
        const answer = () => {
          const out = S.respond ? S.respond(call) : { status: 200, body: { text: 'hello world', ms: 900, seconds: 2, model: 'whisper' } };
          if (out === 'throw') {
            reject(new TypeError('network'));
            return;
          }
          resolve({ ok: out.status >= 200 && out.status < 300, status: out.status, headers: { get: name => (out.headers || {})[name] || null }, json: async () => {
            if (out.html) throw new SyntaxError('Unexpected token < in JSON');
            return out.body;
          } });
        };
        if (init.signal) init.signal.addEventListener('abort', () => reject(new Error('AbortError')));
        if (S.hold) srv.held.push(answer);
        else answer();
      });
    };
    win.navigator.mediaDevices = {
      getUserMedia: constraints => {
        srv.gum += 1;
        srv.constraints = constraints;
        if (S.media && S.media !== 'ok') {
          const error = new Error(S.media);
          error.name = S.media;
          return Promise.reject(error);
        }
        const track = { stopped: false, stop() { this.stopped = true; } };
        const stream = { tracks: [track], getTracks() { return this.tracks; } };
        srv.streams.push(stream);
        return Promise.resolve(stream);
      },
    };
    if (S.recorder !== false) {
      win.MediaRecorder = class {
        static isTypeSupported(type) {
          return (S.opus || []).includes(type);
        }
        constructor(stream, options) {
          this.stream = stream;
          this.options = options;
          this.state = 'inactive';
          this.chunk = null;
          srv.recorders.push(this);
        }
        start() {
          this.state = 'recording';
        }
        stop() {
          if (this.state === 'inactive') throw new Error('InvalidStateError');
          this.state = 'inactive';
          const type = this.options ? this.options.mimeType : 'audio/mp4';
          this.ondataavailable({ data: this.chunk || new Blob([new Uint8Array(64)], { type }) });
          this.onstop();
        }
      };
    }
    if (S.audio) {
      win.AudioContext = class {
        constructor() {
          srv.contexts = (srv.contexts || 0) + 1;
        }
        decodeAudioData(data, ok, bad) {
          srv.decodedBytes = data.byteLength;
          const d = S.decoded;
          const buffer = d && { sampleRate: d.rate, numberOfChannels: d.channels.length, length: d.channels[0].length, getChannelData: ch => d.channels[ch] };
          if (S.audio === 'callback') {
            setImmediate(() => (buffer ? ok(buffer) : bad(new Error('EncodingError'))));
            return undefined;
          }
          return buffer ? Promise.resolve(buffer) : Promise.reject(new Error('EncodingError'));
        }
        close() {
          srv.closed = (srv.closed || 0) + 1;
          return Promise.resolve();
        }
      };
    }
  }
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
    win, document, input, form, mic, privacy, toggle, status, rec, synth, transport, guard, store, writes, emit, makeAllen, srv, page,
    offer: () => page.children.find(el => el.id === 'allen-voice-browser') || null,
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
for (const key of ['cloud', 'local', 'none', 'insecure', 'start', 'stop', 'starting', 'listening', 'done', 'nothing', 'cancelled', 'blocked', 'nomic', 'network', 'lang', 'failed', 'fallback', 'hidden', 'typed', 'noSpeak', 'speakOn', 'speakOff', 'remote', 'drake', 'code', 'link', 'rest', 'server', 'recording', 'sending', 'short', 'toolong', 'format', 'unclear', 'rate', 'retry', 'busy', 'offer', 'switched', 'serverdown']) C[key] = COPY_OF(key);

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

test('pronunciation: whole-word Aien is spoken as Ian; the screen and the Drake label keep their text', async () => {
  const env = load({ recognition: 'webkit', voices: [LOCAL] });
  env.toggle.click();
  assert.equal(env.status.textContent, C.speakOn);
  assert.match(env.status.textContent, /^Aien's replies/, 'displayed copy keeps Aien');
  env.synth.spoken.length = 0;
  const detail = { text: "Hi, I'm **Aien**. Aiendale, AIEN and aien stay; Aien's name is Aien.", outcome: '' };
  env.emit('reply', detail);
  const said = env.synth.spoken.map(u => u.text).join(' ');
  assert.equal(said, "Hi, I'm Ian. Aiendale, AIEN and aien stay; Ian's name is Ian.");
  assert.equal(detail.text, "Hi, I'm **Aien**. Aiendale, AIEN and aien stay; Aien's name is Aien.", 'the reply text itself is untouched');
  env.synth.spoken.length = 0;
  env.emit('human', { text: 'Aien is right.' });
  assert.equal(env.synth.spoken.map(u => u.text).join(' '), `${C.drake} Ian is right.`);
  assert.equal(C.drake, 'Drake wrote:');
  for (const el of env.page.children) assert.doesNotMatch(String(el.textContent), /\bIan\b/, `${el.id} shows no alias`);
});

/* Allen's server path. */
async function settle() {
  for (let i = 0; i < 12; i += 1) await tick();
}
async function serverEnv(server = {}, extra = {}) {
  const env = load({ recognition: 'webkit', voices: [LOCAL], ...extra, server: { presence: { online: false, hours: '', handoff: false, transcribe: true }, opus: ['audio/webm;codecs=opus'], ...server } });
  await settle();
  return env;
}
async function record(env, ms, chunk) {
  env.mic.click();
  await settle();
  env.srv.clock += ms;
  const recorder = env.srv.recorders[env.srv.recorders.length - 1];
  if (recorder && chunk) recorder.chunk = chunk;
  env.mic.click();
  await settle();
  return recorder;
}
const posts = env => env.srv.fetches.filter(call => call.url === '/allen/api/transcribe');
function readWav(buffer) {
  const view = new DataView(buffer);
  const tag = at => String.fromCharCode(...new Uint8Array(buffer, at, 4));
  const n = view.getUint32(40, true) / 2;
  const samples = [];
  for (let i = 0; i < n; i += 1) samples.push(view.getInt16(44 + i * 2, true));
  return {
    riff: tag(0), size: view.getUint32(4, true), wave: tag(8), fmt: tag(12), fmtSize: view.getUint32(16, true),
    format: view.getUint16(20, true), channels: view.getUint16(22, true), rate: view.getUint32(24, true),
    byteRate: view.getUint32(28, true), align: view.getUint16(32, true), bits: view.getUint16(34, true),
    data: tag(36), bytes: view.getUint32(40, true), total: buffer.byteLength, samples,
  };
}
function constant(rate, seconds, ...values) {
  return { rate, channels: values.map(v => new Float32Array(Math.round(rate * seconds)).fill(v)) };
}

test('server: presence transcribe true makes it primary, with its exact privacy note, and records nothing', async () => {
  const env = await serverEnv();
  assert.equal(env.win.AllenVoice.mode(), 'server');
  assert.equal(env.privacy.textContent, C.server);
  assert.ok(C.server.includes("Your clip is sent to Aien's server, turned into text, and never saved: it stays in memory only and is discarded as soon as the text comes back. The text is shown to you; it is not logged."));
  assert.doesNotMatch(code.replace(/\/\*[\s\S]*?\*\//g, ''), /Allen's|reach Allen/, 'no visible copy names Allen');
  assert.doesNotMatch(C.server, /disk/);
  assert.equal(env.srv.fetches.length, 1);
  assert.equal(env.srv.fetches[0].url, '/allen/api/presence');
  assert.equal(env.srv.fetches[0].init.method, undefined, 'a plain GET, no OPTIONS');
  assert.equal(env.srv.fetches[0].init.cache, 'no-store');
  assert.equal(env.srv.fetches[0].init.credentials, 'omit');
  assert.equal(env.srv.gum, 0, 'no microphone before the tap');
  assert.equal(env.srv.recorders.length, 0);
  assert.equal(env.rec.instances.length, 0);
  assert.deepEqual(env.guard, { media: 0, permission: 0 });
});

test('server: false, missing, failing or unreachable presence keeps the browser path', async () => {
  for (const presence of [{ transcribe: false }, { online: true }, { transcribe: 'true' }, 'error', 'throw']) {
    const env = await serverEnv({ presence });
    assert.equal(env.win.AllenVoice.mode(), 'cloud', JSON.stringify(presence));
    assert.equal(env.privacy.textContent, C.cloud);
  }
  const norecorder = await serverEnv({ recorder: false });
  assert.equal(norecorder.srv.fetches.length, 0, 'no presence request when this browser cannot record');
  assert.equal(norecorder.win.AllenVoice.mode(), 'cloud');
  const noformat = await serverEnv({ opus: [] });
  assert.equal(noformat.win.AllenVoice.mode(), 'cloud', 'no opus and no AudioContext: cannot send an accepted format');
  const insecure = await serverEnv({}, { secure: false });
  assert.equal(insecure.win.AllenVoice.mode(), 'insecure');
  assert.equal(insecure.srv.fetches.length, 0);
});

test('server: an installed on-device recognizer still comes first; no browser recognition still gets the server', async () => {
  const local = await serverEnv({}, { recognition: 'standard', available: 'available' });
  assert.equal(local.win.AllenVoice.mode(), 'local');
  assert.equal(local.privacy.textContent, C.local);
  const bare = await serverEnv({}, { recognition: 'none' });
  assert.equal(bare.win.AllenVoice.mode(), 'server');
  assert.equal(bare.mic.disabled, false);
  assert.equal(bare.status.textContent, '');
});

test('server: tap records, tap sends one opus clip, the text fills the box and is not sent', async () => {
  const env = await serverEnv({ hold: true });
  env.input.value = 'Hi';
  env.mic.click();
  assert.equal(env.srv.gum, 1, 'microphone asked only on the tap');
  assert.equal(env.status.textContent, C.starting);
  await settle();
  const recorder = env.srv.recorders[0];
  assert.equal(recorder.options.mimeType, 'audio/webm;codecs=opus');
  assert.equal(recorder.state, 'recording');
  assert.equal(env.win.AllenVoice.state(), 'listening');
  assert.equal(env.status.textContent, C.recording);
  assert.equal(env.mic.getAttribute('aria-pressed'), 'true');
  assert.equal(env.srv.timers[0].ms, 15000);
  env.srv.clock += 2000;
  recorder.chunk = new Blob([new Uint8Array(1200)], { type: 'audio/webm;codecs=opus' });
  env.mic.click();
  assert.equal(env.status.textContent, C.sending);
  await settle();
  assert.equal(env.srv.streams[0].tracks[0].stopped, true, 'microphone released');
  assert.equal(env.srv.timers[0].live, false);
  const sent = posts(env);
  assert.equal(sent.length, 1);
  assert.equal(sent[0].init.method, 'POST');
  assert.equal(sent[0].init.headers['Content-Type'], 'audio/webm;codecs=opus');
  assert.equal(sent[0].init.credentials, 'omit');
  assert.equal(sent[0].init.cache, 'no-store');
  assert.equal(sent[0].init.body.size, 1200);
  assert.equal(env.input.value, 'Hi', 'nothing written before the reply');
  env.srv.held.shift()();
  await settle();
  assert.equal(env.input.value, 'Hi hello world');
  assert.equal(env.inputs(), 1);
  assert.equal(env.status.textContent, C.done);
  assert.equal(env.win.AllenVoice.state(), 'idle');
  assert.equal(env.transport.sends, 0);
  assert.equal(env.form.submits, 0);
  assert.equal(env.rec.instances.length, 0, 'never also sent to the browser service');
  assert.equal(env.writes.length, 0, 'no transcript stored');
});

test('server: 15 seconds stops and sends; too short or too big never uploads', async () => {
  const env = await serverEnv();
  env.mic.click();
  await settle();
  env.srv.clock += 15000;
  env.srv.timers[0].fn();
  await settle();
  assert.equal(env.srv.recorders[0].state, 'inactive');
  assert.equal(posts(env).length, 1);
  const short = await serverEnv();
  await record(short, 299);
  assert.equal(posts(short).length, 0);
  assert.equal(short.status.textContent, C.short);
  const big = await serverEnv();
  await record(big, 5000, new Blob([new Uint8Array(1048577)], { type: 'audio/webm;codecs=opus' }));
  assert.equal(posts(big).length, 0);
  assert.equal(big.status.textContent, C.toolong);
});

test('server: refusals are announced, never retried elsewhere; the browser service is only offered', async () => {
  const cases = [
    [{ status: 413, body: { error: 'too_long', message: 'x' } }, C.toolong],
    [{ status: 415, body: { error: 'format', message: 'x' } }, C.format],
    [{ status: 422, body: { error: 'audio', message: 'x' } }, C.unclear],
    [{ status: 429, body: { error: 'rate', message: 'x', retry_after: 9 }, headers: { 'Retry-After': '7' } }, `${C.rate} ${C.retry} 7 s.`],
    [{ status: 429, body: { error: 'rate', message: 'x', retry_after: 9 } }, `${C.rate} ${C.retry} 9 s.`],
    [{ status: 503, body: { error: 'busy', message: 'x' } }, C.busy],
    [{ status: 400, body: { error: 'bad', message: 'x' } }, C.failed],
    [{ status: 200, body: { text: '   ' } }, C.unclear],
    ['throw', C.serverdown],
    [{ status: 413, html: true }, C.toolong],
    [{ status: 429, body: { error: 'rate', message: 'slow down', retry_after: 20 } }, `${C.rate} ${C.retry} 20 s.`],
    [{ status: 502, html: true }, C.failed],
    [{ status: 200, html: true }, C.failed],
  ];
  for (const [answer, said] of cases) {
    const env = await serverEnv({ respond: () => answer });
    await record(env, 2000);
    assert.equal(env.status.textContent, said, JSON.stringify(answer));
    assert.equal(posts(env).length, 1, 'uploaded once');
    assert.equal(env.rec.instances.length, 0, 'no automatic browser fallback');
    assert.equal(env.win.AllenVoice.mode(), 'server');
    assert.equal(env.privacy.textContent, C.server);
    assert.equal(env.input.value, '');
    const offer = env.offer();
    if (answer.status === 200) continue;
    assert.ok(offer && offer.hidden === false, 'browser service offered');
    assert.equal(offer.textContent, C.offer);
  }
  const env = await serverEnv({ respond: () => ({ status: 503, body: {} }) });
  await record(env, 2000);
  env.offer().click();
  assert.equal(env.win.AllenVoice.mode(), 'cloud');
  assert.equal(env.privacy.textContent, C.cloud, 'the online note is shown before use');
  assert.equal(env.status.textContent, C.switched);
  assert.equal(env.offer().hidden, true);
  assert.equal(env.rec.instances.length, 0, 'still nothing until the visitor taps');
  const gum = env.srv.gum;
  env.mic.click();
  assert.equal(env.rec.instances.length, 1);
  assert.equal(env.srv.gum, gum, 'browser path does not record a clip');
});

test('server: a server failure offers nothing when this browser has no recognition', async () => {
  const env = await serverEnv({ respond: () => ({ status: 503, body: {} }) }, { recognition: 'none' });
  await record(env, 2000);
  assert.equal(env.status.textContent, C.busy);
  assert.equal(env.offer(), null);
});

test('server: blocked or missing microphone', async () => {
  for (const [media, said] of [['NotAllowedError', C.blocked], ['NotFoundError', C.nomic], ['AbortError', C.failed]]) {
    const env = await serverEnv({ media });
    env.mic.click();
    await settle();
    assert.equal(env.status.textContent, said, media);
    assert.equal(env.win.AllenVoice.state(), 'idle');
    assert.equal(posts(env).length, 0);
  }
});

test('server: Escape while recording or sending drops the clip; hiding drops it too', async () => {
  const env = await serverEnv();
  env.mic.click();
  await settle();
  const key = env.escape();
  assert.equal(key.defaultPrevented, true);
  await settle();
  assert.equal(env.srv.streams[0].tracks[0].stopped, true);
  assert.equal(posts(env).length, 0);
  assert.equal(env.status.textContent, C.cancelled);
  const sending = await serverEnv({ hold: true });
  sending.input.value = 'mine';
  await record(sending, 2000);
  assert.equal(posts(sending).length, 1);
  sending.escape();
  await settle();
  assert.equal(posts(sending)[0].init.signal.aborted, true);
  sending.srv.held.shift()();
  await settle();
  assert.equal(sending.input.value, 'mine', 'a cancelled reply never lands');
  assert.equal(sending.status.textContent, C.cancelled);
  const hidden = await serverEnv();
  hidden.mic.click();
  await settle();
  hidden.document.hidden = true;
  hidden.document.dispatchEvent(new Event('visibilitychange'));
  await settle();
  assert.equal(posts(hidden).length, 0);
  assert.equal(hidden.status.textContent, C.hidden);
  assert.equal(hidden.srv.streams[0].tracks[0].stopped, true);
});

test('server: a tap while the browser is still asking for the microphone cancels and releases it', async () => {
  const env = await serverEnv();
  env.mic.click();
  env.mic.click();
  assert.equal(env.status.textContent, C.cancelled);
  await settle();
  assert.equal(env.srv.recorders.length, 0);
  assert.equal(env.srv.streams[0].tracks[0].stopped, true);
});

test('Safari WAV: MP4 clip decoded, mixed to mono, resampled to 16 kHz, exact PCM16 header and samples', async () => {
  const env = await serverEnv({ opus: [], audio: true, decoded: constant(48000, 0.5, 0.25, 0.75) });
  assert.equal(env.win.AllenVoice.mode(), 'server');
  const recorder = await record(env, 600, new Blob([new Uint8Array(333)], { type: 'audio/mp4' }));
  assert.equal(recorder.options, undefined, 'recorded in the browser default format');
  assert.equal(env.srv.decodedBytes, 333);
  assert.equal(env.srv.closed, 1, 'audio context closed');
  const sent = posts(env);
  assert.equal(sent.length, 1);
  assert.equal(sent[0].init.headers['Content-Type'], 'audio/wav');
  const w = readWav(await sent[0].init.body.arrayBuffer());
  assert.deepEqual(
    { riff: w.riff, size: w.size, wave: w.wave, fmt: w.fmt, fmtSize: w.fmtSize, format: w.format, channels: w.channels, rate: w.rate, byteRate: w.byteRate, align: w.align, bits: w.bits, data: w.data, bytes: w.bytes, total: w.total },
    { riff: 'RIFF', size: 36 + 16000, wave: 'WAVE', fmt: 'fmt ', fmtSize: 16, format: 1, channels: 1, rate: 16000, byteRate: 32000, align: 2, bits: 16, data: 'data', bytes: 16000, total: 44 + 16000 },
  );
  assert.equal(w.samples.length, 8000);
  assert.ok(w.samples.every(s => s === 16384), 'mean of 0.25 and 0.75 is 0.5 -> round(0.5 * 32767) = 16384');
  assert.equal(env.input.value, 'hello world');
});

test('Safari WAV: tone keeps its pitch and level; clamping; 44.1 kHz and 8 kHz inputs; callback decode', async () => {
  const rate = 48000;
  const tone = new Float32Array(rate);
  for (let i = 0; i < rate; i += 1) tone[i] = 0.8 * Math.sin(2 * Math.PI * 440 * i / rate);
  const env = await serverEnv({ opus: [], audio: true, decoded: { rate, channels: [tone] } });
  await record(env, 1100);
  const w = readWav(await posts(env)[0].init.body.arrayBuffer());
  assert.equal(w.samples.length, 16000);
  let crossings = 0;
  for (let i = 1; i < w.samples.length; i += 1) if ((w.samples[i - 1] < 0) !== (w.samples[i] < 0)) crossings += 1;
  assert.ok(Math.abs(crossings - 880) <= 2, `440 Hz has 880 sign changes per second, saw ${crossings}`);
  const peak = Math.max(...w.samples.map(Math.abs));
  assert.ok(peak > 0.8 * 32767 * 0.97 && peak <= 0.8 * 32767 + 1, `level kept, peak ${peak}`);

  const loud = new Float32Array(16000);
  for (let i = 0; i < loud.length; i += 1) loud[i] = i % 2 ? -1.5 : 1.5;
  const clamp = await serverEnv({ opus: [], audio: 'callback', decoded: { rate: 16000, channels: [loud] } });
  await record(clamp, 1100);
  const c = readWav(await posts(clamp)[0].init.body.arrayBuffer());
  assert.equal(c.samples[0], 32767);
  assert.equal(c.samples[1], -32768);

  const cd = await serverEnv({ opus: [], audio: true, decoded: constant(44100, 1, 0.1) });
  await record(cd, 1100);
  const d = readWav(await posts(cd)[0].init.body.arrayBuffer());
  assert.equal(d.samples.length, Math.floor(44100 / (44100 / 16000)));
  assert.ok(d.samples.every(s => Math.abs(s - 3277) <= 1), 'DC 0.1 stays round(0.1 * 32767) = 3277');

  const ramp = new Float32Array(8000);
  for (let i = 0; i < ramp.length; i += 1) ramp[i] = i / 10000;
  const phone = await serverEnv({ opus: [], audio: true, decoded: { rate: 8000, channels: [ramp] } });
  await record(phone, 1100);
  const p = readWav(await posts(phone)[0].init.body.arrayBuffer());
  assert.equal(p.samples.length, 16000);
  assert.equal(p.samples[1], Math.round(0.00005 * 32767), 'linear midpoint between ramp samples 0 and 1');
  assert.equal(p.samples[2], Math.round(0.0001 * 32767));
});

test('Safari WAV: undecodable or too-short audio is never uploaded', async () => {
  const broken = await serverEnv({ opus: [], audio: true, decoded: null });
  await record(broken, 2000);
  assert.equal(posts(broken).length, 0);
  assert.equal(broken.status.textContent, C.format);
  const short = await serverEnv({ opus: [], audio: true, decoded: constant(48000, 0.2, 0.3) });
  await record(short, 2000);
  assert.equal(posts(short).length, 0);
  assert.equal(short.status.textContent, C.short);
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

// No dependencies: node scripts/test-allen-intro.cjs [dist/js/allen-intro.js]
const { readFileSync } = require('node:fs');
const vm = require('node:vm');
const assert = require('node:assert/strict');
const { test } = require('node:test');
const { randomUUID } = require('node:crypto');
const source = readFileSync(process.argv[2] || 'public/js/allen-intro.js', 'utf8');
const key = 'aien-allen-intro-consent';

function page({ stored, privacy = false, blocked = false, session, hash = '', search = '', failNo = false, configured = false } = {}) {
  // Fixture event IDs exist only in this isolated VM, never in deployed config.
  const code = configured ? source.replace("pageVisit: '', introducedToAllen: ''", "pageVisit: 'tw-rfrlz-testvisit', introducedToAllen: 'tw-rfrlz-testintro'") : source;
  const storage = new Map(stored === undefined ? [] : [[key, stored]]);
  const sessionData = session || new Map();
  const loads = [];
  const events = {};
  let reloads = 0;
  let context;
  const nodes = {};
  function node(id) {
    return nodes[id] ||= {
      dataset: {}, open: false, hidden: true, isConnected: true,
      addEventListener(name, fn) { this[name] = fn; },
      focus() { document.activeElement = this; },
      showModal() { this.open = true; }, close() { this.open = false; },
    };
  }
  const document = {
    activeElement: node('previous'),
    getElementById(id) { return id === 'allen-intro-privacy' && !privacy ? null : node(id); },
    querySelectorAll(selector) { return privacy ? [node(selector)] : []; },
    createElement(tag) { assert.equal(tag, 'script'); return {}; },
    getElementsByTagName() { return [{ parentNode: { insertBefore(script) { loads.push(script.src); } } }]; },
  };
  const localStorage = {
    getItem(k) { if (blocked) throw Error('blocked'); return storage.get(k) || null; },
    setItem(k, v) { if (blocked || (failNo && v === 'no')) throw Error('blocked'); storage.set(k, v); },
    removeItem(k) { if (blocked) throw Error('blocked'); storage.delete(k); },
  };
  const sessionStorage = {
    getItem(k) { if (blocked) throw Error('blocked'); return sessionData.get(k) || null; },
    setItem(k, v) { if (blocked) throw Error('blocked'); sessionData.set(k, v); },
    removeItem(k) { if (blocked) throw Error('blocked'); sessionData.delete(k); },
  };
  const window = {
    crypto: { randomUUID },
    location: { hash, pathname: '/privacy/', search, reload() { reloads++; } },
    addEventListener(name, fn) { events[name] = fn; },
  };
  context = vm.createContext({ document, localStorage, sessionStorage, window, URLSearchParams,
    history: { replaceState() { window.location.hash = ''; } },
  });
  // Browsers expose window properties as globals; preserve that for the official snippet.
  Object.defineProperty(context, 'twq', { get() { return window.twq; } });
  vm.runInContext(code, context);
  return { nodes, loads, storage, sessionData, window, localStorage,
    reloads: () => reloads, runAgain: () => vm.runInContext(code, context),
    clearElsewhere() { storage.clear(); events.storage({ key: null, newValue: null, storageArea: localStorage }); },
    queued: () => Array.from(window.twq?.queue || [], args => Array.from(args)),
  };
}

function noPixel(p) {
  assert.deepEqual(p.loads, [], 'no external pixel load before explicit consent');
  assert.equal(p.window.twq, undefined, 'no twq before explicit consent');
}

test('unknown, invalid, refused and unavailable storage never load X', () => {
  for (const options of [{}, { stored: 'no' }, { stored: 'YES' }, { blocked: true }]) noPixel(page(options));
});
test('No persists, dismisses and remains tracking-free after navigation', () => {
  const p = page();
  assert.equal(p.nodes['aien-allen-intro'].open, true);
  p.nodes['allen-intro-no'].click();
  assert.equal(p.nodes['aien-allen-intro'].open, false);
  assert.equal(p.storage.get(key), 'no');
  noPixel(p);
  const next = page({ stored: p.storage.get(key) });
  assert.equal(next.nodes['aien-allen-intro'].open, false);
  noPixel(next);
});
test('Escape means No, and privacy is readable without the dialog', () => {
  const p = page();
  let prevented = false;
  p.nodes['aien-allen-intro'].cancel({ preventDefault() { prevented = true; } });
  assert.equal(prevented, true);
  assert.equal(p.storage.get(key), 'no');
  noPixel(p);
  const privacy = page({ privacy: true });
  assert.equal(privacy.nodes['aien-allen-intro'].open, false);
  noPixel(privacy);
});
test('introducing yourself runs the exact official configuration once per document', () => {
  const p = page();
  p.nodes['allen-intro-yes'].click();
  p.nodes['allen-intro-yes'].click();
  p.runAgain();
  assert.deepEqual(p.loads, ['https://static.ads-twitter.com/uwt.js']);
  assert.deepEqual(p.queued(), [['config', 'rfrlz']]);
  assert.equal(p.storage.get(key), 'yes');
  assert.deepEqual(page({ stored: 'yes' }).queued(), [['config', 'rfrlz']]);
});
test('revocation stops on reload, including failed persistent writes', () => {
  for (const failNo of [false, true]) {
    const p = page({ stored: 'yes', privacy: true, failNo });
    p.nodes['[data-allen-intro-stop]'].click();
    assert.equal(p.reloads(), 1);
    noPixel(page({ stored: p.storage.get(key), session: p.sessionData, hash: p.window.location.hash }));
  }
});
test('cross-tab localStorage.clear revokes an active pixel', () => {
  const p = page({ stored: 'yes' });
  p.clearElsewhere();
  assert.equal(p.reloads(), 1);
  noPixel(page());
});
test('twclid is retained only after consent and removed on refusal', () => {
  for (const stored of [undefined, 'no']) {
    const p = page({ stored, search: '?twclid=landing-test' });
    assert.equal(p.storage.has('aien-allen-intro-twclid'), false);
    noPixel(p);
  }
  const p = page({ search: '?twclid=landing-test' });
  p.nodes['allen-intro-yes'].click();
  assert.equal(p.storage.get('aien-allen-intro-twclid'), 'landing-test');
  p.nodes['allen-intro-no'].click();
  assert.equal(p.storage.has('aien-allen-intro-twclid'), false);
  assert.equal(page({ stored: 'yes', search: '?twclid=return-test' }).storage.get('aien-allen-intro-twclid'), 'return-test');
});
test('configured events require consent and carry distinct deduplication IDs', () => {
  const p = page({ configured: true });
  noPixel(p);
  p.nodes['allen-intro-yes'].click();
  p.nodes['allen-intro-yes'].click();
  const events = p.queued().filter(args => args[0] === 'event');
  assert.equal(events.length, 2);
  assert.equal(events[0][1], 'tw-rfrlz-testvisit');
  assert.equal(events[1][1], 'tw-rfrlz-testintro');
  for (const event of events) assert.match(event[2].conversion_id, /^[0-9a-f-]{36}$/);
  assert.notEqual(events[0][2].conversion_id, events[1][2].conversion_id);
  const returning = page({ configured: true, stored: 'yes' }).queued().filter(args => args[0] === 'event');
  assert.equal(returning.length, 1);
  assert.equal(returning[0][1], 'tw-rfrlz-testvisit');
});
test('empty production event IDs do not emit events or generate conversion IDs', () => {
  const p = page();
  p.window.crypto.randomUUID = () => assert.fail('empty IDs must not generate conversion IDs');
  p.nodes['allen-intro-yes'].click();
  assert.deepEqual(p.queued(), [['config', 'rfrlz']]);
});

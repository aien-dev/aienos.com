// No dependencies: node scripts/test-logbook.cjs [dist/js/logbook.js]
const { readFileSync } = require('node:fs');
const vm = require('node:vm');
const assert = require('node:assert/strict');
const { test } = require('node:test');
const source = readFileSync(process.argv[2] || 'public/js/logbook.js', 'utf8');
const key = 'aien-logbook-consent';

function page({ stored, privacy = false, blocked = false, session, hash = '', search = '', failNo = false } = {}) {
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
    getElementById(id) { return id === 'logbook-privacy' && !privacy ? null : node(id); },
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
    location: { hash, pathname: '/privacy/', search, reload() { reloads++; } },
    addEventListener(name, fn) { events[name] = fn; },
  };
  context = vm.createContext({ document, localStorage, sessionStorage, window, URLSearchParams,
    history: { replaceState() { window.location.hash = ''; } },
  });
  // Browsers expose window properties as globals; preserve that for the official snippet.
  Object.defineProperty(context, 'twq', { get() { return window.twq; } });
  vm.runInContext(source, context);
  return { nodes, loads, storage, sessionData, window, localStorage,
    reloads: () => reloads, runAgain: () => vm.runInContext(source, context),
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
  assert.equal(p.nodes['aien-logbook'].open, true);
  p.nodes['logbook-no'].click();
  assert.equal(p.nodes['aien-logbook'].open, false);
  assert.equal(p.storage.get(key), 'no');
  noPixel(p);
  const next = page({ stored: p.storage.get(key) });
  assert.equal(next.nodes['aien-logbook'].open, false);
  noPixel(next);
});
test('Escape means No, and privacy is readable without the dialog', () => {
  const p = page();
  let prevented = false;
  p.nodes['aien-logbook'].cancel({ preventDefault() { prevented = true; } });
  assert.equal(prevented, true);
  assert.equal(p.storage.get(key), 'no');
  noPixel(p);
  const privacy = page({ privacy: true });
  assert.equal(privacy.nodes['aien-logbook'].open, false);
  noPixel(privacy);
});
test('signing runs the exact official configuration once per document', () => {
  const p = page();
  p.nodes['logbook-sign'].click();
  p.nodes['logbook-sign'].click();
  p.runAgain();
  assert.deepEqual(p.loads, ['https://static.ads-twitter.com/uwt.js']);
  assert.deepEqual(p.queued(), [['config', 'rfrlz']]);
  assert.equal(p.storage.get(key), 'yes');
  assert.deepEqual(page({ stored: 'yes' }).queued(), [['config', 'rfrlz']]);
});
test('revocation stops on reload, including failed persistent writes', () => {
  for (const failNo of [false, true]) {
    const p = page({ stored: 'yes', privacy: true, failNo });
    p.nodes['[data-logbook-stop]'].click();
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
    assert.equal(p.storage.has('aien-logbook-twclid'), false);
    noPixel(p);
  }
  const p = page({ search: '?twclid=landing-test' });
  p.nodes['logbook-sign'].click();
  assert.equal(p.storage.get('aien-logbook-twclid'), 'landing-test');
  p.nodes['logbook-no'].click();
  assert.equal(p.storage.has('aien-logbook-twclid'), false);
  assert.equal(page({ stored: 'yes', search: '?twclid=return-test' }).storage.get('aien-logbook-twclid'), 'return-test');
});

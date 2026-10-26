// No dependencies: node scripts/test-allen-intro.cjs [dist/js/allen-intro.js]
const { readFileSync } = require('node:fs');
const vm = require('node:vm');
const assert = require('node:assert/strict');
const { test } = require('node:test');
const { randomUUID } = require('node:crypto');
const source = readFileSync(process.argv[2] || 'public/js/allen-intro.js', 'utf8');
const key = 'aien-allen-intro-consent';

const seenKey = 'aien-allen-intro-seen';
const GREETING = "Hi, I'm Aien (you say it like Ian). It's nice to meet you!";
const INTRO = "You don't have to tell me your name, but if you do, that would be super awesome and we could chat over time. When you come back, I can remember who you are and we can pick up where we left off. We can talk about all kinds of good things in the universe.";
const OFFER = "Separately, if you'd like, say hi and I'll let Drake know you stopped by. That uses Drake's X pixel to note the visit. Totally fine if not.";
const GPC_NOTE = "Your browser sends Global Privacy Control, so I'm not offering Drake's X pixel. It stays off.";
const AFTER_NO = "No problem. You can still chat with me anonymously; I won't remember you or tell X anything.";
const X_ORIGINS = ['https://static.ads-twitter.com', 'https://analytics.twitter.com', 'https://t.co'];

function page({ stored, privacy = false, blocked = false, blockedLocal = blocked, session, local, hash = '', search = '', failNo = false, configured = false, motion = false, gpc = false, chat, defer = false, letters = false, resources = [] } = {}) {
  // Fixture event IDs exist only in this isolated VM, never in deployed config.
  const code = configured ? source.replace("pageVisit: '', introducedToAllen: ''", "pageVisit: 'tw-rfrlz-testvisit', introducedToAllen: 'tw-rfrlz-testintro'") : source;
  const storage = local || new Map(stored === undefined ? [] : [[key, stored]]);
  const sessionData = session || new Map();
  const loads = [];
  const events = {};
  const docEvents = {};
  const timers = [];
  const delays = [];
  const announced = [];
  const network = [];
  // Resource timing as the browser records it; same-origin files plus any given outside ones.
  const entries = ['https://www.aienos.com/assets/style.css', 'https://www.aienos.com/js/allen-intro.js', ...resources].map(name => ({ name }));
  const observers = [];
  function record(names) {
    const fresh = names.map(name => ({ name }));
    entries.push(...fresh);
    for (const fn of observers) fn({ getEntries: () => fresh });
  }
  let reloads = 0;
  let modal = 0;
  let ran = 0;
  let context;
  const nodes = {};
  function node(id) {
    return nodes[id] ||= {
      id, dataset: {}, hidden: true, isConnected: true, textContent: '', classes: new Set(),
      classList: { add(c) { nodes[id].classes.add(c); }, remove(c) { nodes[id].classes.delete(c); } },
      addEventListener(name, fn) { this[name] = fn; },
      contains(other) { return other === this || (this.id === 'aien-allen-intro' && /^allen-intro-(yes|no|close)$/.test(other?.id)); },
      focus() { document.activeElement = this; },
      showModal() { modal++; }, close() {},
    };
  }
  // The server-rendered copy JS reads, and the typed span's two children.
  node('allen-intro-title').textContent = GREETING;
  node('allen-intro-intro').textContent = INTRO;
  node('allen-intro-offer').textContent = OFFER;
  node('allen-intro-gpc').textContent = GPC_NOTE;
  node('allen-intro-after-text').textContent = AFTER_NO;
  node('allen-intro-typed').firstChild = node('typed-head');
  node('allen-intro-typed').lastChild = node('typed-tail');
  if (letters) node('allen-intro-typed').dataset.letters = 'true';
  // Letter mode appends one span per letter; plain mode only sets textContent.
  for (const id of ['typed-head', 'typed-tail']) {
    node(id).letters = [];
    node(id).appendChild = function (child) { this.letters.push(child); };
    Object.defineProperty(node(id), 'textContent', {
      get() { return this.plain + this.letters.map(l => l.textContent).join(''); },
      set(v) { this.plain = v; this.letters = []; },
    });
    node(id).textContent = '';
  }
  for (const id of ['allen-intro-trackers', 'allen-intro-facts']) {
    node(id).items = [];
    node(id).appendChild = function (child) { this.items.push(child.textContent); };
  }
  if (chat) node('allen').dataset.mode = chat;
  const document = {
    referrer: '',
    getElementById(id) {
      if ((id === 'allen-intro-privacy' && !privacy) || (id === 'allen' && !chat)) return null;
      return node(id);
    },
    querySelectorAll(selector) { return privacy ? [node(selector)] : []; },
    querySelector(selector) { assert.equal(selector, '[data-allen-intro-defer]'); return defer ? node('door') : null; },
    addEventListener(name, fn) { docEvents[name] = fn; },
    dispatchEvent(event) { announced.push(event.detail); },
    createElement(tag) {
      // The panel may only build text list items; anything else would be a network-capable node.
      if (tag === 'script') return {};
      assert.ok(['li', 'dt', 'dd', 'span'].includes(tag), `unexpected element ${tag}`);
      return { tag, textContent: '' };
    },
    getElementsByTagName() {
      return [{ parentNode: { insertBefore(script) { loads.push(script.src); record(X_ORIGINS.map(o => o + '/uwt.js')); } } }];
    },
  };
  document.activeElement = node('previous');
  const localStorage = {
    getItem(k) { if (blockedLocal) throw Error('blocked'); return storage.get(k) || null; },
    setItem(k, v) { if (blockedLocal || (failNo && v === 'no')) throw Error('blocked'); storage.set(k, v); },
    removeItem(k) { if (blockedLocal) throw Error('blocked'); storage.delete(k); },
  };
  const sessionStorage = {
    getItem(k) { if (blocked) throw Error('blocked'); return sessionData.get(k) || null; },
    setItem(k, v) { if (blocked) throw Error('blocked'); sessionData.set(k, v); },
    removeItem(k) { if (blocked) throw Error('blocked'); sessionData.delete(k); },
  };
  const window = {
    crypto: { randomUUID },
    location: { hash, pathname: '/privacy/', search, href: 'https://www.aienos.com/privacy/', origin: 'https://www.aienos.com', reload() { reloads++; } },
    addEventListener(name, fn) { events[name] = fn; },
    matchMedia(query) { assert.equal(query, '(prefers-reduced-motion: reduce)'); return { matches: !motion }; },
    navigator: { globalPrivacyControl: gpc, userAgent: 'TestAgent/1', platform: 'TestOS', languages: ['en-US'], doNotTrack: '1', cookieEnabled: true,
      sendBeacon(url) { network.push(url); return true; } },
    screen: { width: 1280, height: 800 },
    performance: { getEntriesByType(type) { assert.equal(type, 'resource'); return entries.slice(); } },
    PerformanceObserver: class { constructor(fn) { this.fn = fn; } observe(opts) { assert.equal(opts.type, 'resource'); observers.push(this.fn); } },
    CustomEvent: class { constructor(type, init) { this.type = type; this.detail = init.detail; } },
    fetch(url) { network.push(String(url)); },
  };
  context = vm.createContext({ document, localStorage, sessionStorage, window, URLSearchParams, URL,
    history: { replaceState() { window.location.hash = ''; } },
    setTimeout(fn, ms) { delays.push(ms); timers.push(fn); return timers.length; }, clearTimeout(id) { if (id) timers[id - 1] = null; },
    fetch: window.fetch, XMLHttpRequest: class { open(m, url) { network.push(url); } send() {} },
    Image: class { set src(url) { network.push(url); } },
  });
  // Browsers expose window properties as globals; preserve that for the official snippet.
  Object.defineProperty(context, 'twq', { get() { return window.twq; } });
  vm.runInContext(code, context);
  const aside = nodes['aien-allen-intro'];
  return { nodes, aside, document, loads, storage, sessionData, window, localStorage, announced, network, entries,
    api: () => context.window.aienAllenIntro,
    delays, head: () => nodes['typed-head'], tail: () => nodes['typed-tail'],
    shown: () => !aside.hidden && aside.classes.has('allen-intro-in'),
    escape: () => docEvents.keydown({ key: 'Escape' }),
    key: k => docEvents.keydown({ key: k }),
    modal: () => modal,
    typed: () => nodes['typed-head'].textContent + nodes['typed-tail'].textContent,
    tick() { const fn = timers[ran]; if (!fn) return false; ran++; fn(); return true; },
    reloads: () => reloads, runAgain: () => vm.runInContext(code, context),
    clearElsewhere() { storage.clear(); events.storage({ key: null, newValue: null, storageArea: localStorage }); },
    queued: () => Array.from(window.twq?.queue || [], args => Array.from(args)),
  };
}

// Values built inside the VM realm compare structurally once copied into this realm.
const plain = v => JSON.parse(JSON.stringify(v));

function noPixel(p) {
  assert.deepEqual(p.loads, [], 'no external pixel load before explicit consent');
  assert.equal(p.window.twq, undefined, 'no twq before explicit consent');
}

test('unknown, invalid, refused and unavailable storage never load X', () => {
  for (const options of [{}, { stored: 'no' }, { stored: 'YES' }, { blocked: true }]) noPixel(page(options));
});
test('first visit: Allen appears with the welcome, records seen, and does not steal focus', () => {
  const p = page();
  assert.equal(p.shown(), true);
  assert.equal(p.storage.get(seenKey), '1');
  assert.equal(p.typed(), GREETING + ' ' + INTRO);
  assert.equal(p.nodes['allen-intro-spoken'].textContent, GREETING + ' ' + INTRO + ' ' + OFFER);
  assert.equal(p.nodes['allen-intro-x'].hidden, false, 'the X ask is offered as its own line');
  assert.deepEqual(plain(p.announced), [{ type: 'show', asked: false, offering: true }]);
  assert.equal(p.document.activeElement, p.nodes.previous, 'auto-display must not move focus');
  noPixel(p);
});
test('second page load does not show Allen, even with no choice made, and the pixel stays off', () => {
  const first = page();
  assert.equal(first.shown(), true);
  assert.equal(first.storage.has(key), false, 'no choice was made');
  const second = page({ local: first.storage, session: first.sessionData });
  assert.equal(second.aside.hidden, true);
  noPixel(second);
});
test('the page stays interactive while Allen is showing', () => {
  const p = page();
  assert.equal(p.shown(), true);
  assert.equal(p.modal(), 0, 'never showModal: no backdrop, page not inert');
  assert.equal(p.document.activeElement, p.nodes.previous);
  // Page controls keep working: focusing and clicking elsewhere neither closes Allen nor decides.
  p.nodes['page-link'] = { focus() { p.document.activeElement = this; } };
  p.nodes['page-link'].focus();
  assert.equal(p.document.activeElement, p.nodes['page-link']);
  assert.equal(p.shown(), true);
  assert.equal(p.storage.has(key), false);
  // Other keys pass through untouched.
  for (const k of ['Tab', 'Enter', ' ', 'a']) p.key(k);
  assert.equal(p.shown(), true);
  assert.equal(p.storage.has(key), false);
  noPixel(p);
});
test('No thanks refuses X, offers an anonymous chat, and stays tracking-free after navigation', () => {
  const p = page();
  p.nodes['allen-intro-no'].click();
  assert.equal(p.storage.get(key), 'no');
  assert.equal(p.aside.hidden, false, 'Allen stays to answer');
  for (const id of ['allen-intro-message', 'allen-intro-chat', 'allen-intro-x']) assert.equal(p.nodes[id].hidden, true, id);
  assert.equal(p.nodes['allen-intro-after'].hidden, false);
  assert.equal(p.nodes['allen-intro-spoken'].textContent, AFTER_NO);
  assert.equal(p.document.activeElement, p.nodes['allen-intro-anon'], 'focus follows the visitor action to Chat anonymously');
  assert.equal(p.announced.at(-1).type, 'reply-no');
  noPixel(p);
  p.escape();
  assert.equal(p.aside.hidden, true);
  assert.equal(p.announced.at(-1).type, 'hide');
  noPixel(p);
  const next = page({ local: p.storage });
  assert.equal(next.aside.hidden, true);
  noPixel(next);
});
test('Escape and the close button mean No; Escape is ignored once Allen is gone', () => {
  for (const act of [p => p.escape(), p => p.nodes['allen-intro-close'].click()]) {
    const p = page();
    act(p);
    assert.equal(p.aside.hidden, true);
    assert.equal(p.storage.get(key), 'no');
    noPixel(p);
  }
  const later = page({ local: new Map([[seenKey, '1']]) });
  later.escape();
  assert.equal(later.storage.has(key), false, 'Escape elsewhere on the page decides nothing');
});
test('Say hi loads the exact official configuration once per document', () => {
  const p = page();
  p.nodes['allen-intro-yes'].click();
  p.nodes['allen-intro-yes'].click();
  p.runAgain();
  assert.equal(p.aside.hidden, true);
  assert.deepEqual(p.loads, ['https://static.ads-twitter.com/uwt.js']);
  assert.deepEqual(p.queued(), [['config', 'rfrlz']]);
  assert.equal(p.storage.get(key), 'yes');
  assert.deepEqual(page({ stored: 'yes' }).queued(), [['config', 'rfrlz']]);
});
test('Review my choice reopens Allen on the privacy page, focuses No thanks, and returns focus', () => {
  for (const local of [new Map([[seenKey, '1']]), new Map([[seenKey, '1'], [key, 'no']])]) {
    const p = page({ privacy: true, local });
    assert.equal(p.aside.hidden, true, 'privacy is readable without Allen');
    const review = p.nodes['[data-allen-intro-open]'];
    assert.equal(review.hidden, false);
    review.focus();
    review.click();
    assert.equal(p.shown(), true);
    assert.equal(p.document.activeElement, p.nodes['allen-intro-no']);
    p.escape();
    assert.equal(p.aside.hidden, true);
    assert.equal(p.document.activeElement, review);
    noPixel(p);
  }
});
test('negative control: clearing storage shows Allen again, and still gives no consent', () => {
  const first = page();
  assert.equal(page({ local: first.storage }).aside.hidden, true, 'seen suppresses before the control');
  first.storage.clear();
  const again = page({ local: first.storage });
  assert.equal(again.shown(), true);
  noPixel(again);
});
test('without local storage, Allen appears at most once per tab session', () => {
  const first = page({ blockedLocal: true });
  assert.equal(first.shown(), true);
  assert.equal(first.sessionData.get(seenKey), '1');
  assert.equal(page({ blockedLocal: true, session: first.sessionData }).aside.hidden, true);
  assert.equal(page({ blockedLocal: true }).shown(), true, 'a new tab session greets again');
});
test('typing is static client text; reduced motion shows it whole at once', () => {
  const p = page({ motion: true });
  assert.equal(p.shown(), true);
  assert.equal(p.typed(), '');
  while (p.tick());
  assert.equal(p.typed(), GREETING + ' ' + INTRO);
  const q = page({ motion: true });
  q.tick();
  q.nodes['allen-intro-no'].click();
  assert.equal(q.tick(), false, 'closing stops the typing timer');
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

function openWho(p) { p.nodes['allen-intro-who'].open = true; p.nodes['allen-intro-who'].toggle(); }

test('who can see me: 0 trackers before consent, then the X origins after Say hi (negative control)', () => {
  const p = page();
  openWho(p);
  assert.equal(p.nodes['allen-intro-tracker-count'].textContent, '0');
  assert.equal(p.nodes['allen-intro-trackers-none'].hidden, false);
  assert.deepEqual(plain(p.api().who().trackers), []);
  p.nodes['allen-intro-yes'].click();
  const trackers = p.api().who().trackers;
  assert.deepEqual(plain(trackers.map(t => t.origin)), X_ORIGINS);
  assert.ok(trackers.every(t => t.pixel), 'each X origin is named as Drake\'s pixel');
  assert.equal(p.nodes['allen-intro-tracker-count'].textContent, String(X_ORIGINS.length));
  assert.ok(p.nodes['allen-intro-trackers'].items.every(line => line.endsWith(": that one's Drake's X pixel, because you said hi.")));
});
test('who can see me lists other outside origins honestly, never as Drake\'s pixel', () => {
  const p = page({ resources: ['https://www.youtube-nocookie.com/embed/x'] });
  openWho(p);
  assert.deepEqual(plain(p.api().who().trackers), [{ origin: 'https://www.youtube-nocookie.com', pixel: false }]);
  assert.deepEqual(p.nodes['allen-intro-trackers'].items, ['www.youtube-nocookie.com']);
  noPixel(p);
});
test('the panel makes no network request and only reads local browser facts', () => {
  const p = page();
  const before = p.entries.length;
  openWho(p);
  p.api().who();
  assert.equal(p.entries.length, before, 'no new resource entries');
  assert.deepEqual(p.network, [], 'no fetch, beacon, XHR or image');
  const facts = Object.fromEntries(p.api().who().facts);
  assert.equal(facts.Browser, 'TestAgent/1');
  assert.equal(facts['Screen size'], '1280 x 800');
  assert.equal(facts['Do Not Track'], 'on');
  assert.equal(facts['Global Privacy Control'], 'off');
  assert.equal(facts['Came from'], 'no referrer sent');
  assert.equal(p.storage.size, 1, 'only the seen mark is stored');
  noPixel(p);
});
test('Global Privacy Control suppresses the pixel offer; only an explicit review offers it', () => {
  const p = page({ gpc: true });
  assert.equal(p.shown(), true);
  assert.equal(p.nodes['allen-intro-x'].hidden, true, 'no Say hi under GPC');
  assert.equal(p.nodes['allen-intro-gpc'].hidden, false);
  assert.equal(p.nodes['allen-intro-spoken'].textContent, GREETING + ' ' + INTRO + ' ' + GPC_NOTE);
  assert.equal(p.api().status().offering, false);
  noPixel(p);
  const review = page({ gpc: true, privacy: true, local: new Map([[seenKey, '1']]) });
  review.nodes['[data-allen-intro-open]'].click();
  assert.equal(review.nodes['allen-intro-x'].hidden, false);
  // Negative control: without GPC the first greeting does offer it.
  assert.equal(page().nodes['allen-intro-x'].hidden, false);
});
test('anonymous chat on /allen/ never loads the pixel, and the chat page gets no greeting', () => {
  const anon = page({ stored: 'yes', chat: 'anonymous' });
  noPixel(anon);
  assert.equal(anon.aside.hidden, true);
  assert.equal(page({ chat: 'anonymous' }).aside.hidden, true);
  assert.deepEqual(page({ stored: 'yes', chat: 'remembered' }).loads, ['https://static.ads-twitter.com/uwt.js']);
});
test('presence API: X consent only, events for the visual layer, memory consent absent', () => {
  const p = page();
  const api = p.api();
  assert.deepEqual(plain(Object.keys(api).sort()), ['dismiss', 'greet', 'noThanks', 'onTrackers', 'review', 'sayHi', 'status', 'text', 'who']);
  assert.deepEqual(plain(api.text), { greeting: GREETING, intro: INTRO, offer: OFFER, gpcNote: GPC_NOTE, afterNo: AFTER_NO });
  assert.deepEqual(plain(api.status()), { choice: null, gpc: false, offering: true, pixelLoaded: false, shown: true });
  const seen = [];
  api.onTrackers(t => seen.push(t.origin));
  api.sayHi();
  assert.deepEqual(seen, X_ORIGINS);
  assert.deepEqual(plain(p.announced.map(e => e.type)), ['show', 'hide', 'consent']);
  assert.equal(api.status().pixelLoaded, true);
  assert.equal(p.storage.get(key), 'yes');
  const q = page();
  q.api().dismiss();
  assert.equal(q.storage.get(key), 'no');
  assert.equal(q.aside.hidden, true);
  noPixel(q);
});
test('a page door defers the greeting until it calls greet(), which still runs only once', () => {
  const p = page({ defer: true });
  assert.equal(p.aside.hidden, true, 'no greeting over the door');
  assert.equal(p.storage.has(seenKey), false, 'deferral does not spend the once-only greeting');
  assert.equal(p.api().greet(), true);
  assert.equal(p.shown(), true);
  assert.equal(p.storage.get(seenKey), '1');
  noPixel(p);
  const next = page({ defer: true, local: p.storage });
  assert.equal(next.api().greet(), false, 'once per browser, even when called again');
  assert.equal(next.aside.hidden, true);
  assert.equal(page({ defer: true, stored: 'no' }).api().greet(), false);
});
test('Allen types at his native 35 ms per letter; data-letters gives one span per letter', () => {
  const plain = page({ motion: true });
  while (plain.tick());
  assert.equal(plain.typed(), GREETING + ' ' + INTRO);
  assert.ok(plain.delays.length > 0 && plain.delays.every(ms => ms === 35), 'native cadence');
  assert.equal(plain.head().letters.length, 0, 'plain mode builds no letter spans');
  const p = page({ motion: true, letters: true });
  assert.equal(p.typed(), '');
  while (p.tick());
  assert.equal(p.typed(), GREETING + ' ' + INTRO, 'letters spell the same words');
  const spans = [...p.head().letters, ...p.tail().letters];
  assert.equal(spans.length, (GREETING + ' ' + INTRO).length);
  assert.ok(spans.every(s => s.tag === 'span' && s.className === 'allen-intro-letter' && s.textContent.length === 1));
  assert.equal(p.head().textContent, GREETING, 'the greeting stays in the strong slot');
  // Reduced motion: whole text at once, plain, no letter spans, no timers.
  const still = page({ letters: true });
  assert.equal(still.typed(), GREETING + ' ' + INTRO);
  assert.equal(still.head().letters.length + still.tail().letters.length, 0);
  assert.equal(still.tick(), false);
  // Reopening restarts cleanly instead of appending a second copy.
  const again = page({ motion: true, letters: true, privacy: true, local: new Map([[seenKey, '1']]) });
  const review = again.nodes['[data-allen-intro-open]'];
  review.click(); while (again.tick());
  again.escape();
  review.click(); while (again.tick());
  assert.equal(again.typed(), GREETING + ' ' + INTRO);
  noPixel(again);
});

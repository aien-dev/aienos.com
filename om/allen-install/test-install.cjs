'use strict';
/* Behaviour harness for the generated public/js/allen-install.js: a small fake
   DOM in node:vm, not a browser or visual claim. Run from the repo root:
   node om/allen-install/test-install.cjs */
const fs = require('node:fs'), vm = require('node:vm'), assert = require('node:assert/strict');
/* ALLEN_INSTALL_JS points the harness at another build, for negative controls. */
const SOURCE = fs.readFileSync(process.env.ALLEN_INSTALL_JS || 'public/js/allen-install.js', 'utf8');
const TITLE = 'Put me on your home screen';
const UA = {
  android: 'Mozilla/5.0 (Linux; Android 15; Pixel 9) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/141.0.0.0 Mobile Safari/537.36',
  iphone: 'Mozilla/5.0 (iPhone; CPU iPhone OS 18_6 like Mac OS X) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/18.6 Mobile/15E148 Safari/604.1',
  ipados: 'Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/18.6 Safari/605.1.15',
  crios: 'Mozilla/5.0 (iPhone; CPU iPhone OS 18_6 like Mac OS X) AppleWebKit/605.1.15 (KHTML, like Gecko) CriOS/141.0.0.0 Mobile/15E148 Safari/604.1',
  firefox: 'Mozilla/5.0 (X11; Linux aarch64; rv:144.0) Gecko/20100101 Firefox/144.0',
};
let inClick = false;
class Events {
  constructor() { this.listeners = new Map(); }
  addEventListener(type, fn) { if (!this.listeners.has(type)) this.listeners.set(type, new Set()); this.listeners.get(type).add(fn); }
  removeEventListener(type, fn) { this.listeners.get(type)?.delete(fn); }
  dispatchEvent(event) { for (const fn of [...(this.listeners.get(event.type) || [])]) fn(event); return !event.defaultPrevented; }
}
function world(opts = {}) {
  const observers = new Set();
  const mutated = () => { for (const o of observers) queueMicrotask(() => o.fn([])); };
  class Node extends Events {
    constructor(tag) { super(); this.tagName = tag; this.children = []; this.parentNode = null; this.attrs = new Map(); this.text = ''; this.className = ''; this.hidden = false; this.disabled = false; }
    get isConnected() { let n = this; while (n.parentNode) n = n.parentNode; return n === doc.documentElement; }
    get textContent() { return this.text + this.children.map(c => c.textContent).join(''); }
    set textContent(v) { this.detachAll(); this.text = String(v); }
    get lastChild() { return this.children[this.children.length - 1] || null; }
    detachAll() { for (const c of this.children) c.parentNode = null; this.children = []; }
    adopt(n) { if (typeof n === 'string') { const t = new Node('#text'); t.text = n; n = t; } if (n.parentNode) n.remove(); n.parentNode = this; return n; }
    append(...ns) { for (const n of ns) this.children.push(this.adopt(n)); mutated(); }
    appendChild(n) { this.append(n); return n; }
    insertBefore(n, ref) { const node = this.adopt(n); const at = this.children.indexOf(ref); this.children.splice(at < 0 ? this.children.length : at, 0, node); mutated(); return node; }
    replaceChildren(...ns) { this.detachAll(); this.append(...ns); }
    remove() { const p = this.parentNode; if (!p) return; p.children = p.children.filter(c => c !== this); this.parentNode = null; mutated(); }
    setAttribute(k, v) { this.attrs.set(k, String(v)); }
    getAttribute(k) { return this.attrs.has(k) ? this.attrs.get(k) : null; }
    removeAttribute(k) { this.attrs.delete(k); }
    click() { if (this.disabled) return; inClick = true; try { this.dispatchEvent({ type: 'click' }); } finally { inClick = false; } }
    all(pred, out = []) { for (const c of this.children) { if (pred(c)) out.push(c); c.all(pred, out); } return out; }
  }
  const doc = new Events();
  doc.documentElement = new Node('html');
  doc.body = new Node('body');
  doc.documentElement.append(doc.body);
  doc.createElement = tag => new Node(tag);
  doc.createElementNS = (ns, tag) => Object.assign(new Node(tag), { ns });
  doc.getElementById = id => doc.documentElement.all(n => n.attrs.get('id') === id)[0] || null;
  const allen = new Node('div'); allen.setAttribute('id', 'allen');
  const current = new Node('section'); current.className = 'allen-current';
  const answer = new Node('div'); answer.setAttribute('id', 'allen-answer');
  const panesRoot = new Node('div'); panesRoot.setAttribute('id', 'allen-panes');
  current.append(answer); allen.append(panesRoot, current); doc.body.append(allen);
  const panes = [];
  if (opts.paneHost !== false) doc.addEventListener('allen-pane-request', event => {
    /* Mirrors the host contract: open the node in a card pane and hand back a handle. */
    event.preventDefault();
    const pane = new Node('section'); pane.className = 'allen-pane'; pane.title = event.detail.title;
    pane.append(event.detail.node); panesRoot.append(pane);
    const rec = { pane, detail: event.detail, closed: 0 };
    panes.push(rec);
    event.detail.handle = { ok: true, id: event.detail.id, close: () => { rec.closed++; pane.remove(); } };
  });
  const store = opts.store || new Map();
  const localStorage = { getItem: k => (store.has(k) ? store.get(k) : null), setItem: (k, v) => { if (opts.quota) throw new Error('QuotaExceededError'); store.set(k, String(v)); } };
  const win = new Events();
  if (opts.noStorage) Object.defineProperty(win, 'localStorage', { get() { throw new Error('SecurityError'); } });
  else win.localStorage = localStorage;
  win.navigator = { userAgent: UA[opts.ua || 'android'], maxTouchPoints: opts.ua === 'ipados' ? 5 : 0 };
  if (opts.iosStandalone) win.navigator.standalone = true;
  win.matchMedia = q => ({ matches: Boolean(opts.display) && q === `(display-mode: ${opts.display})` });
  const handlers = new Map();
  const api = { on: (t, fn) => { if (!handlers.has(t)) handlers.set(t, new Set()); handlers.get(t).add(fn); return () => handlers.get(t).delete(fn); } };
  const emit = (t, d) => { for (const fn of handlers.get(t) || []) fn(Object.freeze({ ...d })); };
  class CustomEvent { constructor(type, init = {}) { this.type = type; this.detail = init.detail; this.cancelable = Boolean(init.cancelable); this.defaultPrevented = false; } preventDefault() { if (this.cancelable) this.defaultPrevented = true; } }
  class MutationObserver { constructor(fn) { this.fn = fn; } observe() { observers.add(this); } disconnect() { observers.delete(this); } }
  const context = { window: win, document: doc, CustomEvent, MutationObserver, Promise, Error, String, Object, queueMicrotask };
  win.window = win;
  const boot = () => { win.allen = Object.freeze(api); doc.dispatchEvent({ type: 'allen-ready' }); };
  vm.runInNewContext(SOURCE, context, { filename: 'allen-install.js' });
  if (opts.ready !== false) boot();
  const bip = (outcome = 'accepted') => {
    const e = { type: 'beforeinstallprompt', prevented: 0, prompts: 0, promptInClick: [], preventDefault() { this.prevented++; } };
    e.prompt = () => { e.prompts++; e.promptInClick.push(inClick); return Promise.resolve(); };
    e.userChoice = Promise.resolve({ outcome, platform: 'web' });
    win.dispatchEvent(e);
    return e;
  };
  const exchange = (opt = {}) => { emit('send', { text: 'hi', silent: Boolean(opt.silent) }); emit('reply', { text: opt.text === undefined ? 'Hello there.' : opt.text, outcome: opt.outcome || '' }); };
  const offers = () => doc.documentElement.all(n => n.className === 'allen-install');
  const button = text => doc.documentElement.all(n => n.tagName === 'button' && n.textContent === text)[0];
  const settle = () => new Promise(r => setTimeout(r, 0));
  return { win, doc, store, panes, bip, exchange, offers, button, settle, boot, emit, answer, current };
}
const results = [];
async function test(name, fn) { await fn(); results.push(name); }
(async () => {
  await test('captured prompt: preventDefault, nothing shown on load', async () => {
    const w = world();
    const e = w.bip();
    assert.equal(e.prevented, 1);
    assert.equal(e.prompts, 0);
    assert.equal(w.offers().length, 0);
    assert.equal(w.panes.length, 0);
  });
  await test('no offer on load, greeting, silent send or failed reply', async () => {
    const w = world();
    w.bip();
    w.exchange({ silent: true });
    w.exchange({ outcome: 'Stopped.' });
    w.exchange({ outcome: 'Could not reach Allen. Check your connection and try again.' });
    w.exchange({ text: '   ' });
    w.emit('reply', { text: 'Hello.', outcome: '' });
    assert.equal(w.offers().length, 0);
  });
  await test('natural good exchange shows one pane offer with the required copy', async () => {
    const w = world();
    const e = w.bip();
    w.exchange();
    assert.equal(w.panes.length, 1);
    assert.equal(w.panes[0].detail.title, TITLE);
    assert.equal(w.panes[0].detail.id, 'install');
    assert.equal(w.panes[0].detail.focus, false);
    assert.ok(w.button(TITLE));
    assert.ok(w.button('No thanks'));
    assert.equal(e.prompts, 0);
    w.exchange();
    assert.equal(w.panes.length, 1, 'a second good exchange does not stack offers');
  });
  await test('offer waits for beforeinstallprompt when the exchange came first', async () => {
    const w = world();
    w.exchange();
    assert.equal(w.offers().length, 0);
    w.bip();
    assert.equal(w.offers().length, 1);
  });
  await test('prompt only inside the tap; accepted is not success; appinstalled is', async () => {
    const w = world();
    const e = w.bip('accepted');
    w.exchange();
    assert.equal(e.prompts, 0);
    w.button(TITLE).click();
    assert.equal(e.prompts, 1);
    assert.deepEqual(e.promptInClick, [true]);
    await w.settle();
    const state = () => w.doc.documentElement.all(n => n.className === 'allen-install-state')[0].textContent;
    assert.match(state(), /^Your browser is adding me/);
    assert.doesNotMatch(state(), /installed/);
    assert.equal(w.store.get('allen-install'), 'accepted');
    assert.ok(w.button('Close'));
    w.win.dispatchEvent({ type: 'appinstalled' });
    assert.match(state(), /Your browser says I am installed/);
    assert.equal(w.store.get('allen-install'), 'installed');
  });
  await test('browser dialog dismissed is recorded honestly', async () => {
    const w = world();
    const e = w.bip('dismissed');
    w.exchange();
    w.button(TITLE).click();
    assert.deepEqual(e.promptInClick, [true]);
    await w.settle();
    assert.equal(w.store.get('allen-install'), 'dismissed');
    assert.ok(w.doc.documentElement.all(n => n.className === 'allen-install-state')[0].textContent.startsWith('No problem.'));
  });
  await test('dismiss once: No thanks closes and the next visit stays quiet', async () => {
    const store = new Map();
    const w = world({ store });
    w.bip();
    w.exchange();
    w.button('No thanks').click();
    assert.equal(store.get('allen-install'), 'dismissed');
    assert.equal(w.panes[0].closed, 1);
    assert.equal(w.offers().length, 0);
    const again = world({ store });
    const e = again.bip();
    again.exchange();
    assert.equal(again.offers().length, 0);
    assert.equal(e.prevented, 1);
  });
  await test('closing the pane with its own close button counts as dismissal', async () => {
    const w = world();
    w.bip();
    w.exchange();
    w.panes[0].pane.remove();
    await w.settle();
    assert.equal(w.store.get('allen-install'), 'dismissed');
  });
  await test('without a pane host the offer sits inline near the reply with an accessible close', async () => {
    const w = world({ paneHost: false });
    w.bip();
    w.exchange();
    const aside = w.current.all(n => n.className === 'allen-install-inline')[0];
    assert.ok(aside, 'inline aside beside #allen-answer');
    assert.equal(aside.getAttribute('aria-label'), TITLE);
    const close = aside.all(n => n.className === 'allen-install-close')[0];
    assert.equal(close.getAttribute('aria-label'), 'Close, and do not offer this again');
    assert.equal(close.children[0].getAttribute('aria-hidden'), 'true');
    close.click();
    assert.equal(aside.isConnected, false);
    assert.equal(w.store.get('allen-install'), 'dismissed');
  });
  await test('iOS Safari gets the two-step guide on tap, with labelled local icons', async () => {
    for (const ua of ['iphone', 'ipados']) {
      const w = world({ ua });
      w.exchange();
      assert.equal(w.offers().length, 1, ua);
      w.button(TITLE).click();
      const steps = w.doc.documentElement.all(n => n.className === 'allen-install-step');
      assert.equal(steps.length, 2);
      assert.match(steps[0].textContent, /tap Share/);
      assert.match(steps[1].textContent, /Add to Home Screen/);
      const icons = w.doc.documentElement.all(n => n.tagName === 'svg' && n.getAttribute('role') === 'img');
      assert.equal(icons.length, 2);
      assert.ok(icons.every(i => i.ns === 'http://www.w3.org/2000/svg' && i.getAttribute('aria-label')));
      assert.equal(w.doc.documentElement.all(n => n.tagName === 'ol')[0].getAttribute('role'), 'list');
      assert.equal(w.store.get('allen-install'), 'guided');
      assert.doesNotMatch(w.offers()[0].textContent, /installed|added you|is on your home screen/);
    }
  });
  await test('no offer where no honest install path exists', async () => {
    for (const ua of ['crios', 'firefox', 'android']) {
      const w = world({ ua });
      w.exchange();
      assert.equal(w.offers().length, 0, ua);
    }
  });
  await test('installed or standalone pages never offer', async () => {
    for (const opts of [{ display: 'standalone' }, { display: 'minimal-ui' }, { ua: 'iphone', iosStandalone: true }, { store: new Map([['allen-install', 'installed']]) }]) {
      const w = world(opts);
      w.bip();
      w.exchange();
      assert.equal(w.offers().length, 0, JSON.stringify(opts.display || opts.ua || 'stored'));
    }
  });
  await test('storage unavailable or full never throws and still remembers for the page', async () => {
    for (const opts of [{ noStorage: true }, { quota: true }]) {
      const w = world(opts);
      const e = w.bip();
      w.exchange();
      assert.equal(w.offers().length, 1);
      w.button('No thanks').click();
      w.bip();
      w.exchange();
      assert.equal(w.offers().length, 0);
      assert.equal(e.prompts, 0);
    }
  });
  await test('late transport: connects on allen-ready', async () => {
    const w = world({ ready: false });
    w.bip();
    w.boot();
    w.exchange();
    assert.equal(w.offers().length, 1);
  });
  await test('artifact has no dashes, markup parsing, network or outside resources', async () => {
    const css = fs.readFileSync('public/assets/allen-install.css', 'utf8');
    for (const text of [SOURCE, css]) assert.doesNotMatch(text, /[\u2013\u2014]|innerHTML|insertAdjacentHTML|fetch\(|XMLHttpRequest|https?:\/\/(?!www\.w3\.org\/2000\/svg)|@import|url\(/);
  });
  for (const name of results) console.log(`PASS ${name}`);
  console.log(`${results.length} passed`);
})().catch(error => { console.error(error); process.exit(1); });

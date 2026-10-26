/* Generated from om/allen-install/install.om; never hand-edit.
 *
 * The offer to put Aien on the home screen. It appears only after the first
 * exchange the visitor started that ended with a real reply, never on load or
 * on the greeting. The browser install prompt opens only inside the visitor's
 * tap on the offer. One local preference (dismissed, accepted, guided or
 * installed) stops further offers; nothing else is stored and nothing is sent.
 * Success is reported only when the browser fires appinstalled.
 */
(() => {
'use strict';
if (window.AllenInstall) return;
const KEY = 'allen-install';
const TITLE = 'Put me on your home screen';
const SVG = 'http://www.w3.org/2000/svg';
let deferred = null, earned = false, asked = false, open = null, memo = '', connected = false;
function remembered() {
 if (memo) return memo;
 try { return window.localStorage.getItem(KEY) || ''; } catch (_) { return ''; }
}
function remember(value) {
 memo = value;
 try { window.localStorage.setItem(KEY, value); } catch (_) {}
}
function installed() {
 try { if (window.navigator.standalone === true) return true; } catch (_) {}
 try {
  for (const mode of ['standalone', 'minimal-ui', 'fullscreen', 'window-controls-overlay']) if (window.matchMedia(`(display-mode: ${mode})`).matches) return true;
 } catch (_) {}
 return false;
}
function appleSafari() {
 const ua = String(window.navigator.userAgent || '');
 const apple = /iPhone|iPad|iPod/.test(ua) || (/Macintosh/.test(ua) && window.navigator.maxTouchPoints > 1);
 return apple && /Safari/.test(ua) && !/CriOS|FxiOS|EdgiOS|OPiOS|GSA|FBAN|FBAV|Instagram/.test(ua);
}
function el(tag, cls, text) {
 const node = document.createElement(tag);
 if (cls) node.className = cls;
 if (text) node.textContent = text;
 return node;
}
function button(cls, text, fn) {
 const node = el('button', cls, text);
 node.type = 'button';
 node.addEventListener('click', fn);
 return node;
}
function icon(label, lines) {
 const svg = document.createElementNS(SVG, 'svg');
 svg.setAttribute('viewBox', '0 0 24 24');
 svg.setAttribute('width', '28');
 svg.setAttribute('height', '28');
 svg.setAttribute('class', 'allen-install-icon');
 if (label) {
  svg.setAttribute('role', 'img');
  svg.setAttribute('aria-label', label);
 } else svg.setAttribute('aria-hidden', 'true');
 for (const d of lines) {
  const path = document.createElementNS(SVG, 'path');
  path.setAttribute('d', d);
  svg.appendChild(path);
 }
 return svg;
}
function step(svg, before, strong, after) {
 const item = el('li', 'allen-install-step');
 const words = el('span', 'allen-install-words');
 words.append(before, el('strong', '', strong), after);
 item.append(svg, words);
 return item;
}
function guide() {
 const list = el('ol', 'allen-install-steps');
 list.setAttribute('role', 'list');
 list.append(
  step(icon('Share icon: a square with an arrow pointing up', ['M8 10H6v10h12V10h-2', 'M12 15V3', 'M8 7l4-4 4 4']), 'In Safari, tap ', 'Share', '. If you do not see it, tap the three dots first.'),
  step(icon('Add to Home Screen icon: a square with a plus', ['M7 4h10a3 3 0 0 1 3 3v10a3 3 0 0 1-3 3H7a3 3 0 0 1-3-3V7a3 3 0 0 1 3-3z', 'M12 8v8', 'M8 12h8']), 'Choose ', 'Add to Home Screen', ', then tap Add.')
 );
 return list;
}
function offer() {
 if (open || !earned || remembered() || installed()) return;
 if (!deferred && !appleSafari()) return;
 const box = el('div', 'allen-install');
 const lede = el('p', 'allen-install-lede', 'Keep me one tap away. Your browser adds me to your home screen, and you can remove me the way you remove any app.');
 const state = el('p', 'allen-install-state');
 state.setAttribute('role', 'status');
 const actions = el('div', 'allen-install-actions');
 const card = { box, state, actions, buttons: [], handle: null, inline: null, watch: null, closing: false };
 open = card;
 card.buttons.push(button('allen-pane-act allen-pane-run allen-install-yes', TITLE, () => accept(card)), button('allen-pane-act allen-install-no', 'No thanks', () => dismiss(card)));
 actions.append(...card.buttons);
 box.append(lede, actions, state);
 place(card);
}
function dismiss(card) {
 if (!remembered()) remember('dismissed');
 shut(card);
}
function finish(card, text) {
 card.state.textContent = text;
 card.actions.replaceChildren(button('allen-pane-act allen-install-done', 'Close', () => shut(card)));
}
function accept(card) {
 const event = deferred;
 if (event) {
  deferred = null;
  for (const b of card.buttons) b.disabled = true;
  let asking;
  try { asking = Promise.resolve(event.prompt()); } catch (_) { asking = Promise.reject(new Error('prompt')); }
  asking.then(() => event.userChoice).then(choice => {
   if (remembered() === 'installed') return;
   if (choice && choice.outcome === 'accepted') {
    remember('accepted');
    finish(card, 'Your browser is adding me. I will say so here when it confirms.');
   } else {
    remember('dismissed');
    finish(card, 'No problem. I will not ask again on this device.');
   }
  }, () => finish(card, 'Your browser did not open its install prompt, so nothing changed.'));
  return;
 }
 if (appleSafari()) {
  card.box.insertBefore(guide(), card.actions);
  remember('guided');
  finish(card, 'I cannot see your home screen from here, so I will not offer this again on this device.');
 }
}
function shut(card) {
 if (card.closing) return;
 card.closing = true;
 if (card.watch) card.watch.disconnect();
 if (open === card) open = null;
 if (card.handle && typeof card.handle.close === 'function') card.handle.close();
 if (card.inline) card.inline.remove();
 else card.box.remove();
}
function place(card) {
 const detail = { id: 'install', title: TITLE, node: card.box, focus: false, handle: null };
 const request = new CustomEvent('allen-pane-request', { cancelable: true, detail });
 if (!document.dispatchEvent(request) && detail.handle && detail.handle.ok) card.handle = detail.handle;
 else {
  const aside = el('aside', 'allen-install-inline');
  aside.setAttribute('aria-label', TITLE);
  const head = el('div', 'allen-install-head');
  const close = button('allen-install-close', '', () => dismiss(card));
  close.setAttribute('aria-label', 'Close, and do not offer this again');
  close.appendChild(icon('', ['M6 6l12 12', 'M18 6L6 18']));
  head.append(el('p', 'allen-install-title', TITLE), close);
  aside.append(head, card.box);
  const answer = document.getElementById('allen-answer');
  const spot = document.getElementById('allen-install-slot') || (answer && answer.parentNode) || document.getElementById('allen') || document.body;
  spot.appendChild(aside);
  card.inline = aside;
 }
 if (typeof MutationObserver === 'function') {
  card.watch = new MutationObserver(() => {
   if (!card.box.isConnected && !card.closing) dismiss(card);
  });
  card.watch.observe(document.documentElement, { childList: true, subtree: true });
 }
}
function connect() {
 const api = window.allen;
 if (connected || !api || typeof api.on !== 'function') return;
 connected = true;
 api.on('send', event => { asked = !event.silent; });
 api.on('reply', event => {
  const good = asked && !event.outcome && typeof event.text === 'string' && event.text.trim() !== '';
  asked = false;
  if (!good) return;
  earned = true;
  offer();
 });
}
window.addEventListener('beforeinstallprompt', event => {
 event.preventDefault();
 deferred = event;
 offer();
});
window.addEventListener('appinstalled', () => {
 deferred = null;
 remember('installed');
 if (open) finish(open, 'Your browser says I am installed. Open me from your home screen or app list.');
});
document.addEventListener('allen-ready', connect);
connect();
window.AllenInstall = Object.freeze({ version: 1 });
})();

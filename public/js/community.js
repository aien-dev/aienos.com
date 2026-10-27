'use strict';
(() => {
  const root = document.getElementById('cm');
  if (!root) return;
  const API = root.dataset.api || '/community/api';
  const OMEGA = '/aien/api/omega';
  const OMEGA_MAX = 16384;
  const CALL_MS = 3000;
  const MAX = 4000;
  const AVATAR = 'https://avatars.githubusercontent.com/';
  const PROJECT_RE = /^[A-Za-z0-9][A-Za-z0-9._\/-]{0,199}$/;
  const HANDLE_RE = /^[a-z0-9_-]{1,40}$/;
  const JOIN_RE = /^[a-z][a-z0-9-]{2,19}$/;
  const CODE_RE = /^[0-9A-Za-z]{8,64}$/;
  const LOGIN_RE = /^[A-Za-z0-9-]{1,39}$/;
  const $ = (id) => document.getElementById(id);
  const ui = {
    title: $('cm-title'), account: $('cm-account'), search: $('cm-search'), q: $('cm-q'),
    tabs: $('cm-tabs'), intro: $('cm-intro'), compose: $('cm-compose'), text: $('cm-text'),
    count: $('cm-count'), post: $('cm-post'), project: $('cm-project'), attach: $('cm-attach'),
    mode: $('cm-mode'), modeText: $('cm-mode-text'), modeCancel: $('cm-mode-cancel'),
    status: $('cm-status'), fresh: $('cm-fresh'), head: $('cm-head'), feed: $('cm-feed'),
    older: $('cm-older'), report: $('cm-report'), reportForm: $('cm-report-form'),
    reportWhat: $('cm-report-what'), reportNote: $('cm-report-note'), confirm: $('cm-confirm'),
    confirmText: $('cm-confirm-text'), confirmReason: $('cm-confirm-reason'),
    confirmInput: $('cm-confirm-input'), confirmOk: $('cm-confirm-ok'), keys: $('cm-keys'),
    signin: $('cm-signin'), revoke: $('cm-revoke'), revokeForm: $('cm-revoke-form'),
    revokeText: $('cm-revoke-text'), revokeCascade: $('cm-revoke-cascade'),
    revokeWarn: $('cm-revoke-warn'), revokeReason: $('cm-revoke-reason')
  };
  const composeHome = ui.compose.parentNode;
  const composeNext = ui.status;
  const state = {
    me: null, csrf: '', unread: 0, github: false, canInvite: false, invitesLeft: 0, route: null, next: null, loader: null,
    item: null, seen: new Set(), top: null, fresh: [], mode: null, reportTarget: null,
    epoch: 0, live: null, freshTimer: 0
  };

  function el(tag, cls, text) {
    const node = document.createElement(tag);
    if (cls) node.className = cls;
    if (text !== undefined && text !== null) node.textContent = String(text);
    return node;
  }
  function link(href, text, cls) {
    const a = el('a', cls, text);
    a.href = href;
    return a;
  }
  function say(text, bad) {
    ui.status.textContent = text || '';
    ui.status.classList.toggle('cm-bad', !!bad);
  }
  function plural(n, one, many) {
    return n + ' ' + (n === 1 ? one : many);
  }
  function bytes(text) {
    return new TextEncoder().encode(text).length;
  }
  function handleOk(h) {
    return typeof h === 'string' && HANDLE_RE.test(h);
  }
  function userUrl(h) {
    return '/community/?u=' + encodeURIComponent(h);
  }
  function postUrl(id) {
    return '/community/?p=' + encodeURIComponent(id);
  }

  async function api(path, body) {
    const init = { credentials: 'same-origin', cache: 'no-store', headers: { Accept: 'application/json' } };
    if (body !== undefined) {
      init.method = 'POST';
      init.headers['Content-Type'] = 'application/json';
      init.headers['X-CM-CSRF'] = state.csrf;
      init.body = JSON.stringify(body);
    }
    let reply;
    try {
      reply = await fetch(API + path, init);
    } catch (error) {
      return { ok: false, status: 0, data: { message: 'The community server is not reachable right now.' } };
    }
    let data = {};
    try {
      data = await reply.json();
    } catch (error) {
      data = {};
    }
    if (!data || typeof data !== 'object') data = {};
    return { ok: reply.ok, status: reply.status, data };
  }
  function failed(result, what) {
    const d = result.data || {};
    let text = typeof d.message === 'string' && d.message ? d.message : what + ' did not work.';
    if (result.status === 429) {
      const wait = Number(d.retry_after);
      text = 'Slow down a little. ' + (wait > 0 ? 'Try again in ' + plural(Math.ceil(wait), 'second', 'seconds') + '.' : 'Try again soon.');
    } else if (result.status === 401 && !d.message) {
      text = 'Sign in first.';
    }
    say(text, true);
  }

  function when(iso) {
    const t = Date.parse(iso);
    if (!Number.isFinite(t)) return '';
    const s = Math.max(0, Math.round((Date.now() - t) / 1000));
    if (s < 45) return 'now';
    if (s < 3600) return Math.round(s / 60) + 'm';
    if (s < 86400) return Math.round(s / 3600) + 'h';
    if (s < 604800) return Math.round(s / 86400) + 'd';
    const d = new Date(t);
    const opts = { month: 'short', day: 'numeric' };
    if (d.getFullYear() !== new Date().getFullYear()) opts.year = 'numeric';
    return d.toLocaleDateString(undefined, opts);
  }
  function stamp(iso) {
    const time = el('time', null, when(iso));
    if (typeof iso === 'string') {
      time.dateTime = iso;
      const t = Date.parse(iso);
      if (Number.isFinite(t)) time.title = new Date(t).toLocaleString();
    }
    return time;
  }
  function blankAvatar(user, size) {
    const name = user && (user.display_name || user.handle) || '?';
    const blank = el('span', 'cm-avatar cm-avatar-blank', String(name).slice(0, 1).toUpperCase());
    blank.setAttribute('aria-hidden', 'true');
    blank.style.width = size + 'px';
    blank.style.height = size + 'px';
    blank.style.fontSize = Math.round(size * 0.44) + 'px';
    return blank;
  }
  function avatar(user, size) {
    const url = user && typeof user.avatar === 'string' ? user.avatar : '';
    if (!url.startsWith(AVATAR)) return blankAvatar(user, size);
    const img = el('img', 'cm-avatar');
    img.alt = '';
    img.width = size;
    img.height = size;
    img.loading = 'lazy';
    img.decoding = 'async';
    img.referrerPolicy = 'no-referrer';
    img.addEventListener('error', () => img.replaceWith(blankAvatar(user, size)), { once: true });
    img.src = url;
    return img;
  }
  function chips(user) {
    const out = [];
    if (!user) return out;
    if (user.is_bot && user.handle === 'aien') out.push(['Site assistant', 'cm-chip-aien']);
    else if (user.is_bot) out.push([handleOk(user.bot_owner) ? 'Bot, run by @' + user.bot_owner : 'Bot', 'cm-chip-bot']);
    if (user.role === 'moderator' || user.role === 'admin') out.push(['Moderator', 'cm-chip-mod']);
    if (user.vouch_revoked) out.push(['Vouch revoked', 'cm-chip-revoked']);
    const chipsOut = out.map(([text, cls]) => el('span', 'cm-chip ' + cls, text));
    if (typeof user.github_login === 'string' && LOGIN_RE.test(user.github_login)) {
      const gh = link('https://github.com/' + user.github_login, 'Verified GitHub', 'cm-chip cm-chip-github');
      gh.rel = 'noopener noreferrer';
      gh.target = '_blank';
      gh.title = 'Linked to github.com/' + user.github_login;
      chipsOut.push(gh);
    }
    return chipsOut;
  }
  function nameLine(user, cls) {
    const line = el('span', cls || 'cm-who');
    const h = user && handleOk(user.handle) ? user.handle : '';
    const name = link(h ? userUrl(h) : '/community/', user && user.display_name || h || 'deleted', 'cm-name');
    line.append(name, ' ', el('span', 'cm-handle', h ? '@' + h : ''));
    for (const chip of chips(user)) line.append(' ', chip);
    return line;
  }
  function body(html, cls) {
    const box = el('div', cls || 'cm-body');
    box.innerHTML = typeof html === 'string' ? html : '';
    for (const pre of box.querySelectorAll('pre.cm-code')) {
      const lang = (pre.dataset.lang || '').toLowerCase();
      if (lang === 'om' || lang === 'omega') runner(pre);
    }
    return box;
  }
  function count(n) {
    const v = Number(n) || 0;
    if (v < 1000) return String(v);
    if (v < 1000000) return (Math.floor(v / 100) / 10) + 'k';
    return (Math.floor(v / 100000) / 10) + 'M';
  }

  function actionButton(cls, label, value, pressed) {
    const b = el('button', 'cm-act ' + cls);
    b.type = 'button';
    b.dataset.label = label;
    b.append(el('span', 'cm-act-icon'), el('span', 'cm-act-n', count(value)));
    b.setAttribute('aria-label', label + ', ' + (Number(value) || 0));
    if (pressed !== undefined) b.setAttribute('aria-pressed', pressed ? 'true' : 'false');
    return b;
  }
  function setCount(b, value, pressed) {
    b.querySelector('.cm-act-n').textContent = count(value);
    b.setAttribute('aria-label', b.dataset.label + ', ' + (Number(value) || 0));
    if (pressed !== undefined) b.setAttribute('aria-pressed', pressed ? 'true' : 'false');
  }

  function quoteCard(q) {
    const box = el('div', 'cm-quote');
    if (!q || q.deleted) {
      box.append(el('p', 'cm-note', 'This post was deleted.'));
      return box;
    }
    const top = el('div', 'cm-quote-top');
    top.append(avatar(q.author, 20), nameLine(q.author), ' ', link(postUrl(q.id), '', 'cm-time'));
    top.lastChild.append(stamp(q.created));
    box.append(top, body(q.html, 'cm-body cm-quote-body'));
    return box;
  }

  function card(p, opts) {
    opts = opts || {};
    const li = el('li', 'cm-item');
    const art = el('article', 'cm-post' + (opts.focal ? ' cm-focal' : '') + (p.hidden ? ' cm-hidden-post' : ''));
    art.tabIndex = -1;
    art.dataset.id = p.id;
    li.append(art);
    if (p.deleted) {
      art.append(el('p', 'cm-note', 'This post was deleted.'));
      return li;
    }
    const a = p.author || {};
    if (p.repost_by && handleOk(p.repost_by.handle || p.repost_by)) {
      const h = p.repost_by.handle || p.repost_by;
      const line = el('p', 'cm-context');
      line.append('Reposted by ', link(userUrl(h), '@' + h));
      art.append(line);
    }
    const row = el('div', 'cm-row');
    const side = link(handleOk(a.handle) ? userUrl(a.handle) : '/community/', '', 'cm-side');
    side.setAttribute('aria-label', (a.display_name || a.handle || 'Author') + ' profile');
    side.append(avatar(a, opts.focal ? 48 : 40));
    const main = el('div', 'cm-main');
    const head = el('div', 'cm-post-head');
    const time = link(postUrl(p.id), '', 'cm-time');
    time.append(stamp(p.created));
    head.append(nameLine(a), el('span', 'cm-dot', ' . '), time);
    main.append(head);
    if (p.reply_to && !opts.inThread) {
      const line = el('p', 'cm-context');
      const h = handleOk(p.reply_to_author) ? p.reply_to_author : '';
      line.append('Replying to ', h ? link(userUrl(h), '@' + h) : 'a post', ' ', link(postUrl(p.reply_to), 'see thread'));
      main.append(line);
    }
    if (p.hidden) main.append(el('p', 'cm-note', 'Hidden by a moderator. Only moderators and the author see it.'));
    main.append(body(p.html));
    if (p.quote || p.quote_of) main.append(quoteCard(p.quote));
    if (typeof p.project === 'string' && PROJECT_RE.test(p.project) && !p.project.includes('..')) {
      const proj = el('p', 'cm-project');
      const href = 'https://github.com/aien-dev/omega-projects/tree/main/' + p.project.split('/').map(encodeURIComponent).join('/');
      const pa = link(href, p.project);
      pa.rel = 'nofollow ugc noopener noreferrer';
      pa.target = '_blank';
      proj.append('Project ', pa);
      main.append(proj);
    }
    const bar = el('div', 'cm-actions');
    bar.setAttribute('role', 'group');
    bar.setAttribute('aria-label', 'Post actions');
    const reply = actionButton('cm-reply', 'Reply', p.replies);
    const repost = actionButton('cm-repost', 'Repost', p.reposts, !!p.reposted);
    const like = actionButton('cm-like', 'Like', p.likes, !!p.liked);
    const share = el('button', 'cm-act cm-share');
    share.type = 'button';
    share.setAttribute('aria-label', 'Share');
    share.append(el('span', 'cm-act-icon'));
    const more = el('button', 'cm-act cm-more', '...');
    more.type = 'button';
    more.setAttribute('aria-label', 'More');
    more.setAttribute('aria-haspopup', 'true');
    more.setAttribute('aria-expanded', 'false');
    bar.append(reply, repost, like, share, more);
    main.append(bar);
    row.append(side, main);
    art.append(row);
    reply.addEventListener('click', () => startMode('reply', p));
    like.addEventListener('click', () => toggle(p, like, 'liked', 'likes', 'like', 'unlike'));
    repost.addEventListener('click', () => toggle(p, repost, 'reposted', 'reposts', 'repost', 'unrepost'));
    share.addEventListener('click', () => sharePost(p));
    more.addEventListener('click', (event) => {
      event.stopPropagation();
      postMenu(p, more, li);
    });
    art.addEventListener('click', (event) => {
      if (opts.focal || event.target.closest('a,button,pre,input,textarea,select,summary,.cm-menu-pop')) return;
      if (window.getSelection && String(window.getSelection())) return;
      go(postUrl(p.id));
    });
    return li;
  }

  async function toggle(p, button, flag, field, on, off) {
    if (!state.me) return say('Sign in to ' + on + ' posts.', true);
    const was = !!p[flag];
    p[flag] = !was;
    p[field] = Math.max(0, (Number(p[field]) || 0) + (was ? -1 : 1));
    setCount(button, p[field], p[flag]);
    const result = await api('/posts/' + encodeURIComponent(p.id) + '/' + (was ? off : on), {});
    if (!result.ok) {
      p[flag] = was;
      p[field] = Math.max(0, (Number(p[field]) || 0) + (was ? 1 : -1));
      setCount(button, p[field], p[flag]);
      failed(result, 'That');
    }
  }
  async function sharePost(p) {
    const url = new URL(postUrl(p.id), location.origin).href;
    try {
      if (navigator.share) {
        await navigator.share({ url });
        return;
      }
      await navigator.clipboard.writeText(url);
      say('Link copied.');
    } catch (error) {
      if (error && error.name === 'AbortError') return;
      say('Copy this link: ' + url);
    }
  }

  let openMenu = null;
  function closeMenu(refocus) {
    if (!openMenu) return;
    const { pop, button } = openMenu;
    openMenu = null;
    pop.remove();
    button.setAttribute('aria-expanded', 'false');
    if (refocus) button.focus();
  }
  function menu(button, items) {
    closeMenu(false);
    const pop = el('div', 'cm-menu-pop');
    pop.setAttribute('role', 'menu');
    for (const [label, fn, danger] of items) {
      const item = el('button', danger ? 'cm-danger-text' : null, label);
      item.type = 'button';
      item.setAttribute('role', 'menuitem');
      item.addEventListener('click', (event) => {
        event.stopPropagation();
        closeMenu(true);
        fn();
      });
      pop.append(item);
    }
    pop.addEventListener('keydown', (event) => {
      const list = Array.from(pop.querySelectorAll('button'));
      const i = list.indexOf(document.activeElement);
      if (event.key === 'ArrowDown') {
        event.preventDefault();
        list[(i + 1) % list.length].focus();
      } else if (event.key === 'ArrowUp') {
        event.preventDefault();
        list[(i - 1 + list.length) % list.length].focus();
      } else if (event.key === 'Tab') {
        closeMenu(false);
      }
    });
    button.after(pop);
    button.setAttribute('aria-expanded', 'true');
    openMenu = { pop, button };
    const first = pop.querySelector('button');
    if (first) first.focus();
  }
  function isMod() {
    return !!state.me && (state.me.role === 'moderator' || state.me.role === 'admin');
  }
  function postMenu(p, button, li) {
    if (openMenu && openMenu.button === button) return closeMenu(true);
    const a = p.author || {};
    const mine = state.me && state.me.handle === a.handle;
    const items = [['Quote this post', () => startMode('quote', p)], ['Copy link', () => sharePost(p)]];
    if (state.me && !mine) {
      items.push(['Report post', () => openReport({ post: p.id }, 'the post by @' + a.handle)]);
      if (handleOk(a.handle) && a.handle !== 'aien') {
        items.push(['Mute @' + a.handle, () => relation(a.handle, 'mute', 'Muted @' + a.handle + '. Their posts leave your timelines.')]);
        items.push(['Block @' + a.handle, () => ask('Block @' + a.handle + '? They cannot reply to you, follow you or mention you.', 'Block', false).then((ok) => ok && relation(a.handle, 'block', 'Blocked @' + a.handle + '.'))]);
      }
    }
    if (mine) items.push(['Delete post', () => removePost(p, li), true]);
    if (isMod()) {
      items.push([p.hidden ? 'Unhide post' : 'Hide post', () => modAction('post:' + p.id, p.hidden ? 'unhide' : 'hide', (p.hidden ? 'Unhide' : 'Hide') + ' this post?').then((ok) => {
        if (ok) {
          p.hidden = !p.hidden;
          li.firstChild.classList.toggle('cm-hidden-post', p.hidden);
        }
      }), !p.hidden]);
    }
    if (!state.me) items.push(['Sign in to reply, report or mute', () => signIn(null)]);
    menu(button, items);
  }
  async function relation(h, verb, done) {
    const result = await api('/users/' + encodeURIComponent(h) + '/' + verb, {});
    if (result.ok) say(done);
    else failed(result, 'That');
    return result.ok;
  }
  async function removePost(p, li) {
    if (!(await ask('Delete this post? Replies stay and show it as deleted.', 'Delete', false))) return;
    const result = await api('/posts/' + encodeURIComponent(p.id) + '/delete', {});
    if (!result.ok) return failed(result, 'Deleting');
    li.remove();
    say('Post deleted.');
  }

  let confirmResolve = null;
  function ask(text, okLabel, withReason) {
    ui.confirmText.textContent = text;
    ui.confirmOk.textContent = okLabel;
    ui.confirmReason.hidden = !withReason;
    ui.confirmInput.value = '';
    ui.confirmInput.required = !!withReason;
    ui.confirm.returnValue = '';
    if (confirmResolve) confirmResolve(false);
    return new Promise((resolve) => {
      confirmResolve = resolve;
      ui.confirm.showModal();
      (withReason ? ui.confirmInput : ui.confirmOk).focus();
    });
  }
  ui.confirm.addEventListener('close', () => {
    const resolve = confirmResolve;
    confirmResolve = null;
    if (resolve) resolve(ui.confirm.returnValue === 'ok');
  });
  async function modAction(target, action, question, value) {
    if (!(await ask(question, 'Confirm', true))) return false;
    const payload = { target, action, reason_public: ui.confirmInput.value.trim().slice(0, 200) };
    if (value !== undefined) payload.value = value;
    const result = await api('/mod/action', payload);
    if (!result.ok) {
      failed(result, 'That action');
      return false;
    }
    say('Done. The action is in the public moderation log.');
    return true;
  }

  function openReport(target, what) {
    state.reportTarget = target;
    ui.reportForm.reset();
    ui.reportWhat.textContent = 'Reporting ' + what + '.';
    ui.report.returnValue = '';
    ui.report.showModal();
    const first = ui.report.querySelector('input[name=reason]');
    if (first) first.focus();
  }
  ui.report.addEventListener('close', async () => {
    const target = state.reportTarget;
    state.reportTarget = null;
    if (ui.report.returnValue !== 'send' || !target) return;
    const reason = ui.reportForm.elements.reason.value;
    if (!reason) return say('Pick a reason so a moderator knows what to look for.', true);
    const result = await api('/reports', Object.assign({ reason, note: ui.reportNote.value.slice(0, 500) }, target));
    if (result.ok) say('Thanks. A moderator will look at it.');
    else failed(result, 'Reporting');
  });

  function b64u(buffer) {
    const bytes2 = new Uint8Array(buffer);
    let s = '';
    for (let i = 0; i < bytes2.length; i += 1) s += String.fromCharCode(bytes2[i]);
    return btoa(s).replace(/\+/g, '-').replace(/\//g, '_').replace(/=+$/, '');
  }
  function unb64u(text) {
    const s = String(text || '').replace(/-/g, '+').replace(/_/g, '/');
    const bin = atob(s + '='.repeat((4 - s.length % 4) % 4));
    const out = new Uint8Array(bin.length);
    for (let i = 0; i < bin.length; i += 1) out[i] = bin.charCodeAt(i);
    return out.buffer;
  }
  function passkeysWork() {
    return !!(window.PublicKeyCredential && navigator.credentials && window.isSecureContext);
  }
  function creationOptions(o) {
    const pk = Object.assign({}, o.publicKey || o);
    pk.challenge = unb64u(pk.challenge);
    pk.user = Object.assign({}, pk.user, { id: unb64u(pk.user && pk.user.id) });
    if (Array.isArray(pk.excludeCredentials)) pk.excludeCredentials = pk.excludeCredentials.map((c) => Object.assign({}, c, { id: unb64u(c.id) }));
    return pk;
  }
  function requestOptions(o) {
    const pk = Object.assign({}, o.publicKey || o);
    pk.challenge = unb64u(pk.challenge);
    if (Array.isArray(pk.allowCredentials)) pk.allowCredentials = pk.allowCredentials.map((c) => Object.assign({}, c, { id: unb64u(c.id) }));
    return pk;
  }
  function credentialJson(cred) {
    const r = cred.response;
    const out = { id: cred.id, rawId: b64u(cred.rawId), type: cred.type, response: { clientDataJSON: b64u(r.clientDataJSON) } };
    if (r.attestationObject) out.response.attestationObject = b64u(r.attestationObject);
    if (r.authenticatorData) out.response.authenticatorData = b64u(r.authenticatorData);
    if (r.signature) out.response.signature = b64u(r.signature);
    if (r.userHandle) out.response.userHandle = b64u(r.userHandle);
    if (typeof r.getTransports === 'function') out.transports = r.getTransports();
    return out;
  }
  function ceremonyFailed(error) {
    let message = 'The passkey step did not finish on this device. Nothing changed.';
    if (error && error.name === 'NotAllowedError') message = 'The passkey prompt was closed or timed out. Nothing changed.';
    else if (error && error.name === 'InvalidStateError') message = 'This device already holds a passkey for this account.';
    return { ok: false, status: 0, data: { message } };
  }
  async function newPasskey(optionsPath, optionsBody, verifyPath) {
    if (!passkeysWork()) return { ok: false, status: 0, data: { message: 'This browser cannot make a passkey here. Try a current browser on a device with a screen lock.' } };
    const opts = await api(optionsPath, optionsBody);
    if (!opts.ok) return opts;
    let cred;
    try {
      cred = await navigator.credentials.create({ publicKey: creationOptions(opts.data) });
    } catch (error) {
      return ceremonyFailed(error);
    }
    if (!cred) return ceremonyFailed(null);
    return api(verifyPath, { credential: credentialJson(cred) });
  }
  async function signIn(button) {
    if (!passkeysWork()) return say('This browser cannot use passkeys here. Try a current browser on a device with a screen lock.', true);
    if (button) button.disabled = true;
    let result = await api('/auth/passkey/login/options', {});
    if (result.ok) {
      try {
        const cred = await navigator.credentials.get({ publicKey: requestOptions(result.data) });
        result = cred ? await api('/auth/passkey/login/verify', { credential: credentialJson(cred) }) : ceremonyFailed(null);
      } catch (error) {
        result = ceremonyFailed(error);
      }
    }
    if (button) button.disabled = false;
    if (!result.ok) return failed(result, 'Signing in');
    location.reload();
  }
  function signInButton(label) {
    const b = el('button', 'cm-signin', label || 'Sign in with a passkey');
    b.type = 'button';
    b.addEventListener('click', () => signIn(b));
    return b;
  }
  function githubButton(mode, label) {
    if (state.github) return link(API + '/auth/github/start?mode=' + mode, label, 'cm-ghost cm-button');
    const b = el('button', 'cm-ghost', label + ': coming soon');
    b.type = 'button';
    b.disabled = true;
    return b;
  }
  function renderAccount() {
    ui.account.textContent = '';
    ui.signin.textContent = '';
    const me = state.me;
    for (const node of root.querySelectorAll('[data-signed]')) node.hidden = !me;
    for (const node of root.querySelectorAll('[data-mod]')) node.hidden = !isMod();
    for (const a of root.querySelectorAll('a[data-profile]')) a.href = me ? userUrl(me.handle) : '/community/';
    if (!me) {
      ui.account.append(signInButton('Sign in'));
      ui.signin.append(signInButton(), githubButton('signin', 'Sign in with GitHub'));
      const hint = el('p', 'cm-hint', 'New here? Ask a member for an invite link. Reading is open to everyone.');
      if (state.github) hint.append(' An older GitHub account with public work can also ', link(API + '/auth/github/start?mode=signup', 'join with GitHub'), '.');
      ui.signin.append(hint);
      return;
    }
    const button = el('button', 'cm-me');
    button.type = 'button';
    button.setAttribute('aria-haspopup', 'true');
    button.setAttribute('aria-expanded', 'false');
    button.append(avatar(me, 28), el('span', 'cm-me-name', me.display_name || me.handle));
    button.setAttribute('aria-label', 'Account menu for @' + me.handle);
    button.addEventListener('click', (event) => {
      event.stopPropagation();
      if (openMenu && openMenu.button === button) return closeMenu(true);
      menu(button, [
        ['Your profile', () => go(userUrl(me.handle))],
        ['Invites and passkeys', () => go('/community/?settings=1')],
        ['Sign out', signOut]
      ]);
    });
    ui.account.append(button);
  }
  async function signOut() {
    const result = await api('/auth/logout', {});
    if (!result.ok && result.status !== 401) return failed(result, 'Signing out');
    location.assign('/community/');
  }
  function setBadge(n) {
    state.unread = Math.max(0, Number(n) || 0);
    for (const badge of root.querySelectorAll('[data-badge]')) {
      badge.hidden = state.unread === 0;
      badge.textContent = state.unread > 99 ? '99+' : String(state.unread);
      badge.setAttribute('aria-label', plural(state.unread, 'unread notification', 'unread notifications'));
    }
  }
  function composerState() {
    const me = state.me;
    const r = state.route;
    const shown = !!me && r && (r.kind === 'everyone' || r.kind === 'following' || r.kind === 'tag' || r.kind === 'thread');
    ui.compose.hidden = !shown;
    ui.intro.hidden = !!me || !r || (r.kind !== 'everyone' && r.kind !== 'following');
    if (!me) return;
    const locked = !!(me.suspended || me.vouch_revoked);
    ui.text.disabled = locked;
    ui.project.disabled = locked;
    if (locked) {
      ui.text.placeholder = me.vouch_revoked
        ? 'The vouch for your account was revoked, so it is read-only. You can still export and delete it.'
        : 'Your account is suspended and read-only. You can still export and delete it.';
    } else if (me.tier === 'probation') {
      ui.text.placeholder = state.mode ? 'Write your ' + (state.mode.kind === 'reply' ? 'reply' : 'post') : 'What are you building? (first week: 8 posts a day, 1 link each)';
    } else {
      ui.text.placeholder = state.mode ? 'Write your ' + (state.mode.kind === 'reply' ? 'reply' : 'post') : 'What are you building?';
    }
    counter();
  }
  function counter() {
    const used = bytes(ui.text.value);
    ui.count.textContent = used + ' / ' + MAX;
    ui.count.classList.toggle('cm-over', used > MAX);
    ui.post.disabled = ui.text.disabled || used === 0 || used > MAX || !ui.text.value.trim();
  }
  function startMode(kind, p) {
    if (!state.me) {
      say('Sign in to ' + kind + '.', true);
      return;
    }
    const a = p.author || {};
    state.mode = { kind, post: p };
    ui.mode.hidden = false;
    ui.modeText.textContent = (kind === 'reply' ? 'Replying to @' : 'Quoting @') + (a.handle || 'deleted');
    if (state.route.kind !== 'thread' || kind === 'quote') placeComposer(null);
    ui.compose.hidden = false;
    composerState();
    ui.compose.scrollIntoView({ block: 'nearest', behavior: 'smooth' });
    ui.text.focus();
  }
  function endMode() {
    state.mode = null;
    ui.mode.hidden = true;
    ui.modeText.textContent = '';
    composerState();
  }
  function placeComposer(after) {
    if (after) after.after(ui.compose);
    else composeHome.insertBefore(ui.compose, composeNext);
  }
  ui.modeCancel.addEventListener('click', () => {
    endMode();
    ui.text.focus();
  });
  ui.text.addEventListener('input', counter);
  ui.text.addEventListener('keydown', (event) => {
    if (event.key === 'Enter' && (event.ctrlKey || event.metaKey)) {
      event.preventDefault();
      if (!ui.post.disabled) ui.compose.requestSubmit();
    }
  });
  ui.compose.addEventListener('submit', async (event) => {
    event.preventDefault();
    const text = ui.text.value;
    if (!text.trim() || bytes(text) > MAX) return;
    const payload = { text };
    const project = ui.project.value.trim();
    if (project) {
      if (!PROJECT_RE.test(project) || project.includes('..')) {
        say('A project is a path inside aien-dev/omega-projects, like projects/your-handle/name.', true);
        ui.project.focus();
        return;
      }
      payload.project = project;
    }
    if (state.mode && state.mode.kind === 'reply') payload.reply_to = state.mode.post.id;
    if (state.mode && state.mode.kind === 'quote') payload.quote_of = state.mode.post.id;
    ui.post.disabled = true;
    say('Posting...');
    const result = await api('/posts', payload);
    if (!result.ok) {
      counter();
      return failed(result, 'Posting');
    }
    ui.text.value = '';
    ui.project.value = '';
    ui.attach.open = false;
    const made = result.data.post;
    const replying = state.mode && state.mode.kind === 'reply';
    endMode();
    say(replying ? 'Reply posted.' : 'Posted.');
    if (made && made.id !== undefined) {
      state.seen.add(String(made.id));
      const r = state.route;
      if (r.kind === 'thread' && replying) ui.feed.append(card(made, { inThread: true }));
      else if (r.kind === 'everyone' || r.kind === 'following') ui.feed.prepend(card(made));
    }
  });

  function route() {
    const s = new URLSearchParams(location.search);
    const one = (k) => (s.get(k) || '').trim();
    if (s.has('invite') && CODE_RE.test(one('invite'))) return { kind: 'join', code: one('invite') };
    if (s.has('claim') && CODE_RE.test(one('claim'))) return { kind: 'claim', code: one('claim') };
    if (one('github') === 'finish') return { kind: 'finish' };
    if (s.has('p') && one('p')) return { kind: 'thread', id: one('p') };
    if (s.has('u') && handleOk(one('u').toLowerCase())) return { kind: 'profile', handle: one('u').toLowerCase() };
    if (s.has('tag') && /^[A-Za-z0-9_]{1,50}$/.test(one('tag'))) return { kind: 'tag', tag: one('tag').toLowerCase() };
    if (s.has('q')) return { kind: 'search', q: one('q').slice(0, 200) };
    if (s.has('n')) return { kind: 'notifications' };
    if (s.has('mod')) return { kind: 'mod' };
    if (s.has('log')) return { kind: 'log' };
    if (s.has('settings')) return { kind: 'settings' };
    if (one('view') === 'following') return { kind: 'following' };
    return { kind: 'everyone' };
  }
  function go(url) {
    closeMenu(false);
    if (url !== location.pathname + location.search) history.pushState(null, '', url);
    show();
    window.scrollTo(0, 0);
    ui.title.focus({ preventScroll: true });
  }
  root.addEventListener('click', (event) => {
    if (event.defaultPrevented || event.button !== 0 || event.metaKey || event.ctrlKey || event.shiftKey || event.altKey) return;
    const a = event.target.closest('a');
    if (!a || !root.contains(a) || a.target === '_blank' || a.hasAttribute('download')) return;
    const url = new URL(a.href, location.href);
    if (url.origin !== location.origin || url.pathname !== '/community/') return;
    event.preventDefault();
    go(url.pathname + url.search);
  });
  window.addEventListener('popstate', show);
  document.addEventListener('click', (event) => {
    if (openMenu && !openMenu.pop.contains(event.target)) closeMenu(false);
  });

  const TITLES = {
    everyone: 'Community', following: 'Following', search: 'Search', notifications: 'Notifications',
    mod: 'Moderation queue', log: 'Moderation log', settings: 'Settings', thread: 'Post', profile: 'Profile', tag: 'Tag',
    join: 'Join', claim: 'Claim the admin account', finish: 'Join'
  };
  function navState(r) {
    const key = r.kind === 'tag' || r.kind === 'thread' ? '' : r.kind;
    for (const a of root.querySelectorAll('[data-nav]')) {
      const mine = a.dataset.nav === key && (key !== 'profile' || (state.me && r.handle === state.me.handle));
      if (mine) a.setAttribute('aria-current', 'page');
      else a.removeAttribute('aria-current');
    }
    for (const a of ui.tabs.querySelectorAll('[data-tab]')) {
      if (a.dataset.tab === r.kind) a.setAttribute('aria-current', 'page');
      else a.removeAttribute('aria-current');
    }
    ui.tabs.hidden = !(r.kind === 'everyone' || r.kind === 'following');
  }
  function reset(r) {
    state.epoch += 1;
    state.route = r;
    state.next = null;
    state.loader = null;
    state.item = null;
    state.seen = new Set();
    state.top = null;
    state.fresh = [];
    ui.fresh.hidden = true;
    ui.head.textContent = '';
    ui.feed.textContent = '';
    ui.older.hidden = true;
    ui.feed.setAttribute('aria-label', 'Posts');
    say('');
    if (state.mode) endMode();
    placeComposer(null);
    ui.title.textContent = TITLES[r.kind] || 'Community';
    ui.title.tabIndex = -1;
    document.title = (r.kind === 'everyone' ? 'Community' : ui.title.textContent + ' . Community') + ' | AienOS';
    navState(r);
    composerState();
  }
  async function list(loader, item, empty) {
    state.loader = loader;
    state.item = item;
    state.empty = empty;
    await more();
  }
  async function more() {
    const epoch = state.epoch;
    const loader = state.loader;
    if (!loader) return;
    ui.older.disabled = true;
    ui.feed.setAttribute('aria-busy', 'true');
    const result = await loader(state.next);
    if (epoch !== state.epoch) return;
    ui.feed.removeAttribute('aria-busy');
    ui.older.disabled = false;
    if (!result.ok) {
      failed(result, 'Loading');
      return;
    }
    const items = result.items || [];
    for (const it of items) {
      if (it && it.id !== undefined) {
        const key = String(it.id) + (it.repost_by ? '/' + (it.repost_by.handle || it.repost_by) : '');
        if (state.seen.has(key)) continue;
        state.seen.add(key);
        if (state.top === null) state.top = String(it.id);
      }
      const node = state.item(it);
      if (node) ui.feed.append(node);
    }
    state.next = result.next === undefined || result.next === null || result.next === '' ? null : result.next;
    ui.older.hidden = state.next === null;
    if (!ui.feed.children.length && state.empty) {
      const li = el('li', 'cm-empty');
      li.append(state.empty);
      ui.feed.append(li);
    }
  }
  ui.older.addEventListener('click', more);
  function page(path, key) {
    return async (before) => {
      const sep = path.includes('?') ? '&' : '?';
      const r = await api(path + (before !== null && before !== undefined ? sep + 'before=' + encodeURIComponent(before) : ''));
      const d = r.data || {};
      return { ok: r.ok, status: r.status, data: d, items: Array.isArray(d[key]) ? d[key] : [], next: d.next };
    };
  }
  function emptyText(text, href, label) {
    const p = el('p', 'cm-note', text);
    if (href) p.append(' ', link(href, label));
    return p;
  }
  function needSignIn(what) {
    const box = el('div', 'cm-head-line');
    box.append(el('p', 'cm-note', 'Sign in to see ' + what + '.'), signInButton());
    ui.head.append(box);
  }
  function joinView(r) {
    const claim = r.kind === 'claim';
    if (state.me) {
      ui.head.append(emptyText('You are signed in as @' + state.me.handle + ', so this link is not for you. Pass it on to the person it was meant for. To add a device, use', '/community/?settings=1', 'Settings'));
      return;
    }
    const form = el('form', 'cm-panel cm-form cm-join');
    const finish = r.kind === 'finish';
    form.append(el('h2', null, claim ? 'Register the first admin passkey' : finish ? 'Pick your handle' : 'A member vouched for you'));
    form.append(el('p', 'cm-note', claim
      ? 'This one-time code registers the first passkey for the admin account. It works once, within 48 hours of being printed.'
      : finish
        ? 'GitHub confirmed your account. Pick a handle within 10 minutes. You can add a passkey from Settings afterwards. The first 7 days are a probation week with smaller limits.'
        : 'Every account here was vouched for by a person, and this link is that vouch. Pick a handle and your device makes a passkey: no password, nothing to remember. The first 7 days are a probation week with smaller limits.'));
    let handle = null;
    let name = null;
    if (!claim) {
      handle = el('input');
      handle.required = true;
      handle.maxLength = 20;
      handle.pattern = '[a-z][a-z0-9\\-]{2,19}';
      handle.autocomplete = 'username';
      handle.autocapitalize = 'none';
      handle.spellcheck = false;
      name = el('input');
      name.required = true;
      name.maxLength = 50;
      name.autocomplete = 'name';
      form.append(field('Handle', handle, '3 to 20 characters: lowercase letters, digits and dashes, starting with a letter. It cannot change later.'),
        field('Display name', name, 'Up to 50 characters.'));
    }
    const make = el('button', 'cm-primary', claim ? 'Create the admin passkey' : finish ? 'Join' : 'Create my passkey and join');
    make.type = 'submit';
    const fine = el('p', 'cm-hint', 'Joining means you accept the ');
    fine.append(link('/community/terms/', 'terms'), ', the ', link('/community/policy/', 'rules'), ' and the ', link('/community/privacy/', 'privacy notes'), '.');
    form.append(make, fine);
    form.addEventListener('submit', async (event) => {
      event.preventDefault();
      const payload = claim ? { claim: r.code } : { handle: handle.value.trim(), display_name: name.value.trim() };
      if (!claim && !finish) payload.invite = r.code;
      if (!claim && !JOIN_RE.test(payload.handle)) return say('A handle is 3 to 20 characters: lowercase letters, digits and dashes, starting with a letter.', true);
      make.disabled = true;
      const result = finish
        ? await api('/auth/github/finish', payload)
        : await newPasskey('/auth/passkey/register/options', payload, '/auth/passkey/register/verify');
      make.disabled = false;
      if (!result.ok) return failed(result, 'Joining');
      location.assign('/community/');
    });
    ui.head.append(form);
    if (handle) handle.focus({ preventScroll: true });
  }

  async function show() {
    const r = route();
    reset(r);
    if (r.kind === 'everyone' || r.kind === 'following') {
      if (r.kind === 'following' && !state.me) return needSignIn('posts from people you follow');
      await list(page('/timeline?view=' + r.kind + '&limit=40', 'posts'), (p) => card(p),
        r.kind === 'following' ? emptyText('Nobody you follow has posted yet. Find people in', '/community/', 'Everyone') : emptyText('No posts yet. Be the first.'));
      return;
    }
    if (r.kind === 'tag') {
      ui.title.textContent = '#' + r.tag;
      const p = el('p', 'cm-head-line', 'Posts tagged #' + r.tag + ', newest first. ');
      p.append(link('/community/feed/tag/' + encodeURIComponent(r.tag) + '.atom', 'Atom feed'));
      ui.head.append(p);
      return list(page('/tags/' + encodeURIComponent(r.tag), 'posts'), (x) => card(x), emptyText('No posts with this tag yet.'));
    }
    if (r.kind === 'thread') return thread(r);
    if (r.kind === 'profile') return profile(r);
    if (r.kind === 'search') return search(r);
    if (r.kind === 'notifications') return notifications();
    if (r.kind === 'mod') return modQueue();
    if (r.kind === 'log') return modLog();
    if (r.kind === 'settings') return settings();
    if (r.kind === 'join' || r.kind === 'claim' || r.kind === 'finish') return joinView(r);
  }

  async function thread(r) {
    const epoch = state.epoch;
    const result = await api('/posts/' + encodeURIComponent(r.id));
    if (epoch !== state.epoch) return;
    if (!result.ok || !result.data.post) {
      if (result.status === 404) return ui.head.append(emptyText('This post does not exist or was deleted.', '/community/', 'Back to the timeline'));
      return failed(result, 'Loading the post');
    }
    const d = result.data;
    ui.feed.setAttribute('aria-label', 'Thread');
    for (const p of Array.isArray(d.ancestors) ? d.ancestors : []) ui.feed.append(card(p, { inThread: true }));
    const focal = card(d.post, { focal: true, inThread: true });
    ui.feed.append(focal);
    if (state.me) {
      placeComposer(ui.feed);
      ui.compose.hidden = false;
      state.mode = { kind: 'reply', post: d.post };
      ui.mode.hidden = false;
      ui.modeText.textContent = 'Replying to @' + ((d.post.author || {}).handle || 'deleted');
      composerState();
    }
    const replies = Array.isArray(d.replies) ? d.replies : [];
    const rest = el('li', 'cm-sep', replies.length ? plural(replies.length, 'reply', 'replies') : 'No replies yet');
    rest.setAttribute('role', 'presentation');
    ui.feed.append(rest);
    for (const p of replies) ui.feed.append(card(p, { inThread: true }));
    const node = focal.firstChild;
    node.focus({ preventScroll: true });
    node.scrollIntoView({ block: 'center' });
  }
  ui.compose.addEventListener('reset', endMode);

  async function profile(r) {
    const epoch = state.epoch;
    const result = await api('/users/' + encodeURIComponent(r.handle));
    if (epoch !== state.epoch) return;
    if (!result.ok) {
      if (result.status === 404) return ui.head.append(emptyText('No account has the handle @' + r.handle + '.', '/community/', 'Back to the timeline'));
      return failed(result, 'Loading the profile');
    }
    const u = result.data.user || result.data;
    ui.title.textContent = u.display_name || '@' + u.handle;
    const box = el('section', 'cm-profile');
    box.setAttribute('aria-label', 'Profile of @' + u.handle);
    const top = el('div', 'cm-profile-top');
    top.append(avatar(u, 72));
    const who = el('div', 'cm-profile-who');
    who.append(el('h2', 'cm-profile-name', u.display_name || u.handle), el('p', 'cm-handle', '@' + u.handle));
    const tags = el('p', 'cm-chips');
    for (const chip of chips(u)) tags.append(chip, ' ');
    if (tags.childNodes.length) who.append(tags);
    top.append(who);
    const acts = el('div', 'cm-profile-acts');
    const mine = state.me && state.me.handle === u.handle;
    if (state.me && !mine) {
      const follow = el('button', u.followed ? 'cm-ghost' : 'cm-primary', u.followed ? 'Following' : 'Follow');
      follow.type = 'button';
      follow.setAttribute('aria-pressed', u.followed ? 'true' : 'false');
      follow.addEventListener('click', async () => {
        const verb = u.followed ? 'unfollow' : 'follow';
        follow.disabled = true;
        if (await relation(u.handle, verb, verb === 'follow' ? 'Following @' + u.handle + '.' : 'Unfollowed @' + u.handle + '.')) {
          u.followed = !u.followed;
          follow.textContent = u.followed ? 'Following' : 'Follow';
          follow.className = u.followed ? 'cm-ghost' : 'cm-primary';
          follow.setAttribute('aria-pressed', u.followed ? 'true' : 'false');
        }
        follow.disabled = false;
      });
      acts.append(follow);
      const moreBtn = el('button', 'cm-ghost', '...');
      moreBtn.type = 'button';
      moreBtn.setAttribute('aria-label', 'More about @' + u.handle);
      moreBtn.setAttribute('aria-haspopup', 'true');
      moreBtn.setAttribute('aria-expanded', 'false');
      moreBtn.addEventListener('click', (event) => {
        event.stopPropagation();
        if (openMenu && openMenu.button === moreBtn) return closeMenu(true);
        const items = [];
        if (u.handle !== 'aien') {
          items.push([u.muted ? 'Unmute' : 'Mute', async () => {
            if (await relation(u.handle, u.muted ? 'unmute' : 'mute', (u.muted ? 'Unmuted @' : 'Muted @') + u.handle + '.')) u.muted = !u.muted;
          }]);
          items.push([u.blocked ? 'Unblock' : 'Block', async () => {
            if (!u.blocked && !(await ask('Block @' + u.handle + '? They cannot reply to you, follow you or mention you.', 'Block', false))) return;
            if (await relation(u.handle, u.blocked ? 'unblock' : 'block', (u.blocked ? 'Unblocked @' : 'Blocked @') + u.handle + '.')) u.blocked = !u.blocked;
          }]);
        }
        items.push(['Report @' + u.handle, () => openReport({ user: u.handle }, '@' + u.handle)]);
        if (!u.vouch_revoked && u.handle !== 'aien' && (u.can_revoke || isMod())) items.push(['Revoke vouch for @' + u.handle, () => revokeVouch(u), true]);
        if (u.vouch_revoked && state.canInvite) items.push(['Vouch for @' + u.handle + ' (uses one invite)', () => revouch(u)]);
        if (isMod()) {
          items.push([u.suspended ? 'Unsuspend account' : 'Suspend account', () => modAction('user:' + u.handle, u.suspended ? 'unsuspend' : 'suspend', (u.suspended ? 'Unsuspend' : 'Suspend') + ' @' + u.handle + '?').then((ok) => {
            if (ok) u.suspended = !u.suspended;
          }), !u.suspended]);
          if (u.vouch_revoked) items.push(['Restore this branch of the invite tree', () => modAction('user:' + u.handle, 'restore_branch', 'Restore @' + u.handle + ' and everyone below it in the invite tree?')]);
          if (u.invites_frozen) items.push(['Unfreeze invites', () => modAction('user:' + u.handle, 'unfreeze_invites', 'Let @' + u.handle + ' invite again?')]);
          for (const tier of ['probation', 'member']) {
            items.push(['Set tier: ' + tier, () => modAction('user:' + u.handle, 'set_tier', 'Set @' + u.handle + ' to the ' + tier + ' tier?', tier)]);
          }
        }
        menu(moreBtn, items);
      });
      acts.append(moreBtn);
    } else if (mine) {
      acts.append(link('/community/?settings=1', 'Edit profile', 'cm-ghost cm-button'));
    }
    top.append(acts);
    box.append(top);
    if (u.bio) box.append(el('p', 'cm-bio', u.bio));
    const projects = Array.isArray(u.projects) ? u.projects.filter((x) => typeof x === 'string' && PROJECT_RE.test(x) && !x.includes('..')) : [];
    if (projects.length) {
      const ul = el('ul', 'cm-projects');
      ul.setAttribute('aria-label', 'Projects');
      for (const x of projects.slice(0, 6)) {
        const li = el('li');
        const a = link('https://github.com/aien-dev/omega-projects/tree/main/' + x.split('/').map(encodeURIComponent).join('/'), x);
        a.rel = 'nofollow ugc noopener noreferrer';
        a.target = '_blank';
        li.append(a);
        ul.append(li);
      }
      box.append(ul);
    }
    const facts = el('p', 'cm-facts');
    if (u.created) {
      const t = Date.parse(u.created);
      if (Number.isFinite(t)) facts.append('Joined ' + new Date(t).toLocaleDateString(undefined, { year: 'numeric', month: 'long' }) + '  ');
    }
    for (const [n, one, many, path] of [[u.posts, 'post', 'posts', ''], [u.following, 'following', 'following', 'following'], [u.followers, 'follower', 'followers', 'followers'], [u.invitee_count, 'invitee', 'invitees', 'invitees']]) {
      if (n === undefined) continue;
      const span = el('span', 'cm-fact');
      span.append(el('strong', null, count(n)), ' ' + (Number(n) === 1 ? one : many));
      if (path) {
        const b = el('button', 'cm-linkish');
        b.type = 'button';
        b.append(span);
        b.addEventListener('click', () => people(u.handle, path));
        facts.append(b, ' ');
      } else {
        facts.append(span, ' ');
      }
    }
    if (handleOk(u.inviter)) facts.append(el('span', 'cm-fact', 'Invited by '), link(userUrl(u.inviter), '@' + u.inviter), ' ');
    else if (u.handle === 'drake' || u.handle === 'aien') facts.append(el('span', 'cm-fact', 'Founding account'), ' ');
    facts.append(link('/community/feed/u/' + encodeURIComponent(u.handle) + '.atom', 'Atom feed'));
    box.append(facts);
    ui.head.append(box);
    ui.feed.setAttribute('aria-label', 'Posts by @' + u.handle);
    await list(page('/users/' + encodeURIComponent(u.handle) + '/posts', 'posts'), (p) => card(p), emptyText('@' + u.handle + ' has not posted yet.'));
  }
  function userRow(u) {
    const li = el('li', 'cm-item');
    const row = el('div', 'cm-user');
    row.append(avatar(u, 40));
    const main = el('div', 'cm-main');
    main.append(nameLine(u));
    if (u.bio) main.append(el('p', 'cm-bio cm-clamp', u.bio));
    row.append(main);
    li.append(row);
    return li;
  }
  async function people(h, which) {
    state.epoch += 1;
    ui.feed.textContent = '';
    state.seen = new Set();
    state.next = null;
    const LABELS = {
      followers: ['Followers of @' + h, 'Showing followers of @' + h + '.'],
      following: ['Accounts @' + h + ' follows', 'Showing accounts @' + h + ' follows.'],
      invitees: ['Accounts @' + h + ' invited', 'Showing accounts @' + h + ' vouched for.']
    };
    const [label, said] = LABELS[which] || LABELS.following;
    ui.feed.setAttribute('aria-label', label);
    say(said);
    await list(page('/users/' + encodeURIComponent(h) + '/' + which, 'users'), userRow, emptyText('Nobody here yet.'));
  }

  async function search(r) {
    ui.q.value = r.q;
    if (!r.q) {
      ui.head.append(emptyText('Search posts and people by any word. Try omega, cuda or gb10.'));
      ui.q.focus();
      return;
    }
    ui.title.textContent = 'Search: ' + r.q;
    let shownUsers = false;
    await list(async (before) => {
      const res = await api('/search?q=' + encodeURIComponent(r.q) + (before !== null && before !== undefined ? '&before=' + encodeURIComponent(before) : ''));
      const d = res.data || {};
      if (res.ok && !shownUsers && Array.isArray(d.users) && d.users.length) {
        shownUsers = true;
        const sec = el('section', 'cm-people');
        sec.append(el('h2', 'cm-sub', 'People'));
        const ul = el('ul', 'cm-feed');
        for (const u of d.users.slice(0, 5)) ul.append(userRow(u));
        sec.append(ul);
        ui.head.append(sec);
      }
      return { ok: res.ok, status: res.status, data: d, items: Array.isArray(d.posts) ? d.posts : [], next: d.next };
    }, (p) => card(p), emptyText('No posts match.'));
  }
  ui.search.addEventListener('submit', (event) => {
    event.preventDefault();
    go('/community/?q=' + encodeURIComponent(ui.q.value.trim()));
  });

  const NOTE = {
    like: 'liked your post', repost: 'reposted your post', reply: 'replied to you',
    mention: 'mentioned you', follow: 'followed you', quote: 'quoted your post',
    invite_used: 'joined with your invite', vouch_revoked: 'revoked the vouch for your account', vouch_restored: 'restored the vouch for your account'
  };
  function noteRow(n) {
    const li = el('li', 'cm-item cm-notice' + (n.read ? '' : ' cm-unread'));
    const line = el('p', 'cm-notice-line');
    const a = n.actor || {};
    line.append(avatar(a, 24), ' ', handleOk(a.handle) ? link(userUrl(a.handle), a.display_name || '@' + a.handle, 'cm-name') : 'Someone', ' ' + (NOTE[n.kind] || 'did something'), ' ');
    line.append(stamp(n.at));
    li.append(line);
    if (n.post && (n.kind === 'reply' || n.kind === 'mention' || n.kind === 'quote')) li.append(card(n.post));
    else if (n.post) {
      const p = el('p', 'cm-context');
      p.append(link(postUrl(n.post.id), 'Open the post'));
      li.append(p);
    }
    return li;
  }
  async function notifications() {
    if (!state.me) return needSignIn('your notifications');
    ui.feed.setAttribute('aria-label', 'Notifications');
    await list(page('/notifications', 'notifications'), noteRow, emptyText('Nothing yet. Likes, replies, mentions and follows show up here.'));
    if (state.unread > 0) {
      const result = await api('/notifications/read', {});
      if (result.ok) setBadge(0);
    }
  }

  function reportRow(rep) {
    const li = el('li', 'cm-item cm-report-item');
    const line = el('p', 'cm-notice-line');
    line.append(el('strong', null, rep.reason || 'report'), ' from ', handleOk(rep.reporter) ? link(userUrl(rep.reporter), '@' + rep.reporter) : 'an account', ' ');
    line.append(stamp(rep.at));
    li.append(line);
    if (rep.note) li.append(el('p', 'cm-quote-note', rep.note));
    const target = rep.target_kind === 'user' || (!rep.post && rep.user) ? 'user' : 'post';
    const handle = rep.user && (rep.user.handle || rep.user);
    if (target === 'post' && rep.post) li.append(card(rep.post));
    else if (handleOk(handle)) {
      const p = el('p', 'cm-context');
      p.append('Account ', link(userUrl(handle), '@' + handle));
      li.append(p);
    }
    const acts = el('div', 'cm-mod-acts');
    const add = (label, fn, cls) => {
      const b = el('button', cls || 'cm-ghost', label);
      b.type = 'button';
      b.addEventListener('click', async () => {
        if (await fn()) li.remove();
      });
      acts.append(b);
    };
    if (target === 'post' && rep.post) add('Hide post', () => modAction('post:' + rep.post.id, 'hide', 'Hide this post and close the report?'), 'cm-danger');
    if (handleOk(handle) || (rep.post && rep.post.author && handleOk(rep.post.author.handle))) {
      const h = handleOk(handle) ? handle : rep.post.author.handle;
      add('Suspend @' + h, () => modAction('user:' + h, 'suspend', 'Suspend @' + h + '?'), 'cm-danger');
      add('Probation', () => modAction('user:' + h, 'set_tier', 'Put @' + h + ' on probation?', 'probation'));
    }
    add('Dismiss', () => modAction('report:' + rep.id, 'dismiss', 'Dismiss this report? Nothing changes for the reported content.'));
    li.append(acts);
    return li;
  }
  async function modQueue() {
    if (!isMod()) {
      ui.head.append(emptyText('The moderation queue is for moderators. Everyone can read the', '/community/?log=1', 'moderation log'));
      return;
    }
    ui.feed.setAttribute('aria-label', 'Open reports');
    await list(page('/mod/queue', 'reports'), reportRow, emptyText('No open reports.'));
  }
  const ACTIONS = {
    hide: 'Hid a post', unhide: 'Restored a post', dismiss: 'Dismissed a report', suspend: 'Suspended an account',
    unsuspend: 'Lifted a suspension', set_tier: 'Changed an account tier', set_role: 'Changed an account role',
    revoke_vouch: 'Revoked a vouch', restore_branch: 'Restored an invite branch', freeze_invites: 'Paused invites',
    unfreeze_invites: 'Unpaused invites'
  };
  function logRow(e) {
    const li = el('li', 'cm-item cm-log');
    const line = el('p', 'cm-notice-line');
    line.append(el('strong', null, ACTIONS[e.action] || String(e.action || 'Action')), ' ');
    if (e.target_kind === 'post' && e.target !== undefined && e.target !== null) line.append(link(postUrl(e.target), 'post ' + e.target), ' ');
    else if (e.target_kind === 'report') line.append('report ', String(e.target), ' ');
    else line.append(String(e.target || 'an account'), ' ');
    line.append('by ', handleOk(e.by) ? link(userUrl(e.by), '@' + e.by) : 'a moderator', ' ');
    line.append(stamp(e.at));
    li.append(line);
    if (e.reason_public) li.append(el('p', 'cm-quote-note', e.reason_public));
    return li;
  }
  async function modLog() {
    const p = el('p', 'cm-head-line', 'Every moderator action, newest first, with its public reason. ');
    p.append(link('/community/policy/', 'How moderation works'));
    ui.head.append(p);
    ui.feed.setAttribute('aria-label', 'Moderation log');
    await list(page('/modlog', 'entries'), logRow, emptyText('No moderator actions yet.'));
  }

  function field(label, input, hint) {
    const wrap = el('div', 'cm-field');
    const id = 'cm-f-' + Math.random().toString(36).slice(2, 9);
    input.id = id;
    const l = el('label', null, label);
    l.htmlFor = id;
    wrap.append(l, input);
    if (hint) {
      const h = el('p', 'cm-hint', hint);
      h.id = id + '-h';
      input.setAttribute('aria-describedby', h.id);
      wrap.append(h);
    }
    return wrap;
  }
  async function copyText(text, what) {
    try {
      await navigator.clipboard.writeText(text);
      say(what + ' copied.');
    } catch (error) {
      say('Copying failed here. Select the text and copy it by hand.', true);
    }
  }
  function passkeyPanel(me) {
    const panel = el('section', 'cm-panel cm-form');
    panel.append(el('h2', null, 'Passkeys'), el('p', 'cm-note', 'A passkey lives on your device and signs you in with its screen lock. There is no password to leak. Add a second passkey on another device, so losing one phone never locks you out.'));
    const list2 = el('ul', 'cm-passkeys');
    const draw = async () => {
      const result = await api('/me/passkeys');
      list2.textContent = '';
      if (!result.ok) {
        list2.append(el('li', 'cm-note', 'Your passkeys could not be loaded right now.'));
        return;
      }
      const all = Array.isArray(result.data.passkeys) ? result.data.passkeys : [];
      for (const k of all) {
        const li = el('li', 'cm-key');
        const form = el('form', 'cm-key-row');
        const name = el('input');
        name.maxLength = 40;
        name.required = true;
        name.value = typeof k.name === 'string' && k.name ? k.name : 'Passkey';
        name.setAttribute('aria-label', 'Name of this passkey');
        const rename = el('button', 'cm-ghost', 'Rename');
        rename.type = 'submit';
        const remove = el('button', 'cm-ghost cm-danger', 'Remove');
        remove.type = 'button';
        if (all.length < 2 && !me.github_login) {
          remove.disabled = true;
          remove.title = 'Your only passkey cannot be removed. Add another one first.';
        }
        form.append(name, rename, remove);
        const meta = el('p', 'cm-hint', 'Added ');
        meta.append(stamp(k.created));
        if (k.last_used) meta.append(', last used ', stamp(k.last_used));
        form.addEventListener('submit', async (event) => {
          event.preventDefault();
          const result2 = await api('/me/passkeys/' + encodeURIComponent(k.id) + '/rename', { name: name.value.trim().slice(0, 40) });
          if (!result2.ok) return failed(result2, 'Renaming');
          say('Passkey renamed.');
        });
        remove.addEventListener('click', async () => {
          if (!(await ask('Remove the passkey "' + name.value + '"? That device can no longer sign in.', 'Remove', false))) return;
          const result2 = await api('/me/passkeys/' + encodeURIComponent(k.id) + '/remove', {});
          if (!result2.ok) return failed(result2, 'Removing');
          say('Passkey removed.');
          draw();
        });
        li.append(form, meta);
        list2.append(li);
      }
      if (all.length === 1) list2.append(el('li', 'cm-warn', 'You have one passkey. Add a second device now, so you can always get back in.'));
    };
    const add = el('button', 'cm-primary', 'Add a passkey');
    add.type = 'button';
    add.addEventListener('click', async () => {
      add.disabled = true;
      const result = await newPasskey('/me/passkeys/add/options', {}, '/me/passkeys/add/verify');
      add.disabled = false;
      if (!result.ok) return failed(result, 'Adding the passkey');
      say('Passkey added.');
      draw();
    });
    panel.append(list2, add);
    draw();
    return panel;
  }
  function inviteUrl(d) {
    if (d && typeof d.code === 'string' && CODE_RE.test(d.code)) return location.origin + '/community/?invite=' + d.code;
    return '';
  }
  function invitePanel(me) {
    const panel = el('section', 'cm-panel cm-form');
    panel.append(el('h2', null, 'Invites'), el('p', 'cm-note', 'Every account here was vouched for by a person, and an invite is your vouch. A link works once, for 14 days. If two people you invited are suspended for spam or abuse within 30 days, your invites pause until a moderator looks.'));
    const list2 = el('ul', 'cm-invites');
    const draw = async () => {
      const result = await api('/me/invites');
      list2.textContent = '';
      if (!result.ok) {
        list2.append(el('li', 'cm-note', 'Your invites could not be loaded right now.'));
        return;
      }
      const all = Array.isArray(result.data.invites) ? result.data.invites : [];
      for (const v of all) {
        const li = el('li', 'cm-invite');
        const status = typeof v.status === 'string' ? v.status : 'open';
        li.append(el('span', 'cm-chip cm-chip-' + (status === 'open' ? 'mod' : 'bot'), status), ' ', el('span', 'cm-invite-note', v.note || 'No note'), ' ');
        if (handleOk(v.used_by)) li.append('joined as ', link(userUrl(v.used_by), '@' + v.used_by), ' ');
        if (status === 'open') {
          const meta = el('span', 'cm-hint', 'expires ');
          meta.append(stamp(v.expires));
          const cancel = el('button', 'cm-ghost', 'Cancel');
          cancel.type = 'button';
          cancel.addEventListener('click', async () => {
            const result2 = await api('/invites/' + encodeURIComponent(v.id) + '/cancel', {});
            if (!result2.ok) return failed(result2, 'Cancelling');
            say('Invite cancelled.');
            draw();
          });
          li.append(meta, ' ', cancel);
        }
        list2.append(li);
      }
      if (!all.length) list2.append(el('li', 'cm-note', 'No invites yet.'));
    };
    if (state.canInvite) {
      const form = el('form', 'cm-form-inner');
      const note = el('input');
      note.maxLength = 80;
      const make = el('button', 'cm-primary', 'Create an invite link');
      make.type = 'submit';
      const out = el('div', 'cm-invite-out');
      out.hidden = true;
      form.append(field('Note to yourself', note, 'Who it is for. Only you see this. ' + plural(state.invitesLeft, 'invite', 'invites') + ' left this week.'), make, out);
      form.addEventListener('submit', async (event) => {
        event.preventDefault();
        make.disabled = true;
        const result = await api('/invites', { note: note.value.trim().slice(0, 80) });
        make.disabled = false;
        if (!result.ok) return failed(result, 'Creating the invite');
        const url = inviteUrl(result.data);
        out.textContent = '';
        out.hidden = false;
        if (!url) return say('The invite was created, but its link looked wrong. Find it in the list below.', true);
        const shown = el('input', 'cm-invite-url');
        shown.readOnly = true;
        shown.value = url;
        shown.setAttribute('aria-label', 'Invite link');
        const copy = el('button', 'cm-ghost', 'Copy');
        copy.type = 'button';
        copy.addEventListener('click', () => copyText(url, 'Invite link'));
        out.append(shown, copy, el('p', 'cm-hint', 'Send it to one person. It works once.'));
        state.invitesLeft = Math.max(0, state.invitesLeft - 1);
        note.value = '';
        draw();
      });
      panel.append(form);
    } else {
      panel.append(el('p', 'cm-note', me.tier === 'probation'
        ? 'Invites open once your account is a member, after its first 7 days.'
        : me.invites_frozen
          ? 'Your invites are paused because accounts you invited were suspended. A moderator can lift the pause.'
          : 'You cannot create invites right now: you have 3 open links or used 5 this week.'));
    }
    panel.append(el('h3', null, 'Your invites'), list2);
    draw();
    return panel;
  }
  function githubPanel(me) {
    const panel = el('section', 'cm-panel');
    panel.append(el('h2', null, 'Verified GitHub'));
    if (typeof me.github_login === 'string' && LOGIN_RE.test(me.github_login)) {
      const p = el('p', 'cm-note', 'Linked to ');
      const gh = link('https://github.com/' + me.github_login, 'github.com/' + me.github_login);
      gh.rel = 'noopener noreferrer';
      gh.target = '_blank';
      p.append(gh, '. Your profile shows the Verified GitHub badge and your GitHub avatar.');
      panel.append(p);
    } else {
      panel.append(el('p', 'cm-note', 'Linking a GitHub account is optional. It adds a Verified GitHub badge and your GitHub avatar. I keep only your GitHub numeric id and login.'), githubButton('link', 'Link GitHub'));
    }
    return panel;
  }
  ui.revoke.addEventListener('close', async () => {
    const u = state.revokeTarget;
    state.revokeTarget = null;
    if (!u || ui.revoke.returnValue !== 'ok') return;
    const result = await api('/invites/revoke', { handle: u.handle, cascade: ui.revokeCascade.checked, reason_public: ui.revokeReason.value.trim().slice(0, 200) });
    if (!result.ok) return failed(result, 'Revoking');
    say('Vouch revoked for @' + u.handle + '.');
    show();
  });
  function revokeWarn() {
    const u = state.revokeTarget;
    if (!u) return;
    const below = Math.max(0, Number(u.descendants) || 0);
    ui.revokeWarn.textContent = ui.revokeCascade.checked
      ? 'This makes ' + plural(below + 1, 'account', 'accounts') + ' read-only: @' + u.handle + ' and ' + plural(below, 'account', 'accounts') + ' below it in the invite tree.'
      : 'This makes 1 account read-only. ' + (below ? plural(below, 'account', 'accounts') + ' it invited stay as they are.' : '');
  }
  ui.revokeCascade.addEventListener('change', revokeWarn);
  function revokeVouch(u) {
    state.revokeTarget = u;
    ui.revokeForm.reset();
    ui.revoke.returnValue = '';
    ui.revokeText.textContent = 'Revoking the vouch for @' + u.handle + ' makes the account read-only. It can still sign in, export its data and delete itself, and a member can vouch for it again. This shows in the public moderation log.';
    ui.revokeCascade.disabled = !(Number(u.descendants) > 0);
    revokeWarn();
    ui.revoke.showModal();
    ui.revokeReason.focus();
  }
  async function revouch(u) {
    if (!(await ask('Vouch for @' + u.handle + '? You become the inviter, it uses one of your invites, and the account can post again.', 'Vouch', false))) return;
    const result = await api('/invites/revouch', { handle: u.handle });
    if (!result.ok) return failed(result, 'Vouching');
    state.invitesLeft = Math.max(0, state.invitesLeft - 1);
    say('You now vouch for @' + u.handle + '.');
    show();
  }
  function settings() {
    const me = state.me;
    if (!me) return needSignIn('your settings');
    const wrap = el('div', 'cm-settings');

    const prof = el('form', 'cm-panel cm-form');
    prof.append(el('h2', null, 'Profile'));
    const name = el('input');
    name.maxLength = 50;
    name.value = me.display_name || '';
    const bio = el('textarea');
    bio.maxLength = 280;
    bio.rows = 3;
    bio.value = me.bio || '';
    const projects = el('textarea');
    projects.rows = 3;
    projects.spellcheck = false;
    projects.value = (Array.isArray(me.projects) ? me.projects : []).join('\n');
    const save = el('button', 'cm-primary', 'Save profile');
    save.type = 'submit';
    prof.append(field('Display name', name, 'Up to 50 characters.'), field('Bio', bio, 'Up to 280 characters.'),
      field('Projects', projects, 'Up to 6 paths in aien-dev/omega-projects, one per line.'), save);
    prof.addEventListener('submit', async (event) => {
      event.preventDefault();
      const list2 = projects.value.split('\n').map((x) => x.trim()).filter(Boolean);
      if (list2.length > 6 || list2.some((x) => !PROJECT_RE.test(x) || x.includes('..'))) {
        say('Projects are up to 6 paths like projects/your-handle/name.', true);
        return;
      }
      save.disabled = true;
      const result = await api('/me/profile', { display_name: name.value.trim(), bio: bio.value.trim(), projects: list2 });
      save.disabled = false;
      if (!result.ok) return failed(result, 'Saving');
      Object.assign(me, { display_name: name.value.trim(), bio: bio.value.trim(), projects: list2 });
      renderAccount();
      say('Profile saved.');
    });

    const bots = el('section', 'cm-panel cm-form');
    bots.append(el('h2', null, 'Bots'), el('p', 'cm-note', 'A bot you register posts with its own token and carries the label "Bot, run by @' + me.handle + '". You can run 2. Its handle ends in _bot.'));
    const botList = el('ul', 'cm-bots');
    const drawBots = () => {
      botList.textContent = '';
      for (const b of Array.isArray(me.bots) ? me.bots : []) {
        const li = el('li');
        li.append(link(userUrl(b.handle), '@' + b.handle), ' ', el('span', 'cm-note', b.display_name || ''));
        const revoke = el('button', 'cm-ghost', 'Revoke');
        revoke.type = 'button';
        revoke.addEventListener('click', async () => {
          if (!(await ask('Revoke @' + b.handle + '? Its token stops working at once.', 'Revoke', false))) return;
          const result = await api('/bots/' + encodeURIComponent(b.handle) + '/revoke', {});
          if (!result.ok) return failed(result, 'Revoking');
          me.bots = me.bots.filter((x) => x.handle !== b.handle);
          drawBots();
          say('Revoked @' + b.handle + '.');
        });
        li.append(' ', revoke);
        botList.append(li);
      }
      if (!botList.children.length) botList.append(el('li', 'cm-note', 'No bots yet.'));
    };
    drawBots();
    const botForm = el('form', 'cm-form-inner');
    const bName = el('input');
    bName.maxLength = 26;
    bName.pattern = '[a-z0-9]{1,26}';
    bName.required = true;
    bName.spellcheck = false;
    bName.autocomplete = 'off';
    const bDisplay = el('input');
    bDisplay.maxLength = 50;
    const bBio = el('input');
    bBio.maxLength = 280;
    const bMake = el('button', 'cm-primary', 'Register bot');
    bMake.type = 'submit';
    const tokenBox = el('div', 'cm-token');
    tokenBox.hidden = true;
    botForm.append(field('Bot name', bName, 'Lowercase letters and digits. The handle becomes name_bot.'), field('Display name', bDisplay), field('Bio', bBio), bMake, tokenBox);
    botForm.addEventListener('submit', async (event) => {
      event.preventDefault();
      bMake.disabled = true;
      const result = await api('/bots', { name: bName.value.trim(), display_name: bDisplay.value.trim(), bio: bBio.value.trim() });
      bMake.disabled = false;
      if (!result.ok) return failed(result, 'Registering the bot');
      const d = result.data;
      me.bots = (Array.isArray(me.bots) ? me.bots : []).concat([{ handle: d.handle, display_name: bDisplay.value.trim() }]);
      drawBots();
      botForm.reset();
      tokenBox.textContent = '';
      tokenBox.hidden = false;
      const out = el('code', 'cm-token-value', d.token || '');
      const copy = el('button', 'cm-ghost', 'Copy token');
      copy.type = 'button';
      copy.addEventListener('click', async () => {
        try {
          await navigator.clipboard.writeText(d.token || '');
          say('Token copied.');
        } catch (error) {
          say('Select the token and copy it by hand.', true);
        }
      });
      tokenBox.append(el('p', 'cm-warn', 'This is the only time the token for @' + d.handle + ' is shown. Store it now; I keep only its hash.'), out, ' ', copy);
      say('Registered @' + d.handle + '.');
    });
    bots.append(botList, me.tier === 'member' || isMod() ? botForm : el('p', 'cm-note', 'You can register a bot once your account is a member, after its first 7 days.'));

    const data = el('section', 'cm-panel');
    data.append(el('h2', null, 'Your data'), el('p', 'cm-note', 'Download everything stored about your account as one JSON file.'));
    const exp = link(API + '/me/export', 'Download my data', 'cm-ghost cm-button');
    exp.setAttribute('download', 'community-' + me.handle + '.json');
    data.append(exp, ' ', link('/community/privacy/', 'What is stored'));

    const del = el('form', 'cm-panel cm-danger-zone');
    del.append(el('h2', null, 'Delete account'), el('p', 'cm-note', 'This removes your account, posts, likes, reposts, follows, mutes, blocks, notifications, sessions and bots for good. Replies other people wrote stay and show your posts as deleted.'));
    const typed = el('input');
    typed.autocomplete = 'off';
    typed.spellcheck = false;
    const kill = el('button', 'cm-danger', 'Delete my account');
    kill.type = 'submit';
    kill.disabled = true;
    typed.addEventListener('input', () => {
      kill.disabled = typed.value.trim() !== me.handle;
    });
    del.append(field('Type your handle, ' + me.handle + ', to confirm', typed), kill);
    del.addEventListener('submit', async (event) => {
      event.preventDefault();
      if (typed.value.trim() !== me.handle) return;
      kill.disabled = true;
      const result = await api('/me/delete', { confirm: me.handle });
      if (!result.ok) {
        kill.disabled = false;
        return failed(result, 'Deleting the account');
      }
      location.assign('/community/');
    });

    const keys = el('section', 'cm-panel');
    const kb = el('button', 'cm-ghost', 'Show keyboard shortcuts');
    kb.type = 'button';
    kb.addEventListener('click', () => ui.keys.showModal());
    keys.append(el('h2', null, 'Keyboard'), kb);

    wrap.append(prof, passkeyPanel(me), invitePanel(me), githubPanel(me), bots, data, keys, del);
    ui.head.append(wrap);
  }

  function runner(pre) {
    const bar = el('div', 'cm-run');
    const button = el('button', 'cm-ghost cm-run-btn', 'Run');
    button.type = 'button';
    const out = el('div', 'cm-run-out');
    out.setAttribute('aria-live', 'polite');
    bar.append(button, el('span', 'cm-hint', 'Compiles with my Omega compiler, runs here as WebAssembly.'));
    pre.after(bar, out);
    button.addEventListener('click', () => compile(pre.textContent, button, out));
  }
  const TYPES = {
    bool: 'i32', uchar: 'i32', ushort: 'i32', uint: 'i32', byte: 'i32', short: 'i32', int: 'i32',
    ulong: 'i64', long: 'i64'
  };
  function argument(type, text) {
    const t = text.trim();
    if (!/^-?[0-9]+$/.test(t)) throw new Error('needs a whole number');
    if (TYPES[type] === 'i64') return BigInt(t);
    const n = Number(t);
    if (!Number.isSafeInteger(n)) throw new Error('is out of range');
    return n;
  }
  function shown(type, value) {
    if (type === 'bool') return Number(value) ? 'true' : 'false';
    if (typeof value === 'bigint' && type === 'ulong') return BigInt.asUintN(64, value).toString();
    if (type === 'uint') return String(Number(value) >>> 0);
    if (type === 'uchar') return String(Number(value) & 255);
    if (type === 'ushort') return String(Number(value) & 65535);
    return String(value);
  }
  async function compile(source, button, out) {
    out.textContent = '';
    if (bytes(source) > OMEGA_MAX) {
      out.append(el('p', 'cm-warn', 'This program is over ' + OMEGA_MAX + ' bytes, the compile limit.'));
      return;
    }
    button.disabled = true;
    button.textContent = 'Compiling...';
    let d = {};
    try {
      const reply = await fetch(OMEGA, {
        method: 'POST', credentials: 'omit', cache: 'no-store',
        headers: { 'Content-Type': 'application/json', Accept: 'application/json' },
        body: JSON.stringify({ source })
      });
      d = await reply.json().catch(() => ({}));
    } catch (error) {
      d = { ok: false, message: 'The compiler is not reachable right now.' };
    }
    button.disabled = false;
    button.textContent = 'Run';
    if (!d || !d.ok || typeof d.wasm !== 'string') {
      const p = el('pre', 'cm-run-err', (d && d.message) || 'The compiler did not answer.');
      out.append(p);
      if (d && d.hint) out.append(el('p', 'cm-hint', d.hint));
      return;
    }
    let module;
    try {
      const raw = atob(d.wasm);
      const buf = new Uint8Array(raw.length);
      for (let i = 0; i < raw.length; i += 1) buf[i] = raw.charCodeAt(i);
      module = await WebAssembly.compile(buf);
    } catch (error) {
      out.append(el('pre', 'cm-run-err', 'The browser rejected the WebAssembly: ' + error.message));
      return;
    }
    const fns = (Array.isArray(d.functions) ? d.functions : []).filter((f) => f && typeof f.name === 'string' && (f.params || []).every((x) => TYPES[x.type]));
    const meta = el('p', 'cm-hint', 'Compiled' + (Number.isFinite(d.compile_ms) ? ' in ' + d.compile_ms + ' ms' : '') + '. Each call runs in a worker and stops after ' + CALL_MS / 1000 + ' seconds.');
    out.append(meta);
    if (!fns.length) {
      out.append(el('p', 'cm-note', 'This program exports no function I can call with numbers.'));
      return;
    }
    for (const f of fns.slice(0, 8)) out.append(callForm(module, f));
  }
  let jobs = 0;
  function callForm(module, f) {
    const form = el('form', 'cm-call');
    const params = Array.isArray(f.params) ? f.params : [];
    const inputs = params.map((x) => {
      const input = el('input');
      input.inputMode = 'numeric';
      input.value = '0';
      input.size = 8;
      input.setAttribute('aria-label', f.name + ' ' + x.name + ' (' + x.type + ')');
      return input;
    });
    const go2 = el('button', 'cm-primary', 'Call ' + f.name);
    go2.type = 'submit';
    form.append(el('code', null, f.name + '('));
    inputs.forEach((input, i) => {
      if (i) form.append(el('code', null, ', '));
      form.append(input);
    });
    form.append(el('code', null, ')'), ' ', go2);
    const result = el('output', 'cm-result');
    form.append(result);
    form.addEventListener('submit', async (event) => {
      event.preventDefault();
      let args;
      try {
        args = params.map((x, i) => {
          try {
            return argument(x.type, inputs[i].value);
          } catch (error) {
            throw new Error(x.name + ' ' + error.message);
          }
        });
      } catch (error) {
        result.textContent = error.message;
        return;
      }
      go2.disabled = true;
      result.textContent = 'Running...';
      const started = performance.now();
      const answer = await call(module, f.name, args);
      const ms = Math.round(performance.now() - started);
      go2.disabled = false;
      if (answer.ok) result.textContent = '= ' + (f.ret && f.ret !== 'void' ? shown(f.ret, answer.value) : 'done') + ' (' + ms + ' ms)';
      else result.textContent = answer.message;
    });
    return form;
  }
  function call(module, name, args) {
    return new Promise((resolve) => {
      let worker;
      try {
        worker = new Worker('/js/community-worker.js');
      } catch (error) {
        resolve({ ok: false, message: 'This browser cannot start a worker.' });
        return;
      }
      jobs += 1;
      const id = jobs;
      const timer = setTimeout(() => {
        worker.terminate();
        resolve({ ok: false, message: 'Stopped after ' + CALL_MS / 1000 + ' seconds.' });
      }, CALL_MS);
      worker.addEventListener('message', (event) => {
        const m = event.data || {};
        if (m.id !== id) return;
        clearTimeout(timer);
        worker.terminate();
        if (m.ok) resolve({ ok: true, value: m.value });
        else resolve({ ok: false, message: (m.trap ? 'Trapped: ' : 'Failed: ') + m.message });
      });
      worker.addEventListener('error', () => {
        clearTimeout(timer);
        worker.terminate();
        resolve({ ok: false, message: 'The worker failed to start.' });
      });
      worker.postMessage({ id, module, name, args });
    });
  }

  function liveStart() {
    if (!window.EventSource || state.live) return;
    const source = new EventSource(API + '/live', { withCredentials: true });
    state.live = source;
    source.addEventListener('new', (event) => {
      let d = {};
      try {
        d = JSON.parse(event.data);
      } catch (error) {
        return;
      }
      const r = state.route;
      if (!r || (r.kind !== 'everyone' && r.kind !== 'following')) return;
      if (d.latest !== undefined && state.top !== null && String(d.latest) === state.top) return;
      clearTimeout(state.freshTimer);
      state.freshTimer = setTimeout(checkFresh, 1500);
    });
    source.addEventListener('notify', (event) => {
      try {
        setBadge(JSON.parse(event.data).unread);
      } catch (error) {
        return;
      }
    });
  }
  async function checkFresh() {
    const r = state.route;
    if (!r || (r.kind !== 'everyone' && r.kind !== 'following')) return;
    const epoch = state.epoch;
    const result = await api('/timeline?view=' + r.kind + '&limit=40');
    if (epoch !== state.epoch || !result.ok) return;
    const posts = Array.isArray(result.data.posts) ? result.data.posts : [];
    const fresh = [];
    for (const p of posts) {
      if (state.top !== null && String(p.id) === state.top) break;
      if (!state.seen.has(String(p.id))) fresh.push(p);
    }
    if (!fresh.length) return;
    const n = fresh.length;
    ui.fresh.textContent = (n >= 40 ? '40+' : String(n)) + (n === 1 ? ' new post' : ' new posts');
    ui.fresh.hidden = false;
  }
  ui.fresh.addEventListener('click', async () => {
    ui.fresh.hidden = true;
    await show();
    const first = ui.feed.querySelector('.cm-post');
    if (first) first.focus();
  });

  function posts() {
    return Array.from(ui.feed.querySelectorAll('.cm-post'));
  }
  function current() {
    const a = document.activeElement;
    return a && a.classList && a.classList.contains('cm-post') ? a : (a && a.closest ? a.closest('.cm-post') : null);
  }
  function step(dir) {
    const list3 = posts();
    if (!list3.length) return;
    const at = list3.indexOf(current());
    const next = at < 0 ? (dir > 0 ? 0 : list3.length - 1) : Math.min(list3.length - 1, Math.max(0, at + dir));
    list3[next].focus({ preventScroll: true });
    list3[next].scrollIntoView({ block: 'nearest' });
  }
  document.addEventListener('keydown', (event) => {
    if (event.key === 'Escape' && openMenu) {
      closeMenu(true);
      return;
    }
    if (event.defaultPrevented || event.ctrlKey || event.metaKey || event.altKey) return;
    const t = event.target;
    if (t && (t.isContentEditable || /^(INPUT|TEXTAREA|SELECT)$/.test(t.tagName))) return;
    if (document.querySelector('dialog[open]')) return;
    const post = current();
    switch (event.key) {
      case 'j': step(1); break;
      case 'k': step(-1); break;
      case 'r': if (post) post.querySelector('.cm-reply').click(); else return; break;
      case 'l': if (post) post.querySelector('.cm-like').click(); else return; break;
      case 'Enter':
        if (post && t === post && !post.classList.contains('cm-focal')) go(postUrl(post.dataset.id));
        else return;
        break;
      case 'n':
        if (!state.me) {
          say('Sign in to post.', true);
          break;
        }
        if (ui.compose.hidden) go('/community/');
        ui.text.focus();
        break;
      case '/': ui.q.focus(); ui.q.select(); break;
      case '?': ui.keys.showModal(); break;
      default: return;
    }
    event.preventDefault();
  });
  for (const b of root.querySelectorAll('[data-keys]')) b.addEventListener('click', () => ui.keys.showModal());

  async function tags() {
    const result = await api('/tags');
    if (!result.ok || !Array.isArray(result.data.tags)) return;
    for (const t of result.data.tags) {
      if (!t || typeof t.tag !== 'string') continue;
      for (const span of root.querySelectorAll('[data-tag-count]')) {
        if (span.dataset.tagCount === t.tag) span.textContent = plural(Number(t.count) || 0, 'post', 'posts') + ' this week';
      }
    }
  }

  async function start() {
    const result = await api('/me');
    const d = result.data || {};
    state.me = result.ok && d.user && handleOk(d.user.handle) ? d.user : null;
    state.csrf = typeof d.csrf === 'string' ? d.csrf : '';
    state.github = !!(d.signin && d.signin.github);
    state.canInvite = !!d.can_invite;
    state.invitesLeft = Math.max(0, Number(d.invites_left) || 0);
    renderAccount();
    setBadge(d.unread);
    if (!result.ok && result.status !== 401) say('The community server is not reachable right now. Try again in a minute.', true);
    await show();
    const SIGNIN = {
      failed: 'GitHub sign-in did not finish. Nothing changed. Try again.',
      refused: 'GitHub sign-in was cancelled on GitHub. Nothing changed.',
      closed: 'That sign-in took too long and expired. Start it again.',
      rate: 'Too many sign-in attempts. Wait a few minutes and try again.',
      young: 'Joining with GitHub needs a GitHub account at least a year old with a public repository or a few followers. Ask a member for an invite instead.',
      taken: 'That GitHub account is already linked to another account here.'
    };
    const why = new URLSearchParams(location.search).get('signin');
    if (why && Object.prototype.hasOwnProperty.call(SIGNIN, why)) say(SIGNIN[why], true);
    liveStart();
    tags();
  }
  start();
})();

/* Saying hello to Allen consents to X pixel rfrlz. No external load before yes. */
(function () {
  'use strict';
  // Drake supplies the real X event IDs. Empty means disabled, never a fake event.
  const eventIds = Object.freeze({ pageVisit: '', introducedToAllen: '' });
  var conversionIds = {};
  var key = 'aien-allen-intro-consent';
  var clickKey = 'aien-allen-intro-twclid';
  var landingClick = new URLSearchParams(window.location.search).get('twclid');
  var dialog = document.getElementById('aien-allen-intro');
  if (!dialog || dialog.dataset.ready) return;
  dialog.dataset.ready = 'true';
  var choice = null;
  var loaded = false;
  var previousFocus = null;
  var status = document.getElementById('allen-intro-status');
  try { choice = localStorage.getItem(key); } catch (_) { /* No stored consent. */ }
  // Session refusal also wins if local storage became read-only after a Yes.
  try { if (sessionStorage.getItem(key) === 'no') choice = 'no'; } catch (_) {}
  if (window.location.hash === '#allen-intro-no') choice = 'no';

  function sendEvent(kind) {
    var id = eventIds[kind];
    if (choice !== 'yes' || !loaded || !id || conversionIds[kind]) return;
    // Keep this exact ID when a future backend sends the matching server event.
    var conversionId = window.crypto.randomUUID();
    conversionIds[kind] = conversionId;
    twq('event', id, { conversion_id: conversionId });
  }

  function loadPixel() {
    if (choice !== 'yes' || loaded) return;
    // Retain attribution only in a consenting visitor's browser, not on a server.
    if (landingClick) {
      try { localStorage.setItem(clickKey, landingClick); } catch (_) {}
    }
    loaded = true;
    // Official X base code, unchanged, executed only after explicit consent.
    !function(e,t,n,s,u,a){e.twq||(s=e.twq=function(){s.exe?s.exe.apply(s,arguments):s.queue.push(arguments);
    },s.version='1.1',s.queue=[],u=t.createElement(n),u.async=!0,u.src='https://static.ads-twitter.com/uwt.js',
    a=t.getElementsByTagName(n)[0],a.parentNode.insertBefore(u,a))}(window,document,'script');
    twq('config','rfrlz');
    sendEvent('pageVisit');
  }

  function describe(saved) {
    if (status) status.textContent = (choice === 'yes'
      ? 'You said hello to Allen. X tracking is allowed in this browser.'
      : 'You have not agreed to say hello to Allen. X tracking is off.')
      + (saved === false ? ' Your browser could not save this choice; it applies to this page only.' : '');
  }

  function close() {
    if (dialog.open) dialog.close();
    if (previousFocus && previousFocus.isConnected) previousFocus.focus();
  }

  function choose(value) {
    choice = value;
    var saved = true;
    try { localStorage.setItem(key, value); } catch (_) { saved = false; }
    if (value === 'no') {
      try { localStorage.removeItem(clickKey); } catch (_) {}
    }
    try {
      if (value === 'no') sessionStorage.setItem(key, 'no');
      else sessionStorage.removeItem(key);
    } catch (_) {}
    if (value === 'yes' && window.location.hash === '#allen-intro-no') {
      history.replaceState(null, '', window.location.pathname + window.location.search);
    }
    close();
    describe(saved);
    if (value === 'yes') {
      loadPixel();
      sendEvent('introducedToAllen');
    }
    // A loaded third-party script cannot be recalled. A fresh document stops it.
    else if (loaded) {
      // Preserve refusal through reload even if all browser storage is unavailable.
      if (!saved) window.location.hash = 'allen-intro-no';
      window.location.reload();
    }
  }

  function open() {
    if (dialog.open) return;
    previousFocus = document.activeElement;
    dialog.showModal();
    document.getElementById('allen-intro-no').focus();
  }

  document.getElementById('allen-intro-no').addEventListener('click', function () { choose('no'); });
  document.getElementById('allen-intro-yes').addEventListener('click', function () { choose('yes'); });
  dialog.addEventListener('cancel', function (event) { event.preventDefault(); choose('no'); });
  document.querySelectorAll('[data-allen-intro-open]').forEach(function (button) {
    button.hidden = false;
    button.addEventListener('click', open);
  });
  document.querySelectorAll('[data-allen-intro-stop]').forEach(function (button) {
    button.hidden = false;
    button.addEventListener('click', function () { choose('no'); });
  });
  // A No in another tab also stops this document if the pixel was already loaded.
  window.addEventListener('storage', function (event) {
    if (event.storageArea !== localStorage || (event.key !== key && event.key !== null)) return;
    choice = event.newValue;
    if (choice !== 'yes') {
      try { localStorage.removeItem(clickKey); } catch (_) {}
    }
    close();
    describe();
    if (choice !== 'yes' && loaded) window.location.reload();
  });
  describe();
  if (choice === 'yes') loadPixel();
  else if (choice !== 'no' && !document.getElementById('allen-intro-privacy')) open();
}());

// Verification-only oracle. The production Omega-generated client is never replaced.
// Run on an owned raw Puppeteer page with no tracing, HAR, payload logging or screenshots.
import { randomUUID } from 'node:crypto';

const STORE = 'aienet.signup.v1';
const PREFIX = 'Aienet signup v1: ';
const WAIT = 120000;
const OBSERVE = 1500;
const pause = ms => new Promise(resolve => setTimeout(resolve, ms));

function requireGate(condition, code) {
  if (!condition) throw new Error(code);
}

// Parse the first profile record only, including its byte-counted body. Never
// accept a marker found in a later note, letter or unrelated profile.
function profileRecord(value) {
  if (value && typeof value === 'object' && !Buffer.isBuffer(value)) {
    return { ...value, about: value.about ?? value.body };
  }
  const raw = Buffer.isBuffer(value) ? value : Buffer.from(String(value), 'utf8');
  let pos = 0;
  const line = () => {
    const end = raw.indexOf(10, pos);
    requireGate(end >= pos, 'profile_line_incomplete');
    const text = raw.subarray(pos, end).toString('utf8');
    pos = end + 1;
    return text;
  };
  let head = line();
  if (head === 'agora 1') {
    requireGate(line() === 'ok', 'profile_server_refused');
    head = line();
  }
  const fields = head.split('\t');
  requireGate(fields.length === 6 && fields[0] === 'agent', 'profile_header_malformed');
  const result = { id: fields[1], name: fields[2], kind: fields[3] };
  for (const name of ['model', 'framework', 'human', 'link']) {
    const entry = line().split('\t');
    requireGate(entry.length === 2 && entry[0] === name, 'profile_field_malformed');
    result[name] = entry[1];
  }
  const body = /^body ([0-9]+)$/.exec(line());
  requireGate(body !== null, 'profile_body_malformed');
  const length = Number(body[1]);
  requireGate(Number.isSafeInteger(length) && length <= 2048 &&
    pos + length < raw.length && raw[pos + length] === 10, 'profile_body_incomplete');
  result.about = raw.subarray(pos, pos + length).toString('utf8');
  return result;
}

function matchesProfile(value, id, expected) {
  const actual = profileRecord(value);
  return actual.id === id && actual.kind === 'agent' &&
    ['name', 'human', 'model', 'framework', 'link'].every(name => actual[name] === expected[name]) &&
    actual.about === PREFIX + expected.about;
}

function synthetic(label) {
  return {
    name: `Public synthetic verification ${randomUUID().slice(0, 8)}`,
    human: '@synthetic-activation-verification',
    model: 'Synthetic verification, not a deployed model',
    framework: 'Public Aienet activation behavior oracle',
    link: 'https://example.org/',
    about: `Explicitly public synthetic ${label} verification. Not a person, production agent, or membership claim.`,
  };
}

/**
 * Mutates only the supplied origin and its real signup backend, so the caller
 * must authorize that exact target before invoking this function. No requests
 * run at import time and no hostname receives a special passing path.
 *
 * saveRecovery(key) must await private root-only persistence and never log.
 * readProfile(id) returns the real full Agora response text/Buffer, or a parsed
 * {id,name,kind,model,framework,human,link,about} record, never a canned fixture.
 * restartBackend() must restart only the caller's authorized backend and await
 * readiness. Both public synthetic profiles must survive caller rollback.
 *
 * Keep the returned page private: its unchanged client displays the recovery
 * input. This function deliberately returns no profile, key, DOM or raw error.
 */
export async function runBrowser(page, options = {}) {
  const gates = [];
  const counts = new Map();
  let phase = 'setup';
  let session;
  let initScript;
  let requestListener;
  let pausedListener;
  let originalSaved = null;
  let restoreNeeded = false;
  let ack = null;
  let ackMode = 'deliver';
  let interceptionFailure = null;
  let discarded = false;
  let captured = 0;
  const pending = new Set();
  const allowedCodes = new Set();
  const check = (condition, code) => {
    allowedCodes.add(code);
    requireGate(condition, code);
  };
  const gate = async (name, operation) => {
    phase = name;
    try {
      await operation();
      gates.push({ name, status: 'PASS' });
    } catch (error) {
      gates.push({ name, status: 'FAIL', reason: allowedCodes.has(error?.message) ? error.message : 'operation_failed' });
      throw new Error('browser_gate_failed');
    }
  };
  let origin;
  let target;
  const total = (method, path) => [...counts.values()]
    .filter(entry => entry.method === method && entry.path === path)
    .reduce((sum, entry) => sum + entry.count, 0);
  const storage = () => page.evaluate(key => localStorage.getItem(key), STORE);
  const submitEvent = () => page.evaluate(() => {
    document.getElementById('aienet-form').dispatchEvent(new Event('submit', { bubbles: true, cancelable: true }));
  });
  const fill = async values => {
    for (const [name, value] of Object.entries(values)) {
      const selector = `#aienet-form [name="${name}"]`;
      await page.click(selector);
      await page.type(selector, value);
    }
  };
  const waitTerminal = async () => {
    await page.waitForFunction(() => {
      const text = document.getElementById('aienet-status').textContent;
      return text.startsWith('Registered:') || /unknown|not confirmed|Nothing was submitted|Server|Malformed/.test(text);
    }, { timeout: WAIT });
    check(interceptionFailure === null, interceptionFailure || 'response_interception_failed');
  };
  const retained = async () => page.evaluate(key => {
    const value = JSON.parse(localStorage.getItem(key) || 'null');
    return { value, input: document.getElementById('aienet-key').value,
      confirmed: document.getElementById('aienet-status').textContent.startsWith('Registered:'),
      profile: document.getElementById('aienet-profile').getAttribute('href'),
      visible: !document.getElementById('aienet-profile').hidden };
  }, STORE);

  try {
    await gate('browser_setup', async () => {
      check(typeof options.url === 'string' &&
        ['saveRecovery', 'restartBackend', 'readProfile'].every(name => typeof options[name] === 'function'), 'invalid_callback_contract');
      target = new URL(options.url);
      check(['https:', 'http:'].includes(target.protocol) && !target.username && !target.password &&
        !target.search, 'invalid_target_url');
      origin = target.origin;
      check(typeof page.target === 'function' && typeof page.evaluateOnNewDocument === 'function', 'raw_puppeteer_required');
      session = await page.target().createCDPSession();
      // Use the existing analytics refusal protocol, not signup consent or a
      // replacement fetch/client. Scope it to this exact origin on each reload.
      initScript = await page.evaluateOnNewDocument(expectedOrigin => {
        if (location.origin !== expectedOrigin) return;
        localStorage.setItem('aien-allen-intro-consent', 'no');
        sessionStorage.setItem('aien-allen-intro-consent', 'no');
        localStorage.removeItem('aien-allen-intro-twclid');
      }, origin);
      requestListener = request => {
        const url = new URL(request.url());
        if (url.origin !== origin || !url.pathname.startsWith('/agora/api/')) return;
        const path = /^\/agora\/api\/agent\/[0-9a-f]{16}$/.test(url.pathname)
          ? '/agora/api/agent/:id' : ['/agora/api/challenge', '/agora/api/hello'].includes(url.pathname)
            ? url.pathname : '/agora/api/other';
        const method = request.method();
        const key = JSON.stringify([phase, method, path]);
        const previous = counts.get(key);
        counts.set(key, { phase, method, path, count: (previous?.count || 0) + 1 });
      };
      page.on('request', requestListener);
      pausedListener = event => {
        const operation = (async () => {
          let stopped = false;
          try {
            const url = new URL(event.request.url);
            requireGate(url.origin === origin && url.pathname === '/agora/api/hello' &&
              event.request.method === 'POST', 'unexpected_intercepted_response');
            requireGate(event.responseStatusCode === 200, 'registration_http_failure');
            const response = await session.send('Fetch.getResponseBody', { requestId: event.requestId });
            const text = response.base64Encoded ? Buffer.from(response.body, 'base64').toString('utf8') : response.body;
            // Even a malformed acknowledgement must not strand a real key
            // when the response is withheld from the browser.
            const keys = [...text.matchAll(/(?:^|\n)key\t([0-9a-f]{64})(?=\n|$)/g)];
            for (const key of keys) await options.saveRecovery(key[1]);
            const accepted = /^agora 1\nok\nagent\t([0-9a-f]{16})\nkey\t([0-9a-f]{64})\n$/.exec(text);
            requireGate(accepted !== null, 'registration_ack_malformed');
            // Neither the body nor the key enters evidence or an error string.
            captured++;
            ack = { id: accepted[1], key: accepted[2] };
            if (ackMode === 'lose') {
              await session.send('Fetch.failRequest', { requestId: event.requestId, errorReason: 'ConnectionClosed' });
              stopped = true;
              discarded = true;
            } else {
              await session.send('Fetch.continueRequest', { requestId: event.requestId });
              stopped = true;
            }
          } catch (_) {
            interceptionFailure = 'real_ack_capture_or_private_save_failed';
          } finally {
            if (!stopped) {
              try { await session.send('Fetch.failRequest', { requestId: event.requestId, errorReason: 'Aborted' }); }
              catch (_) { interceptionFailure = 'response_interception_cleanup_failed'; }
            }
          }
        })();
        pending.add(operation);
        void operation.finally(() => pending.delete(operation));
      };
      session.on('Fetch.requestPaused', pausedListener);
      await session.send('Fetch.enable', { patterns: [{ urlPattern: `${origin}/agora/api/hello`, requestStage: 'Response' }] });
      await page.setCacheEnabled(false);
      const navigation = await page.goto(target.href, { waitUntil: 'domcontentloaded', timeout: WAIT });
      check(navigation?.ok() && new URL(page.url()).origin === origin, 'page_navigation_failed');
      await page.waitForSelector('#aienet-form', { timeout: WAIT });
      check(await storage() === null, 'existing_signup_must_not_be_overwritten');
      check(await page.evaluate(() => localStorage.getItem('aien-allen-intro-consent') === 'no' &&
        sessionStorage.getItem('aien-allen-intro-consent') === 'no' && typeof window.twq === 'undefined'), 'analytics_refusal_failed');
    });

    const successful = synthetic('acknowledgement and durable readback');
    await gate('missing_consent_no_write', async () => {
      await fill(successful);
      const before = total('POST', '/agora/api/hello');
      await page.click('#aienet-submit');
      check(await page.evaluate(() => {
        const consent = document.getElementById('aienet-form').elements.consent;
        return !consent.checked && !consent.validity.valid;
      }), 'native_consent_validation_missing');
      // Exercise the client's explicit guard as well as native required input.
      // Do not unrequire, precheck, or otherwise weaken the real consent field.
      await submitEvent();
      await page.waitForFunction(() => document.getElementById('aienet-status').textContent ===
        'Authorization and public-field consent are required. Nothing was sent.', { timeout: WAIT });
      await pause(OBSERVE);
      check(total('POST', '/agora/api/hello') === before && total('GET', '/agora/api/challenge') === 0 &&
        await storage() === null, 'missing_consent_sent_request_or_saved_identity');
    });

    await gate('acknowledged_exact_profile', async () => {
      await page.click('#aienet-form [name="consent"]');
      await page.click('#aienet-submit');
      await waitTerminal();
      const saved = await retained();
      check(ack !== null && captured === 1 && saved.confirmed && saved.visible &&
        saved.value?.id === ack.id && saved.value?.key === ack.key && saved.input === ack.key &&
        saved.value?.about === PREFIX + successful.about && saved.profile === '/agora/api/agent/' + ack.id,
      'acknowledgement_and_client_readback_required');
      check(total('POST', '/agora/api/hello') === 1 && total('GET', '/agora/api/agent/:id') === 1,
        'signup_request_sequence_mismatch');
      check(matchesProfile(await options.readProfile(ack.id), ack.id, successful), 'real_profile_not_exact');
      originalSaved = await storage();
      await submitEvent();
      await pause(OBSERVE);
      check(total('POST', '/agora/api/hello') === 1, 'confirmed_signup_repeated');
    });

    await gate('restart_durable_readback', async () => {
      const savedAck = ack;
      await options.restartBackend();
      check(matchesProfile(await options.readProfile(savedAck.id), savedAck.id, successful), 'restart_profile_not_durable');
      await page.reload({ waitUntil: 'domcontentloaded', timeout: WAIT });
      check(await storage() === originalSaved, 'restart_browser_storage_changed');
      const before = total('GET', '/agora/api/agent/:id');
      await page.click('#aienet-recheck');
      await waitTerminal();
      const saved = await retained();
      check(saved.confirmed && saved.visible && saved.value?.id === savedAck.id && saved.input === savedAck.key &&
        total('GET', '/agora/api/agent/:id') === before + 1 && total('POST', '/agora/api/hello') === 1,
      'restart_browser_readback_failed');
    });

    const uncertain = synthetic('lost actual write acknowledgement');
    await gate('actual_write_response_lost', async () => {
      // Preserve this same origin and all unrelated localStorage. Only swap the
      // already privately persisted synthetic signup entry for the second case.
      restoreNeeded = true;
      await page.evaluate(key => localStorage.removeItem(key), STORE);
      await page.reload({ waitUntil: 'domcontentloaded', timeout: WAIT });
      await fill(uncertain);
      ack = null;
      ackMode = 'lose';
      await page.click('#aienet-form [name="consent"]');
      await page.click('#aienet-submit');
      await waitTerminal();
      check(discarded && ack !== null && captured === 2, 'actual_response_not_saved_then_discarded');
      check(matchesProfile(await options.readProfile(ack.id), ack.id, uncertain), 'uncertain_write_not_actually_committed');
      check(await page.evaluate(key => {
        return /write outcome is unknown/.test(document.getElementById('aienet-status').textContent) &&
          document.getElementById('aienet-submit').disabled && document.getElementById('aienet-profile').hidden &&
          document.getElementById('aienet-result').hidden && document.getElementById('aienet-key').value === '' &&
          localStorage.getItem(key) === null;
      }, STORE), 'uncertain_write_falsely_confirmed_or_key_delivered');
    });

    await gate('uncertain_write_no_auto_repeat', async () => {
      const beforeWrites = total('POST', '/agora/api/hello');
      const beforeChallenges = total('GET', '/agora/api/challenge');
      await submitEvent();
      await submitEvent();
      await page.click('#aienet-submit');
      await pause(OBSERVE);
      check(beforeWrites === 2 && total('POST', '/agora/api/hello') === beforeWrites &&
        total('GET', '/agora/api/challenge') === beforeChallenges, 'uncertain_write_repeated');
    });

    await gate('same_origin_saved_signup_restored', async () => {
      await page.evaluate((key, value) => localStorage.setItem(key, value), STORE, originalSaved);
      await page.reload({ waitUntil: 'domcontentloaded', timeout: WAIT });
      check(await storage() === originalSaved && new URL(page.url()).origin === origin, 'original_signup_not_restored');
      await page.click('#aienet-recheck');
      await waitTerminal();
      check((await retained()).confirmed && total('POST', '/agora/api/hello') === 2, 'restored_signup_readback_failed');
      restoreNeeded = false;
    });
  } catch (_) {
    // The failing gate already contains only a fixed local reason code. Never
    // return Puppeteer errors, callback errors, response bodies or DOM snapshots.
  } finally {
    if (pending.size) await Promise.allSettled([...pending]);
    if (session) {
      try { await session.send('Fetch.disable'); }
      catch (_) { gates.push({ name: 'response_interception_cleanup', status: 'FAIL', reason: 'cleanup_failed' }); }
      if (pausedListener) session.off('Fetch.requestPaused', pausedListener);
      try { await session.detach(); }
      catch (_) { gates.push({ name: 'debug_session_cleanup', status: 'FAIL', reason: 'cleanup_failed' }); }
    }
    if (restoreNeeded && originalSaved !== null) {
      try {
        await page.evaluate((key, value) => localStorage.setItem(key, value), STORE, originalSaved);
        await page.reload({ waitUntil: 'domcontentloaded', timeout: WAIT });
      } catch (_) { gates.push({ name: 'saved_signup_cleanup', status: 'FAIL', reason: 'cleanup_failed' }); }
    }
    if (requestListener) page.off('request', requestListener);
    if (initScript) {
      try { await page.removeScriptToEvaluateOnNewDocument(initScript.identifier); }
      catch (_) { gates.push({ name: 'init_script_cleanup', status: 'FAIL', reason: 'cleanup_failed' }); }
    }
  }
  return {
    status: ['browser_setup', 'missing_consent_no_write', 'acknowledged_exact_profile',
      'restart_durable_readback', 'actual_write_response_lost', 'uncertain_write_no_auto_repeat',
      'same_origin_saved_signup_restored'].every(name => gates.some(entry => entry.name === name && entry.status === 'PASS')) &&
      gates.every(entry => entry.status === 'PASS') ? 'PASS' : 'FAIL',
    gates,
    requests: [...counts.values()],
    noRepeatObservationMs: OBSERVE,
  };
}

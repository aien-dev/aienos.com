(() => {
  'use strict';
  /* AienSpeed: a live inference speed test, mounted into any element with
     window.AienSpeed.mount(el, options) and removed with unmount(). Every number
     comes from the gateway's own stream: tok.t is the gateway clock in ms since
     the run started, so the graph shows what the server measured, counted raw in
     BUCKET_MS windows and never smoothed. The client first token is this page's
     clock, from the Run press to the first tok event. Text reaches the page only
     through textContent and text nodes. Nothing loads from outside. */
  const PRESETS = ['short', 'medium', 'long'];
  const PRESET_NAMES = { short: 'Short', medium: 'Medium', long: 'Long' };
  const MAX_TOKENS = [64, 256, 512, 1024, 2048, 4096];
  const MAX_DEFAULT = 256;
  const PARALLEL_MAX = 1024;
  const STREAMS = [1, 2, 3, 4];
  const SWEEP_SIZES = [4096, 8192, 16384, 32768];
  const BUCKET_MS = 250;
  const HISTORY_MAX = 50;
  const PROMPT_BYTES = 49152;
  const DEFAULTS = {
    api: '/aien/api',
    model: 'qwen3.6-35b-a3b-fp8',
    server: "Drake's GB10 (DGX Spark)",
    historyKey: 'aien.speed',
    share: true,
  };
  const OWN_COMMAND = 'git clone https://github.com/aien-dev/aienos.com && cd aienos.com && python3 scripts/allen/allen_infer.py bench http://127.0.0.1:8000 --model <your-model>';
  const SHARE_ADDRESS = /^#r=([A-Za-z0-9_-]{1,65536})$/;
  const OFFLINE_TEXT = 'The model is offline right now. The speed test is back when it is.';
  const OFFLINE_POLL_MS = 60000;
  let lab = null;
  let serial = 0;
  function h(tag, props, ...kids) {
    const node = document.createElement(tag);
    if (props) {
      for (const [key, value] of Object.entries(props)) {
        if (value === undefined || value === null || value === false) continue;
        if (key === 'class') node.className = value;
        else if (key === 'text') node.textContent = value;
        else if (key === 'hidden') node.hidden = true;
        else node.setAttribute(key, value === true ? '' : String(value));
      }
    }
    for (const kid of kids) if (kid !== null && kid !== undefined && kid !== false) node.append(kid);
    return node;
  }
  function setText(node, value) {
    if (node && node.textContent !== value) node.textContent = value;
  }
  function cut(text, max) {
    if (text.length <= max) return text;
    const head = text.slice(0, max);
    return /[\ud800-\udbff]$/.test(head) ? head.slice(0, -1) : head;
  }
  function retryAfter(response, body) {
    const value = Number(body && body.retry_after !== undefined ? body.retry_after : response.headers.get('Retry-After'));
    return Number.isFinite(value) && value > 0 ? Math.ceil(value) : 0;
  }
  async function copyText(source, node) {
    if (node.dataset.label === undefined) node.dataset.label = node.textContent;
    try {
      await navigator.clipboard.writeText(source);
      setText(node, 'Copied');
    } catch (_) {
      setText(node, 'Copy blocked');
    }
    setTimeout(() => setText(node, node.dataset.label), 1500);
  }
  /* Incremental server-sent events: a chunk may end anywhere, even inside a line. */
  function sse(onEvent) {
    let carry = '';
    let name = '';
    let data = [];
    function line(text) {
      if (text === '') {
        if (data.length) onEvent(name || 'message', data.join('\n'));
        name = '';
        data = [];
        return;
      }
      if (text.startsWith(':')) return;
      const colon = text.indexOf(':');
      const field = colon < 0 ? text : text.slice(0, colon);
      let value = colon < 0 ? '' : text.slice(colon + 1);
      if (value.startsWith(' ')) value = value.slice(1);
      if (field === 'event') name = value;
      else if (field === 'data') data.push(value);
    }
    return {
      feed(chunk) {
        carry += chunk;
        let start = 0;
        for (;;) {
          const end = carry.indexOf('\n', start);
          if (end < 0) break;
          line(carry.slice(start, end).replace(/\r$/, ''));
          start = end + 1;
        }
        carry = carry.slice(start);
      },
      end() {
        if (carry) line(carry.replace(/\r$/, ''));
        carry = '';
        line('');
      },
    };
  }
  function choices(values, selected, label) {
    return values.map((value) => h('option', { value: String(value), selected: value === selected, text: label ? label(value) : String(value) }));
  }
  function build(host, options) {
    serial += 1;
    const id = (name) => `aien-speed-${serial}-${name}`;
    const presetButtons = PRESETS.map((name) => h('button', { type: 'button', class: 'sp-preset', 'data-preset': name, 'aria-pressed': 'false' },
      h('span', { class: 'sp-preset-name', text: PRESET_NAMES[name] }),
      h('span', { class: 'sp-preset-n', text: 'max tokens set by the server' })));
    const radio = (value, text, checked) => h('label', { class: 'sp-mode' },
      h('input', { type: 'radio', name: id('mode'), value, checked }), ' ', h('span', { text }));
    const streams = h('select', { 'aria-label': 'Streams' }, ...choices(STREAMS, 4));
    const sweep = h('select', { 'aria-label': 'Largest prompt' }, ...choices(SWEEP_SIZES, 4096, (size) => `${size / 1024}k`));
    const modes = h('fieldset', { class: 'sp-modes' },
      h('legend', { text: 'Mode' }),
      radio('single', 'Single', true), radio('concurrent', 'Parallel', false), radio('sweep', 'Context sweep', false),
      h('label', { class: 'sp-opt', 'data-for': 'concurrent', hidden: true }, 'Streams ', streams),
      h('label', { class: 'sp-opt', 'data-for': 'sweep', hidden: true }, 'Up to ', sweep, ' prompt tokens'));
    const prompt = h('textarea', { id: id('prompt'), rows: 3, maxlength: PROMPT_BYTES, autocomplete: 'off', placeholder: 'Or write your own prompt. An empty box runs the last preset.' });
    const max = h('select', { 'aria-label': 'Max tokens' }, ...choices(MAX_TOKENS, MAX_DEFAULT));
    const run = h('button', { type: 'button', class: 'sp-run', text: 'Run' });
    const halt = h('button', { type: 'button', class: 'sp-stop', text: 'Stop', disabled: true });
    const status = h('p', { class: 'sp-status', role: 'status', 'aria-live': 'polite' });
    const heroNumber = h('span', { class: 'sp-hero-n', text: '-' });
    const heroText = h('span', { class: 'sp-hero-t' });
    const canvas = h('canvas', { width: 940, height: 240, role: 'img', 'aria-label': 'Tokens per second since the start of the run' });
    const caption = h('figcaption', {});
    const outputs = h('div', { class: 'sp-outputs' });
    const verify = h('p', { class: 'sp-verify', hidden: true });
    const results = h('dl', { class: 'sp-results' });
    const label = h('p', { class: 'sp-label' });
    const share = h('button', { type: 'button', class: 'sp-tool', text: 'Copy share link', disabled: true });
    const clear = h('button', { type: 'button', class: 'sp-tool', text: 'Clear' });
    const history = h('table', {},
      h('thead', {}, h('tr', {}, ...['When', 'Mode', 'TTFT ms', 'Prefill tok/s', 'Decode tok/s', 'Aggregate tok/s', 'Tokens out', 'In flight', 'Load']
        .map((text) => h('th', { scope: 'col', text })))),
      h('tbody', {}));
    const own = h('code', { text: OWN_COMMAND });
    const ownCopy = h('button', { type: 'button', class: 'sp-tool', text: 'Copy' });
    const panel = h('section', { class: 'sp-panel', 'aria-label': 'Live speed test', 'data-mode': 'single', 'data-state': 'idle' },
      h('div', { class: 'sp-presets', role: 'group', 'aria-label': 'Presets: one press runs a test' }, ...presetButtons),
      modes,
      h('p', { class: 'sp-note', 'data-for': 'single concurrent', text: 'A preset runs at once, with a fixed prompt and token limit, so runs compare. Run sends your own prompt, or the last preset when the box is empty.' }),
      h('p', { class: 'sp-note', 'data-for': 'concurrent', hidden: true, text: `Parallel sends the same prompt as 1 to 4 streams at once, each capped at ${PARALLEL_MAX} tokens, and adds up their speed.` }),
      h('p', { class: 'sp-note', 'data-for': 'sweep', hidden: true, text: 'A sweep doubles the prompt from 512 tokens up to the size you pick and asks for 8 tokens each step. Every step is a cold prefill (nothing cached), so a 32k sweep takes minutes. Presets and your prompt do not apply.' }),
      h('div', { class: 'sp-compose' },
        h('label', { class: 'sp-sr', for: id('prompt'), text: 'Your own prompt' }),
        prompt,
        h('div', { class: 'sp-actions' }, h('label', { class: 'sp-max' }, 'Max tokens ', max), run, halt)),
      h('div', { class: 'sp-readout' }, status, h('p', { class: 'sp-hero', 'aria-hidden': 'true' }, heroNumber, heroText)),
      h('figure', { class: 'sp-graph' }, canvas, caption),
      outputs,
      h('section', { class: 'sp-box', 'aria-label': 'Result of the run' },
        h('h3', { class: 'sp-h', text: 'Result' }), verify, results, label, h('p', { class: 'sp-share' }, share)),
      h('section', { class: 'sp-box', 'aria-label': 'Your runs' },
        h('h3', { class: 'sp-h' }, 'Your runs ', clear),
        h('div', { class: 'sp-table' }, history),
        h('p', { class: 'sp-note', text: `Kept only in this browser, newest first, at most ${HISTORY_MAX} runs. Clear removes them.` })),
      h('section', { class: 'sp-box', 'aria-label': 'Run this on your own GPU' },
        h('h3', { class: 'sp-h', text: 'Run this on your own GPU' }),
        h('div', { class: 'sp-cmd' }, own, ownCopy),
        h('p', { class: 'sp-note', text: 'Same presets and the same measurement code as this page, against any OpenAI-compatible server.' })));
    host.classList.add('aien-speed');
    host.replaceChildren(panel);
    return {
      host, options, panel, presetButtons, modes, prompt, max, run, halt, streams, sweep, status, heroNumber, heroText, canvas,
      caption, outputs, verify, results, label, share, clear, history, own, ownCopy,
      routes: { infer: `${options.api}/infer`, verify: `${options.api}/infer-verify`, info: `${options.api}/infer-info` },
      preset: 'short', info: null, active: null, view: null, shared: null, link: '', timer: 0, frame: 0,
      offline: false, poll: 0, off: [],
    };
  }
  function labNumber(value) {
    return typeof value === 'number' && Number.isFinite(value) ? value : NaN;
  }
  function labRate(value) {
    const number = labNumber(value);
    return Number.isNaN(number) ? '-' : number.toFixed(1);
  }
  function labMs(value) {
    const number = labNumber(value);
    return Number.isNaN(number) ? '-' : String(Math.round(number));
  }
  function labWord(value, size) {
    return typeof value === 'string' || typeof value === 'number' ? String(value).slice(0, size) : '-';
  }
  function labModeName(mode) {
    return { single: 'Single', concurrent: 'Parallel', sweep: 'Context sweep' }[mode] || 'Unknown';
  }
  function labList(result) {
    const list = Array.isArray(result.streams_out) ? result.streams_out : [];
    return list.filter((item) => item && typeof item === 'object').slice(0, 64);
  }
  function labEach(list, key, format) {
    return list.length ? list.map((item) => format(item[key])).join(' / ') : '-';
  }
  function labSlots() {
    return lab.info && Number.isSafeInteger(lab.info.slots) ? lab.info.slots : 4;
  }
  function labSize(tokens) {
    return tokens >= 1024 && tokens % 1024 === 0 ? `${tokens / 1024}k` : String(tokens);
  }
  function labSay(text, wait) {
    clearInterval(lab.timer);
    if (!wait) {
      setText(lab.status, text);
      return;
    }
    let left = wait;
    const show = () => setText(lab.status, left > 0 ? `${text}: busy, try again in ${left} s.` : `${text}. You can try again now.`);
    show();
    lab.timer = setInterval(() => {
      left -= 1;
      show();
      if (left <= 0) clearInterval(lab.timer);
    }, 1000);
  }
  function labRefusal(status, reply) {
    const code = reply && typeof reply.error === 'string' ? reply.error : '';
    const limits = lab.info && lab.info.limits && typeof lab.info.limits === 'object' ? lab.info.limits : {};
    if (code === 'offline') return OFFLINE_TEXT;
    if (status === 503 || code === 'busy') return 'Every test slot is taken';
    if (status === 429 || code === 'rate') {
      const runs = Number.isSafeInteger(limits.runs_per_min) ? limits.runs_per_min : 6;
      const heavy = Number.isSafeInteger(limits.heavy_per_10min) ? limits.heavy_per_10min : 3;
      return `That is more runs than the limit (${runs} a minute, ${heavy} heavy tests in 10 minutes)`;
    }
    if (status === 409 || code === 'one_at_a_time') return 'You already have a test running, maybe in another tab. Let it finish first';
    if (status === 413 || code === 'too_long') return 'That prompt is too long: at most 48 KiB and about 32768 tokens';
    if (status === 400 || code === 'bad_request') return `The gateway could not read that request${reply && typeof reply.message === 'string' ? `: ${reply.message.slice(0, 200)}` : ''}`;
    return `The speed test is not answering right now (HTTP ${status})`;
  }
  function labIdle() {
    const info = lab.info;
    if (!info || lab.active || lab.shared || lab.offline) return;
    const busy = Number.isSafeInteger(info.tests_in_flight) ? info.tests_in_flight : 0;
    const queue = Number.isSafeInteger(info.queue) ? info.queue : 0;
    labSay(`Ready. ${busy} of ${labSlots()} test slots busy${queue ? `, ${queue} in line` : ''}. Press a preset.`);
  }
  function labLabel(source) {
    const model = typeof source.model === 'string' && source.model ? source.model.slice(0, 80) : lab.options.model;
    const flight = source.server_in_flight;
    let text = `Live shared server: ${lab.options.server}, ${model}. Numbers vary with load`;
    if (Number.isSafeInteger(flight)) text += `; ${flight} ${flight === 1 ? 'request was' : 'requests were'} in flight on the server when this run started`;
    if (!Number.isNaN(labNumber(source.load1))) text += `, host load ${source.load1}`;
    setText(lab.label, `${text}.`);
  }
  function labCaption(sweep) {
    setText(lab.caption, sweep
      ? 'Prefill tokens per second against prompt tokens (log2 scale), one point per step, from the server count: raw, no smoothing.'
      : `Tokens per second against seconds since the start, one line per stream: raw, ${BUCKET_MS} ms buckets, no smoothing.`);
    lab.canvas.setAttribute('aria-label', sweep ? 'Prefill tokens per second for each prompt size' : 'Tokens per second since the start of the run');
  }
  function labMode() {
    const pick = lab.modes.querySelector('input[type="radio"]:checked');
    return pick ? pick.value : 'single';
  }
  function labModeShow() {
    const mode = labMode();
    lab.panel.dataset.mode = mode;
    for (const node of lab.panel.querySelectorAll('[data-for]')) node.hidden = !node.dataset.for.split(' ').includes(mode);
    for (const option of lab.max.options) option.disabled = mode === 'concurrent' && Number(option.value) > PARALLEL_MAX;
    if (mode === 'concurrent' && Number(lab.max.value) > PARALLEL_MAX) lab.max.value = String(PARALLEL_MAX);
    lab.max.disabled = mode === 'sweep' || Boolean(lab.active);
    lab.prompt.disabled = mode === 'sweep';
    for (const item of lab.presetButtons) item.disabled = Boolean(lab.active) || lab.offline || mode === 'sweep';
    lab.run.disabled = Boolean(lab.active) || lab.offline;
  }
  function labBusy(on) {
    lab.panel.dataset.state = on ? 'running' : lab.offline ? 'offline' : 'idle';
    lab.halt.disabled = !on;
    for (const input of lab.modes.querySelectorAll('input, select')) input.disabled = on;
    labModeShow();
  }
  /* While the model is down, infer-info says server_ok false: the pane says so,
     turns its run controls off, and asks again every OFFLINE_POLL_MS. */
  function labOffline(on) {
    lab.offline = on;
    clearTimeout(lab.poll);
    lab.poll = on ? setTimeout(labInfo, OFFLINE_POLL_MS) : 0;
    if (!lab.active) labBusy(false);
    if (on && !lab.shared) labSay(OFFLINE_TEXT);
    labPaint();
  }
  async function labInfo() {
    const mine = lab;
    try {
      const response = await fetch(mine.routes.info, { cache: 'no-store', credentials: 'omit' });
      if (!response.ok) return;
      const info = await response.json();
      if (lab !== mine || !info || typeof info !== 'object') return;
      lab.info = info;
      const presets = info.presets && typeof info.presets === 'object' ? info.presets : {};
      for (const item of lab.presetButtons) {
        const preset = presets[item.dataset.preset];
        if (!preset || typeof preset !== 'object') continue;
        if (Number.isSafeInteger(preset.max_tokens)) setText(item.querySelector('.sp-preset-n'), `${preset.max_tokens} tokens`);
        if (typeof preset.prompt === 'string') item.title = cut(preset.prompt, 300);
      }
      if (info.server_ok === false) {
        labOffline(true);
        return;
      }
      if (lab.offline) labOffline(false);
      labIdle();
    } catch (_) {
      /* The presets still run with the server's own numbers. */
    }
  }
  function labView(mode) {
    return {
      mode, click: performance.now(), first: NaN, start: null, done: null, yielded: null,
      outcome: '', tokens: 0, series: [], lanes: [], marks: [], steps: [], stepAt: 0,
    };
  }
  function labLaneName(view, index) {
    if (view.mode === 'sweep') {
      const size = view.steps[index];
      return `Step ${index + 1}${Number.isSafeInteger(size) ? `: ${labSize(size)} prompt tokens` : ''}`;
    }
    return view.mode === 'single' ? 'Output' : `Stream ${index + 1}`;
  }
  function labLane(view, index) {
    while (view.lanes.length <= index) {
      const number = view.lanes.length;
      const tally = h('span', { class: 'sp-out-tally' });
      const text = h('pre', { class: 'sp-text' });
      const stats = h('p', { class: 'sp-out-stats' });
      lab.outputs.append(h('div', { class: view.mode === 'sweep' ? 'sp-step' : 'sp-out', 'data-lane': number % 4 },
        h('p', { class: 'sp-out-head' }, h('span', { class: 'sp-out-name', text: labLaneName(view, number) }), tally),
        text, stats));
      view.lanes.push({ text, tally, stats, buf: '', shown: 0, count: 0, stat: null });
    }
    return view.lanes[index];
  }
  function labStat(stat, mode) {
    const parts = [];
    if (mode === 'sweep') parts.push(`${labWord(stat.prompt_tokens, 12)} prompt tokens`);
    parts.push(`first token ${labMs(stat.ttft_ms)} ms`);
    parts.push(`prefill ${labRate(stat.prefill_tps)} tok/s`);
    if (mode !== 'sweep') parts.push(`decode ${labRate(stat.decode_tps)} tok/s`);
    parts.push(`${labWord(stat.completion_tokens, 12)} tokens out in ${labMs(stat.total_ms)} ms`);
    if (typeof stat.finish === 'string') parts.push(`finish ${stat.finish.slice(0, 24)}`);
    return parts.join(', ');
  }
  function labRows(result, view) {
    const list = labList(result);
    const streams = Number.isSafeInteger(result.streams) && result.streams > 1 ? `, ${result.streams} streams` : '';
    const max = Number.isSafeInteger(result.max_tokens) && result.mode !== 'sweep' ? `, max ${result.max_tokens} tokens` : '';
    let client = 'not in a shared link';
    if (view) client = Number.isNaN(view.first) ? '-' : `${Math.round(view.first - view.click)} ms`;
    const rows = [
      ['Mode', `${labModeName(result.mode)}${streams}${max}${result.yielded === true ? ', yielded to a live chat' : ''}`],
      ['First token, server (ms)', labEach(list, 'ttft_ms', labMs)],
      ['First token, from Run', client],
      ['Prefill tok/s', labEach(list, 'prefill_tps', labRate)],
      ['Decode tok/s', labEach(list, 'decode_tps', labRate)],
      ['Total (ms)', `${labEach(list, 'total_ms', labMs)}${Number.isNaN(labNumber(result.wall_ms)) ? '' : `, wall ${labMs(result.wall_ms)}`}`],
      ['Tokens in / out', list.length ? list.map((item) => `${labWord(item.prompt_tokens, 12)} / ${labWord(item.completion_tokens, 12)}`).join(', ') : '-'],
    ];
    if (result.mode === 'concurrent') rows.push(['Aggregate tok/s', labRate(result.aggregate_tps)]);
    rows.push(['Server requests in flight at start', labWord(result.server_in_flight, 12)]);
    rows.push(['Test slots in use', `${labWord(result.tests_in_flight, 12)} of ${labSlots()}`]);
    rows.push(['Host load (1 min)', labWord(result.load1, 12)]);
    if (typeof result.run === 'string' && result.run) rows.push(['Run', `${result.run.slice(0, 40)}${typeof result.utc === 'string' && result.utc ? ` at ${result.utc.slice(0, 40)}` : ''}`]);
    return rows;
  }
  function labResult(result, view) {
    lab.results.replaceChildren(...labRows(result, view).map(([name, value]) => h('div', { class: 'sp-stat' }, h('dt', { text: name }), h('dd', { text: value }))));
    labLabel(result);
  }
  function labPartial(view) {
    const start = view.start || {};
    return {
      mode: view.mode, model: start.model, run: start.run, streams: start.streams,
      max_tokens: start.max_tokens, server_in_flight: start.server_in_flight,
      tests_in_flight: start.tests_in_flight, load1: start.load1,
      streams_out: view.lanes.map((lane) => lane.stat).filter(Boolean),
    };
  }
  function historyRead() {
    try {
      const rows = JSON.parse(localStorage.getItem(lab.options.historyKey) || '[]');
      return Array.isArray(rows) ? rows.filter((row) => row && typeof row === 'object').slice(0, HISTORY_MAX) : [];
    } catch (_) {
      return [];
    }
  }
  function historyWrite(rows) {
    try {
      localStorage.setItem(lab.options.historyKey, JSON.stringify(rows.slice(0, HISTORY_MAX)));
    } catch (_) {
      /* A full or blocked store keeps the table as it was. */
    }
  }
  function historyClear() {
    try {
      localStorage.removeItem(lab.options.historyKey);
    } catch (_) {
      /* Nothing stored. */
    }
    historyShow();
  }
  function historyAdd(result, link) {
    const list = labList(result);
    const streams = Number.isSafeInteger(result.streams) && result.streams > 1 ? ` x${result.streams}` : '';
    const out = list.reduce((sum, item) => sum + (Number.isSafeInteger(item.completion_tokens) ? item.completion_tokens : 0), 0);
    historyWrite([{
      when: typeof result.utc === 'string' ? result.utc.slice(0, 40) : new Date().toISOString(),
      mode: `${labModeName(result.mode)}${streams}${result.yielded === true ? ' (yielded)' : ''}`,
      ttft: labEach(list, 'ttft_ms', labMs),
      prefill: labEach(list, 'prefill_tps', labRate),
      decode: result.mode === 'sweep' ? '-' : labEach(list, 'decode_tps', labRate),
      agg: result.mode === 'concurrent' ? labRate(result.aggregate_tps) : '-',
      out: String(out),
      inflight: labWord(result.server_in_flight, 12),
      load: labWord(result.load1, 12),
      share: link,
    }, ...historyRead()]);
    historyShow();
  }
  function historyShow() {
    const body = lab.history.tBodies[0];
    const rows = historyRead();
    body.replaceChildren(...rows.map((row) => {
      const stamp = labWord(row.when, 40).replace('T', ' ').slice(0, 19);
      const shared = lab.options.share && typeof row.share === 'string' && /^[A-Za-z0-9_-]{1,65536}$/.test(row.share);
      const when = h('td', {}, shared ? h('a', { href: `#r=${row.share}`, text: stamp }) : stamp);
      return h('tr', {}, when, ...['mode', 'ttft', 'prefill', 'decode', 'agg', 'out', 'inflight', 'load'].map((key) => h('td', { text: labWord(row[key], 200) })));
    }));
    if (!rows.length) body.append(h('tr', {}, h('td', { class: 'sp-empty', colspan: 9, text: 'No runs yet. Your runs appear here, kept only in this browser.' })));
    lab.clear.disabled = !rows.length;
  }
  function labPack(text) {
    let binary = '';
    for (const byte of new TextEncoder().encode(text)) binary += String.fromCharCode(byte);
    return btoa(binary).replace(/\+/g, '-').replace(/\//g, '_').replace(/=+$/, '');
  }
  function labUnpack(text) {
    const plain = text.replace(/-/g, '+').replace(/_/g, '/');
    const binary = atob(plain + '='.repeat((4 - (plain.length % 4)) % 4));
    const bytes = Uint8Array.from(binary, (char) => char.charCodeAt(0));
    return new TextDecoder('utf-8', { fatal: true }).decode(bytes);
  }
  function labVerdict(verdict) {
    lab.verify.hidden = false;
    lab.verify.dataset.state = verdict === true ? 'ok' : verdict === false ? 'bad' : 'unknown';
    setText(lab.verify, verdict === true
      ? 'Verified: produced by this gateway'
      : verdict === false
        ? 'Not verified: this link was edited or is from another server'
        : 'Not checked: the gateway did not answer. Open the link again later.');
  }
  async function labShared() {
    const match = SHARE_ADDRESS.exec(location.hash);
    if (!lab.options.share || !match || lab.active) return;
    const mine = lab;
    let payload = null;
    try {
      payload = JSON.parse(labUnpack(match[1]));
    } catch (_) {
      payload = null;
    }
    lab.view = null;
    lab.link = '';
    lab.share.disabled = true;
    lab.outputs.replaceChildren();
    const readable = payload && typeof payload === 'object' && payload.result && typeof payload.result === 'object'
      && !Array.isArray(payload.result) && typeof payload.sig === 'string' && /^[0-9a-f]{16,256}$/.test(payload.sig);
    if (!readable) {
      lab.shared = null;
      lab.results.replaceChildren();
      labVerdict(false);
      labSay('This share link could not be read.');
      labDirty();
      return;
    }
    lab.shared = payload;
    const result = payload.result;
    labResult(result, null);
    labCaption(result.mode === 'sweep');
    labSay(`Showing a shared run${typeof result.utc === 'string' ? ` from ${result.utc.slice(0, 40)}` : ''}. Press a preset to run your own.`);
    lab.verify.hidden = false;
    lab.verify.dataset.state = 'checking';
    setText(lab.verify, 'Checking this link with the gateway.');
    labDirty();
    let verdict = null;
    try {
      const response = await fetch(mine.routes.verify, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ result, sig: payload.sig }),
        cache: 'no-store',
        credentials: 'omit',
      });
      if (response.status < 500) {
        const reply = await response.json().catch(() => null);
        verdict = Boolean(response.ok && reply && reply.ok === true);
      }
    } catch (_) {
      verdict = null;
    }
    if (lab === mine && lab.shared === payload) labVerdict(verdict);
  }
  function labCopyLink() {
    if (lab.link) copyText(`${location.origin}${location.pathname}${location.search}#r=${lab.link}`, lab.share);
  }
  function labEvent(name, data, view) {
    let value;
    try {
      value = JSON.parse(data);
    } catch (_) {
      return;
    }
    if (!value || typeof value !== 'object') return;
    if (name === 'queue') {
      if (value.reason === 'chat') labSay('Waiting: a live chat is running; tests yield to chat.');
      else labSay(`In line: position ${labWord(value.position, 6)}, ${labWord(value.tests_in_flight, 6)} of ${labWord(value.slots, 6)} test slots busy.`);
    } else if (name === 'start') {
      view.start = value;
      view.steps = Array.isArray(value.steps) ? value.steps.filter(Number.isSafeInteger).slice(0, 16) : [];
      const lanes = view.mode === 'sweep' ? view.steps.length : Number.isSafeInteger(value.streams) ? Math.min(value.streams, 4) : 1;
      for (let index = 0; index < Math.max(lanes, 1); index += 1) labLane(view, index);
      labResult(labPartial(view), view);
      labSay(view.mode === 'sweep'
        ? `Sweep running: ${view.steps.length} steps, up to ${labSize(view.steps[view.steps.length - 1] || 0)} prompt tokens.`
        : 'Running: reading the prompt, waiting for the first token.');
    } else if (name === 'tok') {
      const index = value.i;
      const time = labNumber(value.t);
      if (!Number.isSafeInteger(index) || index < 0 || index > 15 || Number.isNaN(time) || time < 0) return;
      if (Number.isNaN(view.first)) {
        view.first = performance.now();
        labResult(labPartial(view), view);
        if (view.mode !== 'sweep') labSay('Streaming.');
      }
      view.tokens += 1;
      const lane = labLane(view, index);
      if (typeof value.s === 'string') lane.buf += value.s;
      lane.count += 1;
      if (view.mode !== 'sweep') {
        const bucket = Math.floor(time / BUCKET_MS);
        const series = view.series[index] || (view.series[index] = []);
        while (series.length <= bucket) series.push(0);
        series[bucket] += 1;
      }
      labDirty();
    } else if (name === 'stream') {
      const index = Number.isSafeInteger(value.i) && value.i >= 0 && value.i <= 15 ? value.i : view.lanes.length;
      labLane(view, index).stat = value;
      const started = labNumber(value.start_ms);
      const total = labNumber(value.total_ms);
      if (!Number.isNaN(started) && !Number.isNaN(total)) view.stepAt = Math.max(view.stepAt, started + total);
      const tokens = value.prompt_tokens;
      const rate = labNumber(value.prefill_tps);
      if (view.mode === 'sweep' && Number.isSafeInteger(tokens) && tokens > 0 && !Number.isNaN(rate)) view.marks.push({ x: tokens, y: rate });
      labResult(labPartial(view), view);
      if (view.mode === 'sweep' && view.marks.length < view.steps.length) labSay(`Step ${view.marks.length + 1} of ${view.steps.length}: prefilling.`);
      labDirty();
    } else if (name === 'beat') {
      const time = labNumber(value.t);
      const seconds = Number.isNaN(time) ? 0 : Math.max(0, Math.round((time - view.stepAt) / 1000));
      labSay(view.mode === 'sweep'
        ? `Step ${Math.min(view.marks.length + 1, view.steps.length || 1)} of ${view.steps.length || 1}: prefilling, ${seconds} s.`
        : `Prefilling, ${seconds} s.`);
    } else if (name === 'yielded') {
      view.yielded = value;
      labSay(`Yielded to a live chat after ${labWord(value.completed_steps, 6)} completed steps.`);
    } else if (name === 'done') {
      const result = value.result;
      if (!result || typeof result !== 'object' || Array.isArray(result)) return;
      view.done = value;
      labResult(result, view);
      lab.link = typeof value.sig === 'string' ? labPack(JSON.stringify({ result, sig: value.sig })) : '';
      lab.share.disabled = !lab.link || !lab.options.share;
      historyAdd(result, lab.link);
    } else if (name === 'error') {
      view.outcome = `The run stopped with an error: ${labWord(value.message || value.code, 200)}`;
    }
  }
  function labBody(preset) {
    const mode = labMode();
    const body = { mode };
    if (mode === 'sweep') {
      body.sweep_max = Number(lab.sweep.value);
      return body;
    }
    const text = lab.prompt.value.trim() ? lab.prompt.value : '';
    if (preset || !text) {
      body.preset = preset || lab.preset;
    } else {
      body.prompt = text;
      body.max_tokens = Number(lab.max.value);
    }
    if (mode === 'concurrent') body.streams = Number(lab.streams.value);
    return body;
  }
  async function labRun(preset) {
    if (!lab || lab.active || lab.offline) return;
    const body = labBody(preset);
    if (body.prompt !== undefined) {
      const bytes = new TextEncoder().encode(body.prompt).length;
      if (bytes > PROMPT_BYTES) {
        labSay(`Your prompt is ${bytes} bytes; the limit is ${PROMPT_BYTES} bytes.`);
        return;
      }
    }
    if (body.preset) lab.preset = body.preset;
    for (const item of lab.presetButtons) item.setAttribute('aria-pressed', String(Boolean(body.preset) && item.dataset.preset === body.preset));
    if (SHARE_ADDRESS.test(location.hash)) history.replaceState(null, '', `${location.pathname}${location.search}`);
    lab.shared = null;
    lab.verify.hidden = true;
    lab.link = '';
    lab.share.disabled = true;
    lab.outputs.replaceChildren();
    lab.results.replaceChildren();
    const mine = lab;
    const view = labView(body.mode);
    lab.view = view;
    const controller = new AbortController();
    lab.active = controller;
    labBusy(true);
    labCaption(body.mode === 'sweep');
    labSay('Sending.');
    labPaint();
    let wait = 0;
    let offline = false;
    try {
      const response = await fetch(mine.routes.infer, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json', Accept: 'text/event-stream' },
        body: JSON.stringify(body),
        cache: 'no-store',
        credentials: 'omit',
        signal: controller.signal,
      });
      const type = response.headers.get('Content-Type') || '';
      if (!response.ok) {
        const reply = await response.json().catch(() => null);
        wait = retryAfter(response, reply);
        offline = Boolean(reply && reply.error === 'offline');
        view.outcome = labRefusal(response.status, reply);
      } else if (!response.body || !type.includes('text/event-stream')) {
        view.outcome = 'The gateway answered in a form this page does not read.';
      } else {
        const reader = response.body.getReader();
        const decoder = new TextDecoder();
        const parser = sse((name, data) => {
          if (lab === mine) labEvent(name, data, view);
        });
        for (;;) {
          const { value, done } = await reader.read();
          if (done) break;
          parser.feed(decoder.decode(value, { stream: true }));
        }
        parser.feed(decoder.decode());
        parser.end();
        if (!view.done && !view.outcome) view.outcome = 'The connection closed before the run finished.';
      }
    } catch (_) {
      view.outcome = controller.signal.aborted
        ? 'Stopped. The partial output stays above; a stopped run has no signed result.'
        : 'Could not reach the gateway. Check your connection and try again.';
    }
    if (lab !== mine) return;
    lab.active = null;
    labBusy(false);
    labPaint();
    if (offline) labOffline(true);
    else if (view.outcome) labSay(view.outcome, wait);
    else if (view.yielded) labSay('Yielded to a live chat: the numbers below are partial.');
    else labSay(lab.options.share ? 'Done. Copy the share link to show someone these exact numbers.' : 'Done.');
  }
  function labDirty() {
    if (lab && !lab.frame) lab.frame = requestAnimationFrame(labPaint);
  }
  function labPaint() {
    if (!lab) return;
    if (lab.frame) cancelAnimationFrame(lab.frame);
    lab.frame = 0;
    const view = lab.view;
    if (view) {
      for (const lane of view.lanes) {
        if (lane.shown !== lane.buf.length) {
          lane.text.textContent = lane.buf;
          lane.shown = lane.buf.length;
          lane.text.scrollTop = lane.text.scrollHeight;
        }
        setText(lane.tally, lane.count ? `${lane.count} ${lane.count === 1 ? 'token' : 'tokens'}` : '');
        if (lane.stat) setText(lane.stats, labStat(lane.stat, view.mode));
      }
    }
    labHero();
    labDraw();
  }
  /* The big number: live tok/s over the last full second of buckets while a run
     streams, then the run's own headline figure from the server's result. */
  function labHero() {
    const view = lab.view;
    const result = view && view.done ? view.done.result : lab.shared ? lab.shared.result : null;
    let value = NaN;
    let text = 'tok/s shows here as the first token lands';
    if (result) {
      const list = labList(result);
      if (result.mode === 'concurrent') {
        value = labNumber(result.aggregate_tps);
        text = `aggregate tok/s over ${list.length} streams`;
      } else if (result.mode === 'sweep') {
        value = list.reduce((most, item) => Math.max(most, Number.isNaN(labNumber(item.prefill_tps)) ? 0 : item.prefill_tps), 0) || NaN;
        text = 'peak prefill tok/s';
      } else {
        value = list.length ? labNumber(list[0].decode_tps) : NaN;
        text = 'decode tok/s';
      }
    } else if (view && view.mode === 'sweep') {
      const mark = view.marks[view.marks.length - 1];
      if (mark) {
        value = mark.y;
        text = 'prefill tok/s, last step';
      }
    } else if (view) {
      const last = view.series.reduce((most, line) => Math.max(most, line ? line.length - 1 : 0), 0);
      const from = Math.max(0, last - 4);
      if (last > 0) {
        let count = 0;
        for (const line of view.series) if (line) for (let bucket = from; bucket < last; bucket += 1) count += line[bucket] || 0;
        value = (count * 1000) / ((last - from) * BUCKET_MS);
        text = view.mode === 'concurrent' ? 'tok/s right now, all streams' : 'tok/s right now';
      }
    }
    setText(lab.heroNumber, labRate(value));
    setText(lab.heroText, text);
  }
  function labStep(span, parts) {
    const raw = span / parts;
    const power = Math.pow(10, Math.floor(Math.log10(raw)));
    for (const scale of [1, 2, 5]) if (scale * power >= raw) return scale * power;
    return 10 * power;
  }
  function labTop(peak) {
    const step = labStep(peak > 0 ? peak : 10, 4);
    return { top: Math.max(step, Math.ceil(peak / step) * step), step };
  }
  function labAxes(g, box, xs, ys, names, ink, faint) {
    g.lineWidth = 1;
    g.strokeStyle = faint;
    g.fillStyle = ink;
    g.textAlign = 'right';
    g.textBaseline = 'middle';
    for (const [text, y] of ys) {
      g.beginPath();
      g.moveTo(box.l, y);
      g.lineTo(box.r, y);
      g.stroke();
      g.fillText(text, box.l - 6, y);
    }
    g.textAlign = 'center';
    g.textBaseline = 'top';
    for (const [text, x] of xs) {
      g.beginPath();
      g.moveTo(x, box.b);
      g.lineTo(x, box.b + 4);
      g.stroke();
      g.fillText(text, x, box.b + 6);
    }
    g.fillText(names[0], (box.l + box.r) / 2, box.b + 20);
    g.save();
    g.translate(11, (box.t + box.b) / 2);
    g.rotate(-Math.PI / 2);
    g.textBaseline = 'middle';
    g.fillText(names[1], 0, 0);
    g.restore();
  }
  function labEmpty(g, box, text, ink) {
    g.fillStyle = ink;
    g.textAlign = 'center';
    g.textBaseline = 'middle';
    g.fillText(text, (box.l + box.r) / 2, (box.t + box.b) / 2);
  }
  function labDraw() {
    const canvas = lab.canvas;
    if (!canvas.isConnected || !canvas.clientWidth) return;
    const width = Math.max(240, Math.round(canvas.clientWidth));
    const height = Math.max(160, Math.round(canvas.clientHeight || 240));
    const ratio = Math.min(window.devicePixelRatio || 1, 2);
    if (canvas.width !== Math.round(width * ratio)) canvas.width = Math.round(width * ratio);
    if (canvas.height !== Math.round(height * ratio)) canvas.height = Math.round(height * ratio);
    const g = canvas.getContext('2d');
    if (!g) return;
    g.setTransform(ratio, 0, 0, ratio, 0, 0);
    g.clearRect(0, 0, width, height);
    const css = getComputedStyle(lab.host);
    const color = (name) => css.getPropertyValue(name).trim() || 'gray';
    const colors = [color('--speed-lane-1'), color('--speed-lane-2'), color('--speed-lane-3'), color('--speed-lane-4')];
    g.font = `${width < 420 ? 10 : 11}px ${css.getPropertyValue('--speed-mono').trim() || 'monospace'}`;
    const box = { l: 52, r: width - 12, t: 12, b: height - 36 };
    const ink = color('--speed-muted');
    const faint = color('--speed-line');
    const view = lab.view;
    const shared = lab.shared ? lab.shared.result : null;
    const mode = view ? view.mode : shared ? shared.mode : labMode();
    if (mode === 'sweep') {
      let marks = view ? view.marks : [];
      let steps = view ? view.steps : [];
      if (!view && shared) {
        const list = labList(shared);
        marks = list.filter((item) => Number.isSafeInteger(item.prompt_tokens) && item.prompt_tokens > 0 && !Number.isNaN(labNumber(item.prefill_tps)))
          .map((item) => ({ x: item.prompt_tokens, y: item.prefill_tps }));
        steps = list.map((item) => item.target_tokens).filter(Number.isSafeInteger);
      }
      labSweepDraw(g, box, marks, steps, colors, ink, faint, color('--speed-fg'));
    } else {
      labRateDraw(g, box, view, colors, ink, faint, Boolean(shared));
    }
  }
  function labRateDraw(g, box, view, colors, ink, faint, shared) {
    const series = view ? view.series : [];
    let buckets = 0;
    let peak = 0;
    for (const line of series) {
      if (!line) continue;
      buckets = Math.max(buckets, line.length);
      for (const value of line) peak = Math.max(peak, value);
    }
    const seconds = Math.max(2, Math.ceil((buckets * BUCKET_MS) / 1000));
    const { top, step } = labTop((peak * 1000) / BUCKET_MS);
    const xAt = (second) => box.l + ((box.r - box.l) * second) / seconds;
    const yAt = (rate) => box.b - ((box.b - box.t) * rate) / top;
    const xs = [];
    const xStep = labStep(seconds, box.r - box.l < 360 ? 4 : 8);
    for (let second = 0; second <= seconds + 1e-9; second += xStep) xs.push([String(Math.round(second * 10) / 10), xAt(second)]);
    const ys = [];
    for (let rate = 0; rate <= top + 1e-9; rate += step) ys.push([String(Math.round(rate * 10) / 10), yAt(rate)]);
    labAxes(g, box, xs, ys, ['seconds since start', 'tok/s'], ink, faint);
    series.forEach((line, index) => {
      if (!line || !line.length) return;
      g.strokeStyle = colors[index % colors.length];
      g.lineWidth = 1.6;
      g.beginPath();
      g.moveTo(xAt(0), yAt(0));
      for (let bucket = 0; bucket < line.length; bucket += 1) {
        const rate = (line[bucket] * 1000) / BUCKET_MS;
        g.lineTo(xAt((bucket * BUCKET_MS) / 1000), yAt(rate));
        g.lineTo(xAt(((bucket + 1) * BUCKET_MS) / 1000), yAt(rate));
      }
      g.stroke();
    });
    if (series.filter(Boolean).length > 1) {
      g.textAlign = 'left';
      g.textBaseline = 'middle';
      series.forEach((line, index) => {
        const x = box.r - 70;
        const y = box.t + 8 + index * 15;
        g.fillStyle = colors[index % colors.length];
        g.fillRect(x, y - 2, 12, 4);
        g.fillText(`Stream ${index + 1}`, x + 16, y);
      });
    }
    if (!buckets) labEmpty(g, box, shared ? 'A shared link carries the result, not the token timeline.' : lab.offline ? 'Offline: the graph fills in when the model is back.' : 'Press a preset: each token is drawn as it arrives.', ink);
  }
  function labSweepDraw(g, box, marks, steps, colors, ink, faint, fg) {
    const sizes = [...new Set([...steps, ...marks.map((mark) => mark.x)].filter((size) => size > 0))].sort((a, b) => a - b);
    const low = Math.log2(sizes.length ? sizes[0] : 512) - 0.5;
    const high = Math.log2(sizes.length ? sizes[sizes.length - 1] : Number(lab.sweep.value)) + 0.5;
    const peak = marks.reduce((most, mark) => Math.max(most, mark.y), 0);
    const { top, step } = labTop(peak);
    const xAt = (size) => box.l + ((box.r - box.l) * (Math.log2(size) - low)) / (high - low);
    const yAt = (rate) => box.b - ((box.b - box.t) * rate) / top;
    const ticks = sizes.length ? sizes.filter((size) => steps.includes(size) || !steps.length) : [512, 1024, 2048, 4096];
    const xs = ticks.map((size) => [labSize(size), xAt(size)]);
    const ys = [];
    for (let rate = 0; rate <= top + 1e-9; rate += step) ys.push([String(Math.round(rate * 10) / 10), yAt(rate)]);
    labAxes(g, box, xs, ys, ['prompt tokens (log2 scale)', 'prefill tok/s'], ink, faint);
    g.textAlign = 'center';
    g.textBaseline = 'bottom';
    for (const mark of marks) {
      const x = xAt(mark.x);
      const y = yAt(mark.y);
      g.fillStyle = colors[0];
      g.beginPath();
      g.arc(x, y, 4, 0, Math.PI * 2);
      g.fill();
      g.fillStyle = fg;
      g.fillText(mark.y.toFixed(1), x, y - 6);
    }
    if (!marks.length) labEmpty(g, box, 'Each finished step adds one point: prompt size against prefill speed.', ink);
  }
  function listen(target, type, handler) {
    target.addEventListener(type, handler);
    lab.off.push(() => target.removeEventListener(type, handler));
  }
  function settings(given) {
    const options = { ...DEFAULTS };
    if (given && typeof given === 'object') {
      if (typeof given.api === 'string' && /^(\/|https:\/\/)[^\s]*$/.test(given.api)) options.api = given.api.replace(/\/+$/, '');
      for (const key of ['model', 'server', 'historyKey']) if (typeof given[key] === 'string' && given[key]) options[key] = given[key].slice(0, 120);
      if (typeof given.share === 'boolean') options.share = given.share;
    }
    return options;
  }
  function mount(host, given) {
    if (!(host instanceof Element)) throw new TypeError('AienSpeed.mount needs an element');
    unmount();
    lab = build(host, settings(given));
    for (const item of lab.presetButtons) listen(item, 'click', () => labRun(item.dataset.preset));
    listen(lab.run, 'click', () => labRun(''));
    listen(lab.halt, 'click', () => {
      if (lab.active) lab.active.abort();
    });
    listen(lab.prompt, 'keydown', (event) => {
      if (event.key === 'Enter' && (event.ctrlKey || event.metaKey)) {
        event.preventDefault();
        labRun('');
      }
    });
    listen(lab.modes, 'change', () => {
      labModeShow();
      if (!lab.active && !lab.shared) {
        lab.view = null;
        lab.outputs.replaceChildren();
        labCaption(labMode() === 'sweep');
        labDirty();
      }
    });
    listen(lab.clear, 'click', historyClear);
    listen(lab.share, 'click', labCopyLink);
    listen(lab.ownCopy, 'click', () => copyText(lab.own.textContent, lab.ownCopy));
    listen(window, 'resize', labDirty);
    listen(window, 'hashchange', () => {
      if (SHARE_ADDRESS.test(location.hash)) labShared();
    });
    if (typeof ResizeObserver === 'function') {
      const watch = new ResizeObserver(() => labDirty());
      watch.observe(lab.canvas);
      lab.off.push(() => watch.disconnect());
    }
    if (!lab.options.share) lab.share.hidden = true;
    labCaption(false);
    labModeShow();
    historyShow();
    labShared();
    labInfo();
    labPaint();
    return true;
  }
  function unmount() {
    if (!lab) return false;
    const gone = lab;
    lab = null;
    if (gone.active) gone.active.abort();
    clearInterval(gone.timer);
    clearTimeout(gone.poll);
    if (gone.frame) cancelAnimationFrame(gone.frame);
    for (const off of gone.off) off();
    gone.host.replaceChildren();
    gone.host.classList.remove('aien-speed');
    return true;
  }
  window.AienSpeed = Object.freeze({ mount, unmount });
})();

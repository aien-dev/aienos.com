/* Public snapshot consumer. Feed strings only enter text nodes, never markup. */
(() => {
  'use strict';
  const page = document.getElementById('hive');
  if (!page) return;
  const get = id => document.getElementById(`hive-${id}`);
  const statuses = new Set(['working', 'waiting', 'idle', 'completed', 'failed', 'unknown']);
  const actions = new Set(['running', 'finished', 'unknown']);
  const hierarchyParents = new Map();
  const records = new Map();
  const messages = new Map();
  let snapshot = null;
  let received = null;
  let selected = null;
  let scope = 'all';
  let sector = 0;
  let order = 0;
  let connected = false;
  let advancedAt = 0;
  let sourceAgeAtAdvance = 0;
  let latestGenerated = -Infinity;
  let freshnessTrusted = false;
  let regressed = false;
  let requestActive = false;
  const familyButtons = new Map();
  let newMessages = 0;
  let query = '';
  const reducedMotion = window.matchMedia('(prefers-reduced-motion: reduce)');
  let paused = false;
  try { paused = localStorage.getItem('hive-motion-paused') === 'true'; } catch (_) { /* Storage is optional. */ }

  function text(node, value) {
    if (node.textContent !== value) node.textContent = value;
  }
  function make(tag, className, value) {
    const node = document.createElement(tag);
    if (className) node.className = className;
    if (value !== undefined) node.textContent = value;
    return node;
  }
  function utc(value) {
    return value ? new Date(value).toISOString().slice(0, 19).replace('T', ' ') + ' UTC' : 'Not reported';
  }
  function setTime(node, value) {
    text(node, utc(value));
    if (value) node.setAttribute('datetime', value);
    else node.removeAttribute('datetime');
  }
  function timestamp(value) {
    if (typeof value !== 'string' || !/^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}(?:\.\d{1,9})?Z$/.test(value)) return false;
    const date = new Date(value);
    return Number.isFinite(date.getTime()) && date.toISOString().slice(0, 19) === value.slice(0, 19);
  }
  function object(value, fields) {
    return value !== null && typeof value === 'object' && !Array.isArray(value)
      && Object.keys(value).length === fields.length && fields.every(field => Object.hasOwn(value, field));
  }
  function string(value, max, empty = false) {
    return typeof value === 'string' && (empty || value.length > 0) && value.length <= max && !/[\u0000-\u0008\u000b\u000c\u000e-\u001f]/.test(value);
  }
  function id(value) { return typeof value === 'string' && /^[a-zA-Z0-9_-]{1,160}$/.test(value); }
  function validate(value) {
    const fail = () => { throw new Error('Invalid public snapshot'); };
    if (!object(value, ['schema_version', 'generated_at', 'sequence', 'poll_ms', 'stale_after_ms', 'coverage', 'agents', 'interactions'])
      || value.schema_version !== 1 || !timestamp(value.generated_at) || !string(value.sequence, 256)
      || value.poll_ms !== 1000 || !Number.isInteger(value.stale_after_ms) || value.stale_after_ms < 1000 || value.stale_after_ms > 120000
      || !object(value.coverage, ['label', 'roots', 'agents', 'complete', 'notice'])
      || !string(value.coverage.label, 1000) || !string(value.coverage.notice, 8000, true)
      || typeof value.coverage.complete !== 'boolean' || !Number.isSafeInteger(value.coverage.roots) || value.coverage.roots < 0
      || !Number.isSafeInteger(value.coverage.agents) || value.coverage.agents < 0
      || !Array.isArray(value.agents) || !Array.isArray(value.interactions) || value.interactions.length > 100) fail();
    const agents = new Map();
    let roots = 0;
    for (const agent of value.agents) {
      if (!object(agent, ['id', 'parent_id', 'name', 'runtime', 'provider', 'model', 'role', 'task', 'status', 'activity', 'activity_state', 'started_at', 'updated_at', 'observed_at'])
        || !id(agent.id) || agents.has(agent.id) || !(agent.parent_id === null || id(agent.parent_id)) || agent.parent_id === agent.id
        || !['name', 'runtime', 'provider', 'model', 'role'].every(key => string(agent[key], 1000))
        || !string(agent.task, 12000, true) || !string(agent.activity, 12000, true)
        || !statuses.has(agent.status) || !actions.has(agent.activity_state)
        || !(agent.started_at === null || timestamp(agent.started_at)) || !(agent.updated_at === null || timestamp(agent.updated_at))
        || !timestamp(agent.observed_at)) fail();
      agents.set(agent.id, agent);
      if (agent.parent_id === null) roots++;
    }
    if (value.coverage.agents !== agents.size || value.coverage.roots !== roots) fail();
    const checked = new Set();
    for (const agent of agents.values()) {
      const path = new Set();
      let current = agent;
      while (current && !checked.has(current.id)) {
        if (path.has(current.id)) fail();
        path.add(current.id);
        current = agents.get(current.parent_id);
      }
      for (const entry of path) checked.add(entry);
    }
    const interactionIds = new Set();
    for (const item of value.interactions) {
      if (!object(item, ['id', 'agent_id', 'role', 'text', 'at', 'runtime', 'provider', 'model']) || !id(item.id) || !id(item.agent_id)
        || interactionIds.has(item.id) || !['drake', 'assistant'].includes(item.role)
        || !['runtime', 'provider', 'model'].every(key => string(item[key], 1000, true))
        || !string(item.text, 8000) || [...item.text].length > 4000 || !timestamp(item.at)) fail();
      interactionIds.add(item.id);
    }
    return value;
  }

  function glyph(child = false) {
    const ns = 'http://www.w3.org/2000/svg';
    const svg = document.createElementNS(ns, 'svg');
    svg.setAttribute('viewBox', '0 0 30 36');
    svg.setAttribute('aria-hidden', 'true');
    const hull = document.createElementNS(ns, 'path');
    hull.setAttribute('d', child ? 'M5 27L15 5L25 27L15 21Z' : 'M15 2L28 29L18 24L15 30L12 24L2 29ZM15 10L19 19L15 23L11 19Z');
    hull.setAttribute('fill', 'none');
    hull.setAttribute('stroke', 'currentColor');
    hull.setAttribute('stroke-width', '1.25');
    hull.setAttribute('class', 'hive-hull');
    const engine = document.createElementNS(ns, 'path');
    engine.setAttribute('d', 'M12 32L15 35L18 32Z');
    engine.setAttribute('class', 'hive-engine');
    svg.append(hull, engine);
    return svg;
  }
  function button(label, handler, className) {
    const node = make('button', className, label);
    node.type = 'button';
    node.addEventListener('click', handler);
    return node;
  }
  function place(parent, nodes) {
    let cursor = parent.firstElementChild;
    for (const node of nodes) {
      if (node === cursor) cursor = cursor.nextElementSibling;
      else parent.insertBefore(node, cursor);
    }
  }
  let pulseWindow = 0;
  let pulseBudget = 0;
  function pulse(node) {
    if (page.dataset.motion !== 'on' || page.dataset.fresh !== 'true' || document.hidden) return;
    // A busy fleet changes every second; a strobe of accents is not information.
    const now = performance.now();
    if (now - pulseWindow > 1000) { pulseWindow = now; pulseBudget = 3; }
    if (pulseBudget <= 0) return;
    pulseBudget--;
    node.classList.add('hive-changed');
    setTimeout(() => node.classList.remove('hive-changed'), 1300);
  }
  function createRecord(agent) {
    const li = make('li', 'hive-node');
    li.id = `hive-agent-${agent.id}`;
    const record = make('div', 'hive-node-record');
    const parent = make('p', 'hive-node-parent');
    const select = button('', () => choose(agent.id), 'hive-node-select');
    const status = make('p', 'hive-state');
    const model = make('p', 'hive-node-model');
    const task = make('p', 'hive-node-task');
    const action = make('p', 'hive-node-action');
    const times = make('div', 'hive-node-times');
    const updated = make('time');
    const observed = make('time');
    const updatedLine = make('span', '', 'Work updated: ');
    const observedLine = make('span', '', 'Source observed: ');
    updatedLine.append(updated);
    observedLine.append(observed);
    times.append(updatedLine, observedLine);
    record.append(parent, select, status, model, task, action, times);
    const children = make('ul', 'hive-tree');
    li.append(record, children);
    const station = make('div', 'hive-station');
    const ship = button('', () => choose(agent.id), 'hive-ship');
    const shipLabel = make('span', 'hive-ship-label');
    const shipName = make('span', 'hive-ship-name');
    const shipState = make('span', 'hive-ship-state');
    const shipModel = make('span', 'hive-ship-model');
    shipLabel.append(shipName, shipState, shipModel);
    ship.append(glyph(), shipLabel);
    const satellites = make('div', 'hive-satellites');
    station.append(ship, satellites);
    const satellite = button('', () => choose(agent.id), 'hive-satellite');
    satellite.append(glyph(true));
    return { data:agent, li, parent, select, status, model, task, action, updated, observed, children, station, ship, shipName, shipState, shipModel, satellites, satellite, order:order++, absentAt:null };
  }
  function actionLabel(record) {
    return record.absentAt === null && record.data.activity_state === 'running' ? 'Reported current activity' : 'Last observed action';
  }
  function recordState(record) {
    if (record.absentAt !== null) return 'Unknown: no longer in the latest snapshot';
    const age = snapshot ? Date.parse(snapshot.generated_at) - Date.parse(record.data.observed_at) : 0;
    return record.data.status + (age > snapshot?.stale_after_ms ? ` · source observation ${Math.floor(age / 1000)}s behind snapshot` : '');
  }
  function updateRecord(record) {
    const agent = record.data;
    text(record.select, agent.name);
    text(record.parent, agent.parent_id ? `Parent: ${records.get(agent.parent_id)?.data.name || 'not in this observation'}` : 'Root mission');
    text(record.status, recordState(record));
    text(record.model, `${agent.model} · ${agent.provider} · ${agent.runtime} · ${agent.role}`);
    text(record.task, agent.task || 'Current task not reported.');
    text(record.action, `${actionLabel(record)}: ${agent.activity || 'Not reported.'}`);
    setTime(record.updated, agent.updated_at);
    setTime(record.observed, agent.observed_at);
    for (const node of [record.ship, record.satellite]) node.dataset.observationOld = String(snapshot && Date.parse(snapshot.generated_at) - Date.parse(agent.observed_at) > snapshot.stale_after_ms);
    for (const node of [record.li, record.ship, record.satellite]) node.dataset.status = record.absentAt === null ? agent.status : 'unknown';
    text(record.shipName, agent.name);
    text(record.shipModel, `${agent.model} · ${agent.provider}`);
    record.ship.setAttribute('aria-label', `Select ${agent.name}, ${recordState(record)}, ${agent.model}`);
    record.satellite.setAttribute('aria-label', `Select ${agent.name}, ${recordState(record)}, ${agent.model}`);
    record.satellite.title = `${agent.name} · ${agent.model} · ${recordState(record)}`;
  }
  function treeOrder() { return [...records.values()].sort((a, b) => a.order - b.order); }
  function rootFor(record) {
    let root = record;
    const visited = new Set();
    while (hierarchyParents.get(root.data.id) && !visited.has(root.data.id)) {
      visited.add(root.data.id);
      root = records.get(hierarchyParents.get(root.data.id));
    }
    return root;
  }
  function reconcileTree() {
    const ordered = treeOrder();
    const focused = document.activeElement;
    hierarchyParents.clear();
    for (const record of ordered) hierarchyParents.set(record.data.id, records.has(record.data.parent_id) ? record.data.parent_id : null);
    // Old missing observations cannot override the current source's acyclic tree.
    for (const record of ordered) {
      if (record.absentAt === null) continue;
      const visited = new Set([record.data.id]);
      let parent = hierarchyParents.get(record.data.id);
      while (parent) {
        if (visited.has(parent)) { hierarchyParents.set(record.data.id, null); break; }
        visited.add(parent);
        parent = hierarchyParents.get(parent);
      }
    }
    const groups = new Map([[null, []]]);
    for (const record of ordered) groups.set(record.data.id, []);
    for (const record of ordered) {
      const parent = hierarchyParents.get(record.data.id);
      groups.get(parent).push(record.li);
    }
    // Detach changed branches first so a legitimate reparent cannot temporarily
    // insert an ancestor into its old descendant.
    for (const record of ordered) {
      const parent = hierarchyParents.get(record.data.id);
      const destination = parent ? records.get(parent).children : get('tree');
      if (record.li.parentElement && record.li.parentElement !== destination) record.li.remove();
    }
    place(get('tree'), groups.get(null));
    for (const record of ordered) {
      place(record.children, groups.get(record.data.id));
      record.children.hidden = !groups.get(record.data.id).length;
    }
    get('missions').dataset.large = String(records.size > 50);
    applySearch();
    renderAtlas();
    if (focused !== document.activeElement && focused?.isConnected) focused.focus({ preventScroll:true });
  }
  function renderAtlas() {
    const ordered = treeOrder();
    const roots = ordered.filter(record => !hierarchyParents.get(record.data.id));
    const sectors = Math.max(1, Math.ceil(roots.length / 6));
    sector = Math.max(0, Math.min(sector, sectors - 1));
    const visible = new Set(roots.slice(sector * 6, sector * 6 + 6).map(record => record.data.id));
    for (const root of roots) {
      const descendants = ordered.filter(record => record !== root && rootFor(record) === root);
      text(root.shipState, `${recordState(root)} · ${descendants.length} nested`);
      place(root.satellites, descendants.map(record => record.satellite));
      root.satellites.hidden = descendants.length === 0;
      root.station.hidden = !visible.has(root.data.id);
    }
    for (const record of ordered) {
      if (hierarchyParents.get(record.data.id)) record.station.remove();
      else record.satellite.remove();
    }
    place(get('ships'), roots.map(root => root.station));
    get('atlas-empty').hidden = records.size > 0;
    get('sector-prev').disabled = sector === 0;
    get('sector-next').disabled = sector === sectors - 1;
    text(get('sector-label'), roots.length ? `Sector ${sector + 1} of ${sectors} · ${visible.size} of ${roots.length} root missions · complete tree below` : 'The atlas will populate from real source observations.');
  }
  function applySearch() {
    const matches = new Set();
    const visible = new Set();
    for (const record of records.values()) {
      const agent = record.data;
      if (!query || [agent.name, agent.task, agent.activity, agent.model, agent.provider, agent.runtime, agent.role, agent.status].join(' ').toLowerCase().includes(query)) {
        matches.add(agent.id);
        let current = record;
        while (current && !visible.has(current.data.id)) {
          visible.add(current.data.id);
          current = records.get(current.data.parent_id);
        }
      }
    }
    for (const record of records.values()) {
      record.li.hidden = !visible.has(record.data.id);
      record.li.dataset.context = String(!matches.has(record.data.id));
    }
    text(get('search-count'), query ? `${matches.size} matching / ${records.size} displayed agents. Parent branches remain visible for context.` : `${records.size} displayed agents, including any briefly retained missing observations.`);
    get('tree-empty').hidden = matches.size > 0;
    if (snapshot) text(get('tree-empty'), records.size ? 'No missions match this search. Clear it to see the complete observed tree.' : 'No agents reported in this snapshot. An empty fleet is not a connection failure.');
    renderManifest();
  }
  function fields(container, entries, times = false) {
    for (const [index, [label, value]] of entries.entries()) {
      let term = container.children[index * 2];
      let description = container.children[index * 2 + 1];
      if (!term) {
        term = make('dt');
        description = make('dd');
        if (times) description.append(make('time'));
        container.append(term, description);
      }
      text(term, label);
      if (times) setTime(description.firstElementChild, value);
      else text(description, value);
    }
  }
  function choose(agentId, updateFragment = true) {
    if (!records.has(agentId)) return;
    selected = agentId;
    if (updateFragment) {
      try { history.replaceState(null, '', `#agent=${encodeURIComponent(agentId)}`); } catch (_) { /* Sandboxed previews may deny history. */ }
    }
    const ancestors = new Set();
    let parent = records.get(agentId);
    while (parent && !ancestors.has(parent.data.id)) {
      ancestors.add(parent.data.id);
      parent = records.get(parent.data.parent_id);
    }
    for (const record of records.values()) {
      const active = record.data.id === selected;
      record.li.dataset.selected = String(active);
      record.li.dataset.ancestor = String(!active && ancestors.has(record.data.id));
      for (const control of [record.select, record.ship, record.satellite]) control.setAttribute('aria-pressed', String(active));
    }
    renderManifest();
    renderMessages();
  }
  function renderManifest() {
    const record = records.get(selected);
    get('manifest-body').hidden = !record;
    if (!record) return;
    const agent = record.data;
    text(get('manifest-title'), agent.name);
    const parent = records.get(agent.parent_id);
    const context = agent.parent_id ? `Nested under ${parent?.data.name || 'an unobserved parent'}.` : 'Root mission.';
    text(get('manifest-context'), context + (record.li.hidden ? ' Outside the current search filter.' : '') + (record.absentAt !== null ? ' Retained from an earlier observation; disappearance does not mean completion.' : ''));
    text(get('manifest-state'), recordState(record));
    fields(get('manifest-identity'), [['Model', agent.model], ['Provider', agent.provider], ['Runtime', agent.runtime], ['Role', agent.role]]);
    text(get('manifest-task'), agent.task || 'Not reported.');
    text(get('manifest-action-label'), actionLabel(record));
    text(get('manifest-action'), agent.activity || 'Not reported.');
    fields(get('manifest-times'), [['Agent started', agent.started_at], ['Source work updated', agent.updated_at], ['Agent observed by source', agent.observed_at], ['Snapshot generated', snapshot?.generated_at || null], ['Browser received', received]], true);
    const relatives = treeOrder().filter(item => item.data.parent_id === agent.id);
    const family = [
      ...(parent ? [{ id: parent.data.id, label: `Parent: ${parent.data.name}` }] : []),
      ...relatives.map(child => ({ id: child.data.id, label: `Child: ${child.data.name}` })),
    ];
    const wanted = new Set(family.map(member => member.id));
    for (const [id, control] of familyButtons) {
      if (!wanted.has(id)) { control.remove(); familyButtons.delete(id); }
    }
    const controls = family.map(member => {
      let control = familyButtons.get(member.id);
      if (!control) {
        control = button(member.label, () => choose(member.id));
        familyButtons.set(member.id, control);
      }
      text(control, member.label);
      return control;
    });
    place(get('manifest-family'), controls);
  }

  function renderMessages() {
    if (!snapshot) return;
    const list = get('chat');
    const atEnd = list.scrollHeight - list.scrollTop - list.clientHeight < 70;
    const items = snapshot.interactions.filter(item => scope === 'all' || item.agent_id === selected)
      .slice().sort((a, b) => Date.parse(a.at) - Date.parse(b.at) || a.id.localeCompare(b.id));
    const ids = new Set(items.map(item => item.id));
    let added = 0;
    for (const [messageId, entry] of messages) {
      if (!ids.has(messageId)) { entry.li.remove(); messages.delete(messageId); }
    }
    for (const item of items) {
      let entry = messages.get(item.id);
      if (!entry) {
        const li = make('li', 'hive-message');
        const who = make('div', 'hive-message-who');
        const speaker = make('strong');
        const model = make('span', 'hive-message-model');
        const time = make('time');
        const body = make('p', 'hive-message-text');
        who.append(speaker, model, time);
        li.append(who, body);
        entry = { li, speaker, model, time, body };
        messages.set(item.id, entry);
        added++;
      }
      const agent = records.get(item.agent_id)?.data;
      entry.li.dataset.role = item.role;
      text(entry.speaker, item.role === 'drake' ? 'Drake' : agent?.name || 'Agent identity not in the current observation');
      text(entry.model, item.role === 'drake' ? `To: ${agent?.name || 'agent not in the current observation'}` : `${item.model || 'Model unknown'} · ${item.provider || 'Provider unknown'} · ${item.runtime || 'Runtime unknown'}`);
      setTime(entry.time, item.at);
      text(entry.body, item.text);
    }
    place(list, items.map(item => messages.get(item.id).li));
    get('chat-empty').hidden = items.length > 0;
    text(get('chat-empty'), scope === 'selected' ? selected ? 'No published live messages for this agent in the current window.' : 'Choose a mission to read its published conversations.' : 'No published live conversations in this snapshot. There are no sample messages here.');
    text(get('conversation-range'), items.length ? `${items.length} messages · ${utc(items[0].at)} to ${utc(items[items.length - 1].at)} · oldest first` : 'Only real messages published after this live feed started appear here.');
    if (atEnd) { list.scrollTop = list.scrollHeight; newMessages = 0; }
    else newMessages += added;
    get('chat-latest').hidden = !newMessages;
    text(get('chat-latest'), `${newMessages} new messages · go to latest`);
  }

  function freshness() {
    if (!snapshot) {
      page.dataset.fresh = 'false';
      return;
    }
    const elapsed = Math.max(0, performance.now() - advancedAt);
    const age = sourceAgeAtAdvance + elapsed;
    const stale = age > snapshot.stale_after_ms;
    page.dataset.fresh = String(connected && !stale && freshnessTrusted);
    const label = !connected ? (regressed ? 'Snapshot timestamp moved backwards; last observation retained' : 'Connection interrupted; last snapshot retained') : stale ? 'Source stale; last observation retained' : !freshnessTrusted ? 'Connected; source age not verified' : 'Receiving source observations';
    text(get('connection'), label);
    text(get('freshness'), `${Math.floor(elapsed / 1000)}s since the source timestamp last advanced here. ${freshnessTrusted ? `Source age at least ${Math.floor(age / 1000)}s, using the HTTP server clock.` : 'No usable server clock received; browser time is not proof of freshness.'}`);
  }
  function ingest(value, serverDate) {
    const now = performance.now();
    const generated = Date.parse(value.generated_at);
    if (snapshot && generated < Date.parse(snapshot.generated_at)) { regressed = true; throw new Error('Snapshot moved backwards'); }
    if (generated > latestGenerated) {
      regressed = false;
      latestGenerated = generated;
      advancedAt = now;
      const server = Date.parse(serverDate || '');
      freshnessTrusted = Number.isFinite(server) && generated <= server + 2000;
      sourceAgeAtAdvance = freshnessTrusted ? Math.max(0, server - generated) : 0;
    }
    const previouslyObserved = snapshot !== null;
    snapshot = value;
    received = new Date().toISOString();
    connected = true;
    freshness();
    const currentIds = new Set(value.agents.map(agent => agent.id));
    const messageAgents = new Set(value.interactions.map(item => item.agent_id));
    for (const agent of value.agents) {
      let record = records.get(agent.id);
      if (!record) {
        record = createRecord(agent);
        records.set(agent.id, record);
        if (previouslyObserved) { pulse(record.li); pulse(record.ship); pulse(record.satellite); }
      } else if (record.data.task !== agent.task || record.data.activity !== agent.activity || record.data.status !== agent.status || record.data.activity_state !== agent.activity_state) {
        pulse(record.li);
        pulse(record.ship);
        pulse(record.satellite);
      }
      record.data = agent;
      record.absentAt = null;
    }
    for (const [agentId, record] of records) {
      if (!currentIds.has(agentId)) {
        if (record.absentAt === null) record.absentAt = now;
        const hasPresentChildren = value.agents.some(agent => agent.parent_id === agentId);
        if (now - record.absentAt > 30000 && selected !== agentId && !hasPresentChildren && !messageAgents.has(agentId)) {
          // Reattach any retained descendants before removing their old ancestor.
          for (const child of records.values()) if (child.data.parent_id === agentId) get('tree').append(child.li);
          record.li.remove(); record.station.remove(); record.satellite.remove(); records.delete(agentId);
        }
      }
    }
    for (const record of records.values()) updateRecord(record);
    setTime(get('source-time'), value.generated_at);
    setTime(get('fetch-time'), received);
    text(get('counts'), `${value.coverage.agents} agents observed across ${value.coverage.roots} root missions.`);
    get('coverage').dataset.complete = String(value.coverage.complete);
    text(get('coverage'), `${value.coverage.complete ? 'Reported coverage' : 'Partial coverage'}: ${value.coverage.label}. ${value.coverage.notice}`);
    reconcileTree();
    if (!selected) {
      let fragment = '';
      try { fragment = location.hash.startsWith('#agent=') ? decodeURIComponent(location.hash.slice(7)) : ''; } catch (_) { /* Invalid fragments do not select a node. */ }
      selected = records.has(fragment) ? fragment : treeOrder()[0]?.data.id || null;
    }
    if (selected) choose(selected, false);
    else renderMessages();
    if (!records.size) {
      const empty = get('atlas-empty');
      text(empty.children[1], 'A quiet sky.');
      text(empty.children[2], 'The source reports no agents right now. This is an observation, not a simulation.');
    }
  }
  async function poll() {
    if (requestActive) return;
    requestActive = true;
    const began = performance.now();
    const controller = new AbortController();
    const timeout = setTimeout(() => controller.abort(), 4500);
    try {
      const response = await fetch('/hive/live.json', { cache:'no-store', credentials:'omit', signal:controller.signal, headers:{ Accept:'application/json' } });
      if (!response.ok || !response.headers.get('content-type')?.toLowerCase().includes('application/json')) throw new Error('Public source unavailable');
      ingest(validate(await response.json()), response.headers.get('date'));
    } catch (_) {
      connected = false;
      if (!snapshot) {
        text(get('connection'), 'Telemetry unavailable');
        text(get('atlas-empty').children[1], 'No signal yet.');
        text(get('atlas-empty').children[2], 'The source is unavailable or its snapshot could not be validated. We will keep listening.');
      }
      freshness();
    } finally {
      clearTimeout(timeout);
      requestActive = false;
      setTimeout(poll, Math.max(0, 1000 - (performance.now() - began)));
    }
  }
  function motion() {
    const off = paused || reducedMotion.matches;
    page.dataset.motion = off ? 'off' : 'on';
    page.dataset.hidden = String(document.hidden);
    get('motion').setAttribute('aria-pressed', String(off));
    text(get('motion'), reducedMotion.matches ? 'Motion off: device preference' : off ? 'Resume motion' : 'Pause motion');
    get('motion').disabled = reducedMotion.matches;
  }
  get('motion').addEventListener('click', () => {
    paused = !paused;
    try { localStorage.setItem('hive-motion-paused', String(paused)); } catch (_) { /* Keep the in-memory preference. */ }
    motion();
  });
  reducedMotion.addEventListener('change', motion);
  document.addEventListener('visibilitychange', () => { motion(); freshness(); });
  get('search').addEventListener('input', event => { query = event.target.value.trim().toLowerCase(); applySearch(); });
  get('sector-prev').addEventListener('click', () => { sector--; renderAtlas(); });
  get('sector-next').addEventListener('click', () => { sector++; renderAtlas(); });
  get('show-selected').addEventListener('click', () => {
    const record = records.get(selected);
    if (!record) return;
    if (record.li.hidden) { query = ''; get('search').value = ''; applySearch(); }
    record.select.focus({ preventScroll:true });
    record.li.scrollIntoView({ block:'center', behavior:'auto' });
  });
  for (const name of ['all', 'selected']) get(`chat-${name}`).addEventListener('click', () => {
    scope = name;
    get('chat-all').setAttribute('aria-pressed', String(scope === 'all'));
    get('chat-selected').setAttribute('aria-pressed', String(scope === 'selected'));
    newMessages = 0;
    renderMessages();
  });
  get('chat-latest').addEventListener('click', () => {
    get('chat').scrollTop = get('chat').scrollHeight;
    newMessages = 0;
    get('chat-latest').hidden = true;
  });
  get('chat').addEventListener('scroll', () => {
    const chat = get('chat');
    if (chat.scrollHeight - chat.scrollTop - chat.clientHeight < 70) { newMessages = 0; get('chat-latest').hidden = true; }
  }, { passive:true });
  window.addEventListener('hashchange', () => {
    if (!location.hash.startsWith('#agent=')) return;
    try { choose(decodeURIComponent(location.hash.slice(7)), false); } catch (_) { /* Ignore malformed fragments. */ }
  });
  if ('IntersectionObserver' in window) new IntersectionObserver(entries => {
    page.dataset.offscreen = String(!entries[0].isIntersecting);
  }).observe(get('atlas'));
  motion();
  setInterval(freshness, 1000);
  poll();
})();

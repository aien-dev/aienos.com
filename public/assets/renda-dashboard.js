(() => {
  const panel = document.querySelector("[data-renda-feed]");
  if (!panel) return;

  const number = (value, digits = 1) => {
    if (typeof value !== "number" || !Number.isFinite(value)) return null;
    return new Intl.NumberFormat(undefined, { maximumFractionDigits: digits }).format(value);
  };
  const stamp = value => {
    if (typeof value !== "string" || !Number.isFinite(Date.parse(value))) return "unknown";
    return new Date(value).toISOString().replace("T", " ").replace(/\.\d{3}Z$/, " UTC").replace(/Z$/, " UTC");
  };
  const text = (root, field, value) => {
    const target = root.querySelector(`[data-field="${field}"]`);
    if (target) target.textContent = value;
  };
  const ageSeconds = value => {
    if (typeof value !== "string" || !Number.isFinite(Date.parse(value))) return null;
    const age = (Date.now() - Date.parse(value)) / 1000;
    return age >= 0 ? age : null;
  };
  const showCount = (root, field, value, suffix = "") => {
    const formatted = number(value, 0);
    text(root, field, formatted === null ? "Unknown" : `${formatted}${suffix}`);
  };

  function paintWorker(card, worker) {
    const lastAge = ageSeconds(worker.last_pass_at);
    const isStale = lastAge !== null && lastAge > 300;
    const passes = number(worker.window_passes, 0);
    const current = number(worker.current_cpm, 2);
    let state = "Rate unknown";
    let display = "Unknown";
    if (isStale) {
      state = "Stale";
      display = "Stale";
    } else if (lastAge !== null && current !== null) {
      state = "Current snapshot";
      display = current;
    } else if (lastAge === null && passes === "0") {
      state = "Idle in this window";
      display = "Idle";
    }
    text(card, "state", state);
    text(card, "current", display);
    const max = number(worker.max_cpm, 2);
    text(card, "max", max === null ? "Unknown" : `${max} cycles / min`);
    const min = number(worker.min_cpm, 2);
    text(card, "range", min === null || max === null ? "Unknown" : `${min} to ${max} cycles / min`);
    const passSeconds = number(worker.last_pass_seconds, 1);
    text(card, "pass", passSeconds === null ? "Unknown" : `${passSeconds} sec`);
    text(card, "last", worker.last_pass_at ? `Last completed cycle ${stamp(worker.last_pass_at)}${lastAge === null ? "" : ` · ${Math.floor(lastAge)} sec ago`}` : "Last completed cycle unknown");

    const arc = card.querySelector('[data-field="arc"]');
    const needle = card.querySelector('[data-field="needle"]');
    const fraction = !isStale && lastAge !== null && Number.isFinite(worker.current_cpm) && Number.isFinite(worker.max_cpm) && worker.max_cpm > 0 ? Math.max(0, Math.min(1, worker.current_cpm / worker.max_cpm)) : 0;
    if (arc) arc.style.strokeDasharray = `${fraction * 302} 302`;
    if (needle) needle.style.transform = `rotate(${fraction * 180 - 90}deg)`;
    card.classList.toggle("renda-stale", isStale);
    card.classList.toggle("renda-idle", state === "Idle in this window");
  }

  async function refresh() {
    try {
      const response = await fetch(panel.dataset.rendaFeed, { cache: "no-store", credentials: "same-origin" });
      if (!response.ok) throw new Error(`HTTP ${response.status}`);
      const data = await response.json();
      if (data.schema !== "renda-metrics/1" || data.window_seconds !== 3600 || !Array.isArray(data.workers) || data.workers.length !== 2 || new Set(data.workers.map(w => w?.name)).size !== 2 || !data.workers.every(w => w && ["renda1", "renda2"].includes(w.name) && ["merges", "rebases", "reruns", "operators", "window_passes", "active_pass_seconds"].every(k => Number.isSafeInteger(w[k]) && w[k] >= 0))) throw new Error("Unexpected telemetry schema");
      text(panel, "feed-state", "Snapshot received");
      const generatedAge = ageSeconds(data.generated_at);
      text(panel, "generated", `Snapshot generated ${stamp(data.generated_at)}${generatedAge === null ? "" : ` · ${Math.floor(generatedAge / 60)} min old`}`);
      const workers = new Map(data.workers.map(worker => [worker.name, worker]));
      panel.querySelectorAll("[data-worker]").forEach(card => {
        const worker = workers.get(card.dataset.worker);
        if (worker) paintWorker(card, worker);
        else text(card, "state", "Unknown worker");
      });
      const knownWorkers = data.workers.filter(worker => worker && typeof worker === "object");
      for (const field of ["merges", "rebases", "reruns", "operators", "window_passes", "active_pass_seconds"]) {
        const values = knownWorkers.map(worker => worker[field]).filter(value => typeof value === "number" && Number.isFinite(value));
        showCount(panel, field, values.length ? values.reduce((total, value) => total + value, 0) : null, field === "active_pass_seconds" ? " sec" : "");
      }
    } catch (error) {
      text(panel, "feed-state", "Telemetry unavailable");
      panel.querySelectorAll("[data-worker]").forEach(card => {
        paintWorker(card, {});
        text(card, "state", "Unknown · snapshot unavailable");
      });
      for (const field of ["merges", "rebases", "reruns", "operators", "window_passes", "active_pass_seconds"]) showCount(panel, field, null);
      console.warn("Renda telemetry could not be read", error);
    }
  }

  refresh();
  window.setInterval(refresh, 60_000);
})();

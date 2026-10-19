/* allen.js: puts Allen's window in the page. The picture is drawn by allen.wasm, a port of the AIENOS
 * kernel's own integer drawing code (crates/allen-sim, checked frame by frame against the C original).
 * This file only carries things to and from it: the canvas, the keyboard, the sound, the clock.
 * First party only: no outside requests, nothing leaves the page. */
(function () {
  "use strict";

  var B = window.AllenBrain;
  var FORMED_MS = 15300;          // Allen has formed (from the kernel's own timeline)
  var WAKE_AT_MS = 13000;         // the bar may come up once he has formed
  var MERGE_MS = 12000;
  var PREROLL_MS = 3008;          // where the very first look begins (the sky is already fading in)
  var wasmPromise = null;

  function loadWasm() {
    if (!wasmPromise) {
      wasmPromise = fetch("/allen/allen.wasm").then(function (r) { return r.arrayBuffer(); })
        .then(function (b) { return WebAssembly.instantiate(b, {}); })
        .then(function (r) { return r.instance.exports; });
    }
    return wasmPromise;
  }

  function mount(root) {
    var canvas = root.querySelector("[data-allen-canvas]");
    var input = root.querySelector("[data-allen-input]");
    var statusEl = root.querySelector("[data-allen-status]");
    var soundBtn = root.querySelector("[data-allen-sound]");
    var skipBtn = root.querySelector("[data-allen-skip]");
    var replayBtn = root.querySelector("[data-allen-replay]");
    var popBtn = root.querySelector("[data-allen-pop]");
    var wakeBtn = root.querySelector("[data-allen-wake]");
    var stage = root.querySelector("[data-allen-stage]") || root;
    if (!canvas || !B) return;
    var ctx = canvas.getContext("2d", { alpha: false });
    var reduced = window.matchMedia && window.matchMedia("(prefers-reduced-motion: reduce)").matches;
    var enc = new TextEncoder();

    var x = null, w = 0, h = 0, image = null, imageBuf = null;
    var clock = 0, last = 0, running = false, visible = true, target = -1;
    var woke = false, greeted = false, mind = null, started = false;
    var touched = false, demoStep = -1, demoAt = 0, sleeping = false;
    var audio = null, soundOn = false, popScheduled = false;
    var chirp = null;

    function say(text) { if (statusEl) statusEl.textContent = text; }

    function newMind() {
      mind = B.createMind(B.pageFacts(navigator.hardwareConcurrency || 0, screen.width, screen.height));
      mind.lasting = false;
    }

    function size() {
      var narrow = root.clientWidth < 700;
      w = narrow ? 800 : 1280;
      h = narrow ? 450 : 720;
      canvas.width = w;
      canvas.height = h;
    }

    function put(bytes) {
      var m = new Uint8Array(x.memory.buffer);
      m.set(bytes, x.allen_scratch());
      return bytes.length;
    }
    function putText(s) { return put(enc.encode(s).subarray(0, 1000)); }

    function open() {
      size();
      if (x.allen_new(w, h) !== 0) { say("Allen could not open a window of this size."); return false; }
      image = null;
      newMind();
      clock = 0; last = 0; woke = false; greeted = false; target = -1; demoStep = -1; sleeping = false;
      popScheduled = false;
      started = true;
      return true;
    }

    function paint() {
      var buf = x.memory.buffer;
      if (!image || imageBuf !== buf) {
        image = new ImageData(new Uint8ClampedArray(buf, x.allen_screen(), w * h * 4), w, h);
        imageBuf = buf;
      }
      ctx.putImageData(image, 0, 0);
    }

    /* ---- the pop (the chirp): drawn from the kernel's own formula, played in time with the merger ---- */
    function ensureChirp() {
      if (chirp) return chirp;
      var from = x.allen_chirp_start(), total = x.allen_chirp_frames(), n = total - from;
      var data = new Float32Array(n), off = 0;
      while (off < n) {
        var k = Math.min(4096, n - off);
        var ptr = x.allen_chirp(from + off, k);
        var s = new Int16Array(x.memory.buffer, ptr, k);
        for (var i = 0; i < k; i++) data[off + i] = s[i] / 32768;
        off += k;
      }
      chirp = { data: data, from: from };
      return chirp;
    }
    function playChirp(delaySec) {
      if (!audio) return;
      var c = ensureChirp();
      var buf = audio.createBuffer(1, c.data.length, 48000);
      buf.copyToChannel(c.data, 0);
      var src = audio.createBufferSource();
      src.buffer = buf;
      src.connect(audio.destination);
      src.start(audio.currentTime + Math.max(0, delaySec));
    }
    function schedulePop() {
      if (!soundOn || !audio || popScheduled || !started) return;
      var c = ensureChirp();
      var startMs = c.from / 48;               // where the sound begins on the sim's clock
      if (clock <= startMs) {
        popScheduled = true;
        playChirp((startMs - clock) / 1000);
      }
    }
    function setSound(on) {
      soundOn = on;
      if (soundBtn) {
        soundBtn.setAttribute("aria-pressed", on ? "true" : "false");
        soundBtn.textContent = on ? "Sound on" : "Sound off";
      }
      if (on) {
        if (!audio) {
          var AC = window.AudioContext || window.webkitAudioContext;
          if (AC) audio = new AC();
        }
        if (audio && audio.state === "suspended") audio.resume();
        schedulePop();
      }
    }

    /* ---- talking ---- */
    function showMap() {
      var live = [], i, j;
      for (i = 0; i < mind.notes.length; i++) if (!mind.notes[i].gone) live.push(i);
      live = live.slice(-48);
      var ea = [], eb = [], links = 0;
      for (i = 0; i < live.length; i++)
        for (j = i + 1; j < live.length; j++)
          if (B.noteLinks(mind.notes[live[i]], mind.notes[live[j]]) > 0 && ea.length < 160) { ea.push(i); eb.push(j); links++; }
      var lit = -1, label = "";
      if (mind.lit > 0) {
        lit = live.indexOf(mind.lit - 1);
        if (mind.notes[mind.lit - 1]) label = mind.notes[mind.lit - 1].text;
      }
      var lb = enc.encode(label).subarray(0, 200);
      var bytes = new Uint8Array(ea.length * 2 + lb.length);
      bytes.set(ea, 0); bytes.set(eb, ea.length); bytes.set(lb, ea.length * 2);
      put(bytes);
      x.allen_map(live.length, ea.length, lit, lb.length, B.notesLive(mind.notes), links);
    }

    function submit(raw) {
      var text = B.clean(raw);
      if (!text.replace(/ /g, "")) return;
      if (sleeping) { wake(); return; }
      var reply = B.answer(mind, text, Math.floor(clock / 1000));
      var lb = enc.encode(text).subarray(0, 150), rb = enc.encode(reply).subarray(0, 150);
      var bytes = new Uint8Array(lb.length + rb.length);
      bytes.set(lb, 0); bytes.set(rb, lb.length);
      put(bytes);
      x.allen_said(0, lb.length, rb.length);
      say("Allen: " + reply);
      if (mind.mapChanged) { mind.mapChanged = false; showMap(); }
      if (reply === B.GOODBYE) setTimeout(function () { sleep(); }, 2600 + reply.length * 30);
    }

    function greet() {
      var rb = enc.encode("Hello. I'm Allen.");
      put(rb);
      x.allen_said(1, 0, rb.length);
      say("Allen: Hello. I'm Allen.");
    }

    function sleep() {
      sleeping = true;
      x.allen_sleep(1);
      say("Allen is asleep.");
      if (wakeBtn) wakeBtn.hidden = false;
    }
    function wake() {
      sleeping = false;
      x.allen_sleep(0);
      say("Allen is awake.");
      if (wakeBtn) wakeBtn.hidden = true;
    }

    /* ---- a gentle demonstration for a visitor who only watches ---- */
    var DEMO = [["Hello Allen", 0], ["what can you do", 9000]];
    function demo() {
      if (touched || sleeping || !woke || clock < FORMED_MS + 6000) return;
      if (demoStep >= DEMO.length) return;
      if (demoStep < 0) { demoStep = 0; demoAt = clock; }
      var line = DEMO[demoStep][0];
      var t = clock - demoAt - DEMO[demoStep][1];
      if (t < 0) return;
      var n = Math.min(line.length, Math.floor(t / 120));
      x.allen_input(putText(line.slice(0, n)));
      if (n === line.length && t > line.length * 120 + 500) {
        x.allen_input(putText(""));
        submit(line);
        demoStep++;
        demoAt = clock;
      }
    }

    /* ---- the clock ---- */
    function frame(now) {
      if (!running) return;
      requestAnimationFrame(frame);
      var dt = now - last;
      last = now;
      if (!visible) return;
      if (dt > 100) dt = 100;
      var drew = 0;
      if (target >= 0) {
        // fast forward: many steps of the same clock, one picture
        var steps = 40;
        while (steps-- > 0 && clock < target) { clock += 16; drew |= x.allen_tick(clock); }
        if (clock >= target) target = -1;
      } else {
        clock += dt;
        drew = x.allen_tick(Math.floor(clock));
      }
      if (!woke && clock >= WAKE_AT_MS) {
        woke = true;
        x.allen_wake(putText("Press Enter to send. Everything runs in this page."));
      }
      if (woke && !greeted && clock >= FORMED_MS + 400) { greeted = true; greet(); }
      if (soundOn) schedulePop();
      demo();
      if (drew) paint();
      if (clock > MERGE_MS && clock < MERGE_MS + 200) say("Two black holes merge. Allen forms from the light.");
    }

    function run() {
      if (running) return;
      running = true;
      last = performance.now();
      requestAnimationFrame(frame);
    }
    function stop() { running = false; }

    function restart() {
      if (!open()) return;
      root.classList.add("allen-live");
      if (replayBtn) replayBtn.textContent = "Replay";
      if (popBtn) popBtn.hidden = false;
      say("The night sky. Two black holes are circling.");
      x.allen_tick(0);
      paint();
      run();
    }

    /* ---- wiring ---- */
    function focusInput() { touched = true; if (input) input.focus({ preventScroll: true }); }
    canvas.addEventListener("click", focusInput);
    if (input) {
      input.addEventListener("input", function () {
        touched = true;
        if (!woke) return;
        x.allen_input(putText(input.value));
      });
      input.addEventListener("keydown", function (e) {
        touched = true;
        if (e.key === "Enter") {
          e.preventDefault();
          var v = input.value;
          input.value = "";
          x.allen_input(putText(""));
          if (woke) submit(v);
        }
      });
    }
    if (soundBtn) soundBtn.addEventListener("click", function () { setSound(!soundOn); });
    if (popBtn) popBtn.addEventListener("click", function () {
      if (!audio) { var AC = window.AudioContext || window.webkitAudioContext; if (AC) audio = new AC(); }
      if (audio && audio.state === "suspended") audio.resume();
      playChirp(0);
    });
    if (skipBtn) skipBtn.addEventListener("click", function () { if (clock < FORMED_MS + 800) target = FORMED_MS + 800; });
    if (replayBtn) replayBtn.addEventListener("click", function () { restart(); });
    if (wakeBtn) wakeBtn.addEventListener("click", function () { wake(); focusInput(); });
    if ("IntersectionObserver" in window) {
      new IntersectionObserver(function (es) { visible = es[0].isIntersecting; }, { threshold: 0.05 }).observe(canvas);
    }
    document.addEventListener("visibilitychange", function () { if (!document.hidden) last = performance.now(); });
    window.addEventListener("pagehide", stop);

    stage.setAttribute("data-allen-state", "loading");
    loadWasm().then(function (exports) {
      x = exports;
      if (!open()) return;
      stage.setAttribute("data-allen-state", "ready");
      if (!reduced) root.classList.add("allen-live");
      if (reduced) {
        // no motion by itself: the recorded still stays until the visitor presses Play
        stage.setAttribute("data-allen-state", "paused");
        root.classList.remove("allen-live");
        say("Motion is reduced on this device. Press Play to start Allen's window.");
        if (replayBtn) replayBtn.textContent = "Play";
      } else {
        say("The night sky. Two black holes are circling.");
        x.allen_tick(0);
        // the sky fades in from black over the first seconds; the first look starts a little way in
        for (var pre = 16; pre <= PREROLL_MS; pre += 16) x.allen_tick(pre);
        clock = PREROLL_MS;
        paint();
        run();
      }
    }).catch(function () {
      stage.setAttribute("data-allen-state", "failed");
      say("The live window could not start in this browser. The recorded video below shows the same screen.");
    });
  }

  function init() {
    var nodes = document.querySelectorAll("[data-allen]");
    for (var i = 0; i < nodes.length; i++) mount(nodes[i]);
  }
  if (document.readyState === "loading") document.addEventListener("DOMContentLoaded", init);
  else init();
})();

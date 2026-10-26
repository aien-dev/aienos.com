/* Generated from om/allen-scene/scene.om by pinned seeded Omega.
 * Controller: Omega-generated JavaScript. Rendering/audio: unchanged native C,
 * aien-dev/aienux@da55660ea7337f2b6a42095fcd1fb6ddebe1cc7c, compiled to WASM.
 * This source pin was staged, not hardware-confirmed by the available report.
 * Web opening reuses native dusk; enter('birth') is the original binary merger.
 * Catalogue/licences and native dependency boundary: om/allen-scene/provenance.txt.
 * WolframLanguageEvaluator: 1000/60 -> 50/3 ms; (1000/60)/2 -> 25/3 ms;
 * 48000 (12300-11360)/1000 -> 45120 frames (native chirp non-silent window).
 */
(() => {
  'use strict';
  const PIN = 'da55660ea7337f2b6a42095fcd1fb6ddebe1cc7c';
  const mounted = new WeakMap();
  let compiled;
  function module() {
    if (!compiled) compiled = fetch('/allen/scene.wasm', {credentials:'omit'})
      .then(response => { if (!response.ok) throw new Error('Native scene unavailable'); return response.arrayBuffer(); })
      .then(bytes => WebAssembly.compile(bytes)).catch(error => { compiled = null; throw error; });
    return compiled;
  }
  function mount(canvas, options = {}) {
    if (!(canvas instanceof HTMLCanvasElement)) throw new TypeError('AllenScene requires a canvas');
    if (mounted.has(canvas)) return mounted.get(canvas);
    const compact = Boolean(options.compact), interactive = Boolean(options.interactive) && !compact;
    const opening = options.opening === undefined ? (compact ? 'idle' : 'dusk') : options.opening;
    if(opening !== 'idle' && opening !== 'dusk' && opening !== 'birth') throw new RangeError('Invalid Allen scene opening');
    const ctx = canvas.getContext('2d', {alpha:false});
    if (!ctx) throw new Error('Canvas rendering unavailable');
    const motion = matchMedia('(prefers-reduced-motion: reduce)');
    const cleanup = [], letters = Array.from({length:64}, () => ({char:0,start:0,slot:0}));
    const wordTimes = new Float64Array(256);
    let ex, image, width = 0, height = 0, disposed = false, failed = false;
    let raf = 0, lastWall = 0, clock = 20000, kind = 0, modeValue = 0;
    let visible = true, reduced = motion.matches, dirty = true, resizePending = true;
    const connection = typeof navigator === 'object' ? navigator.connection : null;
    let calm = options.calm === undefined ? Boolean(connection && connection.saveData) : Boolean(options.calm);
    let quality = 0, overBudget = 0, underBudget = 0, qualityHold = 0, samples = 0;
    const renderScales = [1, 3/4, 9/16, 9/16];
    const qualityNames = ['full-60','balanced-60','small-60','small-30'];
    const frameBudget = 1000/60;
    // Wolfram: 1000/60 -> 50/3 ms; budget/2 -> 25/3; 1280*{1,3/4,9/16} -> {1280,960,720}.
    function still() { return reduced || compact || calm; }
    function qualityLabel() { canvas.dataset.sceneQuality = reduced ? 'reduced' : compact ? 'compact' : calm ? 'calm' : qualityNames[quality]; }
    function measureDraw(cost) {
      samples++;
      if(samples % 60 === 0) canvas.dataset.sceneDrawMs=String(cost);
      if(qualityHold) { qualityHold--; return; }
      overBudget=cost>frameBudget?overBudget+1:0;
      underBudget=cost<frameBudget/2?underBudget+1:0;
      const next=overBudget>=8?Math.min(3,quality+1):underBudget>=180?Math.max(0,quality-1):quality;
      if(next!==quality) {
        quality=next;qualityHold=60;overBudget=underBudget=0;
        resizePending=dirty=true;canvas.dataset.sceneDrawMs=String(cost);qualityLabel();
      }
    }
    let letterHead = 0, wordRead = 0, wordWrite = 0, lastWord = -1000, speakUntil = 0, keyUntil = 0;
    let ra = 113697000, dec = -72757000, fov = 80000, viewPending = false;
    let dragId = null, dragX = 0, dragY = 0, pinchDistance = 0;
    const touches = [{id:null,x:0,y:0},{id:null,x:0,y:0}];
    let ownsAudio = false, completeSent = false, resolveSize, rejectSize;
    let pendingEnter = compact || opening === 'idle' ? 0 : opening === 'birth' ? 2 : 1, pendingStatic = false;
    canvas.dataset.sceneSource = PIN;
    canvas.dataset.sceneEngine = 'loading';
    qualityLabel();
    function listen(target, name, fn, opts) {
      target.addEventListener(name, fn, opts);
      cleanup.push(() => target.removeEventListener(name, fn, opts));
    }
    function stopAudio() {
      if (ownsAudio && window.AllenAudio) window.AllenAudio.stop();
      ownsAudio=false;
    }
    function awake() { return !disposed && !failed && !document.hidden && visible; }
    function request() {
      if (!raf && ex && awake() && (!still() || dirty)) raf = requestAnimationFrame(tick);
    }
    function size() {
      const box = canvas.getBoundingClientRect();
      if (box.width <= 0 || box.height <= 0) return false;
      // Policy bound, not a physical-resolution claim. Same aspect ratio, native design scaling.
      const scale = Math.min(devicePixelRatio || 1, 1280 / Math.max(box.width,box.height))*renderScales[quality];
      const w = Math.max(64,Math.round(box.width*scale)), h = Math.max(64,Math.round(box.height*scale));
      resizePending = false;
      if (w === width && h === height) return true;
      width=w; height=h; canvas.width=w; canvas.height=h;
      if (!ex.scene_init(w,h,compact?1:0)) throw new Error('Native scene size refused');
      image = new ImageData(new Uint8ClampedArray(ex.memory.buffer,ex.scene_pixels(),w*h*4),w,h);
      ex.scene_enter(kind);
      // Resize keeps phase and mode but does not manufacture the old frame history.
      ex.scene_step(Math.floor(clock),modeValue,0);
      viewPending=true;
      if(resolveSize) {resolveSize();resolveSize=null;rejectSize=null;}
      return true;
    }
    function reset(kindValue) {
      kind=kindValue; clock=kind===0?20000:0; lastWall=0;completeSent=false;
      wordRead=wordWrite=0; lastWord=-1000; speakUntil=keyUntil=0; modeValue=0;
      for (const letter of letters) letter.char=0;
      stopAudio();
      if (ex && width) ex.scene_enter(kind);
      dirty=true; request();
    }
    function render() {
      if (!ex || !awake()) return;
      if (resizePending && !size()) return;
      if (!image) return;
      if (viewPending && !(kind===1 && clock<4200)) {
        if (!ex.scene_view(Math.round(ra),Math.round(dec),Math.round(fov))) throw new Error('Native sky view refused');
        viewPending=false;
      }
      const time=Math.floor(clock);
      while(wordRead!==wordWrite && wordTimes[wordRead]<=clock) {
        lastWord=wordTimes[wordRead];ex.scene_word(Math.floor(lastWord));wordRead=(wordRead+1)%wordTimes.length;
      }
      let currentMode=modeValue;
      if(clock<speakUntil) currentMode=3;
      else if(currentMode===3) currentMode=0;
      if(currentMode===0 && clock<keyUntil) currentMode=1;
      // Native allen_ui.c voice envelope (per-word exp decay, 200 ms).
      const voice=clock<speakUntil && lastWord>=0?ex.scene_voice(time,Math.floor(lastWord)):0;
      const shimmer=clock<speakUntil?256:0;
      ex.scene_step(time,currentMode,reduced?0:voice);
      ex.scene_draw(reduced?0:shimmer,reduced?0:voice);
      if(!reduced && !compact) for(const letter of letters) {
        if(!letter.char || clock<letter.start) continue;
        const u=Math.min(1,(clock-letter.start)/900);
        if(u>=1) {letter.char=0;continue;}
        // Web enhancement: native glyphs lift from input to Allen. No HTML/text injection.
        const ease=1-(1-u)*(1-u)*(1-u);
        const x=width/2+(letter.slot-4)*width/90*(1-ease);
        const y=height*(0.86+(0.4-0.86)*ease);
        ex.scene_glyph(letter.char,Math.round(x),Math.round(y),Math.round(256*(1-u)));
      }
      ctx.putImageData(image,0,0);
      canvas.dataset.scenePhase=kind===1 && clock<4200?'tube':kind===2 && clock<12300?'merger':clock<(kind===1?7200:15300)?'forming':'presence';
      canvas.dataset.sceneTime=String(time);
      canvas.dataset.sceneEngine='native-wasm';
      dirty=false;
      if(!completeSent && (kind===0 || clock>=(kind===1?7200:15300))) {
        completeSent=true;
        canvas.dispatchEvent(new CustomEvent('allen-scene-complete',{bubbles:true,detail:{kind:kind===2?'birth':kind===1?'dusk':'idle',time}}));
      }
      return true;
    }
    function error(err) {
      failed=true;cancelAnimationFrame(raf);raf=0;stopAudio();
      if(rejectSize) {rejectSize(err);resolveSize=null;rejectSize=null;}
      canvas.dataset.sceneEngine='unavailable';
      canvas.dispatchEvent(new CustomEvent('allen-scene-error',{bubbles:true,detail:{message:String(err.message||err)}}));
    }
    function tick(wall) {
      raf=0;
      if(!awake()) {lastWall=0;return;}
      const dt=lastWall?wall-lastWall:0;
      // Lowest level skips alternate 60 Hz callbacks; 25 ms tolerates timestamp jitter around 50/3 and 100/3 ms.
      if(!still() && quality===3 && dt<25 && !dirty) {request();return;}
      lastWall=wall;
      if(!still()) clock+=dt; // source clock advances; native integrators bound their own catch-up
      const sample=!still() && !resizePending && !viewPending;
      const began=sample?performance.now():0;
      let drawn=false;
      try { drawn=render(); } catch(err) { error(err); }
      if(sample && drawn) measureDraw(performance.now()-began);
      if(drawn && !still()) request();
    }
    function motionChanged() {
      reduced=motion.matches;lastWall=0;cancelAnimationFrame(raf);raf=0;stopAudio();
      if(still()) { reset(0); pendingStatic=true; }
      overBudget=underBudget=0;qualityLabel();
      dirty=true;request();
    }
    function visibility() {
      lastWall=0;
      if(!awake()) {cancelAnimationFrame(raf);raf=0;if(ownsAudio && window.AllenAudio) window.AllenAudio.pause();}
      else {if(ownsAudio && !still() && window.AllenAudio) void window.AllenAudio.resume();overBudget=underBudget=0;dirty=true;request();}
    }
    function changeView(dx,dy,zoom) {
      if(!interactive || kind===1 && clock<4200) return;
      ra=((ra+dx)%360000000+360000000)%360000000;
      dec=Math.max(-89000000,Math.min(89000000,dec+dy));
      fov=Math.max(10000,Math.min(120000,fov*zoom));
      viewPending=dirty=true;request();
    }
    function home() {ra=113697000;dec=-72757000;fov=80000;viewPending=dirty=true;request();}
    const api = {
      key(text) {
        if(disposed) return;
        text=String(text).slice(-64);keyUntil=clock+4000;
        if(ex) ex.scene_key(Math.floor(clock));
        if(!reduced && !compact) for(let i=0;i<text.length;i++) {
          const c=text.charCodeAt(i);if(c<32 || c>126) continue;
          const letter=letters[letterHead];letterHead=(letterHead+1)%letters.length;
          letter.char=c;letter.start=clock+i*35;letter.slot=i%9;
        }
        dirty=true;request();
      },
      speak(text) {
        if(disposed) return;
        text=String(text).slice(0,4096);
        let offset=0;const start=Math.max(clock,speakUntil-300);
        for(let i=0;i<text.length;i++) {
          if(!/\s/.test(text[i]) && (i===0 || /\s/.test(text[i-1]))) {
            const next=(wordWrite+1)%wordTimes.length;
            if(next===wordRead) break;
            wordTimes[wordWrite]=start+offset;wordWrite=next;
          }
          offset+=35;
        }
        speakUntil=Math.min(clock+30000,start+offset+300);dirty=true;request();
      },
      mode(name) {
        const modes={idle:0,listening:1,listen:1,thinking:2,think:2,speaking:3,speak:3};
        if(Object.prototype.hasOwnProperty.call(modes,name)) {modeValue=modes[name];dirty=true;request();}
      },
      enter(name='dusk') {
        if(disposed) return;
        const next=still() || name==='idle'?0:(name==='birth' || name==='boot' || name==='night')?2:1;
        if(next===2) ownsAudio=false; // caller starts audio in this same gesture before entering
        pendingEnter=next;reset(next);
        if(next===2 && window.AllenAudio) {
          ownsAudio=true;
          // AudioPort is invoked by the caller's gesture; this instance only owns its pause/stop lifecycle.
        }
        if(ex && width) pendingEnter=null;
        request();
      },
      home,
      zoom(direction) {
        if(direction!=='in' && direction!=='out') throw new RangeError('Scene zoom must be in or out');
        changeView(0,0,direction==='in'?1/1.1:1.1);
      },
      calm(enabled) {
        if(disposed) return;
        calm=Boolean(enabled);lastWall=0;cancelAnimationFrame(raf);raf=0;
        if(calm) reset(0);
        overBudget=underBudget=0;qualityLabel();dirty=true;request();
      },
      frame(ms) {
        if(!Number.isFinite(ms) || ms<0 || ms>86400000) throw new RangeError('Scene time outside one day');
        cancelAnimationFrame(raf);raf=0;lastWall=0;
        if(reduced) clock=20000;else clock=ms;
        dirty=true;
        if(ex) {try {render();} catch(err) {error(err);}}
      },
      destroy() {
        if(disposed) return;
        disposed=true;cancelAnimationFrame(raf);raf=0;stopAudio();
        if(rejectSize) {rejectSize(new Error('Scene destroyed before initialization'));resolveSize=null;rejectSize=null;}
        for(const fn of cleanup) fn();
        mounted.delete(canvas);image=null;ex=null;
        canvas.dataset.sceneEngine='destroyed';
      }
    };
    mounted.set(canvas,api);
    listen(document,'visibilitychange',visibility);
    listen(motion,'change',motionChanged);
    if(typeof ResizeObserver==='function') {
      const resize=new ResizeObserver(() => {resizePending=dirty=true;request();});resize.observe(canvas);cleanup.push(() => resize.disconnect());
    } else listen(window,'resize',() => {resizePending=dirty=true;request();});
    if(typeof IntersectionObserver==='function') {
      const observer=new IntersectionObserver(entries => {visible=entries[0].isIntersecting;visibility();});observer.observe(canvas);cleanup.push(() => observer.disconnect());
    }
    if(interactive) {
      const oldTouch=canvas.style.touchAction;canvas.style.touchAction='pinch-zoom';cleanup.push(() => {canvas.style.touchAction=oldTouch;});
      listen(canvas,'pointerdown',event => {
        if(event.button!==0) return;
        if(event.pointerType==='touch') {
          const slot=touches.find(touch => touch.id===null);
          if(!slot) return;
          slot.id=event.pointerId;slot.x=event.clientX;slot.y=event.clientY;
          if(touches[0].id!==null && touches[1].id!==null) pinchDistance=Math.hypot(touches[0].x-touches[1].x,touches[0].y-touches[1].y);
        }
        dragId=event.pointerId;dragX=event.clientX;dragY=event.clientY;
        canvas.setPointerCapture(dragId);canvas.focus({preventScroll:true});
      });
      listen(canvas,'pointermove',event => {
        if(event.pointerType==='touch') {
          const touch=touches.find(touch => touch.id===event.pointerId);
          if(!touch) return;
          touch.x=event.clientX;touch.y=event.clientY;
          if(touches[0].id!==null && touches[1].id!==null) {
            const distance=Math.hypot(touches[0].x-touches[1].x,touches[0].y-touches[1].y);
            if(distance>0 && pinchDistance>0) changeView(0,0,pinchDistance/distance);
            pinchDistance=distance;return;
          }
        }
        if(event.pointerId!==dragId) return;
        const box=canvas.getBoundingClientRect();
        changeView(-(event.clientX-dragX)*fov*1000/Math.max(1,box.width),(event.clientY-dragY)*fov*1000/Math.max(1,box.width),1);
        dragX=event.clientX;dragY=event.clientY;
      });
      const release=event => {
        for(const touch of touches) if(touch.id===event.pointerId) touch.id=null;
        pinchDistance=0;
        if(event.pointerId===dragId) {
          dragId=null;
          if(canvas.hasPointerCapture(event.pointerId)) canvas.releasePointerCapture(event.pointerId);
          const remaining=touches.find(touch => touch.id!==null);
          if(remaining) {dragId=remaining.id;dragX=remaining.x;dragY=remaining.y;}
        }
      };
      listen(canvas,'pointerup',release);listen(canvas,'pointercancel',release);listen(canvas,'lostpointercapture',release);
      listen(canvas,'wheel',event => {if(event.ctrlKey || event.metaKey) return;event.preventDefault();changeView(0,0,Math.pow(2,Math.max(-600,Math.min(600,event.deltaY))/600));},{passive:false});
      listen(canvas,'keydown',event => {
        const step=fov*1000/20;
        switch(event.key) {
          case 'ArrowLeft':changeView(-step,0,1);break;
          case 'ArrowRight':changeView(step,0,1);break;
          case 'ArrowUp':changeView(0,step,1);break;
          case 'ArrowDown':changeView(0,-step,1);break;
          case '+':case '=':changeView(0,0,1/1.1);break;
          case '-':changeView(0,0,1.1);break;
          case 'Home':home();break;
          default:return;
        }
        event.preventDefault();
      });
    }
    api.ready=module().then(module => WebAssembly.instantiate(module,{})).then(async instance => {
      if(disposed) return api;
      ex=instance.exports;
      const initial=still()?0:pendingEnter===null?0:pendingEnter;
      // Preserve an audio gesture made before compilation completes.
      const audioOwner=ownsAudio;ownsAudio=false;reset(initial);ownsAudio=audioOwner;
      pendingEnter=null;
      if(!size()) await new Promise((resolve,reject) => {resolveSize=resolve;rejectSize=reject;});
      if(disposed) return api;
      if(still() || pendingStatic) {clock=20000;ex.scene_step(20000,0,0);pendingStatic=false;}
      render();request();
      canvas.dispatchEvent(new CustomEvent('allen-scene-initialized',{bubbles:true,detail:{source:PIN}}));
      return api;
    }).catch(err => {if(!disposed) error(err);throw err;});
    // Expose the rejection to callers while avoiding an unhandled rejection for decorative mounts.
    api.ready.catch(() => {});
    return api;
  }
  window.AllenScene=Object.freeze({mount,source:PIN});
  document.dispatchEvent(new CustomEvent('allen-scene-ready'));
})();

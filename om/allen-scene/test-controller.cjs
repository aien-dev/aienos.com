'use strict';
/* Lifecycle/interaction unit harness with real WASM, not a browser/visual claim. */
const fs=require('node:fs'),vm=require('node:vm'),assert=require('node:assert/strict');
class Events {
  constructor(){this.listeners=new Map();}
  addEventListener(name,fn){if(!this.listeners.has(name))this.listeners.set(name,new Set());this.listeners.get(name).add(fn);}
  removeEventListener(name,fn){this.listeners.get(name)?.delete(fn);}
  dispatchEvent(event){event.target=this;for(const fn of this.listeners.get(event.type)||[])fn(event);return true;}
}
class Canvas extends Events {
  constructor(){super();this.dataset={};this.style={};this.width=0;this.height=0;this.draws=0;this.captured=null;this.box={width:640,height:400};}
  getContext(){return {putImageData:image=>{assert.equal(image.data.length,this.width*this.height*4);this.draws++;}};}
  getBoundingClientRect(){if(this.onMeasure)this.onMeasure();return this.box;}
  setPointerCapture(id){this.captured=id;}
  hasPointerCapture(id){return this.captured===id;}
  releasePointerCapture(){this.captured=null;}
  focus(){}
}
(async()=>{
  const document=new Events();document.hidden=false;
  const motion=new Events();motion.matches=false;
  const window=new Events(),raf=new Map(),resizes=[],intersections=[];let serial=0;
  const wasm=fs.readFileSync('public/allen/scene.wasm');
  const audio={enter:0,pause:0,resume:0,stop:0};
  window.AllenAudio={enter:()=>{audio.enter++;return Promise.resolve(true);},pause:()=>audio.pause++,resume:()=>{audio.resume++;return Promise.resolve();},stop:()=>audio.stop++};
  const context={window,document,HTMLCanvasElement:Canvas,WebAssembly,Uint8ClampedArray,Float64Array,Int16Array,
    ImageData:class{constructor(data,width,height){this.data=data;this.width=width;this.height=height;}},
    CustomEvent:class{constructor(type,options){this.type=type;Object.assign(this,options);}},
    matchMedia:()=>motion,devicePixelRatio:1,
    fetch:async url=>{assert.equal(url,'/allen/scene.wasm');return {ok:true,arrayBuffer:async()=>wasm};},
    requestAnimationFrame:fn=>{const id=++serial;raf.set(id,fn);return id;},cancelAnimationFrame:id=>raf.delete(id),
    ResizeObserver:class{constructor(fn){this.fn=fn;resizes.push(this);}observe(){}disconnect(){this.disconnected=true;}},
    IntersectionObserver:class{constructor(fn){this.fn=fn;intersections.push(this);}observe(){}disconnect(){this.disconnected=true;}}
  };
  let ready=0;document.addEventListener('allen-scene-ready',()=>ready++);
  vm.runInNewContext(fs.readFileSync('public/js/allen-scene.js','utf8'),context);
  assert.equal(ready,1);
  const canvas=new Canvas(),scene=window.AllenScene.mount(canvas,{interactive:true});
  assert.equal(window.AllenScene.mount(canvas),scene,'one instance per canvas');
  let initialized=0,complete=0;
  canvas.addEventListener('allen-scene-initialized',()=>initialized++);
  canvas.addEventListener('allen-scene-complete',()=>complete++);
  window.AllenAudio.enter('home');scene.enter('birth');await scene.ready;
  assert.equal(audio.enter,1,'visual enter must not start duplicate audio');
  assert.equal(initialized,1,'initialized event follows WASM and buffer readiness');
  assert.equal(canvas.dataset.sceneEngine,'native-wasm');
  scene.frame(8000);assert.equal(canvas.dataset.scenePhase,'merger');
  scene.enter('dusk');scene.frame(2000);assert.equal(canvas.dataset.scenePhase,'tube');
  scene.frame(7216);assert.equal(canvas.dataset.scenePhase,'presence');
  assert.equal(complete,1,'dusk completion announced once');
  scene.key('abc');scene.speak('a safe word');scene.mode('thinking');scene.frame(7600);
  const event={type:'keydown',key:'ArrowRight',preventDefault(){this.prevented=true;}};
  canvas.dispatchEvent(event);assert.ok(event.prevented,'keyboard exploration handled');
  scene.enter('birth');document.hidden=true;document.dispatchEvent({type:'visibilitychange'});
  assert.equal(raf.size,0,'hidden document has no frame loop');assert.ok(audio.pause>0);
  document.hidden=false;document.dispatchEvent({type:'visibilitychange'});assert.ok(raf.size>0);assert.ok(audio.resume>0);
  intersections[0].fn([{isIntersecting:false}]);assert.equal(raf.size,0,'offscreen has no frame loop');
  intersections[0].fn([{isIntersecting:true}]);
  motion.matches=true;motion.dispatchEvent({type:'change'});
  const batch=[...raf];raf.clear();for(const [,fn] of batch)fn(1000);
  assert.equal(raf.size,0,'reduced-motion redraw settles without a frame loop');
  assert.equal(canvas.dataset.scenePhase,'presence');
  const frozen=canvas.dataset.sceneTime;scene.frame(999);assert.equal(canvas.dataset.sceneTime,frozen);
  scene.destroy();assert.equal(raf.size,0);assert.equal(canvas.dataset.sceneEngine,'destroyed');
  assert.ok(resizes[0].disconnected && intersections[0].disconnected);
  motion.matches=false;
  const icon=new Canvas();icon.box={width:160,height:160};
  const compact=window.AllenScene.mount(icon,{compact:true});await compact.ready;
  const compactBatch=[...raf];raf.clear();for(const [,fn] of compactBatch)fn(2000);
  assert.equal(raf.size,0,'compact has static idle, no background loop');
  assert.equal(icon.dataset.scenePhase,'presence');compact.destroy();
  const late=new Canvas();late.box={width:0,height:0};
  let measured,readyLate=false;
  const measurement=new Promise(resolve=>{measured=resolve;});
  late.onMeasure=()=>{late.onMeasure=null;measured();};
  const deferred=window.AllenScene.mount(late,{});
  deferred.ready.then(()=>{readyLate=true;});
  await measurement;assert.equal(readyLate,false,'zero-size canvas is not initialized');
  late.box={width:640,height:400};resizes[2].fn();
  const lateBatch=[...raf];raf.clear();for(const [,fn] of lateBatch)fn(3000);
  await deferred.ready;assert.equal(readyLate,true);
  assert.equal(late.dataset.scenePhase,'tube','zero-size initial mount retains opening choice');
  deferred.destroy();assert.equal(raf.size,0);
  for(const opening of ['idle','dusk','birth']) {
    const target=new Canvas(),instance=window.AllenScene.mount(target,{opening});
    await instance.ready;
    assert.equal(target.dataset.scenePhase,opening==='idle'?'presence':opening==='dusk'?'tube':'merger');
    instance.destroy();
  }
  for(const opening of ['other','',null,0,{},['idle']]) {
    assert.throws(()=>window.AllenScene.mount(new Canvas(),{opening}),/Invalid Allen scene opening/);
  }
  assert.equal(raf.size,0,'opening option tests leave no scheduled work');
  const skippedCanvas=new Canvas(),skipped=window.AllenScene.mount(skippedCanvas,{opening:'idle'});
  await skipped.ready;
  skipped.enter('birth');
  const stopsBeforeSkip=audio.stop;
  skipped.enter('idle');
  const skipBatch=[...raf];raf.clear();for(const [,fn] of skipBatch)fn(4000);
  assert.equal(skippedCanvas.dataset.scenePhase,'presence','idle entry skips the opening without caller timing');
  assert.ok(audio.stop>stopsBeforeSkip,'idle entry stops owned birth audio');
  assert.equal(skippedCanvas.dataset.sceneEngine,'native-wasm','skip retains live native scene');
  skipped.destroy();assert.equal(raf.size,0);
  console.log('Controller lifecycle: ready, aliases, native rendering, keys, speech, hidden/offscreen pause, reduced motion, compact, zero-size mount, bounded opening options, teardown');
})().catch(error=>{console.error(error);process.exitCode=1;});

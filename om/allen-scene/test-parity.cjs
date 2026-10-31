/* Targeted renderer ABI/parity test; not an iMac hardware qualification. */
'use strict';
const fs=require('node:fs'), assert=require('node:assert/strict');
const {execFileSync}=require('node:child_process');
(async () => {
  const oracle=process.argv[2], wasm=process.argv[3]||'public/allen/scene.wasm', original=process.argv[4];
  if(!oracle || !original) throw new Error('usage: node om/allen-scene/test-parity.cjs /path/to/native-oracle wasm /path/to/original-ui-oracle');
  const module=await WebAssembly.compile(fs.readFileSync(wasm));
  assert.deepEqual(WebAssembly.Module.imports(module),[],'native WASM must have no imports');
  const {exports:ex}=await WebAssembly.instantiate(module,{});
  assert.equal(ex.scene_init(63,400,0),0,'undersized canvas refusal');
  assert.equal(ex.scene_init(1920,1920,0),0,'pixel capacity refusal');
  assert.equal(ex.scene_view(-1,0,80000),0,'uninitialized/invalid camera refusal');
  const cases=[[1,0,0,0],[1,600,0,0],[1,2000,0,0],[1,3200,0,0],[1,4192,0,0],[1,5104,0,0],[1,7216,0,0],[2,8000,0,0],[2,12000,0,0],[2,12608,0,0],[2,16512,0,0],[0,17600,0,1],[0,16512,0,2],[0,16512,1,0]];
  for(const [kind,target,compact,variant] of cases) {
    assert.equal(ex.scene_init(640,400,compact),1);
    ex.scene_enter(kind);
    if(variant===2) assert.equal(ex.scene_view(119000000,-71000000,60000),1);
    for(let t=0;t<=target;t+=16) {
      if(variant===1 && t===16000) ex.scene_key(t);
      if(variant===1 && t===17008) ex.scene_word(t);
      ex.scene_step(t,variant===1?(t<16000?0:t<17008?1:3):0,variant===1?128:0);
    }
    ex.scene_step(target,variant===1?3:0,variant===1?128:0);
    ex.scene_draw(variant===1?256:0,variant===1?128:0);
    if(variant===1) ex.scene_glyph(65,300,300,200);
    const observed=Buffer.from(ex.memory.buffer,ex.scene_pixels(),640*400*4);
    const expected=execFileSync(oracle,[kind,target,compact,variant].map(String),{maxBuffer:640*400*4+1024});
    assert.equal(expected.length,observed.length,'oracle cannot silently omit pixels');
    assert.equal(observed.compare(expected),0,`RGBA mismatch kind=${kind} t=${target} compact=${compact} variant=${variant}`);
    if(!compact && !variant && ((kind===1 && target<4200) || (kind===2 && target<15300))) {
      const originalFrame=execFileSync(original,[kind,target].map(String),{maxBuffer:640*400*4+1024});
      assert.equal(observed.compare(originalFrame),0,`ORIGINAL allen_ui.c mismatch kind=${kind} t=${target}`);
      console.log(`Independent original allen_ui.c RGB parity kind=${kind} time=${target} (opaque alpha normalized)`);
    }
    const mutant=Buffer.from(observed);mutant[0]^=1;
    assert.notEqual(mutant.compare(expected),0,'wrong-pixel negative control must differ');
    console.log(`RGBA byte parity kind=${kind} time=${target} compact=${compact} variant=${variant}`);
  }
  const ptr=ex.scene_audio(11360*48,45120),pcm=Buffer.from(ex.memory.buffer,ptr,45120*4);
  assert.equal(pcm.compare(execFileSync(oracle,['audio'],{maxBuffer:45120*4+1024})),0,'native chirp PCM byte mismatch');
  assert.equal(ex.scene_audio(0,48001),0,'audio capacity refusal');
  console.log('PCM byte parity; invalid sizes/camera/audio refused; wrong-pixel control rejected');
})().catch(error => {console.error(error);process.exitCode=1;});

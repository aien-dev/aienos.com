import test from 'node:test';
import assert from 'node:assert/strict';
import fs from 'node:fs';
import {classify,physicalLines,countEntries,paginate,day,validateSnapshot} from './collect-progress.mjs';
test('measurement boundaries preserve named tests and exclude evidence, third-party and symlinks',()=>{
  assert.equal(classify('src/runtime/main.c').kind,'source');assert.equal(classify('tests/compiler/check.inc').kind,'test');assert.equal(classify('src/lib.rs').kind,'source');
  for(const p of ['vendor/d3.js','public/vendor/d3.js','evidence/old.c','tests/fixtures/sample.rs','data/generated.c'])assert.equal(classify(p),null);
  assert.equal(classify('alias.c','120000'),null);assert.equal(classify('scripts/collect-progress.mjs').lang,'Web');
});
test('physical lines handle empty, unterminated and binary blobs',()=>{
  assert.equal(physicalLines(Buffer.from('')),0);assert.equal(physicalLines(Buffer.from('x')),1);assert.equal(physicalLines(Buffer.from('x\n')),1);assert.equal(physicalLines(Buffer.from('x\n\n')),2);assert.equal(physicalLines(Buffer.from([65,0,10])),0);
});
test('the shared blob set prevents double counting migrated files across repositories',()=>{
  const lines=new Map([['shared',10],['test',5]]),seen=new Set();const a=countEntries([{oid:'shared',kind:'source',lang:'C'}],lines,seen);const b=countEntries([{oid:'shared',kind:'source',lang:'C'},{oid:'test',kind:'test',lang:'Rust'}],lines,seen);
  assert.equal(a.source+b.source,10);assert.equal(b.test,5);assert.equal(b.files,1);assert.throws(()=>countEntries([{oid:'missing',kind:'test',lang:'C'}],lines));
});
test('GitHub collections follow every page and deduplicate overlapping records',async()=>{
  const calls=[];const rows=await paginate('/example','workflow_runs',async url=>{calls.push(url);return {workflow_runs:url.endsWith('page=1')?Array.from({length:100},(_,id)=>({id})): [{id:99},{id:100}]};});
  assert.equal(rows.length,101);assert.equal(calls.length,2);await assert.rejects(()=>paginate('/bad','workflow_runs',async()=>({})),/Invalid GitHub collection/);
});
test('daily boundaries use Chicago rather than UTC, including daylight saving time',()=>{assert.equal(day('2026-10-01T03:00:00Z'),'2026-09-30');assert.equal(day('2026-12-01T05:30:00Z'),'2026-11-30');});
test('incomplete and inconsistent observations are refused before publication',()=>{
  const data=JSON.parse(fs.readFileSync('public/data/project-progress.json','utf8'));assert.equal(validateSnapshot(data),data);
  for(const corrupt of [d=>d.series.at(-1).loc++,d=>d.summary.ci++,d=>d.repos[0].head='main',d=>d.series=[],d=>delete d.series[0].testfiles]){const copy=structuredClone(data);corrupt(copy);assert.throws(()=>validateSnapshot(copy));}
});

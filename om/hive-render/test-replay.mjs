// SPDX-License-Identifier: Apache-2.0 WITH LLVM-exception
// Verification-only replay of the existing renderer oracle cases; no production use.
import fs from 'node:fs';
import path from 'node:path';
import os from 'node:os';
import assert from 'node:assert/strict';
import {spawnSync,spawn} from 'node:child_process';
const [binary,omc,scratch,receiver,collector]=process.argv.slice(2).map(x=>path.resolve(x));
if(fs.existsSync(scratch)) assert.equal(fs.readdirSync(scratch).length,0,'scratch directory must be empty');
fs.mkdirSync(scratch,{recursive:true});
const canonical=v=>Buffer.from(JSON.stringify(v,(_,x)=>x&&typeof x==='object'&&!Array.isArray(x)?Object.fromEntries(Object.keys(x).sort().map(k=>[k,x[k]])):x)+'\n');
const agent=(id='a1',parent=null)=>({id,parent_id:parent,name:'Worker',runtime:'OMP',provider:'provider',model:'model',role:'worker',task:'A public task',status:'working',activity:'A public action',activity_state:'running',started_at:'2026-10-25T23:59:59.900Z',updated_at:null,observed_at:'2026-10-26T00:00:00.100Z'});
const snapshot=()=>({schema_version:1,generated_at:'2026-10-26T00:00:01.100Z',sequence:'snapshot',poll_ms:1000,stale_after_ms:15000,coverage:{label:'OMP',roots:1,agents:1,complete:false,notice:'Partial observation'},agents:[agent()],interactions:[]});
const execute=(exe,input,args=[],env=process.env)=>spawnSync(exe,args,{input,env,timeout:30000,maxBuffer:67108864});
const run=(raw,good=true)=>{const r=execute(binary,raw);assert(!r.error,String(r.error));if(good){assert.equal(r.status,0,r.stderr.toString());assert(r.stdout.toString().startsWith('<!doctype html>'));assert(r.stdout.toString().endsWith('</html>\n'));}else{assert.notEqual(r.status,0);assert.equal(r.stdout.length,0);}return r.stdout;};
const escaped=x=>x.replaceAll('&','&amp;').replaceAll('<','&lt;').replaceAll('>','&gt;').replaceAll('"','&quot;').replaceAll("'",'&#x27;');
const safeDoc=out=>{const html=out.toString();assert(!/<(?:script|img)(?:\s|>)/i.test(html));for(const [tag] of html.matchAll(/<[A-Za-z][^>]*>/g)){assert(!/\son[a-z]+\s*=/i.test(tag));assert(!/javascript:/i.test(tag));}const ids=[...html.matchAll(/\bid="([^"]*)"/g)].map(x=>x[1]);assert.equal(ids.length,new Set(ids).size);};
const raw=canonical(snapshot());
const first=run(raw);assert.deepEqual(first,run(raw));safeDoc(first);assert(first.includes(Buffer.from('1s')));
fs.writeFileSync(path.join(scratch,'fixture.json'),raw);
for(const sequence of ['', '\"><script>alert(1)</script>', 'é'.repeat(64)]){const v=snapshot();v.sequence=sequence;const out=run(canonical(v));safeDoc(out);assert(out.includes(Buffer.from('data-sequence="s-'+Buffer.from(sequence).toString('hex')+'"')));}
const source=fs.readFileSync('render.om','utf8').split('fn main to int\n')[0];
const escapeMain=`fn main to int
n = ulong 0
while lt n 16000
r = read 0 dec n sub ulong 16000 n
if lt r 0
ret 1
end
if eq r 0
break
end
n = add n ulong r
end
escape 0 n
r = write 1 out 0 get st 5
if ne r long get st 5
ret 2
end
ret 0
end
`;
const compiled=execute(omc,source+escapeMain);assert.equal(compiled.status,0,compiled.stderr.toString());
const probe=path.join(scratch,'escape');fs.writeFileSync(probe,compiled.stdout,{mode:0o700});
const values=Array.from({length:256},(_,i)=>Buffer.from([i]));values.push(Buffer.from(Array.from({length:256},(_,i)=>i)),Buffer.from(`&<>"'`.repeat(144)),Buffer.from(`&<>"'`.repeat(3200)));
for(const value of values){const r=execute(probe,value);assert.equal(r.status,0);const expected=Buffer.from(escaped(value.toString('latin1')),'latin1');assert.deepEqual(r.stdout,expected);if(value.includes(60))assert.notDeepEqual(r.stdout,Buffer.from(expected.toString('latin1').replaceAll('&lt;','<'),'latin1'));}
const probes=['<img src=x onerror=alert(1)>','</li><script>alert(1)</script>','\"><a href="javascript:alert(1)">','</details>'];
for(const payload of [...probes,'\b\f\n\r\t\\"','é 🐝 漢字',`&<>"'`.repeat(3200)]){const v=snapshot();v.agents[0].activity=payload;v.interactions=[{id:'m1',agent_id:'a1',role:'assistant',text:payload,at:v.generated_at,runtime:'OMP',provider:'provider',model:'model'}];const out=run(canonical(v));assert(out.includes(Buffer.from(escaped(payload))));safeDoc(out);}
for(const role of ['drake','user','guest','Assistant']){const v=snapshot();v.interactions=[{id:'m1',agent_id:'a1',role,text:'HUMAN_MESSAGE_CANARY',at:v.generated_at,runtime:'OMP',provider:'provider',model:'model'}];run(canonical(v),false);}
{const v=snapshot();v.interactions=[{id:'m1',agent_id:'a1',role:'assistant',text:'Agent reply',at:v.generated_at,runtime:'OMP',provider:'provider',model:'model'}];const out=run(canonical(v));assert(out.includes(Buffer.from('<h2>What the agents are saying, live</h2><ol class="hive-chat"')));assert(!out.includes(Buffer.from('Drake')));}
for(const [field,size] of [['name',720],['task',16000],['activity',16000]]){const v=snapshot();v.agents[0][field]='x'.repeat(size);run(canonical(v));v.agents[0][field]+='x';run(canonical(v),false);}
const text=raw.toString();
const bad=[raw.subarray(0,raw.length-2),Buffer.from(text.replace('"Worker"','"unterminated')),Buffer.alloc(2097153,120),Buffer.from(text.replace('"Worker"','"\\u0001"')),Buffer.from(text.replace('"Worker"','"\\q"')),Buffer.from(text.replace('"Worker"','"\\u0041"')),Buffer.concat([Buffer.from(text.split('"Worker"')[0]+'"'),Buffer.from([192,188]),Buffer.from('"'+text.split('"Worker"')[1])]),Buffer.from(text.replace('"agents":[','"agents":[],"agents":[')),Buffer.from(text.replace('"coverage":{','"unknown":0,"coverage":{'))];
for(const encoded of bad)run(encoded,false);
for(const mutation of ['bad-id','parent','cycle','count','date','enum','fraction','duplicate-id']){const v=snapshot();if(mutation==='bad-id')v.agents[0].id='a" onclick="x';if(mutation==='parent')v.agents[0].parent_id='missing';if(mutation==='cycle'){v.agents[0].parent_id='a1';v.coverage.roots=0;}if(mutation==='count')v.coverage.agents=2;if(mutation==='date')v.generated_at='2026-02-29T00:00:01Z';if(mutation==='enum')v.agents[0].status='invented';if(mutation==='fraction')v.generated_at='2026-10-26T00:00:01.Z';if(mutation==='duplicate-id'){v.agents.push(agent());v.coverage.agents=2;v.coverage.roots=2;}run(canonical(v),false);}
{const v=snapshot();v.agents=Array.from({length:500},(_,i)=>agent('a'+i,i?'a'+(i-1):null));v.coverage.agents=500;const out=run(canonical(v)).toString();assert.equal(out.split('<summary class="hive-record">').length-1,500);assert.equal(out.split('</ul></details></li>').length-1,500);}
{const v=snapshot();v.agents=Array.from({length:60},(_,i)=>({...agent('a'+i),activity:"'".repeat(16000),task:"'".repeat(16000)}));v.coverage.agents=60;v.coverage.roots=60;const input=canonical(v);assert(input.length<2097152);const out=run(input);assert(out.length>16777216);safeDoc(out);}
// Wolfram DateDifference[{y,2,28},{y,3,1},"Second"] supplied these exact second counts.
for(const [year,seconds] of [[1900,86400],[2000,172800],[2024,172800],[2026,86400]]){const v=snapshot();v.generated_at=`${year}-03-01T00:00:00Z`;v.agents[0].started_at=`${year}-02-28T00:00:00Z`;v.agents[0].observed_at=v.generated_at;assert(run(canonical(v)).includes(Buffer.from(`>${seconds}s</span>`)));}
console.log('PASS renderer replay: escaper, Unicode, XSS, schema, limits, human roles, dates, deep tree and output pools');
const value={schema_version:1,generated_at:'2026-10-26T01:00:00Z',sequence:'integration',poll_ms:1000,stale_after_ms:15000,agents:[],interactions:[],coverage:{label:'OMP',roots:0,agents:0,complete:false,notice:'Integration oracle'}};
const directory=path.join(scratch,'receiver');fs.mkdirSync(directory,{recursive:true});
const board=path.join(directory,'board.html'), live=path.join(directory,'live.json'), fail=path.join(directory,'fail-renderer');
fs.writeFileSync(fail,'#!/bin/sh\nprintf incomplete\nexit 7\n',{mode:0o700});fs.writeFileSync(board,'old useful board');
const env={...process.env,SSH_ORIGINAL_COMMAND:'hive-feed',HIVE_RENDER:fail};
let r=execute(receiver,canonical(value),[directory],env);assert.equal(r.status,0);assert.equal(fs.readFileSync(board,'utf8'),'old useful board');assert(r.stderr.includes(Buffer.from('retained previous observation')));assert.deepEqual(JSON.parse(fs.readFileSync(live)),value);assert(!fs.readdirSync(directory).some(f=>f.startsWith('.board-tmp-')));
env.HIVE_RENDER=binary;r=execute(receiver,canonical(value),[directory],env);assert.equal(r.status,0,r.stderr.toString());assert.equal(r.stderr.length,0);assert.deepEqual(fs.readFileSync(board),run(fs.readFileSync(live)));const before=fs.readFileSync(board);
for(const v of [{...value,schema_version:2},...['drake','user','guest'].map(role=>({...value,generated_at:'2026-10-26T02:00:00Z',sequence:'human-'+role,interactions:[{id:'m1',agent_id:'a1',role:'assistant',text:'Agent reply',at:value.generated_at,runtime:'OMP',provider:'unknown',model:'unknown'},{id:'m2',agent_id:'a1',role,text:'HUMAN_MESSAGE_CANARY',at:value.generated_at,runtime:'OMP',provider:'unknown',model:'unknown'}]}))]){r=execute(receiver,canonical(v),[directory],env);assert.notEqual(r.status,0);assert.deepEqual(fs.readFileSync(board),before);assert.deepEqual(JSON.parse(fs.readFileSync(live)),value);}
const overlap=path.join(scratch,'overlap');fs.mkdirSync(overlap,{recursive:true});
const initial={...value,generated_at:'2026-10-26T00:00:00Z',sequence:'initial'},newest={...value,generated_at:'2026-10-26T02:00:00Z',sequence:'newest'},delayed={...value,generated_at:'2026-10-26T01:00:00Z',sequence:'delayed'};
const stream=spawn(receiver,[overlap],{env,stdio:['pipe','pipe','pipe']});const finished=new Promise(resolve=>stream.on('close',resolve));
try{stream.stdin.write(canonical(initial));let ready=false;for(let i=0;i<1000;i++){if(fs.existsSync(path.join(overlap,'board.html'))){ready=true;break;}await new Promise(resolve=>setTimeout(resolve,10));}assert(ready);r=execute(receiver,canonical(newest),[overlap],env);assert.equal(r.status,0);const newestBoard=fs.readFileSync(path.join(overlap,'board.html'));stream.stdin.end(canonical(delayed));const status=await Promise.race([finished,new Promise((_,reject)=>setTimeout(()=>reject(new Error('receiver timeout')),10000))]);assert.notEqual(status,0);assert.equal(JSON.parse(fs.readFileSync(path.join(overlap,'live.json'))).sequence,'newest');assert.deepEqual(fs.readFileSync(path.join(overlap,'board.html')),newestBoard);}finally{if(stream.exitCode===null)stream.kill('SIGTERM');}
console.log('PASS receiver replay: failed render retains board, valid publication, schema/human refusal, overlapping stream ordering');
const cfg=fs.mkdtempSync(path.join(os.tmpdir(),'hive-config-control-'));
try {
  const empty=path.join(cfg,'empty');fs.mkdirSync(empty);
  const list=path.join(cfg,'repos.tsv');
  for(const [name,content,ok] of [['missing',null,false],['invalid','fixture\tUNREDACTED_PRIVATE_LABEL\n',false],['empty','',true],['valid','fixture\tprivate-03\n',true]]) {
    if(content!==null)fs.writeFileSync(list,content);
    const out=path.join(cfg,name+'.json');
    const args=['--once','--out',out,'--sessions',empty,'--claude',empty,'--codex',empty,'--private-repos',list,'--salt',path.join(cfg,'salt')];
    const result=execute(collector,'',args);
    if(ok){assert.equal(result.status,0,result.stderr.toString());const value=JSON.parse(fs.readFileSync(out));assert.equal(value.interactions.length,0);assert(!JSON.stringify(value).includes(cfg));}
    else{assert.notEqual(result.status,0);assert(!fs.existsSync(out));assert(!result.stderr.includes(Buffer.from('UNREDACTED_PRIVATE_LABEL')));}
  }
} finally { fs.rmSync(cfg,{recursive:true,force:true}); }
console.log('PASS collector startup: missing/invalid privacy config refuses before publication; explicit empty and valid mapping accepted');

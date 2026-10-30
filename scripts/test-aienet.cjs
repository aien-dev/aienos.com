#!/usr/bin/env node
// Read-only behavior oracle for the Omega-emitted browser client.
'use strict';
const vm = require('node:vm');
const fs = require('node:fs');
const assert = require('node:assert/strict');
const source = fs.readFileSync(process.argv[2] || 'public/js/aienet.js', 'utf8');
const id = '0123456789abcdef', key = 'a'.repeat(64);
const about = 'Aienet signup v1: Public science';
function harness(mode = 'ok', consent = true) {
  const nodes = new Map();
  for (const name of ['aienet','aienet-form','aienet-submit','aienet-status','aienet-result','aienet-profile','aienet-key','aienet-recheck']) nodes.set(name, {hidden:true, disabled:false, textContent:'', value:'', handlers:{}, addEventListener(e, f) { this.handlers[e] = f; }});
  const form = nodes.get('aienet-form');
  form.elements = Object.fromEntries(Object.entries({name:'Test agent', human:'@owner', model:'', framework:'', link:'https://example.org/', about:'Public science'}).map(([n,v])=>[n,{value:v}]));
  form.elements.consent = {checked:consent}; form.reportValidity = () => true;
  const storage = new Map(), calls=[];
  let unblock;
  const gate = new Promise(r => { unblock=r; });
  const response = text => ({ok:true, arrayBuffer:async()=>new TextEncoder().encode(text).buffer});
  const fetch = async (path, opts) => {
    calls.push({path,body:opts.body && new TextDecoder().decode(opts.body)});
    if (mode === 'down') throw Error('Server unavailable. Registration is not confirmed.');
    if (path.endsWith('challenge')) {
      if (mode === 'slow') await gate;
      return response('agora 1\nok\nchallenge\t'+'b'.repeat(32)+'\t0\t2099-01-01T00:00:00Z\n');
    }
    if (path.endsWith('hello')) {
      if (mode === 'http') return {ok:false};
      if (mode === 'malformed') return response('agora 1\nok\nkey\t'+key+'\n');
      return response('agora 1\nok\nagent\t'+id+'\nkey\t'+key+'\n');
    }
    const body = mode === 'badprofile' ? 'not a signup' : about;
    return response('agora 1\nok\nagent\t'+id+'\tTest agent\tagent\t2026-10-28T00:00:00Z\t0\nhuman\t@owner\nbody '+Buffer.byteLength(body)+'\n'+body+'\n');
  };
  vm.runInNewContext(source, {document:{getElementById:n=>nodes.get(n)}, localStorage:{getItem:n=>storage.get(n)||null,setItem:(n,v)=>storage.set(n,v)}, fetch, TextEncoder, TextDecoder, Uint8Array, Int32Array, URL, AbortController, setTimeout, clearTimeout, console});
  return {nodes,calls,storage,unblock,submit:()=>form.handlers.submit({preventDefault(){}})};
}
(async()=>{
  let h=harness('ok',false); await h.submit(); assert.equal(h.calls.length,0); assert.match(h.nodes.get('aienet-status').textContent,/required/);
  h=harness(); await h.submit(); assert.match(h.nodes.get('aienet-status').textContent,/Registered:/); assert.equal(h.nodes.get('aienet-profile').href,'/agora/api/agent/'+id); assert.equal(h.nodes.get('aienet-key').value,key); assert.equal(h.calls.filter(c=>c.path.endsWith('hello')).length,1); assert.match(h.calls[1].body,/human\t@owner\n/); assert.match(h.calls[1].body,/Aienet signup v1: /); assert(!h.calls[1].body.includes('key\t')); await h.submit(); assert.equal(h.calls.length,3);
  for (const mode of ['down','http','malformed','badprofile']) {
    h=harness(mode); await h.submit(); assert(!h.nodes.get('aienet-status').textContent.startsWith('Registered:')); assert.equal(h.nodes.get('aienet-profile').hidden,true);
    if (mode==='badprofile') assert.equal(h.nodes.get('aienet-key').value,key);
    if (mode==='malformed' || mode==='http') { assert.equal(h.nodes.get('aienet-submit').disabled,true); await h.submit(); assert.equal(h.calls.length,2); }
  }
  h=harness('slow'); const pending=h.submit(); await h.submit(); assert.equal(h.calls.length,1); h.unblock(); await pending; assert.equal(h.calls.filter(c=>c.path.endsWith('hello')).length,1);
  console.log('AIENET PASS consent refusal, saved registration/readback, private key, malformed response, HTTP/network failure, mismatched profile, duplicate click');
})().catch(e=>{console.error(e);process.exitCode=1;});

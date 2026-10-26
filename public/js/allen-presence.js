/* Generated from om/allen-presence/presence.om. */
(() => {
'use strict';
const root = document.getElementById('allen');
const homeDoor = Boolean(root && root.closest('[data-home-door]'));
const reduced = matchMedia('(prefers-reduced-motion: reduce)');
const byId = id => document.getElementById(id);
let scene = null, panes = null, subscribed = false, entered = false;
let reply = '', painted = 0, timer = 0, cardId = 0;
let resolveReady, rejectReady;
const ready = new Promise((resolve,reject) => { resolveReady=resolve; rejectReady=reject; });
ready.catch(() => {});
const dispose = [];
function status(text) { const node = byId('allen-status'); if (node) node.textContent = text; }
function showSession(open) {
 const panel = byId('allen-session'), trigger = byId('allen-history');
 panel.hidden = !open; trigger.setAttribute('aria-expanded', String(open));
 if (open) { panel.tabIndex = -1; panel.focus(); } else trigger.focus();
}
function paintReply() {
 clearTimeout(timer); timer=0;
 const out = byId('allen-answer');
 if (!out || document.hidden || painted >= reply.length) return;
 if (reduced.matches) { painted=reply.length; out.textContent=reply; return; }
 const letter=String.fromCodePoint(reply.codePointAt(painted));
 if (scene && !/\s/.test(letter) && (!painted || /\s/.test(reply[painted-1]))) scene.speak(letter);
 const span=document.createElement('span'); span.className='allen-letter'; span.textContent=letter; out.append(span);
 painted+=letter.length;
 if (painted < reply.length) timer=setTimeout(paintReply,35);
}
function replaceReply(text, speaker) {
 clearTimeout(timer); timer=0; reply=String(text); painted=0;
 byId('allen-speaker').textContent=speaker;
 const out=byId('allen-answer'); out.textContent='';
 if (speaker==='Allen') out.dataset.speaker='allen'; else delete out.dataset.speaker;
 paintReply();
}
function mountScene() {
 if (!window.AllenScene || scene) return;
 const canvas=byId('allen-universe');
 if (!canvas) return;
 scene=window.AllenScene.mount(canvas,{compact:false,interactive:true,opening:homeDoor?'idle':'dusk'});
 window.allenPresenceScene=scene;
 scene.ready.then(() => {
  byId('allen-enter').disabled=false; resolveReady(scene);
  document.dispatchEvent(new CustomEvent('allen-presence-ready'));
 }, error => { rejectReady(error); byId('allen-enter').disabled=false; status('The scene could not load. You can still chat with Allen.'); });
}
function mountPanes() {
 if (!window.AllenPanes || panes) return;
 panes=window.AllenPanes.mount(byId('allen-panes'),{});
}
function enter(options={}) {
 if (entered) return; entered=true; root.dataset.entered='true';
 byId('allen-arrival').hidden=true;
 mountScene(); mountPanes();
 if (options.scene!==false && scene && !homeDoor) scene.enter('dusk');
 // Native dusk is silent. The home-door owner starts the merger chirp on a real gesture.
 byId('allen-input').focus();
 const initial=byId('allen-log').querySelector('.allen-msg-allen .allen-p');
 if (!reply && initial) replaceReply(initial.textContent,'Allen');
}
function audioLabel() {
 const muted=window.AllenAudio ? window.AllenAudio.muted() : true;
 const button=byId('allen-mute'); button.setAttribute('aria-pressed',String(muted)); button.textContent=muted?'Sound off':'Sound on';
}
function connect() {
 if (!root || !window.allen || subscribed) return; subscribed=true;
 const api=window.allen;
 byId('allen-log').setAttribute('aria-live','off');
 dispose.push(api.on('send',event => {
  if (!entered) enter({scene:false});
  if (!event.silent) byId('allen-guest').textContent=event.text;
  replaceReply('','Allen');
  if (scene) { scene.key(event.text); scene.mode('think'); }
  byId('allen-regenerate').disabled=true;
 }));
 dispose.push(api.on('token',event => { reply+=event.text; if (!timer) paintReply(); }));
 dispose.push(api.on('reply',event => {
  if (!event.text.startsWith(reply.slice(0,painted))) replaceReply(event.text,'Allen');
  else { reply=event.text; if (!timer) paintReply(); }
  byId('allen-regenerate').disabled=false;
  if (scene) scene.mode('idle');
 }));
 dispose.push(api.on('error',event => { status(event.message); byId('allen-regenerate').disabled=false; if (scene) scene.mode('idle'); }));
 dispose.push(api.on('human',event => replaceReply(event.text,'Drake')));
 dispose.push(api.on('card',event => {
  mountPanes();
  if (panes && panes.openNode) panes.openNode({id:'chat-'+(++cardId),title:event.type==='relay'?'Message to Drake':event.type==='introduce'?'Remember me':'Allen · '+event.type,node:event.node});
  else showSession(true);
 }));
 for (const name of ['pane.open','pane.update','pane.close']) dispose.push(api.on(name,event => { mountPanes(); if (panes) panes.event(name,event); }));
 byId('allen-regenerate').addEventListener('click',() => api.regenerate());
 byId('allen-copy').addEventListener('click',async () => {
  try { await navigator.clipboard.writeText(reply); status('Reply copied.'); } catch (_) { status('Clipboard unavailable. Select the reply to copy it.'); }
 });
 byId('allen-introduce').addEventListener('click',() => showSession(true));
 byId('allen-new').addEventListener('click',() => {
  byId('allen-guest').textContent='';
  const hello=byId('allen-log').querySelector('.allen-msg-allen .allen-p');
  replaceReply(hello?hello.textContent:'','Allen');
 });
}
function full() {
 window.allenPresence=Object.freeze({ready,enter,scene:() => scene,panes:() => panes});
 byId('allen-enter').disabled=true;
 if (homeDoor) byId('allen-arrival').hidden=true;
 mountScene(); mountPanes(); connect(); audioLabel();
 byId('allen-enter').addEventListener('click',() => enter());
 byId('allen-history').addEventListener('click',() => showSession(byId('allen-session').hidden));
 byId('allen-mute').addEventListener('click',() => { if (window.AllenAudio) { window.AllenAudio.setMuted(!window.AllenAudio.muted()); audioLabel(); } });
 byId('allen-projects').addEventListener('click',() => { mountPanes(); if (panes && panes.project) panes.project(); });
 const input=byId('allen-input');
 input.addEventListener('input',event => {
  input.style.height='36px'; input.style.height=Math.min(120,input.scrollHeight)+'px';
  if (scene && event.data && !event.isComposing) scene.key(event.data);
 });
 document.addEventListener('keydown',event => {
  if (event.key==='Escape' && !byId('allen-session').hidden) { event.preventDefault(); showSession(false); }
  if ((event.ctrlKey || event.metaKey) && event.key==='/') { event.preventDefault(); input.focus(); }
 });
 document.addEventListener('visibilitychange',() => { if (!document.hidden && painted<reply.length) paintReply(); });
 reduced.addEventListener('change',() => { if (reduced.matches) { clearTimeout(timer); timer=0; painted=reply.length; byId('allen-answer').textContent=reply; } });
 document.addEventListener('allen-ready',connect);
 document.addEventListener('allen-scene-ready',mountScene);
 document.addEventListener('allen-panes-ready',mountPanes);
 window.addEventListener('pagehide',() => { clearTimeout(timer); for (const off of dispose) off(); if (scene) scene.destroy(); if (panes) panes.destroy(); if (window.AllenAudio) window.AllenAudio.stop(); },{once:true});
}
if(root) {
 let layoutFrame=0;
 function layout() {
  layoutFrame=0;
  const viewport=window.visualViewport;
  root.style.setProperty('--allen-view-height',(viewport?viewport.height:innerHeight)+'px');
  root.style.setProperty('--allen-view-top',(viewport?viewport.offsetTop:0)+'px');
  const bottom=root.querySelector('.allen-bottom');
  if(bottom) root.style.setProperty('--allen-panes-sheet-bottom',Math.max(0,root.getBoundingClientRect().bottom-bottom.getBoundingClientRect().top+8)+'px');
 }
 function requestLayout() {if(!layoutFrame) layoutFrame=requestAnimationFrame(layout);}
 window.addEventListener('resize',requestLayout);
 if(window.visualViewport) {visualViewport.addEventListener('resize',requestLayout);visualViewport.addEventListener('scroll',requestLayout);}
 const observer=new ResizeObserver(requestLayout); const bottom=root.querySelector('.allen-bottom');if(bottom)observer.observe(bottom);
 document.addEventListener('allen-pane-request',event => {
  const d=event.detail;mountPanes();
  if(!panes || !panes.openNode || !d || !(d.node instanceof Node))return;
  const handle=panes.openNode({id:d.id,title:d.title,node:d.node,focus:false});
  if(handle) {event.preventDefault();d.handle=handle;}
 });
 window.addEventListener('pagehide',() => {observer.disconnect();cancelAnimationFrame(layoutFrame);window.removeEventListener('resize',requestLayout);if(window.visualViewport){visualViewport.removeEventListener('resize',requestLayout);visualViewport.removeEventListener('scroll',requestLayout);}},{once:true});
 requestLayout();
}
function compact() {
 const launcher = byId('aien-allen-launcher');
 if (!launcher || !window.AllenScene) return;
 const canvas = launcher.querySelector('canvas');
 const small = window.AllenScene.mount(canvas,{compact:true,interactive:false});
 const aside = byId('aien-allen-intro');
 function intro(event) {
  if (event.detail.type === 'show' && window.aienAllenIntro) small.speak(window.aienAllenIntro.text.greeting);
  if (event.detail.type === 'reply-no' && window.aienAllenIntro) small.speak(window.aienAllenIntro.text.afterNo);
 }
 document.addEventListener('aien-allen-intro',intro);
 if (aside && !aside.hidden && window.aienAllenIntro) small.speak(window.aienAllenIntro.text.greeting);
 launcher.addEventListener('click', event => {
  if (event.metaKey || event.ctrlKey || event.shiftKey || event.altKey || reduced.matches) return;
  event.preventDefault();
  launcher.classList.add('allen-launch-enter');
  const finish = () => location.assign(launcher.href);
  launcher.addEventListener('transitionend',finish,{once:true}); setTimeout(finish,450);
 });
 window.addEventListener('pagehide', () => { small.destroy(); document.removeEventListener('aien-allen-intro',intro); }, {once:true});
}
if (root) full(); else { compact(); if (!window.AllenScene) document.addEventListener('allen-scene-ready',compact,{once:true}); }
})();

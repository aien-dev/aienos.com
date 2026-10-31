(function(){
"use strict";
var $ = function(id){ return document.getElementById(id); };

/* ---------- scroll progress ---------- */
var prog = $("progress");
function onScroll(){
  var h = document.documentElement;
  var p = h.scrollTop / (h.scrollHeight - h.clientHeight);
  prog.style.width = (p*100).toFixed(2) + "%";
}
window.addEventListener("scroll", onScroll, {passive:true}); onScroll();

/* ---------- reveal on scroll ---------- */
var io = new IntersectionObserver(function(es){
  es.forEach(function(e){ if(e.isIntersecting){ e.target.classList.add("visible"); io.unobserve(e.target); } });
},{threshold:.12});
document.querySelectorAll(".reveal").forEach(function(el){ io.observe(el); });

/* ---------- hero particle field: scattered bits coalescing ---------- */
var cv = $("field"), ctx = cv.getContext("2d"), W=0, H=0, parts=[];
function sizeCanvas(){
  W = cv.width = cv.offsetWidth; H = cv.height = cv.offsetHeight;
  parts = [];
  var n = Math.max(90, Math.min(240, Math.floor(W*H/9000)));
  for(var i=0;i<n;i++){
    parts.push({
      hx: Math.random()*W, hy: Math.random()*H,
      sa: Math.random()*Math.PI*2,
      sr: 26 + Math.random()*Math.min(W,H)*0.30,
      s: .7 + Math.random()*1.9,
      sp: .5 + Math.random()*.9,
      c: Math.random()<.72 ? "45,127,249" : "0,229,153",
      dr: (Math.random()-.5)*.14
    });
  }
}
sizeCanvas(); window.addEventListener("resize", sizeCanvas);
function ease(x){ return x<.5 ? 2*x*x : 1-Math.pow(-2*x+2,2)/2; }
var t0 = performance.now();
function frame(now){
  var t = ((now - t0)/16000) % 1;              /* 16s loop */
  var k = t<.5 ? ease(t*2) : ease((1-t)*2);    /* 0 scattered, 1 coalesced */
  ctx.clearRect(0,0,W,H);
  ctx.globalCompositeOperation = "lighter";
  var cx = W/2, cy = H*0.40, rot = now*0.00012;
  for(var i=0;i<parts.length;i++){
    var p = parts[i];
    p.hx += p.dr; p.hy += p.dr*.6;
    if(p.hx<0)p.hx=W; if(p.hx>W)p.hx=0; if(p.hy<0)p.hy=H; if(p.hy>H)p.hy=0;
    var a = p.sa + rot*p.sp;
    var tx = cx + Math.cos(a)*p.sr, ty = cy + Math.sin(a)*p.sr*.72;
    var x = p.hx + (tx-p.hx)*k, y = p.hy + (ty-p.hy)*k;
    var al = .16 + .6*k;
    ctx.beginPath(); ctx.arc(x, y, p.s*(0.7+0.9*k), 0, 6.2832);
    ctx.fillStyle = "rgba("+p.c+","+al.toFixed(3)+")"; ctx.fill();
  }
  ctx.globalCompositeOperation = "source-over";
  requestAnimationFrame(frame);
}
requestAnimationFrame(frame);

/* ---------- GAME 1: crack the code ---------- */
var g1rules = document.querySelectorAll("#g1-rules .rule-btn");
var g1tiles = [$("g1-h0"),$("g1-h1"),$("g1-h2")];
function paintTiles(vals, ok){
  g1tiles.forEach(function(t,i){
    t.textContent = vals[i];
    t.style.borderStyle = "solid";
    if(ok===true){ t.style.background="rgba(0,229,153,.14)"; t.style.borderColor="#00e599"; t.style.color="#00e599"; }
    else if(ok===false){ t.style.background="rgba(255,93,93,.12)"; t.style.borderColor="#ff5d5d"; t.style.color="#ff5d5d"; }
  });
}
function resetTiles(){
  g1tiles.forEach(function(t){
    t.textContent="?"; t.style.background=""; t.style.borderColor=""; t.style.color=""; t.style.borderStyle="";
  });
}
var g1data = {
  good: { vals:["28","33","38"], ok:true, cls:"good",
    verdict:"+2.6 T. The rule earns its keep.",
    math:"3 sealed events nailed &nbsp;<b>+3.0</b><br>rule rent, 4 words &times; 0.1 &nbsp;<b>&minus;0.4</b><br>= <b>+2.6 T</b>",
    explain:"<strong>Why it wins:</strong> four words that keep working on data nobody has seen. The rent is tiny and the prediction is total. This is what a Turing rewards." },
  memo: { vals:["23","23","23"], ok:false, cls:"bad",
    verdict:"&minus;1.0 T. Memory is not understanding.",
    math:"0 sealed events predicted &nbsp;<b>+0.0</b><br>rule rent, 10 words &times; 0.1 &nbsp;<b>&minus;1.0</b><br>= <b>&minus;1.0 T</b>",
    explain:"<strong>Why it fails:</strong> the rule only recites what was already shown, so when it meets sealed numbers it just repeats the last thing it saw. Ten words of rent, zero prediction. Negative Turings." },
  wrong: { vals:["43","83","163"], ok:false, cls:"bad",
    verdict:"&minus;0.5 T. Confident and wrong.",
    math:"0 sealed events predicted &nbsp;<b>+0.0</b><br>rule rent, 5 words &times; 0.1 &nbsp;<b>&minus;0.5</b><br>= <b>&minus;0.5 T</b>",
    explain:"<strong>Why it fails:</strong> it sounds like a rule, but the sealed data disagrees with every guess. The scorer does not grade confidence. It grades sealed predictions minus rent." }
};
g1rules.forEach(function(btn){
  btn.addEventListener("click", function(){
    var d = g1data[btn.getAttribute("data-rule")];
    g1rules.forEach(function(b){ b.disabled=true; b.classList.add("dim"); });
    btn.classList.remove("dim"); btn.classList.add("picked");
    paintTiles(d.vals, d.ok);
    $("g1-score").textContent = d.cls==="good" ? "+2.6 T" : (btn.getAttribute("data-rule")==="memo" ? "-1.0 T" : "-0.5 T");
    var v=$("g1-verdict"); v.className="verdict "+d.cls; v.innerHTML=d.verdict;
    $("g1-math").innerHTML=d.math; $("g1-explain").innerHTML=d.explain;
    $("g1-result").classList.add("show");
  });
});
$("g1-again").addEventListener("click", function(){
  g1rules.forEach(function(b){ b.disabled=false; b.classList.remove("dim","picked"); });
  resetTiles(); $("g1-result").classList.remove("show"); $("g1-score").textContent="--";
});

/* ---------- RULE LAB ---------- */
var rGain=$("r-gain"), rCost=$("r-cost");
function lab(){
  var g=+rGain.value, c=+rCost.value, rent=c*0.1, t=g-rent;
  $("o-gain").textContent=g; $("o-cost").textContent=c;
  var tv=$("lab-t");
  var txt=(t>0?"+":"")+(Math.round(t*10)/10).toFixed(1)+" T";
  tv.textContent=txt;
  tv.className="tval "+(t>0?"pos":t<0?"neg":"zero");
  $("lab-f-gain").textContent=g; $("lab-f-cost").textContent=rent.toFixed(1);
  var ft=$("lab-f-t"); ft.textContent=txt; ft.style.color=t>0?"#00e599":t<0?"#ff5d5d":"#94a3b8";
  var w=Math.max(3,Math.min(100,(t+4)/16*100));
  var bar=$("lab-bar"); bar.style.width=w+"%";
  bar.style.background=t>0?"linear-gradient(90deg,#2d7ff9,#00e599)":t<0?"linear-gradient(90deg,#7f1d1d,#ff5d5d)":"#475569";
  var note=$("lab-note"), msg;
  if(t>=8) msg="Theory territory. Short, powerful, keeps predicting on unseen data.";
  else if(t>=3) msg="A solid explanation. It earns its keep with room to spare.";
  else if(t>0) msg="Positive but thin. Can you say it in fewer words?";
  else if(t===0) msg="Exactly at baseline. It predicts nothing beyond its own cost.";
  else msg="Memorizer zone: longer than the insight it carries. The scorer charges rent.";
  note.textContent=msg;
}
rGain.addEventListener("input", lab); rCost.addEventListener("input", lab); lab();

/* ---------- GAME 2: beat the memorizer ---------- */
var rounds=[
 {q:"Duel 1: the number game", sub:"A machine watched 3, 8, 13, 18, 23, then sealed data.",
  a:{who:"CANDIDATE A", text:"\u201Cadd 5 each time\u201D (4 words)"},
  b:{who:"CANDIDATE B", text:"\u201Cthe numbers are 3, 8, 13, 18, 23, 28, 33, 38\u201D (12 words)"},
  correct:"a",
  why:"<strong>A wins.</strong> Both name the hidden numbers, but B is three times longer and carries no reusable rule. On a brand-new game, A still works and B knows nothing. Turings reward the rule, not the recital."},
 {q:"Duel 2: the night sky", sub:"Two explanations of dots moving overhead.",
  a:{who:"CANDIDATE A", text:"\u201Cplanets orbit the sun in ellipses\u201D (6 words)"},
  b:{who:"CANDIDATE B", text:"\u201C4,000 sky-dots move along these 4,000 listed paths\u201D (plus a 4,000-line table)"},
  correct:"a",
  why:"<strong>A wins.</strong> Six words that predict every future night. B is a phone book of the sky: enormous rent, zero generalization. This is the whole game in one duel."},
 {q:"Duel 3: the twist", sub:"Now length is not the whole story.",
  a:{who:"CANDIDATE A", text:"\u201Cadd 5 each time\u201D (4 words), predicts 3 sealed events"},
  b:{who:"CANDIDATE B", text:"\u201Cadd 5 each time, except after primes add 7\u201D (10 words), predicts 9 sealed events"},
  correct:"b",
  why:"<strong>B wins.</strong> 9 predicted minus 1.0 rent = +8.0 T, against A's 3 minus 0.4 = +2.6 T. A longer rule CAN win, but only if the extra words buy more prediction than they cost. That is what \u201Cnet\u201D means."}
];
var g2score=0, g2done=0;
var wrap=$("g2-rounds");
rounds.forEach(function(r,i){
  var card=document.createElement("div"); card.className="quiz-card";
  card.innerHTML='<p class="quiz-q">'+r.q+'</p><p class="quiz-sub">'+r.sub+'</p>'+
    '<div class="duel"><button data-pick="a"><span class="who">'+r.a.who+'</span>'+r.a.text+'</button>'+
    '<button data-pick="b"><span class="who">'+r.b.who+'</span>'+r.b.text+'</button></div>'+
    '<div class="quiz-why"></div>';
  wrap.appendChild(card);
  var btns=card.querySelectorAll("button"), why=card.querySelector(".quiz-why"), open=false;
  btns.forEach(function(b){
    b.addEventListener("click", function(){
      if(open) return; open=true; g2done++;
      var pick=b.getAttribute("data-pick"), win=pick===r.correct;
      btns.forEach(function(x){
        x.disabled=true;
        if(x.getAttribute("data-pick")===r.correct) x.classList.add("right"); else x.classList.add("wrong");
      });
      if(win){ g2score++; }
      why.innerHTML=(win?"<strong style='color:#00e599'>Correct. </strong>":"<strong style='color:#ff5d5d'>Not this one. </strong>")+r.why;
      why.classList.add("show");
      $("g2-score").textContent=g2score+" / 3";
      if(g2done===3){
        var msg=g2score===3?"Flawless. You score explanations like the instrument does.":
                g2score===2?"Strong. Round 3 tricks almost everyone the first time.":
                "The twist got you. Replay it: net means prediction minus rent, always.";
        var d=document.createElement("p"); d.className="explain"; d.style.marginTop="22px";
        d.innerHTML="<strong>"+msg+"</strong>";
        wrap.appendChild(d);
      }
    });
  });
});

/* ---------- big number count-up ---------- */
var bignum=$("bignum"), counted=false;
function fmt(n){ return n.toLocaleString("en-US",{minimumFractionDigits:3,maximumFractionDigits:3}); }
var bio=new IntersectionObserver(function(es){
  es.forEach(function(e){
    if(e.isIntersecting && !counted){
      counted=true; var target=2559679.825, start=performance.now(), dur=2200;
      (function tick(now){
        var p=Math.min(1,(now-start)/dur), v=target*(1-Math.pow(1-p,3));
        bignum.textContent=fmt(v);
        if(p<1) requestAnimationFrame(tick); else bignum.textContent=fmt(target);
      })(start);
      var gc=$("gcurve");
      gc.style.transition="stroke-dashoffset 2.2s ease .3s"; gc.style.strokeDashoffset="0";
      bio.disconnect();
    }
  });
},{threshold:.3});
bio.observe(document.querySelector(".bigstat"));
var cio=new IntersectionObserver(function(es){
  es.forEach(function(e){
    if(e.isIntersecting){
      var gc=$("gcurve");
      gc.style.transition="stroke-dashoffset 2.2s ease .3s"; gc.style.strokeDashoffset="0";
      cio.disconnect();
    }
  });
},{threshold:.4});
cio.observe($("gcurve"));
})();

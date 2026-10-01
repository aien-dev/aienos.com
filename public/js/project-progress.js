/* AIEN project telemetry. Public data only; credentials stay in CI. */
function renderProgress(data,previousDate){
    const root=document.getElementById('aien-growth-observatory');

    const days=data.series.map(d=>({...d,x:new Date(d.date+'T12:00:00Z'),C:d.langs.C||0,Rust:d.langs.Rust||0,Other:d.loc-(d.langs.C||0)-(d.langs.Rust||0)}));
    const fmt=new Intl.NumberFormat('en-US');
    const short=d3.format('~s');
    const dateFmt=d3.utcFormat('%b %-d');
    const longDate=d3.utcFormat('%B %-d, %Y');
    const esc=s=>String(s).replace(/[&<>"']/g,c=>({'&':'&amp;','<':'&lt;','>':'&gt;','"':'&quot;',"'":'&#39;'}[c]));
    const color=i=>'var(--viz-series-'+i+')';
    const tooltip=root.querySelector('#ag-tooltip');
    let selected=previousDate ? days.findIndex(d=>d.date===previousDate) : days.length-1; if(selected<0) selected=days.length-1;
    const charts=[];
    const definitions=[
      {id:'ag-code',legend:'ag-code-legend',height:300,unit:'Physical source lines',series:[{key:'loc',label:'All measured source',color:1},{key:'test',label:'Test-path source (included)',color:2}],fill:true},
      {id:'ag-revision',legend:'ag-revision-legend',height:240,unit:'Recorded revisions / PRs',series:[{key:'commits',label:'Commits',color:1},{key:'prs',label:'All PRs',color:3},{key:'merged',label:'Merged PRs',color:4}]},
      {id:'ag-ci-plot',height:240,unit:'Available workflow runs',series:[{key:'ci',label:'CI runs',color:1}],fill:true},
      {id:'ag-tests',height:240,unit:'Distinct named test files',series:[{key:'testfiles',label:'Test files',color:2}],fill:true},
      {id:'ag-languages',legend:'ag-lang-legend',height:240,unit:'Physical source lines',series:[{key:'C',label:'C',color:3},{key:'Rust',label:'Rust',color:4},{key:'Other',label:'Other',color:5}],stack:true}
    ];
    const bisect=d3.bisector(d=>d.x).left;
    function valueAt(key,time){
      const i=Math.max(1,Math.min(days.length-1,bisect(days,time)));
      const a=days[i-1],b=days[i];const t=Math.max(0,Math.min(1,(time-a.x)/(b.x-a.x)));
      return a[key]+(b[key]-a[key])*t;
    }
    function updateSelection(){
      const d=days[selected];
      root.querySelector('#ag-selected-date').textContent='· '+longDate(d.x);
      root.querySelector('#ag-lines').textContent=fmt.format(d.loc);
      root.querySelector('#ag-revisions').textContent=fmt.format(d.commits);
      root.querySelector('#ag-ci').textContent=fmt.format(d.ci);
      root.querySelector('#ag-line-context').textContent=fmt.format(d.test)+' in named test files';
      root.querySelector('#ag-rev-context').textContent=fmt.format(d.merged)+' merged PRs / '+fmt.format(d.prs)+' total';
      root.querySelector('#ag-ci-context').textContent=d.repos+' repositories in retained history';
      root.querySelector('#ag-test-summary').textContent=fmt.format(d.testfiles)+' files · '+fmt.format(d.test)+' physical lines';
      charts.forEach(c=>{if(c.selection){const line=matchMedia('(prefers-reduced-motion: reduce)').matches?c.selection:c.selection.transition().duration(160);line.attr('x1',c.x(d.x)).attr('x2',c.x(d.x));}});
    }
    function moveTooltip(event,c,time){
      const bounds=root.getBoundingClientRect();const px=event.clientX-bounds.left;const py=event.clientY-bounds.top;
      const active=c.def.series.filter(s=>s.visible!==false);
      tooltip.innerHTML='<div class="text-small">'+longDate(time)+'</div>'+active.map(s=>'<div class="ag-tooltip-row text-small"><span>'+esc(s.label)+'</span><span class="tabular-nums">'+fmt.format(Math.round(valueAt(s.key,time)))+'</span></div>').join('')+'<div class="text-small text-muted">Between daily snapshots</div>';
      tooltip.style.display='block';
      const tw=tooltip.getBoundingClientRect().width;
      tooltip.style.left=Math.max(0,Math.min(bounds.width-tw,px+16))+'px';tooltip.style.top=Math.max(0,py-88)+'px';
      c.hover.attr('x1',c.x(time)).attr('x2',c.x(time)).attr('visibility','visible');
      c.markers.attr('visibility',s=>s.visible===false?'hidden':'visible').attr('cx',c.x(time)).attr('cy',s=>{
        let v=valueAt(s.key,time);if(c.def.stack){v=0;for(const p of active){v+=valueAt(p.key,time);if(p.key===s.key)break;}}return c.y(v);
      });
    }
    function draw(c){
      const box=c.box;const W=box.clientWidth; if(!W)return;
      const H=c.def.height;const m={l:70,r:22,t:24,b:48};const a=m.l,b=W-m.r,top=m.t,bottom=H-m.b;
      const active=c.def.series.filter(s=>s.visible!==false);
      const x=d3.scaleUtc().domain(d3.extent(days,d=>d.x)).range([a+4,b-4]);
      const vals=c.def.stack?days.map(d=>active.reduce((s,p)=>s+d[p.key],0)):days.flatMap(d=>active.map(s=>d[s.key]));
      const extent=d3.extent([0,...vals]);const y=d3.scaleLinear().domain([extent[0],Math.max(1,extent[1])*1.12]).nice().range([bottom-4,top+4]);
      c.x=x;c.y=y;
      const svg=d3.select(box).selectAll('svg').data([0]).join('svg').attr('class','ag-chart').attr('viewBox',`0 0 ${W} ${H}`).attr('role','img').attr('aria-label',c.def.unit+' over retained project history');
      svg.selectAll('*').remove();svg.append('title').text(c.def.unit+' from '+longDate(days[0].x)+' to '+longDate(days.at(-1).x));
      svg.append('desc').text('Daily pinned Git snapshots. The source import on September 19 and subsequent compiler and operating system work account for much of the visible growth.');
      svg.append('rect').attr('data-chart-frame','').attr('x',a).attr('y',top).attr('width',Math.max(1,b-a)).attr('height',bottom-top).attr('fill','none').attr('stroke','var(--border)');
      const ticks=W<420?3:W<650?4:6;
      const yg=svg.append('g').attr('transform',`translate(${a},0)`).call(d3.axisLeft(y).ticks(4).tickFormat(short).tickSize(-(b-a)));
      yg.selectAll('.tick text').attr('dx',-4);yg.selectAll('.tick line').attr('opacity',.45);
      const xg=svg.append('g').attr('transform',`translate(0,${bottom})`).call(d3.axisBottom(x).ticks(ticks).tickFormat(dateFmt).tickSize(0).tickPadding(10));
      xg.selectAll('.tick').each(function(d){const tx=x(d);d3.select(this).select('text').attr('text-anchor',tx<a+25?'start':tx>b-25?'end':'middle');});
      svg.append('text').attr('class','axis-title').attr('data-axis','y').attr('x',a).attr('y',13).text(c.def.unit);
      svg.append('text').attr('class','axis-title').attr('data-axis','x').attr('x',b).attr('y',H-3).attr('text-anchor','end').text('Date · '+days[0].date.slice(0,4)+(days[0].date.slice(0,4)===days.at(-1).date.slice(0,4)?'':'–'+days.at(-1).date.slice(0,4)));
      const clipId=c.def.id+'-clip';svg.append('defs').append('clipPath').attr('id',clipId).append('rect').attr('x',a).attr('y',top).attr('width',b-a).attr('height',bottom-top);
      const marks=svg.append('g').attr('clip-path',`url(#${clipId})`);
      const previous=c.pathPositions||{};const next={};
      function paintPath(path,key,high,low){
        const shape=lo=>lo?d3.area().x((d,i)=>x(days[i].x)).y0((d,i)=>lo[i]).y1(d=>d):d3.line().x((d,i)=>x(days[i].x)).y(d=>d);
        const build=(hi,lo)=>shape(lo)(hi);const before=previous[key];next[key]={high,low};
        if(before&&c.lastWidth===W&&!matchMedia('(prefers-reduced-motion: reduce)').matches){
          const ih=d3.interpolateArray(before.high,high);const il=low?d3.interpolateArray(before.low||before.high,low):null;
          path.attr('d',build(before.high,before.low)).transition().duration(180).attrTween('d',()=>t=>build(ih(t),il?il(t):null));
        }else path.attr('d',build(high,low));
      }
      if(c.def.stack){
        const stack=d3.stack().keys(active.map(s=>s.key))(days);
        stack.forEach(s=>{const path=marks.append('path').attr('class','area').attr('fill',color(active.find(p=>p.key===s.key).color)).attr('opacity',.48);paintPath(path,s.key+'-stack',s.map(d=>y(d[1])),s.map(d=>y(d[0])));});
      }else{
        active.forEach(s=>{
          if(c.def.fill)paintPath(marks.append('path').attr('fill',color(s.color)).attr('opacity',s.key==='test'?.12:.08),s.key+'-area',days.map(d=>y(d[s.key])),days.map(()=>y(0)));
          paintPath(marks.append('path').attr('fill','none').attr('stroke',color(s.color)).attr('stroke-width',2),s.key+'-line',days.map(d=>y(d[s.key])),null);
        });
      }
      c.pathPositions=next;c.lastWidth=W;
      active.forEach(s=>{let v=days.at(-1)[s.key];if(c.def.stack){v=0;for(const p of active){v+=days.at(-1)[p.key];if(p.key===s.key)break;}}marks.append('circle').attr('cx',x(days.at(-1).x)).attr('cy',y(v)).attr('r',3).attr('fill',color(s.color));});
      c.selection=marks.append('line').attr('x1',x(days[selected].x)).attr('x2',x(days[selected].x)).attr('y1',top).attr('y2',bottom).attr('stroke','var(--foreground)').attr('stroke-width',1).attr('opacity',.35);
      c.hover=marks.append('line').attr('data-chart-hover-guide','').attr('y1',top).attr('y2',bottom).attr('stroke','var(--foreground)').attr('stroke-width',1).attr('opacity',.55).attr('visibility','hidden');
      c.markers=marks.selectAll('circle.hover').data(c.def.series).join('circle').attr('class','hover').attr('data-chart-hover-marker','').attr('r',4).attr('fill',s=>color(s.color)).attr('visibility','hidden');
      svg.append('rect').attr('data-chart-hit','').attr('data-chart-hover-overlay','cross-series').attr('class','cursor-interaction').attr('x',a).attr('y',top).attr('width',b-a).attr('height',bottom-top).attr('fill','transparent').on('pointermove',function(event){const [px]=d3.pointer(event,this);const t=x.invert(Math.max(x.range()[0],Math.min(x.range()[1],px)));moveTooltip(event,c,t);}).on('pointerleave',()=>{if(!c.pinned){tooltip.style.display='none';c.hover.attr('visibility','hidden');c.markers.attr('visibility','hidden');}}).on('click',function(event){c.pinned=!c.pinned;const [px]=d3.pointer(event,this);moveTooltip(event,c,x.invert(Math.max(x.range()[0],Math.min(x.range()[1],px))));});
      // Tick labels are measured after every responsive redraw; optional crowded ticks are removed.
      let lastRight=-Infinity; xg.selectAll('.tick text').each(function(){const r=this.getBoundingClientRect();if(r.left<lastRight+4)d3.select(this.parentNode).remove();else lastRight=r.right;});
      const yboxes=[];yg.selectAll('.tick text').each(function(){yboxes.push(this.getBoundingClientRect());});
      svg.selectAll('.axis-title').each(function(){const r=this.getBoundingClientRect();if(yboxes.some(t=>r.left<t.right+4&&r.right>t.left-4&&r.top<t.bottom+4&&r.bottom>t.top-4))d3.select(this).attr('y',9);});
    }
    definitions.forEach(def=>{
      def.series.forEach(s=>s.visible=true);const c={def,box:root.querySelector('#'+def.id),pinned:false};charts.push(c);
      if(def.legend){const legend=root.querySelector('#'+def.legend);def.series.forEach(s=>{const button=document.createElement('button');button.type='button';button.className='ag-legend cursor-interaction text-small';button.setAttribute('aria-pressed','true');button.innerHTML='<span class="ag-swatch" style="background:'+color(s.color)+'"></span>'+esc(s.label);button.addEventListener('click',()=>{if(s.visible&&def.series.filter(x=>x.visible).length===1)return;s.visible=!s.visible;button.setAttribute('aria-pressed',String(s.visible));tooltip.style.display='none';draw(c);});legend.append(button);});}
      c.observer=new ResizeObserver(()=>draw(c));c.observer.observe(c.box);draw(c);
    });
    const slider=root.querySelector('#ag-date-slider');slider.max=days.length-1;slider.value=selected;slider.addEventListener('input',()=>{selected=+slider.value;updateSelection();});
    root.querySelector('#ag-events').innerHTML=data.events.map(e=>'<details class="ag-event"><summary class="cursor-interaction"><div class="ag-event-meta"><time class="text-small text-muted" datetime="'+e.date+'">'+longDate(new Date(e.date+'T12:00:00Z'))+'</time><span class="text-small text-muted">'+esc(e.status)+'</span></div><div class="ag-event-title"><span>'+esc(e.title)+'</span><span aria-hidden="true">+</span></div></summary><p class="text-small">'+esc(e.detail)+'</p><a class="text-small" href="'+e.url+'" target="_blank" rel="noopener noreferrer">'+esc(e.repo)+(e.pr?' #'+e.pr:' · first retained commit')+'</a></details>').join('');
    const maxRepo=d3.max(data.repos,r=>(r.size.source||0)+(r.size.test||0));
    root.querySelector('#ag-repos').innerHTML=data.repos.map(r=>{const total=(r.size.source||0)+(r.size.test||0);return '<tr><td><a href="https://github.com/aien-dev/'+r.name+'" target="_blank" rel="noopener noreferrer">'+esc(r.name)+'</a><div class="text-small text-muted">'+esc(r.archived?'Archived precursor':r.group)+' · '+dateFmt(new Date(r.first+'T12:00:00Z'))+'</div></td><td class="ag-barcell text-end tabular-nums">'+fmt.format(total)+'<div class="ag-bar" aria-label="'+fmt.format(r.size.source||0)+' general source lines, '+fmt.format(r.size.test||0)+' test-path lines"><span style="width:'+((r.size.source||0)/maxRepo*100)+'%;background:'+color(1)+'"></span><span style="width:'+((r.size.test||0)/maxRepo*100)+'%;background:'+color(2)+'"></span></div></td><td class="text-end tabular-nums">'+fmt.format(r.revisions)+'</td><td class="text-end tabular-nums">'+fmt.format(r.prCount)+'</td><td class="text-end tabular-nums">'+fmt.format(r.ciCount)+'</td></tr>';}).join('');
    root.querySelector('#ag-pins').innerHTML=data.repos.map(r=>'<div class="ag-pin"><a href="https://github.com/aien-dev/'+r.name+'/tree/'+r.head+'" target="_blank" rel="noopener noreferrer">'+esc(r.name)+'</a><code>'+r.head.slice(0,10)+'</code></div>').join('');
    root.querySelector('#ag-scope').textContent=dateFmt(days[0].x)+' to '+longDate(days.at(-1).x)+' · '+data.repos.length+' public repositories';
    root.querySelector('#ag-repo-counts').textContent=data.repos.filter(r=>r.archived).length+' archived · '+data.repos.filter(r=>!r.archived).length+' unarchived';
    root.querySelector('#ag-activity').innerHTML=(data.activity||[]).map(p=>'<div class="ag-event"><div class="ag-event-meta text-small text-muted"><span>'+esc(p.repo)+'</span><span>'+esc(p.state)+'</span></div><a href="'+p.url+'" target="_blank" rel="noopener noreferrer" class="ag-event-title">'+esc(p.title)+'</a><time class="text-small text-muted">'+longDate(new Date(p.date+'T12:00:00Z'))+'</time></div>').join('')||'<p>Recent activity will appear after the next collection.</p>';
    root.querySelector('#ag-asof').textContent='Measurement cutoff: '+new Intl.DateTimeFormat('en-US',{dateStyle:'long',timeStyle:'short',timeZone:'America/Chicago'}).format(new Date(data.asof))+' America/Chicago. Default branches were read and pinned during collection.';
    root.querySelector('#ag-secondary-counts').textContent='Additional footprint: '+fmt.format(data.summary.files)+' distinct source blobs, '+fmt.format(data.summary.docfiles)+' distinct documentation blobs and '+fmt.format(data.summary.docs)+' documentation lines (excluded from source totals). Available CI conclusions at collection: '+fmt.format(data.ciConclusions.success||0)+' success, '+fmt.format(data.ciConclusions.failure||0)+' failure; remaining records were cancelled, skipped or unfinished. These are workflow outcomes, not unique correctness verdicts.';
    updateSelection();


    return {destroy(){charts.forEach(c=>c.observer.disconnect());},date(){return selected===days.length-1?null:days[selected].date;}};
}

(function startProgress(){
  const root=document.getElementById('aien-growth-observatory');if(!root)return;
  let data=JSON.parse(root.querySelector('#ag-data').textContent),view,busy=false,error=false;
  const template=root.innerHTML;
  function freshness(){
    const age=Math.max(0,Date.now()-Date.parse(data.asof));
    const stamp=new Intl.DateTimeFormat('en-US',{month:'short',day:'numeric',hour:'numeric',minute:'2-digit',timeZoneName:'short',timeZone:'America/Chicago'}).format(new Date(data.asof));
    const node=root.querySelector('#ag-freshness');node.dataset.stale=String(age>3*3600000||error);
    node.textContent=(error?'Update check unavailable · ':age>3*3600000?'Older snapshot · ':'Collected · ')+stamp;
  }
  function display(date){
    view?.destroy();root.innerHTML=template;view=renderProgress(data,date);freshness();
    root.querySelector('#ag-refresh').addEventListener('click',()=>refresh(true));
  }
  function valid(next){
    if(!next||!Array.isArray(next.series)||!next.series.length||!Array.isArray(next.repos)||!next.repos.length||!Number.isFinite(Date.parse(next.asof)))throw Error('Invalid snapshot');
    const last=next.series.at(-1);
    for(const key of ['loc','commits','ci','testfiles'])if(!Number.isSafeInteger(last[key])||last[key]<0||next.summary[key]!==last[key])throw Error('Invalid metric');
    const urls=[...(next.events||[]),...(next.activity||[])].map(p=>p.url);
    for(const url of urls)if(!/^https:\/\/github\.com\/aien-dev\/[A-Za-z0-9_.\/-]+$/.test(url))throw Error('Invalid source URL');
    for(const repo of next.repos)if(!/^[.A-Za-z0-9_-]+$/.test(repo.name)||!/^[a-f0-9]{40}$/.test(repo.head))throw Error('Invalid source pin');
    return next;
  }
  async function refresh(manual=false){
    if(busy)return;busy=true;const button=root.querySelector('#ag-refresh');button.disabled=true;button.textContent='Checking…';
    try{
      const response=await fetch('/data/project-progress.json?check='+Date.now(),{cache:'no-store',signal:AbortSignal.timeout(20000)});
      if(!response.ok)throw Error('Unavailable');const next=valid(await response.json());
      if(Date.parse(next.asof)>Date.parse(data.asof)){const date=view.date();data=next;display(date);}
      error=false;freshness();if(manual)root.querySelector('#ag-refresh').textContent='Up to date';
    }catch(e){error=true;freshness();}
    finally{busy=false;root.querySelector('#ag-refresh').disabled=false;if(!manual||error)root.querySelector('#ag-refresh').textContent='Check for updates';}
  }
  valid(data);display();refresh();setInterval(()=>{freshness();if(!document.hidden)refresh();},5*60000);
  document.addEventListener('visibilitychange',()=>{if(!document.hidden)refresh();});
})();

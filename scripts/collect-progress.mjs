// Public GitHub telemetry for the website, separate from the trusted runtime.
// No credentials, executable remote content, or private repositories are published.
import fs from 'node:fs';
import path from 'node:path';
import {execFileSync, spawn} from 'node:child_process';
import {fileURLToPath} from 'node:url';

export const OWNER='aien-dev';
export const TIMEZONE='America/Chicago';
export const extensions=new Map(Object.entries({'.rs':'Rust','.c':'C','.h':'C','.inc':'C','.mojo':'Mojo','.py':'Python','.sh':'Shell','.bash':'Shell','.js':'Web','.mjs':'Web','.cjs':'Web','.jsx':'Web','.ts':'Web','.tsx':'Web','.html':'Web','.css':'Web','.S':'Assembly','.s':'Assembly','.asm':'Assembly','.cu':'CUDA','.cuh':'CUDA','.cc':'C++','.cpp':'C++','.hpp':'C++','.zig':'Zig'}));
const excluded=/(^|\/)(vendor|vendored|third_party|third-party|node_modules|target|dist|build|generated|evidence|receipts|artifacts|data|results|fixtures)(\/|$)/i;
const testPath=/(^|\/)(tests?|test_[^/]*|[^/]*_tests?)(\/|\.|$)|\.(test|spec)\./i;
const seedNames=['aien-sovereign-core','spark-hive','aien-harness','spark-dream','cortex-rs','spark-supervisor','spark-debugger','rad-id-sync','spark-adapters','spark-crumbs','crumb-spec','spark-rsi','open-humanity','harvester','aegis-runtime','.github','spark-inquisitor','aien-dev','benchmarks','drakestapleton.com','aienos.com','aien-protocols','aien-local-stack','aien-architecture','aien-edge','aienos','atlas','physics','omega'];
const coreNames=new Set(['aien-sovereign-core','aienos','omega','physics','forge','atlas','aien-protocols','aien-architecture','benchmarks']);
const dayFormatter=new Intl.DateTimeFormat('en-CA',{timeZone:TIMEZONE,year:'numeric',month:'2-digit',day:'2-digit'});
export const day=timestamp=>dayFormatter.format(new Date(timestamp));
export function classify(file,mode='100644'){
  if(mode==='120000'||excluded.test(file))return null;
  const ext=path.posix.extname(file),lang=extensions.get(ext);
  if(lang)return {kind:testPath.test(file)?'test':'source',lang};
  if(['.md','.rst','.txt'].includes(ext.toLowerCase())&&!/LICENSE|NOTICE|COPYING/.test(file))return {kind:'docs',lang:'Docs'};
  return null;
}
export function physicalLines(bytes){
  if(bytes.includes(0))return 0;
  let lines=0;for(const byte of bytes)if(byte===10)lines++;
  return lines+(bytes.length&&bytes.at(-1)!==10?1:0);
}
export function countEntries(entries,blobLines,seen=new Set()){
  const counts={source:0,test:0,docs:0,files:0,testfiles:0,docfiles:0,langs:{}};
  for(const e of entries){
    if(seen.has(e.oid))continue;seen.add(e.oid);
    const lines=blobLines.get(e.oid);if(!Number.isSafeInteger(lines)||lines<0)throw Error('Invalid blob line count');
    counts[e.kind]+=lines;
    if(e.kind==='docs')counts.docfiles++;
    else{counts.files++;if(e.kind==='test')counts.testfiles++;counts.langs[e.lang]=(counts.langs[e.lang]||0)+lines;}
  }
  return counts;
}
export function validateSnapshot(data){
  const required=['source','test','docs','loc','files','testfiles','docfiles','repos','commits','prs','merged','ci'];
  if(!data.series?.length||!data.repos?.length||!Number.isFinite(Date.parse(data.asof)))throw Error('Empty or invalid snapshot');
  let prev='';
  for(const row of data.series){
    if(!/^\d{4}-\d{2}-\d{2}$/.test(row.date)||row.date<=prev)throw Error('Unordered snapshot dates');prev=row.date;
    for(const key of required)if(!Number.isSafeInteger(row[key])||row[key]<0)throw Error('Missing or invalid metric: '+key);
    if(row.loc!==row.source+row.test||row.merged>row.prs||row.testfiles>row.files)throw Error('Snapshot totals do not reconcile');
    if(Object.values(row.langs).reduce((a,b)=>a+b,0)!==row.loc)throw Error('Language total mismatch');
  }
  const latest=data.series.at(-1);
  for(const key of required)if(data.summary[key]!==latest[key])throw Error('Summary differs from the latest snapshot: '+key);
  if(new Set(data.repos.map(r=>r.name)).size!==data.repos.length)throw Error('Duplicate repositories');
  if(data.summary.prs!==data.repos.reduce((s,r)=>s+r.prCount,0))throw Error('PR totals do not reconcile');
  if(data.summary.ci!==data.repos.reduce((s,r)=>s+r.ciCount,0))throw Error('CI totals do not reconcile');
  if(data.summary.ci!==Object.values(data.ciConclusions).reduce((s,c)=>s+c,0))throw Error('CI conclusions do not reconcile');
  for(const repo of data.repos)if(!/^[a-f0-9]{40}$/.test(repo.head))throw Error('Missing pinned head');
  return data;
}

const pause=ms=>new Promise(resolve=>setTimeout(resolve,ms));
export async function githubGet(endpoint,fetcher=fetch){
  const headers={Accept:'application/vnd.github+json','X-GitHub-Api-Version':'2022-11-28','User-Agent':'AIEN-Project-Progress'};
  if(process.env.GITHUB_TOKEN)headers.Authorization='Bearer '+process.env.GITHUB_TOKEN;
  for(let attempt=0;attempt<4;attempt++){
    const response=await fetcher('https://api.github.com'+endpoint,{headers,signal:AbortSignal.timeout(60000)});
    if(response.ok)return response.json();
    if((response.status===429||response.status>=500)&&attempt<3){await pause(2000*(attempt+1));continue;}
    throw Error(`GitHub refused ${endpoint}: HTTP ${response.status}`);
  }
}
export async function paginate(endpoint,key=null,get=githubGet){
  const records=new Map();
  for(let page=1;page<=10000;page++){
    const response=await get(endpoint+(endpoint.includes('?')?'&':'?')+'per_page=100&page='+page);
    const rows=key?response[key]:response;
    if(!Array.isArray(rows))throw Error('Invalid GitHub collection: '+endpoint);
    for(const row of rows)records.set(row.id??row.url??row.number,row);
    if(rows.length<100)return [...records.values()];
  }
  throw Error('Pagination exceeded the supported limit');
}
const git=(dir,args,binary=false,input)=>execFileSync('git',['--git-dir='+dir,...args],{encoding:binary?null:'utf8',input,maxBuffer:256*1024*1024});
const command=(exe,args)=>new Promise((resolve,reject)=>{
  const process=spawn(exe,args,{stdio:['ignore','ignore','pipe']});let error='';
  process.stderr.on('data',chunk=>error=(error+chunk).slice(-3000));
  process.on('error',reject);process.on('close',code=>code===0?resolve():reject(Error(exe+' failed: '+error)));
});
async function limited(items,limit,fn){
  const results=new Array(items.length);let i=0;
  await Promise.all(Array.from({length:Math.min(limit,items.length)},async()=>{while(i<items.length){const index=i++;results[index]=await fn(items[index]);}}));
  return results;
}
function commitsAt(dir,head,firstParent=false){
  const args=['log',...(firstParent?['--first-parent']:[]),'--format=%H%x09%ct',head];
  return git(dir,args).trim().split('\n').filter(Boolean).map(line=>{const [sha,ts]=line.split('\t');return {sha,ts:+ts,date:day(+ts*1000)};});
}
function readEntries(repo,head,cache,trees){
  const key=repo.name+':'+head;if(trees.has(key))return trees.get(key);
  const listing=git(repo.dir,['ls-tree','-r','-z',head],true).toString('utf8');
  const entries=[];
  for(const row of listing.split('\0')){
    if(!row)continue;const tab=row.indexOf('\t'),[mode,type,oid]=row.slice(0,tab).split(' '),file=row.slice(tab+1);
    const spec=classify(file,mode);if(type==='blob'&&spec)entries.push({oid,...spec});
  }
  const missing=[...new Set(entries.map(e=>e.oid).filter(oid=>!cache.has(oid)))];
  if(missing.length){
    const bytes=git(repo.dir,['cat-file','--batch'],true,missing.join('\n')+'\n');let cursor=0;
    for(const oid of missing){
      const end=bytes.indexOf(10,cursor);if(end<0)throw Error('Truncated Git blob batch');
      const [actual,type,length]=bytes.subarray(cursor,end).toString().split(' ');const size=+length;
      if(actual!==oid||type!=='blob'||!Number.isSafeInteger(size)||size<0||end+1+size>=bytes.length)throw Error('Malformed Git blob batch');
      const body=bytes.subarray(end+1,end+1+size);cache.set(oid,physicalLines(body));cursor=end+size+2;
    }
  }
  trees.set(key,entries);return entries;
}

export async function collect({metadata=null,cacheDir='.progress-cache',output='public/data/project-progress.json',asof=null}={}){
  let cutoff=asof?Date.parse(asof):Infinity;if(Number.isNaN(cutoff))throw Error('Invalid measurement cutoff');
  const existing=JSON.parse(fs.readFileSync(output,'utf8'));
  let repositoryInfo,offline;
  if(metadata){offline=JSON.parse(fs.readFileSync(metadata,'utf8'));repositoryInfo=offline.repositories;}
  else repositoryInfo=await paginate('/users/'+OWNER+'/repos?type=owner&sort=full_name');
  repositoryInfo=repositoryInfo.filter(r=>!r.private&&!r.fork&&(seedNames.includes(r.name)||/^(aien(?:-|os)|aegis-|spark-|omega(?:-|$)|forge(?:-|$)|physics(?:-|$)|atlas(?:-|$)|cortex-|crumb-)/.test(r.name)));
  for(const name of seedNames)if(!repositoryInfo.some(r=>r.name===name))throw Error('Expected public project repository missing: '+name);
  const repos=await limited(repositoryInfo,4,async info=>{
    if(!/^[.a-zA-Z0-9_-]+$/.test(info.name)||!info.default_branch)throw Error('Invalid repository identity');
    const dir=path.resolve(cacheDir,info.name+'.git');fs.mkdirSync(path.dirname(dir),{recursive:true});
    if(!metadata){
      if(fs.existsSync(dir))await command('git',['--git-dir='+dir,'fetch','--prune','origin']);
      else await command('git',['clone','--mirror',`https://github.com/${OWNER}/${info.name}.git`,dir]);
    }
    const head=metadata&&info.head?info.head:git(dir,['rev-parse','refs/heads/'+info.default_branch]).trim();
    const commits=commitsAt(dir,head),fp=commitsAt(dir,head,true);
    // Normal Git history cannot place the candidate's parent after its measurement cutoff.
    if(commits.some(c=>c.ts*1000>cutoff))throw Error('Pinned repository contains a future-dated commit: '+info.name);
    let prs,runs;
    if(metadata){prs=offline.metadata.find(r=>r.name===info.name).prs;runs=offline.ci.find(r=>r.name===info.name).rows;}
    else{
      prs=await paginate(`/repos/${OWNER}/${info.name}/pulls?state=all`);
      runs=await paginate(`/repos/${OWNER}/${info.name}/actions/runs`,'workflow_runs');
    }
    prs=prs.filter(p=>Date.parse(p.created_at??p.created)<=cutoff).map(p=>({number:p.number,title:p.title,created:p.created_at??p.created,merged:p.merged_at??p.merged,closed:p.closed_at??p.closed,state:p.state,draft:p.draft,url:p.html_url??p.url,createdDay:day(p.created_at??p.created),mergedDay:(p.merged_at??p.merged)?day(p.merged_at??p.merged):null}));
    const uniqueRuns=new Map(runs.map(r=>[r.html_url??r.url,r]));
    runs=[...uniqueRuns.values()].filter(r=>Date.parse(r.created_at??r.created)<=cutoff).map(r=>({created:r.created_at??r.created,conclusion:r.conclusion,status:r.status,url:r.html_url??r.url,createdDay:day(r.created_at??r.created)}));
    return {name:info.name,dir,head,branch:info.default_branch,commits,fp,prs,runs,archived:!!info.archived,group:info.archived?'Archived':coreNames.has(info.name)?'Core':['aienos.com','drakestapleton.com','.github','aien-dev'].includes(info.name)?'Sites':'Experimental',first:commits.reduce((min,c)=>c.date<min?c.date:min,'9999-12-31'),last:commits.reduce((max,c)=>c.date>max?c.date:max,'0000-01-01')};
  });
  if(!asof){asof=new Date().toISOString();cutoff=Date.parse(asof);}
  if(repos.some(r=>r.commits.some(c=>c.ts*1000>cutoff)))throw Error('Pinned history contains a future-dated commit');
  const first=repos.reduce((min,r)=>r.first<min?r.first:min,'9999-12-31'),last=day(cutoff);
  const dates=[];for(let time=Date.parse(first+'T12:00:00Z');time<=Date.parse(last+'T12:00:00Z');time+=86400000)dates.push(new Date(time).toISOString().slice(0,10));
  if(dates.length>10000)throw Error('Unexpected historical range');
  const blobs=new Map(),trees=new Map(),commits=new Map();for(const r of repos)for(const c of r.commits)commits.set(c.sha,c);
  const allPrs=repos.flatMap(r=>r.prs),allRuns=repos.flatMap(r=>r.runs),allCommits=[...commits.values()];
  const series=dates.map(date=>{
    const seen=new Set();let total={source:0,test:0,docs:0,files:0,testfiles:0,docfiles:0,langs:{}};let number=0;
    for(const r of repos){
      const candidate=r.fp.find(c=>c.date<=date);if(!candidate)continue;number++;
      const counts=countEntries(readEntries(r,candidate.sha,blobs,trees),blobs,seen);
      for(const key of ['source','test','docs','files','testfiles','docfiles'])total[key]+=counts[key];
      for(const [lang,count] of Object.entries(counts.langs))total.langs[lang]=(total.langs[lang]||0)+count;
    }
    const ps=allPrs.filter(p=>p.createdDay<=date),rs=allRuns.filter(r=>r.createdDay<=date);
    return {date,...total,loc:total.source+total.test,repos:number,commits:allCommits.filter(c=>c.date<=date).length,daily:allCommits.filter(c=>c.date===date).length,prs:ps.length,merged:ps.filter(p=>p.merged&&Date.parse(p.merged)<=cutoff&&p.mergedDay<=date).length,ci:rs.length,ciDaily:rs.filter(r=>r.createdDay===date).length};
  });
  const ciConclusions={};for(const r of repos)for(const run of r.runs){const key=run.conclusion||run.status;ciConclusions[key]=(ciConclusions[key]||0)+1;}
  const publicRepos=repos.map(r=>{
    const counts=countEntries(readEntries(r,r.head,blobs,trees),blobs);
    const {langs,...size}=counts;
    return {name:r.name,head:r.head,branch:r.branch,first:r.first,last:r.last,archived:r.archived,group:r.group,size,langs,revisions:r.commits.length,prCount:r.prs.length,mergedCount:r.prs.filter(p=>p.merged&&Date.parse(p.merged)<=cutoff).length,ciCount:r.runs.length};
  }).sort((a,b)=>b.size.source+b.size.test-a.size.source-a.size.test);
  const {date,langs,...summary}=series.at(-1);summary.active=repos.filter(r=>!r.archived).length;summary.archived=repos.length-summary.active;
  const activity=repos.flatMap(r=>r.prs.filter(p=>p.state==='open'||p.merged).map(p=>({repo:r.name,title:p.title,number:p.number,url:p.url,state:p.merged?'Merged':p.draft?'Draft':'Open',date:day(p.merged||p.created),timestamp:p.merged||p.created}))).sort((a,b)=>Date.parse(b.timestamp)-Date.parse(a.timestamp)).slice(0,12);
  const result={schemaVersion:2,asof,timezone:TIMEZONE,first,last,series,summary,ciConclusions,repos:publicRepos,events:existing.events||[],activity,sourcePolicy:existing.sourcePolicy,commitPolicy:existing.commitPolicy};
  validateSnapshot(result);
  fs.mkdirSync(path.dirname(output),{recursive:true});const tmp=output+'.tmp';fs.writeFileSync(tmp,JSON.stringify(result));fs.renameSync(tmp,output);
  console.log(`Collected ${repos.length} public repositories: ${summary.loc} source lines, ${summary.commits} revisions, ${summary.prs} PRs, ${summary.ci} CI runs. Cutoff ${asof}`);
  return result;
}
if(process.argv[1]&&path.resolve(process.argv[1])===fileURLToPath(import.meta.url)){
  const args=process.argv.slice(2);const option=name=>{const i=args.indexOf(name);return i<0?undefined:args[i+1];};
  collect({metadata:option('--metadata'),cacheDir:option('--cache')||'.progress-cache',output:option('--output')||'public/data/project-progress.json',asof:option('--asof')||null}).catch(error=>{console.error('Progress refresh failed:',error.message);process.exitCode=1;});
}

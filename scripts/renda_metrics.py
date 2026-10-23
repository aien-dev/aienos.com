#!/usr/bin/env python3
"""Publish allowlisted Renda counters, never raw logs or local paths."""
import argparse,datetime,json,re
from pathlib import Path
PASS=re.compile(r'^(\d{4}-\d\d-\d\dT\d\d:\d\d:\d\dZ) (renda[12]) pass done in (\d+) s: merged (\d+), rebased (\d+), reruns (\d+), operator runs (\d+)$')
def iso(ts):return datetime.datetime.fromtimestamp(ts,datetime.timezone.utc).strftime('%Y-%m-%dT%H:%M:%SZ')
def collect(text,name,now,window=3600):
 records=[];cycles=[];previous=None
 for line in text.splitlines():
  if f'{name} start lanes=' in line:previous=None
  m=PASS.fullmatch(line)
  if not m or m[2]!=name:continue
  t=datetime.datetime.strptime(m[1],'%Y-%m-%dT%H:%M:%SZ').replace(tzinfo=datetime.timezone.utc).timestamp()
  if t>now:continue
  record=(t,*map(int,m.groups()[2:]))
  if previous is not None and t>previous and now-window<=previous:
   cycles.append((t,60/(t-previous)))
  previous=t
  if t>=now-window:records.append(record)
 rates=[v for _,v in cycles if _>=now-window]
 last=records[-1] if records else None
 current=round(rates[-1],4) if rates and now-cycles[-1][0]<=300 else None
 return dict(name=name,last_pass_at=iso(last[0]) if last else None,current_cpm=current,min_cpm=round(min(rates),4) if rates else None,max_cpm=round(max(rates),4) if rates else None,last_pass_seconds=last[1] if last else None,samples=len(rates),merges=sum(r[2] for r in records),rebases=sum(r[3] for r in records),reruns=sum(r[4] for r in records),operators=sum(r[5] for r in records),active_pass_seconds=sum(r[1] for r in records),window_passes=len(records))
def main():
 p=argparse.ArgumentParser();p.add_argument('--logs',type=Path,default=Path.home()/'handoffs/renda-local');p.add_argument('--output',type=Path);a=p.parse_args();now=datetime.datetime.now(datetime.timezone.utc).timestamp()
 workers=[]
 for name in ('renda1','renda2'):
  path=a.logs/f'harness-{name}.log'
  workers.append(collect(path.read_text() if path.exists() else '',name,now))
 data=dict(schema='renda-metrics/1',generated_at=iso(now),window_seconds=3600,workers=workers)
 text=json.dumps(data,indent=2)+'\n'
 if a.output:
  a.output.parent.mkdir(parents=True,exist_ok=True);tmp=a.output.with_suffix('.tmp');tmp.write_text(text);tmp.replace(a.output)
 else:print(text,end='')
if __name__=='__main__':main()

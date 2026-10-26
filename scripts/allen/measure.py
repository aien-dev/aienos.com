import json, sys, time, http.client, ssl
def run(host, port, tls, q, path="/allen/api/chat", hdr=None):
    c = (http.client.HTTPSConnection(host, port, timeout=200, context=ssl.create_default_context()) if tls
         else http.client.HTTPConnection(host, port, timeout=200))
    body = json.dumps({"messages": [{"role": "user", "content": q}]})
    t0 = time.monotonic(); c.request("POST", path, body, dict({"Content-Type": "application/json"}, **(hdr or {}))); r = c.getresponse()
    ev=None; start=None; first=None; last=None; n=0; done=None
    for line in r:
        line=line.rstrip(b"\n")
        if line.startswith(b"event: "): ev=line[7:].decode(); continue
        if line.startswith(b"data: "):
            d=json.loads(line[6:]); t=time.monotonic()
            if ev=="start": start=t
            elif ev is None and d:
                n+=1; last=t
                if first is None: first=t
            elif ev=="done": done=d
            elif ev=="error": done={"error":d}
            ev=None
    tps = (n-1)/(last-first) if n>1 and last>first else None
    return {"status": r.status, "queue_ms": int(((start or t0)-t0)*1000), "ttft_ms": int(((first or t0)-t0)*1000),
            "tokens": n, "tok_s": round(tps,2) if tps else None, "server": done}
for arg in sys.argv[1:]:
    where, q = arg.split("|",1)
    if where=="hub": res=run("www.aienos.com",443,True,q)
    else: res=run("100.64.0.2",18110,False,q)
    print(where, json.dumps(res)); sys.stdout.flush()

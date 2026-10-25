use crate::{privacy::{label, public_text}, source::{self, Cursor}};
use chrono::{DateTime, SecondsFormat, Utc};
use hmac::{Hmac, Mac};
use serde::Serialize;
use serde_json::Value;
use sha2::Sha256;
use std::{collections::{BTreeMap, HashMap, HashSet, VecDeque}, path::{Path, PathBuf}};

pub fn timestamp(ms: i64) -> String { DateTime::<Utc>::from_timestamp_millis(ms).unwrap_or(DateTime::<Utc>::UNIX_EPOCH).to_rfc3339_opts(SecondsFormat::Millis, true) }
pub fn milliseconds(value: &Value) -> Option<i64> { value.as_i64().or_else(|| value.as_str().and_then(|s| DateTime::parse_from_rfc3339(s).ok()).map(|t| t.timestamp_millis())) }
fn text<'a>(v: &'a Value, key: &str) -> &'a str { v[key].as_str().unwrap_or("") }
fn safe_date(v: &Value) -> Option<String> { milliseconds(v).map(timestamp) }

#[derive(Clone, Serialize)]
pub struct Agent {
    pub id: String, pub parent_id: Option<String>, pub name: String, pub runtime: String,
    pub provider: String, pub model: String, pub role: String, pub task: String, pub status: String,
    pub activity: String, pub activity_state: String, pub started_at: Option<String>, pub updated_at: Option<String>, pub observed_at: String,
}
#[derive(Clone, Serialize)]
pub struct Interaction { pub id: String, pub agent_id: String, pub role: String, pub text: String, pub at: String, pub runtime: String, pub provider: String, pub model: String }
#[derive(Serialize)]
pub struct Coverage { label: &'static str, roots: usize, agents: usize, complete: bool, notice: String }
#[derive(Serialize)]
pub struct Feed { schema_version: u8, generated_at: String, sequence: String, poll_ms: u32, stale_after_ms: u32, coverage: Coverage, agents: Vec<Agent>, interactions: Vec<Interaction> }
struct Node { public: Agent, root: String, seen: i64, terminal: Option<i64>, tools: BTreeMap<String,(String,String)>, placeholder: bool, unavailable: Option<(String,String)>, has_summary: bool }
pub struct Collector {
    salt: Vec<u8>, since: i64, sequence: u64, nodes: BTreeMap<String,Node>, cursors: HashMap<PathBuf,Cursor>,
    interactions: VecDeque<Interaction>, seen_events: HashSet<String>, known_hosts: HashMap<u32,String>, task_summaries: HashMap<String,String>,
}
impl Collector {
    pub fn new(salt: Vec<u8>, since: i64) -> Self { Self { salt, since, sequence: 0, nodes: BTreeMap::new(), cursors: HashMap::new(), interactions: VecDeque::new(), seen_events: HashSet::new(), known_hosts: HashMap::new(), task_summaries: HashMap::new() } }
    fn id(&self, private: &str) -> String {
        let mut mac = Hmac::<Sha256>::new_from_slice(&self.salt).expect("HMAC accepts any key length"); mac.update(private.as_bytes());
        let digest = mac.finalize().into_bytes(); let mut out = String::from("h");
        for byte in &digest[..16] { use std::fmt::Write; let _ = write!(out, "{byte:02x}"); } out
    }
    fn ensure(&mut self, key: &str, root: &str, parent: Option<&str>, name: &str, now: i64) {
        if self.nodes.contains_key(key) { return; }
        let id = self.id(key); let parent_id = parent.map(|p| self.id(p));
        let summary = self.task_summaries.get(key);
        self.nodes.insert(key.to_owned(), Node { public: Agent { id, parent_id, name: label(name, "Agent"), runtime: "OMP".into(), provider: "unknown".into(), model: "unknown".into(), role: if parent.is_some() { "worker" } else { "root" }.into(), task: summary.cloned().unwrap_or_else(|| label(name, "Task label withheld")), status: "unknown".into(), activity: "No action observed".into(), activity_state: "unknown".into(), started_at: None, updated_at: None, observed_at: timestamp(now) }, root: root.to_owned(), seen: now, terminal: None, tools: BTreeMap::new(), placeholder: true, unavailable: None, has_summary: summary.is_some() });
    }
    fn model(node: &mut Node, raw: &str, provider: Option<&str>) {
        let (p,m) = if let Some(p) = provider { (p,raw) } else { raw.split_once('/').unwrap_or(("unknown",raw)) };
        node.public.provider = label(p, "withheld"); node.public.model = label(m, "withheld");
    }
    fn child_key(root: &str, parent: &str, name: &str) -> String {
        if name.contains('.') || parent == root { format!("{root}/{name}") } else { format!("{parent}.{name}") }
    }
    fn assignment_summaries(&mut self, args: &Value, root: &str, parent: &str) {
        let Some(tasks) = args["tasks"].as_array() else { return; };
        for task in tasks {
            let name = text(task,"name");
            if name.trim().is_empty() { continue; }
            // This is the sole public assignment-summary field. Never use task,
            // context or any other tool argument as a display fallback.
            let Some(summary) = public_text(text(task,"solutionSpace"),180) else { continue; };
            let key = Self::child_key(root,parent,name);
            if let Some(node) = self.nodes.get_mut(&key) {
                node.public.task = summary.clone(); node.has_summary = true;
            }
            // Do not create a live worker merely because a spawn was requested.
            self.task_summaries.insert(key,summary);
        }
    }
    fn terminal(&mut self, key: &str, status: &str, now: i64) {
        if let Some(node) = self.nodes.get_mut(key) {
            node.public.status = status.into(); node.public.activity_state = "finished".into(); node.tools.clear(); node.terminal = Some(now);
        }
    }
    fn event(&mut self, key: &str, value: &Value, now: i64) {
        let Some(root) = self.nodes.get(key).map(|n| n.root.clone()) else { return; };
        let kind = text(value,"type"); let custom = text(value,"customType");
        let at = safe_date(&value["timestamp"]);
        let event_time = milliseconds(&value["timestamp"]).unwrap_or(now);
        if let Some(node) = self.nodes.get_mut(key) {
            node.placeholder = false;
            if kind == "title" && !node.has_summary && !text(value,"title").trim().is_empty() { node.public.task = label(text(value,"title"), "Task label withheld"); }
            if kind == "session" { node.public.started_at = at.clone(); }
            if kind == "session_init" {
                node.public.role = label(text(value,"agent"), "worker");
                Self::model(node, text(value,"resolvedModel"), None);
                node.public.status = "working".into();
            }
            if kind == "model_usage" {
                let data = value.get("data").unwrap_or(value);
                if let Some(model) = data["model"].as_str() { Self::model(node, model, data["provider"].as_str()); }
            }
            if custom == "tool_execution_start" {
                let d = &value["data"]; let tool = label(text(d,"toolName"), "tool");
                let action = public_text(text(d,"intent"), 180).map(|i| format!("{tool}: {i}")).unwrap_or(tool);
                let date = safe_date(&d["startedAt"]).or(at.clone());
                node.tools.insert(text(d,"toolCallId").to_owned(), (action.clone(), date.clone().unwrap_or_default()));
                node.public.activity = action; node.public.activity_state = "running".into(); node.public.status = "working".into(); node.public.updated_at = date; node.terminal = None;
            }
        }
        if custom == "tool_execution_start" && value["data"]["toolName"] == "task" {
            self.assignment_summaries(&value["data"]["args"],&root,key);
        }
        if custom == "session_exit" {
            let d = &value["data"];
            let pending = d["pendingToolCalls"].as_array().is_some_and(|a| !a.is_empty());
            let state = if text(d,"kind") == "normal" && !pending { "completed" } else if matches!(text(d,"kind"), "fatal" | "signal" | "process_exit") { "failed" } else { "unknown" };
            self.terminal(key, state, event_time);
        }
        let m = value.get("message").unwrap_or(value);
        let role = text(m,"role");
        if role == "assistant" {
            if let Some(parts) = m["content"].as_array() {
                for part in parts {
                    if part["type"] == "toolCall" && part["name"] == "task" {
                        self.assignment_summaries(&part["arguments"],&root,key);
                    }
                }
            }
        }
        if role == "toolResult" {
            if let Some(node) = self.nodes.get_mut(key) {
                let finished = node.tools.remove(text(m,"toolCallId"));
                node.public.updated_at = safe_date(&m["timestamp"]).or(at.clone());
                if let Some((_,(action,date))) = node.tools.iter().max_by_key(|(_,(_,date))| date) {
                    node.public.activity = action.clone(); node.public.activity_state = "running".into();
                    if !date.is_empty() { node.public.updated_at = Some(date.clone()); }
                } else {
                    node.public.activity = finished.map(|(a,_)| a).unwrap_or_else(|| label(text(m,"toolName"), "tool"));
                    if m["isError"] == true { node.public.activity.push_str(" (reported error)"); }
                    node.public.activity_state = "finished".into();
                }
            }
            if text(m,"toolName") == "task" {
                if let Some(progress) = m["details"]["progress"].as_array() {
                    for child in progress {
                        let name = text(child,"id"); if name.is_empty() { continue; }
                        let ck = Self::child_key(&root,key,name); self.ensure(&ck,&root,Some(key),name,now);
                        let n = self.nodes.get_mut(&ck).unwrap(); n.public.role = label(text(child,"agent"), "worker");
                        match text(child,"status") { "pending" | "running" => { if n.placeholder { n.public.status = "waiting".into(); } }, "completed" => self.terminal(&ck,"completed",event_time), "failed" | "error" | "cancelled" => self.terminal(&ck,"failed",event_time), _ => {} }
                    }
                }
            }
        }
        if text(m,"customType") == "async-result" || custom == "async-result" {
            let empty = Value::Null;
            let details = m.get("details").or_else(|| value.get("data")).unwrap_or(&empty);
            if let Some(jobs) = details["jobs"].as_array() {
                for job in jobs {
                    if text(job,"type") != "task" { continue; }
                    let name = text(job,"agentId"); if name.is_empty() { continue; }
                    let ck = Self::child_key(&root,key,name); self.ensure(&ck,&root,Some(key),name,now);
                    match text(job,"status") { "completed" => self.terminal(&ck,"completed",event_time), "failed" | "error" | "cancelled" => self.terminal(&ck,"failed",event_time), _ => {} }
                }
            }
        }
        // Attribution is a strict provenance boundary, not a heuristic on visible text.
        let direct_user = role == "user" && key == root && m["attribution"] == "user";
        if role != "assistant" && !direct_user { return; }
        let Some(time) = milliseconds(&m["timestamp"]).or_else(|| milliseconds(&value["timestamp"])) else { return; };
        if time < self.since { return; }
        let Some(parts) = m["content"].as_array() else { return; };
        let mut joined = String::new();
        for part in parts { if part["type"] == "text" { if let Some(t) = part["text"].as_str() { if !joined.is_empty() { joined.push('\n'); } joined.push_str(t); } } }
        let Some(public) = public_text(&joined,4000) else { return; };
        let eid = self.id(&format!("interaction:{key}:{}:{time}:{joined}", text(value,"id")));
        if !self.seen_events.insert(eid.clone()) { return; }
        self.interactions.push_back(Interaction { id: eid, agent_id: self.id(key), role: if direct_user { "drake" } else { "assistant" }.into(), text: public, at: timestamp(time), runtime: "OMP".into(), provider: if direct_user { "unknown".into() } else { label(text(m,"provider"),"unknown") }, model: if direct_user { "unknown".into() } else { label(text(m,"model"),"unknown") } });
        self.interactions.make_contiguous().sort_by(|a,b| a.at.cmp(&b.at).then(a.id.cmp(&b.id)));
        while self.interactions.len() > 100 { if let Some(old) = self.interactions.pop_front() { self.seen_events.remove(&old.id); } }
    }
    fn read_node(&mut self, key: &str, path: &Path, now: i64) -> bool {
        let terminal = self.nodes.get(key).and_then(|n| n.terminal.map(|t| (t,n.public.status.clone())));
        let cursor = self.cursors.entry(path.to_owned()).or_default();
        let result = cursor.read(path); let damaged = cursor.damaged;
        match result {
            Ok(values) => {
                if !damaged {
                    if let Some(node) = self.nodes.get_mut(key) { if let Some((status,activity)) = node.unavailable.take() { node.public.status = status; node.public.activity_state = activity; } }
                }
                let latest = values.iter().filter(|v| v["customType"] == "tool_execution_start" || v["customType"] == "session_exit").filter_map(|v| milliseconds(&v["timestamp"])).max().unwrap_or(0);
                for value in values { self.event(key,&value,now); }
                if let Some((time,status)) = terminal { if latest <= time { self.terminal(key,&status,time); } }
                !damaged
            },
            Err(_) => false,
        }
    }
    pub fn poll(&mut self, sessions: &Path, fixture: Option<&Path>, now: i64) -> Feed {
        self.sequence += 1;
        let mut faults = 0usize; let mut roots = HashSet::new(); let mut host_pids = HashMap::new(); let mut authoritative = HashMap::new();
        let mut observed = HashSet::new();
        let result = source::hosts(fixture);
        let source_ok = result.is_ok();
        if let Ok(source) = result {
            for host in source["hosts"].as_array().unwrap() {
                let key = text(host,"sessionId"); if key.is_empty() { faults += 1; continue; }
                roots.insert(key.to_owned()); self.ensure(key,key,None,"Coding session",now);
                observed.insert(key.to_owned());
                if let Some(pid) = host["pid"].as_u64() { host_pids.insert(pid as u32,key.to_owned()); }
                let n = self.nodes.get_mut(key).unwrap(); n.seen = now; n.terminal = None;
                n.public.task = label(text(host,"sessionName"), "Task label withheld"); n.public.name = n.public.task.clone();
                n.public.started_at = safe_date(&host["startedAt"]);
                Self::model(n,text(&host["model"],"id"),host["model"]["provider"].as_str());
                n.public.status = if host["inputRequired"] == true { "waiting" } else { match host["busy"].as_bool() { Some(true) => "working", Some(false) => "idle", None => "unknown" } }.into();
                authoritative.insert(key.to_owned(), (n.public.status.clone(), n.public.provider.clone(), n.public.model.clone(), n.public.task.clone()));
            }
            self.known_hosts = host_pids.clone();
        } else { faults += 1; host_pids = self.known_hosts.clone(); }
        let paths = source::root_paths(sessions,&roots);
        let mut found = HashSet::new();
        if let Ok(paths) = paths {
            for (root,path) in paths {
                found.insert(root.clone());
                if !self.read_node(&root,&path,now) {
                    faults += 1;
                    if let Some(n) = self.nodes.get_mut(&root) { n.public.activity_state = "unknown".into(); }
                }
                let stem = path.with_extension("");
                match source::descendants(&stem) {
                    Ok(children) => for child in children {
                        let Some(name) = child.file_stem().and_then(|n| n.to_str()) else { continue; };
                        let key = format!("{root}/{name}");
                        let parent_name = child.parent().filter(|p| *p != stem.as_path()).and_then(|p| p.file_name()).and_then(|p| p.to_str());
                        let parent = parent_name.map(|p| format!("{root}/{p}")).unwrap_or_else(|| root.clone());
                        self.ensure(&key,&root,Some(&parent),name,now);
                        if !self.read_node(&key,&child,now) { faults += 1; } else {
                            observed.insert(key.clone());
                            if let Some(n) = self.nodes.get_mut(&key) { n.seen = now; }
                        }
                    }, Err(_) => faults += 1,
                }
            }
        } else { faults += 1; }
        faults += roots.difference(&found).count();
        // Process-only sessions are actual live processes, not historical transcript guesses.
        // Their task, activity and absent model metadata remain explicitly unknown.
        if fixture.is_none() {
            match source::processes() {
                Ok(processes) => {
                    let process_ids: HashMap<_,_> = processes.iter().map(|p| (p.pid,p.key.clone())).chain(host_pids.iter().map(|(p,k)| (*p,k.clone()))).collect();
                    for p in processes {
                        if host_pids.contains_key(&p.pid) { continue; }
                        let ancestor = source::nearest_parent(p.parent,&process_ids);
                        let parent = ancestor.as_deref();
                        self.ensure(&p.key,&p.key,parent,"Process-observed session",now);
                        observed.insert(p.key.clone());
                        let n = self.nodes.get_mut(&p.key).unwrap(); n.seen = now;
                        n.public.runtime = p.runtime.into(); n.public.role = if p.headless { "headless worker" } else { "process-observed session" }.into();
                        n.public.task = "Task unavailable from process metadata".into(); n.public.status = "unknown".into();
                        Self::model(n,&p.model,None); roots.insert(p.key);
                    }
                }, Err(_) => faults += 1,
            }
        }
        for (key,node) in &mut self.nodes {
            if observed.contains(key) {
                node.public.observed_at = timestamp(now);
            } else if !(node.placeholder && roots.contains(&node.root) && source_ok) && (node.terminal.is_none() || !source_ok) {
                if node.unavailable.is_none() { node.unavailable = Some((node.public.status.clone(),node.public.activity_state.clone())); }
                node.public.status = "unknown".into(); node.public.activity_state = "unknown".into();
            }
            if let Some((status,provider,model,task)) = authoritative.get(key) {
                node.public.status = status.clone(); node.public.provider = provider.clone(); node.public.model = model.clone(); node.public.task = task.clone(); node.terminal = None;
                if !found.contains(key) { node.public.activity_state = "unknown".into(); }
            }
        }
        let interaction_agents: HashSet<_> = self.interactions.iter().map(|i| i.agent_id.clone()).collect();
        let mut visible: HashSet<String> = self.nodes.values().filter(|n| {
            (n.terminal.is_none_or(|t| now - t < 120_000) && ((n.placeholder && roots.contains(&n.root)) || now - n.seen < 120_000)) || interaction_agents.contains(&n.public.id)
        }).map(|n| n.public.id.clone()).collect();
        // Preserve ancestor identities so the public tree never contains dangling edges.
        loop {
            let before = visible.len();
            for n in self.nodes.values() { if visible.contains(&n.public.id) { if let Some(p) = &n.public.parent_id { visible.insert(p.clone()); } } }
            if visible.len() == before { break; }
        }
        self.nodes.retain(|_,n| roots.contains(&n.root) || visible.contains(&n.public.id));
        let agents: Vec<_> = self.nodes.values().filter(|n| visible.contains(&n.public.id)).map(|n| n.public.clone()).collect();
        let mut interactions: Vec<_> = self.interactions.iter().cloned().collect(); interactions.sort_by(|a,b| a.at.cmp(&b.at));
        Feed { schema_version: 1, generated_at: timestamp(now), sequence: self.id(&format!("snapshot:{now}:{}",self.sequence)), poll_ms: 1000, stale_after_ms: 15000,
            coverage: Coverage { label: "Observed live coding sessions", roots: agents.iter().filter(|a| a.parent_id.is_none()).count(), agents: agents.len(), complete: false,
                notice: format!("OMP collaboration trees and recognized live coding processes. Process-only workers have limited metadata. Other runtimes and disconnected sessions may be unobserved. {faults} source gaps this snapshot. Disappearance is unknown, not completion.") }, agents, interactions }
    }
}

#[cfg(test)]
#[path = "tests.rs"]
mod tests;

use super::*;
use serde_json::json;
use std::{fs, io::Write, sync::atomic::{AtomicU64, Ordering}};

fn collector() -> Collector { let mut c = Collector::new(vec![7;32],1000); c.ensure("private-root","private-root",None,"Root",2000); c }
fn message(role: &str, attribution: &str, content: Value, at: i64) -> Value { json!({"type":"message","id":format!("{role}-{at}"),"timestamp":at,"message":{"role":role,"attribution":attribution,"timestamp":at,"content":content}}) }
#[test] fn explicit_conversation_whitelist_and_cutoff() {
    let mut c = collector();
    c.event("private-root",&message("user","user",json!([{"type":"text","text":"Hello Drake"}]),1001),2000);
    c.event("private-root",&message("user","agent",json!([{"type":"text","text":"injected assignment"}]),1002),2000);
    c.event("private-root",&message("user","user",json!([{"type":"text","text":"historical"}]),999),2000);
    c.event("private-root",&message("assistant","",json!([{"type":"thinking","thinking":"HIDDEN_CANARY"},{"type":"text","text":"Public answer"},{"type":"providerPayload","text":"PROVIDER_CANARY"},{"type":"toolCall","text":"TOOL_CANARY"}]),1003),2000);
    c.ensure("private-root/Child","private-root",Some("private-root"),"Child",2000);
    c.event("private-root/Child",&message("user","user",json!([{"type":"text","text":"child assignment"}]),1004),2000);
    assert_eq!(c.interactions.len(),2); assert_eq!(c.interactions[0].role,"drake"); assert_eq!(c.interactions[1].text,"Public answer");
}
#[test] fn every_published_source_field_is_filtered_before_truncation() {
    let secret = format!("{} password=secret-canary", "ordinary words ".repeat(400));
    let mut c = collector();
    c.ensure("private-root/Child","private-root",Some("private-root"),&secret,2000);
    for event in [
        json!({"type":"title","title":secret}),
        json!({"type":"session_init","agent":secret,"resolvedModel":secret,"task":"TASK_CANARY","systemPrompt":"SYSTEM_CANARY"}),
        json!({"type":"model_usage","model":secret,"provider":secret}),
        json!({"type":"custom","customType":"tool_execution_start","data":{"toolCallId":"PRIVATE_ID","toolName":secret,"intent":secret,"args":{"token":"ARGS_CANARY"}}}),
        message("assistant","",json!([{"type":"text","text":secret}]),1001),
    ] { c.event("private-root/Child",&event,2000); }
    let output = serde_json::to_string(&c.nodes.values().map(|n| &n.public).collect::<Vec<_>>()).unwrap();
    for needle in ["secret-canary","TASK_CANARY","SYSTEM_CANARY","PRIVATE_ID","ARGS_CANARY","private-root"] { assert!(!output.contains(needle),"{needle}"); }
    assert!(c.interactions.is_empty());
}
#[test] fn grandchildren_pending_terminal_and_long_running_tool() {
    let mut c = collector();
    c.event("private-root",&json!({"type":"message","timestamp":2000,"message":{"role":"toolResult","toolName":"task","details":{"progress":[{"id":"Parent","agent":"task","status":"pending","assignment":"DO_NOT_PUBLISH"}]}}}),2000);
    c.event("private-root/Parent",&json!({"type":"message","timestamp":2001,"message":{"role":"toolResult","toolName":"task","details":{"progress":[{"id":"Parent.Child","agent":"reviewer","status":"running"}]}}}),2001);
    let parent = c.id("private-root/Parent"); assert_eq!(c.nodes["private-root/Parent.Child"].public.parent_id.as_deref(),Some(parent.as_str()));
    c.event("private-root/Parent.Child",&json!({"type":"custom","timestamp":2002,"customType":"tool_execution_start","data":{"toolCallId":"t","toolName":"bash","intent":"Running long operation","startedAt":2002}}),2002);
    assert_eq!(c.nodes["private-root/Parent.Child"].public.activity_state,"running");
    c.event("private-root/Parent.Child",&json!({"type":"message","timestamp":999999,"message":{"role":"assistant","content":[{"type":"text","text":"Still waiting"}]}}),999999);
    assert_eq!(c.nodes["private-root/Parent.Child"].public.status,"working");
    c.event("private-root/Parent",&json!({"type":"custom_message","customType":"async-result","timestamp":1000000,"details":{"jobs":[{"type":"task","agentId":"Parent.Child","status":"completed"}]}}),1000000);
    assert_eq!(c.nodes["private-root/Parent.Child"].public.status,"completed");
    c.event("private-root",&json!({"type":"custom_message","customType":"async-result","timestamp":1000001,"details":{"jobs":[{"type":"task","agentId":"NoFile","status":"failed"}]}}),1000001);
    assert_eq!(c.nodes["private-root/NoFile"].public.status,"failed");
}
struct Fixture { dir: PathBuf, hosts: PathBuf, sessions: PathBuf, root: PathBuf }
impl Fixture {
    fn new() -> Self {
        static COUNT: AtomicU64 = AtomicU64::new(0);
        let dir = std::env::temp_dir().join(format!("hive-feed-{}-{}",std::process::id(),COUNT.fetch_add(1,Ordering::Relaxed)));
        let sessions = dir.join("sessions"); fs::create_dir_all(sessions.join("cwd")).unwrap();
        let hosts = dir.join("hosts.json");
        fs::write(&hosts,json!({"version":1,"hosts":[{"sessionId":"private-root","sessionName":"Live coding","pid":1234567,"model":{"provider":"openai-codex","id":"gpt-6-astra"},"startedAt":1000,"busy":true,"inputRequired":false}]}).to_string()).unwrap();
        let root = sessions.join("cwd/date_private-root.jsonl");
        fs::write(&root,format!("{}\n",json!({"type":"session","timestamp":1000}))).unwrap();
        Self { dir,hosts,sessions,root }
    }
    fn append(&self, value: Value) { let mut f=fs::OpenOptions::new().append(true).open(&self.root).unwrap(); writeln!(f,"{value}").unwrap(); }
}
impl Drop for Fixture { fn drop(&mut self) { let _ = fs::remove_dir_all(&self.dir); } }
#[test] fn failures_and_disappearance_are_not_completion() {
    let f=Fixture::new(); let mut c=Collector::new(vec![8;32],1000);
    f.append(json!({"type":"custom","customType":"tool_execution_start","timestamp":1001,"data":{"toolCallId":"t","toolName":"bash","intent":"Running operation","startedAt":1001}}));
    let feed=c.poll(&f.sessions,Some(&f.hosts),2000); assert_eq!(feed.agents[0].status,"working");
    let feed=c.poll(&f.sessions,Some(&f.hosts),100000); assert_eq!(feed.agents[0].activity_state,"running");
    fs::write(&f.hosts,"malformed").unwrap(); let feed=c.poll(&f.sessions,Some(&f.hosts),100001);
    assert_eq!(feed.agents.len(),1); assert_eq!(feed.agents[0].status,"unknown"); assert!(!feed.coverage.complete); assert!(feed.coverage.notice.contains("source gaps"));
    fs::write(&f.hosts,"{\"version\":1,\"hosts\":[]}").unwrap(); let feed=c.poll(&f.sessions,Some(&f.hosts),100002); assert_eq!(feed.agents[0].status,"unknown");
}
#[test] fn terminal_history_not_resurrected_and_root_busy_is_authoritative() {
    let f=Fixture::new(); let child=f.root.with_extension("").join("Child.jsonl"); fs::create_dir_all(child.parent().unwrap()).unwrap();
    fs::write(child,format!("{}\n{}\n",json!({"type":"session_init","timestamp":1000,"agent":"task","resolvedModel":"anthropic/claude-opus-5-5"}),json!({"type":"custom","timestamp":1001,"customType":"tool_execution_start","data":{"toolName":"read","toolCallId":"t","startedAt":1001}}))).unwrap();
    f.append(json!({"type":"custom_message","customType":"async-result","timestamp":2000,"details":{"jobs":[{"type":"task","agentId":"Child","status":"completed"}]}}));
    f.append(json!({"type":"custom","customType":"session_exit","timestamp":2001,"data":{"kind":"normal"}}));
    let mut c=Collector::new(vec![8;32],5000); let feed=c.poll(&f.sessions,Some(&f.hosts),200000);
    assert_eq!(feed.agents.len(),1); assert_eq!(feed.agents[0].status,"working");
    assert_eq!(c.poll(&f.sessions,Some(&f.hosts),201000).agents.len(),1);
}
#[test] fn interaction_bound_and_stable_opaque_identity() {
    let mut c=collector();
    for i in 0..110 { c.event("private-root",&message("assistant","",json!([{"type":"text","text":"Public response"}]),1000+i),2000); }
    assert_eq!(c.interactions.len(),100); assert!(!c.id("private-root").contains("private-root"));
    assert_eq!(c.id("private-root"),collector().id("private-root"));
    assert_ne!(c.id("private-root"),Collector::new(vec![9;32],0).id("private-root"));
}

#[test] fn missing_child_becomes_unknown_without_refreshing_observation() {
    let f=Fixture::new(); let child=f.root.with_extension("").join("Child.jsonl"); fs::create_dir_all(child.parent().unwrap()).unwrap();
    let data=format!("{}\n{}\n",json!({"type":"session_init","timestamp":1000,"agent":"task","resolvedModel":"anthropic/claude-opus-5-5"}),json!({"type":"custom","timestamp":1001,"customType":"tool_execution_start","data":{"toolName":"read","toolCallId":"t","startedAt":1001}}));
    fs::write(&child,&data).unwrap();
    let mut c=Collector::new(vec![8;32],5000); let initial=c.poll(&f.sessions,Some(&f.hosts),2000);
    let id=c.id("private-root/Child"); let before=initial.agents.iter().find(|a| a.id==id).unwrap().observed_at.clone();
    fs::remove_file(&child).unwrap();
    let snapshot=c.poll(&f.sessions,Some(&f.hosts),3000); let node=snapshot.agents.iter().find(|a| a.id==id).unwrap();
    assert_eq!(node.status,"unknown"); assert_eq!(node.activity_state,"unknown"); assert_eq!(node.observed_at,before);
    fs::write(&child,&data).unwrap();
    let snapshot=c.poll(&f.sessions,Some(&f.hosts),4000); let node=snapshot.agents.iter().find(|a| a.id==id).unwrap();
    assert_eq!(node.status,"working"); assert_eq!(node.activity_state,"running");
}

#[test] fn interaction_bound_uses_event_time_not_scan_order() {
    let mut c=collector();
    for i in (0..150).rev() { c.event("private-root",&message("assistant","",json!([{"type":"text","text":"Public response"}]),1000+i),2000); }
    assert_eq!(c.interactions.len(),100);
    assert_eq!(c.interactions.front().unwrap().at,timestamp(1050));
    assert_eq!(c.interactions.back().unwrap().at,timestamp(1149));
}

#[test] fn root_fields_and_unknown_fields_cannot_bypass_boundary() {
    let f=Fixture::new();
    fs::write(&f.hosts,json!({"version":1,"hosts":[{"sessionId":"private-root","sessionName":"password=HOST_CANARY","pid":987654321,"cwd":"/home/PRIVATE_PERSON/path","instanceId":"PRIVATE_INSTANCE","token":"UNKNOWN_CANARY","model":{"provider":"password=PROVIDER_CANARY","id":"password=MODEL_CANARY"},"startedAt":1000,"busy":true}]}).to_string()).unwrap();
    let mut c=Collector::new(vec![8;32],5000);
    let output=serde_json::to_string(&c.poll(&f.sessions,Some(&f.hosts),2000)).unwrap();
    for needle in ["CANARY","987654321","PRIVATE_PERSON","PRIVATE_INSTANCE","private-root"] { assert!(!output.contains(needle),"{needle}"); }
}

#[test] fn interactions_keep_message_model_not_current_node_model() {
    let mut c=collector();
    c.event("private-root",&json!({"type":"message","timestamp":1001,"message":{"role":"assistant","provider":"anthropic","model":"claude-opus-5-5","content":[{"type":"text","text":"First reply"}]}}),2000);
    c.event("private-root",&json!({"type":"message","timestamp":1002,"message":{"role":"assistant","provider":"openai-codex","model":"gpt-6-astra","content":[{"type":"text","text":"Second reply"}]}}),2000);
    c.event("private-root",&message("user","user",json!([{"type":"text","text":"Hello"}]),1003),2000);
    assert_eq!(c.interactions[0].provider,"anthropic"); assert_eq!(c.interactions[0].model,"claude-opus-5-5");
    assert_eq!(c.interactions[1].provider,"openai-codex"); assert_eq!(c.interactions[1].model,"gpt-6-astra");
    assert_eq!(c.interactions[2].model,"unknown");
    c.event("private-root",&json!({"type":"message","timestamp":1004,"message":{"role":"assistant","provider":"password=PROVIDER_CANARY","model":"password=MODEL_CANARY","content":[{"type":"text","text":"Safe reply"}]}}),2000);
    assert!(!serde_json::to_string(&c.interactions).unwrap().contains("CANARY"));
}

#[test] fn assignment_summary_whitelist_tracks_child_without_publishing_payload() {
    let mut c=collector();
    let args=json!({"context":"CONTEXT_CANARY","other":"OTHER_CANARY","tasks":[
        {"name":"Parent","solutionSpace":"Implement orbital motion controls","task":"TASK_CANARY","other":"CHILD_CANARY"},
        {"name":"Rejected","solutionSpace":format!("{} password=SUFFIX_CANARY","ordinary words ".repeat(100)),"task":"ASSIGNMENT_CANARY"}
    ]});
    c.event("private-root",&message("assistant","",json!([{"type":"toolCall","name":"task","arguments":args}]),1001),2000);
    assert!(!c.nodes.contains_key("private-root/Parent"));
    c.event("private-root",&json!({"type":"message","timestamp":1002,"message":{"role":"toolResult","toolName":"task","details":{"progress":[{"id":"Parent","agent":"task","status":"pending"},{"id":"Rejected","agent":"task","status":"pending"}]}}}),2000);
    assert_eq!(c.nodes["private-root/Parent"].public.task,"Implement orbital motion controls");
    assert_eq!(c.nodes["private-root/Rejected"].public.task,"Rejected");
    c.event("private-root/Parent",&json!({"type":"title","title":""}),2000);
    c.event("private-root/Parent",&json!({"type":"title","title":"Parent"}),2000);
    assert_eq!(c.nodes["private-root/Parent"].public.task,"Implement orbital motion controls");
    c.event("private-root/Parent",&json!({"type":"custom","customType":"tool_execution_start","timestamp":1003,"data":{"toolName":"task","toolCallId":"t","args":{"context":"NESTED_CONTEXT_CANARY","tasks":[{"name":"Child","solutionSpace":"Check reduced motion behavior","task":"NESTED_TASK_CANARY"}]}}}),2000);
    c.event("private-root/Parent",&json!({"type":"message","timestamp":1004,"message":{"role":"toolResult","toolName":"task","details":{"progress":[{"id":"Parent.Child","agent":"reviewer","status":"pending"}]}}}),2000);
    assert_eq!(c.nodes["private-root/Parent.Child"].public.task,"Check reduced motion behavior");
    let output=serde_json::to_string(&c.nodes.values().map(|n| &n.public).collect::<Vec<_>>()).unwrap();
    assert!(!output.contains("CANARY")); assert!(c.interactions.is_empty());
}

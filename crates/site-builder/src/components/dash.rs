//! /dash/: Drake's one-screen dashboard. How much model quota is left on
//! every account the agents run on, the Spark's own pulse, how much work is
//! open, and which model each omp role and subagent uses.
//!
//! The Spark writes two files to the `data/now` branch (`~/.local/bin/dash-snapshot`,
//! called by `now-publish`) and the pages workflow lays them into `public/data/`:
//!
//! * `accounts.tsv`: `checked<TAB>utc`, then one row per limit:
//!   `limit, provider, account number, plan, window (5h|week), label, percent used,
//!   resets utc, ok|exhausted`. Accounts are numbered per provider; the
//!   publisher never writes an email, account id, org or project name.
//! * `routing.tsv`: `role|agent|seat, name, model selector`.
//!
//! Everything read here passes `word`, so even a publisher fault cannot put
//! an email address or a markup character on the page.

use std::fs;
use std::path::Path;

use maud::{html, Markup};

use super::boxlink::{foot_labelled, Target};
use super::now::{fmt_utc, parse_utc, Now};

pub const TITLE: &str = "Dashboard | AienOS";
pub const DESCRIPTION: &str = "One screen: model quota left on every account the agents run on, the Spark's pulse, open work, and which model each role uses.";
pub const URL: &str = "https://www.aienos.com/dash/";

pub const ACCOUNTS_PATH: &str = "public/data/accounts.tsv";
pub const ROUTING_PATH: &str = "public/data/routing.tsv";
const ACCOUNTS_URL: &str = "https://github.com/aien-dev/aienos.com/blob/data/now/accounts.tsv";
const ROUTING_URL: &str = "https://github.com/aien-dev/aienos.com/blob/data/now/routing.tsv";
const PULSE_URL: &str = "https://github.com/aien-dev/aienos.com/blob/data/now/now-pulse.tsv";
const LEDGER_URL: &str = "https://github.com/aien-dev/aienos.com/blob/data/now/now.tsv";

/// Providers the page shows, in display order.
const PROVIDERS: [&str; 4] = ["Anthropic", "OpenAI Codex", "Google Antigravity", "xAI Grok"];
/// A quota reading older than this, at build time, is called out as old.
const STALE_S: i64 = 2 * 3600;

#[derive(Debug, Clone, PartialEq)]
pub struct Limit {
    pub window: String,
    pub label: String,
    pub used_pct: u32,
    pub resets_utc: String,
    pub exhausted: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Account {
    pub provider: &'static str,
    pub n: u32,
    pub plan: String,
    pub limits: Vec<Limit>,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct Quota {
    pub checked_utc: Option<String>,
    pub accounts: Vec<Account>,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct Routing {
    pub roles: Vec<(String, String)>,
    pub agents: Vec<(String, String)>,
    pub seats: Vec<(String, String)>,
}

pub struct Dash {
    pub quota: Quota,
    pub routing: Routing,
}

/// The only characters a published field may carry: no `@`, no markup, no
/// separators, at most 48 of them.
fn word(s: &str) -> String {
    s.chars()
        .filter(|c| c.is_ascii_alphanumeric() || " .:_/&()+-".contains(*c))
        .take(48)
        .collect::<String>()
        .trim()
        .to_string()
}

pub fn parse_accounts(tsv: &str) -> Quota {
    let mut q = Quota::default();
    for line in tsv.lines() {
        let f: Vec<&str> = line.split('\t').collect();
        match f.as_slice() {
            ["checked", t, ..] => q.checked_utc = parse_utc(t).map(fmt_utc),
            ["limit", provider, n, plan, window, label, used, resets, state, ..] => {
                let Some(provider) = PROVIDERS.iter().copied().find(|p| p == provider) else { continue };
                let (Ok(n), Ok(used)) = (n.trim().parse::<u32>(), used.trim().parse::<f64>()) else { continue };
                let window = match *window {
                    "5h" | "week" => window.to_string(),
                    _ => continue,
                };
                let limit = Limit {
                    window,
                    label: word(label),
                    used_pct: used.round().clamp(0.0, 100.0) as u32,
                    resets_utc: parse_utc(resets).map(fmt_utc).unwrap_or_default(),
                    exhausted: *state == "exhausted",
                };
                match q.accounts.iter_mut().find(|a| a.provider == provider && a.n == n) {
                    Some(a) => a.limits.push(limit),
                    None => q.accounts.push(Account { provider, n, plan: word(plan), limits: vec![limit] }),
                }
            }
            _ => {}
        }
    }
    q.accounts.sort_by_key(|a| (PROVIDERS.iter().position(|p| *p == a.provider), a.n));
    for a in &mut q.accounts {
        // Five-hour windows first, then the week; the publisher's order within each.
        a.limits.sort_by_key(|l| l.window != "5h");
    }
    q
}

pub fn parse_routing(tsv: &str) -> Routing {
    let mut r = Routing::default();
    for line in tsv.lines() {
        let f: Vec<&str> = line.split('\t').collect();
        let [kind, name, model, ..] = f.as_slice() else { continue };
        let pair = (word(name), word(model));
        if pair.0.is_empty() || pair.1.is_empty() {
            continue;
        }
        match *kind {
            "role" => r.roles.push(pair),
            "agent" => r.agents.push(pair),
            "seat" => r.seats.push(pair),
            _ => {}
        }
    }
    r
}

/// Load from the laid-in files. Either may be absent; the page says so.
pub fn load() -> Dash {
    Dash {
        quota: fs::read_to_string(Path::new(ACCOUNTS_PATH)).map(|s| parse_accounts(&s)).unwrap_or_default(),
        routing: fs::read_to_string(Path::new(ROUTING_PATH)).map(|s| parse_routing(&s)).unwrap_or_default(),
    }
}

/// "2026-10-31T16:00Z" as "Oct 31 16:00".
fn short_utc(s: &str) -> String {
    const MONTHS: [&str; 12] = ["Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"];
    let Some((date, time)) = s.trim_end_matches('Z').split_once('T') else { return s.to_string() };
    let mut d = date.split('-').skip(1);
    match (d.next().and_then(|m| m.parse::<usize>().ok()), d.next()) {
        (Some(m @ 1..=12), Some(day)) => format!("{} {} {}", MONTHS[m - 1], day.trim_start_matches('0'), time),
        _ => s.to_string(),
    }
}

fn limit_row(l: &Limit, provider: &str) -> Markup {
    let left = 100 - l.used_pct;
    // Inside the Anthropic card "Claude 5 Hour" says nothing "5 Hour" does not.
    let label = if provider == "Anthropic" { l.label.strip_prefix("Claude ").unwrap_or(&l.label) } else { &l.label };
    html! {
        div.dash-limit.dash-out[l.exhausted || left == 0] {
            span class="dash-limit-k" { (label) }
            meter class="dash-meter" min="0" max="100" low="30" high="70" optimum="100" value=(left) { (left) "% left" }
            span class="dash-limit-v mono" { (left) "%" }
            span class="dash-limit-r mono" {
                @if !l.resets_utc.is_empty() { (short_utc(&l.resets_utc)) }
            }
        }
    }
}

fn provider_card(provider: &str, accounts: &[&Account]) -> Markup {
    html! {
        article class="dash-card" {
            h2 class="dash-h" { (provider) " " span class="dash-h-note" { "% left" } }
            @for a in accounts {
                div class="dash-acct" {
                    div class="dash-acct-h" {
                        "Account " (a.n)
                        @if !a.plan.is_empty() { span class="dash-plan" { (a.plan) } }
                    }
                    @for l in &a.limits { (limit_row(l, provider)) }
                }
            }
            (foot_labelled(ACCOUNTS_URL, "Reading on GitHub", Target::Evidence))
        }
    }
}

fn pairs(title: &str, rows: &[(String, String)]) -> Markup {
    html! {
        div class="dash-pairs" {
            h3 class="dash-pairs-h" { (title) }
            @if rows.is_empty() {
                p class="dash-none" { "Not published." }
            } @else {
                dl {
                    @for (k, v) in rows {
                        dt { (k) }
                        dd class="mono" { (v) }
                    }
                }
            }
        }
    }
}

pub fn render_dash_page(d: &Dash, now: &Now) -> Markup {
    let built = parse_utc(&now.built_utc);
    let checked = d.quota.checked_utc.clone();
    let stale = built.zip(checked.as_deref().and_then(parse_utc)).is_some_and(|(b, c)| b - c > STALE_S);
    let open = now.items.iter().filter(|i| i.status == "open").count();
    let for_drake = now.items.iter().filter(|i| i.status == "open" && i.owner_kind == "operator").count();
    let settled_day = now
        .items
        .iter()
        .filter(|i| (i.status == "done" || i.status == "dropped") && (0..86_400).contains(&i.age_s))
        .count();
    let answering: Vec<&str> = now.models.iter().filter(|m| m.state == "answering").map(|m| m.name.as_str()).collect();
    let pulse = |k: &str| now.pulse.iter().find(|p| p.key == k).map(|p| p.value.clone());
    html! {
        article class="dash container" {
            header class="dash-top" {
                h1 class="dash-title" { "Dashboard" }
                p class="dash-meta mono" {
                    "quota read " (checked.as_deref().map(short_utc).unwrap_or_else(|| "never".into()))
                    " · built " (short_utc(&now.built_utc)) " UTC"
                }
            }
            @if stale {
                p class="dash-warn" { "This is the last quota reading the Spark published; nothing newer has arrived since." }
            }
            div class="dash-flow" {
                @if d.quota.accounts.is_empty() {
                    article class="dash-card" {
                        h2 class="dash-h" { "Quota" }
                        p class="dash-none" { "No quota reading has been published yet." }
                        (foot_labelled(ACCOUNTS_URL, "Reading on GitHub", Target::Evidence))
                    }
                }
                @for p in PROVIDERS {
                    @let accts: Vec<&Account> = d.quota.accounts.iter().filter(|a| a.provider == p).collect();
                    @if !accts.is_empty() { (provider_card(p, &accts)) }
                }
                article class="dash-card" {
                    h2 class="dash-h" { "Spark" }
                    dl class="dash-kv" {
                        dt { "Serving" } dd { @if answering.is_empty() { "nothing answered" } @else { (answering.join(", ")) } }
                        @for k in ["memory free", "disk free", "wall power"] {
                            @if let Some(v) = pulse(k) { dt { (k) } dd { (v) } }
                        }
                    }
                    (foot_labelled(PULSE_URL, "Pulse on GitHub", Target::Evidence))
                }
                article class="dash-card" {
                    h2 class="dash-h" { a href="/now/" { "Work" } }
                    dl class="dash-kv" {
                        dt { "Open" } dd { (open) }
                        dt { "Waiting on Drake" } dd { (for_drake) }
                        dt { "Settled, last day" } dd { (settled_day) }
                        dt { "Ledger written" } dd { (short_utc(&now.ledger_written_utc)) }
                    }
                    (foot_labelled(LEDGER_URL, "Ledger on GitHub", Target::Evidence))
                }
            }
            section class="dash-sec" aria-labelledby="dash-routing-h" {
                article class="dash-card" {
                    h2 id="dash-routing-h" class="dash-h" { "Which model does what" }
                    div class="dash-cols" {
                        (pairs("Roles", &d.routing.roles))
                        (pairs("Subagents", &d.routing.agents))
                        (pairs("Advisor seats", &d.routing.seats))
                    }
                    (foot_labelled(ROUTING_URL, "Routing on GitHub", Target::Evidence))
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const ACCOUNTS: &str = "checked\t2026-10-26T12:32Z\n\
limit\tOpenAI Codex\t2\tplus\tweek\t7 days\t0\t2026-11-02T12:21Z\tok\n\
limit\tAnthropic\t2\t\tweek\tClaude 7 Day\t100\t2026-10-31T16:00Z\texhausted\n\
limit\tAnthropic\t2\t\t5h\tClaude 5 Hour\t0.4\t\tok\n\
limit\tAnthropic\t1\t\t5h\tClaude 5 Hour\t1\t2026-10-26T17:00Z\tok\n\
limit\tOpenAI Codex\t1\tpro\tweek\t7 days\t250\t2026-11-02T12:28Z\tok\n\
limit\tSomeone Else\t1\t\tweek\tx\t5\t\tok\n\
limit\tAnthropic\t1\t\tmonth\tClaude Month\t5\t\tok\n\
limit\tAnthropic\t1\t\tweek\tClaude 7 Day\tlots\t\tok\n";

    #[test]
    fn accounts_group_in_provider_order_and_bound_their_numbers() {
        let q = parse_accounts(ACCOUNTS);
        assert_eq!(q.checked_utc.as_deref(), Some("2026-10-26T12:32Z"));
        let order: Vec<(&str, u32)> = q.accounts.iter().map(|a| (a.provider, a.n)).collect();
        assert_eq!(order, [("Anthropic", 1), ("Anthropic", 2), ("OpenAI Codex", 1), ("OpenAI Codex", 2)]);
        // An unknown provider, an unknown window and an unreadable percent are dropped.
        assert_eq!(q.accounts[0].limits.len(), 1);
        // The five-hour window comes first even when the publisher wrote it second.
        let a2 = &q.accounts[1];
        assert_eq!(a2.limits.iter().map(|l| l.window.as_str()).collect::<Vec<_>>(), ["5h", "week"]);
        assert!(a2.limits[1].exhausted);
        assert_eq!(a2.limits[1].used_pct, 100);
        assert_eq!(q.accounts[2].limits[0].used_pct, 100, "percent is clamped");
        assert_eq!(q.accounts[3].plan, "plus");
    }

    #[test]
    fn a_publisher_fault_cannot_put_an_email_or_markup_on_the_page() {
        let q = parse_accounts("limit\tAnthropic\t1\tdrake@example.com\tweek\t<b>x</b>@y.z\t5\t\tok\n");
        let a = &q.accounts[0];
        assert!(!a.plan.contains('@') && !a.limits[0].label.contains('@'));
        assert!(!a.limits[0].label.contains('<'));
        let r = parse_routing("role\tdefault\tme@example.com/model\n");
        assert!(!r.roles[0].1.contains('@'));
    }

    #[test]
    fn routing_sorts_rows_by_kind() {
        let r = parse_routing("role\tdefault\tanthropic/claude-opus-5-5\nagent\tscout\tdeepseek/deepseek-flash\nseat\tWitness\tdeepseek/deepseek-flash:max\nbogus\tx\ty\nrole\t\tz\n");
        assert_eq!(r.roles, [("default".into(), "anthropic/claude-opus-5-5".into())]);
        assert_eq!(r.agents.len(), 1);
        assert_eq!(r.seats[0].0, "Witness");
    }

    #[test]
    fn short_times_read_as_month_day_and_clock() {
        assert_eq!(short_utc("2026-10-31T16:00Z"), "Oct 31 16:00");
        assert_eq!(short_utc("2026-11-02T04:00Z"), "Nov 2 04:00");
        assert_eq!(short_utc("never"), "never");
    }
}

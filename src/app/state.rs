use std::collections::VecDeque;

use gpui::{App, AppContext, Context, Entity, Global, SharedString};
use serde_json::Value;

use crate::backend::{dirs, streaming::StreamEvent};

pub const DEFAULT_LANGUAGE: &str = "en-US";

/// Supported UI languages: (locale code, native display name).
pub const LANGUAGES: &[(&str, &str)] = &[
    ("en-US", "English"),
    ("ru-RU", "Русский"),
    ("zh-CN", "简体中文"),
];

/// Number of per-second traffic samples kept for the Home graph.
const MAX_HISTORY: usize = 60;
/// Cap on the in-memory log ring buffer.
const MAX_LOGS: usize = 1000;
/// Cap on the remembered recently-closed connections.
const MAX_CLOSED: usize = 300;

#[derive(Clone, PartialEq, Eq)]
pub enum CoreStatus {
    Stopped,
    Starting,
    Running,
    Failed(SharedString),
}

impl CoreStatus {
    pub fn is_running(&self) -> bool {
        matches!(self, CoreStatus::Running)
    }
}

/// A single proxy node inside a group.
#[derive(Clone)]
pub struct ProxyNode {
    pub name: SharedString,
    pub kind: SharedString,
    pub delay: Option<u32>,
}

/// A proxy group with its members and the current selection.
#[derive(Clone)]
pub struct ProxyGroup {
    pub name: SharedString,
    pub kind: SharedString,
    pub now: SharedString,
    pub all: Vec<ProxyNode>,
}

/// A profile entry (subscription or local file).
#[derive(Clone)]
pub struct ProfileItem {
    pub id: SharedString,
    pub name: SharedString,
    pub kind: SharedString,
    pub is_current: bool,
    /// Subscription usage (bytes): consumed, quota, and expiry unix-ts. 0 = absent.
    pub used: u64,
    pub total: u64,
    pub expire: i64,
}

/// One log line for the Logs page / ring buffer.
#[derive(Clone)]
pub struct LogLine {
    pub time: SharedString,
    pub level: SharedString,
    pub message: SharedString,
}

/// Active connections grouped by originating process (Connections page).
#[derive(Clone)]
pub struct ConnProcess {
    pub name: SharedString,
    pub count: usize,
    pub up: u64,
    pub down: u64,
    /// Executable path of the process (for the app icon), if known.
    pub process_path: SharedString,
    /// Individual connections belonging to this process (for the detail view).
    pub conns: Vec<ConnItem>,
}

/// A single active connection (shown in a process's detail view).
#[derive(Clone)]
pub struct ConnItem {
    pub id: SharedString,
    pub host: SharedString,
    pub network: SharedString,
    pub chains: SharedString,
    pub rule: SharedString,
    pub up: u64,
    pub down: u64,
    pub conn_type: SharedString,
    pub dest_ip: SharedString,
    pub dest_port: SharedString,
    pub dest_geoip: SharedString,
    pub dest_asn: SharedString,
    pub src_ip: SharedString,
    pub src_port: SharedString,
    pub process: SharedString,
    pub process_path: SharedString,
    pub dns_mode: SharedString,
}

/// One routing rule (Rules page).
#[derive(Clone)]
pub struct Rule {
    pub kind: SharedString,
    pub payload: SharedString,
    pub proxy: SharedString,
}

/// Shared, observable application state. Views hold the `Entity<AppState>`
/// (via [`AppState::global`]) and `observe` it to re-render on change.
pub struct AppState {
    pub language: SharedString,
    pub core_status: CoreStatus,
    pub mihomo_version: Option<SharedString>,
    pub tun_enabled: bool,

    pub up_speed: u64,
    pub down_speed: u64,
    pub total_up: u64,
    pub total_down: u64,
    pub conn_count: usize,
    pub history: VecDeque<(u64, u64)>,
    last_totals: Option<(u64, u64)>,

    pub groups: Vec<ProxyGroup>,
    pub profiles: Vec<ProfileItem>,
    pub current_profile_name: Option<SharedString>,
    pub current_profile_item: Option<Value>,
    pub mode: SharedString,
    pub app_config: Value,
    /// Full controlled mihomo config (used by the TUN settings sub-page).
    pub controled_config: Value,
    pub logs: VecDeque<LogLine>,
    /// Monotonic counter of every log line ever appended. Unlike `logs.len()`
    /// (which saturates at the ring-buffer cap) this keeps growing, so the Logs
    /// view can reliably detect new lines and keep itself scrolled to the tail.
    pub log_seq: usize,
    pub connections: Vec<ConnProcess>,
    /// Recently-closed connections (grouped by process), built by diffing
    /// consecutive `/connections` snapshots. Powers the Connections "Closed" tab.
    pub closed_connections: Vec<ConnProcess>,
    /// Last snapshot's active connections keyed by id (process-name, item) — used
    /// to detect which ones disappeared (closed) on the next snapshot.
    active_by_id: std::collections::HashMap<String, (String, ConnItem)>,
    /// Capped ring of closed (process-name, item) pairs.
    closed_items: VecDeque<(String, ConnItem)>,
    pub rules: Vec<Rule>,
}

struct GlobalAppState(Entity<AppState>);
impl Global for GlobalAppState {}

impl AppState {
    /// Loads persisted state, sets the active locale, and registers the global.
    /// Call once after `gpui_component::init`.
    pub fn init(cx: &mut App) {
        let language = load_language();
        rust_i18n::set_locale(&language);
        let entity = cx.new(|_| AppState {
            language: language.into(),
            core_status: CoreStatus::Stopped,
            mihomo_version: None,
            tun_enabled: false,
            up_speed: 0,
            down_speed: 0,
            total_up: 0,
            total_down: 0,
            conn_count: 0,
            history: VecDeque::with_capacity(MAX_HISTORY),
            last_totals: None,
            groups: Vec::new(),
            profiles: Vec::new(),
            current_profile_name: None,
            current_profile_item: None,
            mode: "rule".into(),
            app_config: Value::Null,
            controled_config: Value::Null,
            logs: VecDeque::with_capacity(MAX_LOGS),
            log_seq: 0,
            connections: Vec::new(),
            closed_connections: Vec::new(),
            active_by_id: std::collections::HashMap::new(),
            closed_items: VecDeque::new(),
            rules: Vec::new(),
        });
        cx.set_global(GlobalAppState(entity));
    }

    /// The shared `AppState` entity.
    pub fn global(cx: &App) -> Entity<AppState> {
        cx.global::<GlobalAppState>().0.clone()
    }

    /// Switches the UI language, persists it, and notifies observers so the
    /// whole UI re-renders with the new translations.
    pub fn set_language(&mut self, lang: impl Into<SharedString>, cx: &mut Context<Self>) {
        let lang: SharedString = lang.into();
        if lang == self.language {
            return;
        }
        rust_i18n::set_locale(lang.as_ref());
        persist_language(lang.as_ref());
        self.language = lang;
        cx.notify();
    }

    pub fn set_core_status(&mut self, status: CoreStatus, cx: &mut Context<Self>) {
        if self.core_status != status {
            self.core_status = status;
            cx.notify();
        }
    }

    pub fn set_groups(&mut self, groups: Vec<ProxyGroup>, cx: &mut Context<Self>) {
        self.groups = groups;
        cx.notify();
    }

    pub fn set_rules(&mut self, rules: Vec<Rule>, cx: &mut Context<Self>) {
        self.rules = rules;
        cx.notify();
    }

    pub fn set_tun_enabled(&mut self, enabled: bool, cx: &mut Context<Self>) {
        if self.tun_enabled != enabled {
            self.tun_enabled = enabled;
            cx.notify();
        }
    }

    pub fn set_current_profile_name(&mut self, name: Option<SharedString>, cx: &mut Context<Self>) {
        self.current_profile_name = name;
        cx.notify();
    }

    pub fn set_profiles(&mut self, profiles: Vec<ProfileItem>, cx: &mut Context<Self>) {
        self.profiles = profiles;
        cx.notify();
    }

    pub fn set_current_profile_item(&mut self, item: Option<Value>, cx: &mut Context<Self>) {
        self.current_profile_item = item;
        cx.notify();
    }

    pub fn set_app_config(&mut self, cfg: Value, cx: &mut Context<Self>) {
        self.app_config = cfg;
        cx.notify();
    }

    pub fn set_controled_config(&mut self, cfg: Value, cx: &mut Context<Self>) {
        self.controled_config = cfg;
        cx.notify();
    }

    /// Reads a value from the controlled mihomo config by dot path.
    pub fn ctl(&self, path: &str) -> Option<&Value> {
        let mut cur = &self.controled_config;
        for seg in path.split('.') {
            cur = cur.get(seg)?;
        }
        Some(cur)
    }

    /// Reads a boolean from the controlled mihomo config (dot path).
    pub fn ctl_bool(&self, path: &str, default: bool) -> bool {
        self.ctl(path).and_then(Value::as_bool).unwrap_or(default)
    }

    /// Reads a boolean from the loaded app config (dot path, e.g. `sysProxy.enable`).
    pub fn app_flag(&self, path: &str) -> bool {
        let mut cur = &self.app_config;
        for seg in path.split('.') {
            match cur.get(seg) {
                Some(v) => cur = v,
                None => return false,
            }
        }
        cur.as_bool().unwrap_or(false)
    }

    pub fn set_mode(&mut self, mode: impl Into<SharedString>, cx: &mut Context<Self>) {
        let mode = mode.into();
        if self.mode != mode {
            self.mode = mode;
            cx.notify();
        }
    }

    /// Updates a single node's delay (after a proxy delay test).
    pub fn set_node_delay(
        &mut self,
        group: &str,
        node: &str,
        delay: Option<u32>,
        cx: &mut Context<Self>,
    ) {
        if let Some(g) = self.groups.iter_mut().find(|g| g.name.as_ref() == group) {
            if let Some(n) = g.all.iter_mut().find(|n| n.name.as_ref() == node) {
                n.delay = delay;
                cx.notify();
            }
        }
    }

    /// Folds one streaming event into the state and notifies observers.
    pub fn apply_stream_event(&mut self, ev: StreamEvent, cx: &mut Context<Self>) {
        match ev {
            StreamEvent::Connections(data) => self.apply_connections(&data),
            StreamEvent::Log(entry) => self.push_log(&entry),
        }
        cx.notify();
    }

    fn apply_connections(&mut self, data: &Value) {
        let down = data
            .get("downloadTotal")
            .and_then(Value::as_u64)
            .unwrap_or(0);
        let up = data.get("uploadTotal").and_then(Value::as_u64).unwrap_or(0);
        let conns = data.get("connections").and_then(Value::as_array);
        self.conn_count = conns.map(|a| a.len()).unwrap_or(0);

        // Parse the snapshot, then diff against the previous active set: any id we
        // had before that's gone now has closed — remember it for the Closed tab.
        let parsed: Vec<(String, String, ConnItem)> = conns
            .map(|arr| arr.iter().filter_map(parse_conn).collect())
            .unwrap_or_default();
        let new_ids: std::collections::HashSet<&str> =
            parsed.iter().map(|(id, _, _)| id.as_str()).collect();
        let disappeared: Vec<(String, ConnItem)> = self
            .active_by_id
            .iter()
            .filter(|(id, _)| !new_ids.contains(id.as_str()))
            .map(|(_, v)| v.clone())
            .collect();
        for item in disappeared {
            if self.closed_items.len() >= MAX_CLOSED {
                self.closed_items.pop_front();
            }
            self.closed_items.push_back(item);
        }
        self.active_by_id = parsed
            .iter()
            .map(|(id, proc, item)| (id.clone(), (proc.clone(), item.clone())))
            .collect();

        self.connections = group_items(parsed.into_iter().map(|(_, proc, item)| (proc, item)));
        self.closed_connections = group_items(self.closed_items.iter().rev().cloned());

        if let Some((last_up, last_down)) = self.last_totals {
            self.up_speed = up.saturating_sub(last_up);
            self.down_speed = down.saturating_sub(last_down);
        } else {
            self.up_speed = 0;
            self.down_speed = 0;
        }
        self.last_totals = Some((up, down));
        self.total_up = up;
        self.total_down = down;

        if self.history.len() >= MAX_HISTORY {
            self.history.pop_front();
        }
        self.history.push_back((self.up_speed, self.down_speed));
    }

    fn push_log(&mut self, entry: &Value) {
        let level = entry
            .get("type")
            .and_then(Value::as_str)
            .unwrap_or("info")
            .to_string();
        let message = entry
            .get("payload")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_string();
        if message.is_empty() {
            return;
        }
        if self.logs.len() >= MAX_LOGS {
            self.logs.pop_front();
        }
        self.log_seq = self.log_seq.wrapping_add(1);
        self.logs.push_back(LogLine {
            time: chrono::Local::now()
                .format("%H:%M:%S%.3f")
                .to_string()
                .into(),
            level: level.into(),
            message: message.into(),
        });
    }

    /// Empties the in-memory log buffer (Logs page "clear" button).
    pub fn clear_logs(&mut self, cx: &mut Context<Self>) {
        self.logs.clear();
        cx.notify();
    }
}

/// Parses the `{ rules: [...] }` payload from `/rules` into [`Rule`]s.
pub fn parse_rules(value: &Value) -> Vec<Rule> {
    value
        .get("rules")
        .and_then(Value::as_array)
        .map(|arr| {
            arr.iter()
                .map(|r| Rule {
                    kind: r
                        .get("type")
                        .and_then(Value::as_str)
                        .unwrap_or("")
                        .to_string()
                        .into(),
                    payload: r
                        .get("payload")
                        .and_then(Value::as_str)
                        .unwrap_or("")
                        .to_string()
                        .into(),
                    proxy: r
                        .get("proxy")
                        .and_then(Value::as_str)
                        .unwrap_or("")
                        .to_string()
                        .into(),
                })
                .collect()
        })
        .unwrap_or_default()
}

/// Parses one `/connections` entry into `(id, process-name, ConnItem)`.
fn parse_conn(c: &Value) -> Option<(String, String, ConnItem)> {
    let id = c
        .get("id")
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_string();
    let meta = c.get("metadata");
    let name = meta
        .and_then(|m| m.get("process"))
        .and_then(Value::as_str)
        .filter(|s| !s.is_empty())
        .map(|s| s.trim_end_matches(".exe").to_string())
        .or_else(|| {
            meta.and_then(|m| m.get("host"))
                .and_then(Value::as_str)
                .filter(|s| !s.is_empty())
                .map(str::to_string)
        })
        .unwrap_or_else(|| "—".to_string());
    let up = c.get("upload").and_then(Value::as_u64).unwrap_or(0);
    let down = c.get("download").and_then(Value::as_u64).unwrap_or(0);

    let str_meta = |k: &str| meta.and_then(|m| m.get(k)).and_then(Value::as_str);
    let host = str_meta("host")
        .filter(|s| !s.is_empty())
        .map(str::to_string)
        .unwrap_or_else(|| str_meta("destinationIP").unwrap_or("—").to_string());
    let port = str_meta("destinationPort").unwrap_or("");
    let host = if port.is_empty() {
        host
    } else {
        format!("{host}:{port}")
    };
    let network = {
        let n = str_meta("network").unwrap_or("");
        let ty = str_meta("type").unwrap_or("");
        match (n.is_empty(), ty.is_empty()) {
            (false, false) => format!("{} · {}", n.to_uppercase(), ty),
            (false, true) => n.to_uppercase(),
            _ => ty.to_string(),
        }
    };
    let chains = c
        .get("chains")
        .and_then(Value::as_array)
        .map(|a| {
            a.iter()
                .filter_map(Value::as_str)
                .rev()
                .collect::<Vec<_>>()
                .join(" → ")
        })
        .unwrap_or_default();
    let rule = {
        let r = c.get("rule").and_then(Value::as_str).unwrap_or("");
        let payload = c.get("rulePayload").and_then(Value::as_str).unwrap_or("");
        if payload.is_empty() {
            r.to_string()
        } else {
            format!("{r}({payload})")
        }
    };
    let geoip = |k: &str| -> String {
        meta.and_then(|m| m.get(k))
            .and_then(Value::as_array)
            .map(|a| {
                a.iter()
                    .filter_map(Value::as_str)
                    .collect::<Vec<_>>()
                    .join(", ")
            })
            .unwrap_or_default()
    };
    let owned = |k: &str| str_meta(k).unwrap_or("").to_string();
    let item = ConnItem {
        id: id.clone().into(),
        host: host.into(),
        network: network.into(),
        chains: chains.into(),
        rule: rule.into(),
        up,
        down,
        conn_type: owned("type").into(),
        dest_ip: owned("destinationIP").into(),
        dest_port: owned("destinationPort").into(),
        dest_geoip: geoip("destinationGeoIP").into(),
        dest_asn: owned("destinationIPASN").into(),
        src_ip: owned("sourceIP").into(),
        src_port: owned("sourcePort").into(),
        process: owned("process").into(),
        process_path: owned("processPath").into(),
        dns_mode: owned("dnsMode").into(),
    };
    Some((id, name, item))
}

/// Groups `(process-name, item)` pairs by process, summing counts + cumulative
/// up/down bytes. Sorted by total traffic (processes and rows within them).
fn group_items(items: impl Iterator<Item = (String, ConnItem)>) -> Vec<ConnProcess> {
    use std::collections::HashMap;
    let mut map: HashMap<String, ConnProcess> = HashMap::new();
    for (name, item) in items {
        let path = item.process_path.clone();
        let e = map.entry(name.clone()).or_insert_with(|| ConnProcess {
            name: name.clone().into(),
            count: 0,
            up: 0,
            down: 0,
            process_path: SharedString::default(),
            conns: Vec::new(),
        });
        e.count += 1;
        e.up += item.up;
        e.down += item.down;
        if e.process_path.is_empty() && !path.is_empty() {
            e.process_path = path;
        }
        e.conns.push(item);
    }
    let mut out: Vec<ConnProcess> = map.into_values().collect();
    for p in out.iter_mut() {
        p.conns.sort_by_key(|c| std::cmp::Reverse(c.up + c.down));
    }
    out.sort_by_key(|c| std::cmp::Reverse(c.up + c.down));
    out
}

/// Parses the array from `mihomo::groups` into [`ProxyGroup`]s.
pub fn parse_groups(value: &Value) -> Vec<ProxyGroup> {
    let Some(arr) = value.as_array() else {
        return Vec::new();
    };
    arr.iter()
        .map(|g| {
            let name = g
                .get("name")
                .and_then(Value::as_str)
                .unwrap_or("")
                .to_string();
            let kind = g
                .get("type")
                .and_then(Value::as_str)
                .unwrap_or("")
                .to_string();
            let now = g
                .get("now")
                .and_then(Value::as_str)
                .unwrap_or("")
                .to_string();
            let all = g
                .get("all")
                .and_then(Value::as_array)
                .map(|nodes| nodes.iter().map(parse_node).collect())
                .unwrap_or_default();
            ProxyGroup {
                name: name.into(),
                kind: kind.into(),
                now: now.into(),
                all,
            }
        })
        .collect()
}

fn parse_node(node: &Value) -> ProxyNode {
    let name = node
        .get("name")
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_string();
    let kind = node
        .get("type")
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_string();
    let delay = node
        .get("history")
        .and_then(Value::as_array)
        .and_then(|h| h.last())
        .and_then(|last| last.get("delay"))
        .and_then(Value::as_u64)
        .map(|d| d as u32)
        .filter(|d| *d > 0);
    ProxyNode {
        name: name.into(),
        kind: kind.into(),
        delay,
    }
}

fn load_language() -> String {
    let path = dirs::app_config_path();
    std::fs::read_to_string(&path)
        .ok()
        .and_then(|s| serde_yaml::from_str::<Value>(&s).ok())
        .and_then(|v| {
            v.get("language")
                .and_then(|l| l.as_str())
                .map(str::to_string)
        })
        .unwrap_or_else(|| DEFAULT_LANGUAGE.to_string())
}

fn persist_language(lang: &str) {
    let path = dirs::app_config_path();
    let mut value: Value = std::fs::read_to_string(&path)
        .ok()
        .and_then(|s| serde_yaml::from_str(&s).ok())
        .unwrap_or_else(|| serde_json::json!({}));

    if let Some(obj) = value.as_object_mut() {
        obj.insert("language".to_string(), Value::String(lang.to_string()));
    }

    if let Ok(serialized) = serde_yaml::to_string(&value) {
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let _ = std::fs::write(&path, serialized);
    }
}

/// Parses `profile.yaml` (`{current, items: [...]}`) into [`ProfileItem`]s.
pub fn parse_profiles(cfg: &Value) -> Vec<ProfileItem> {
    let current = cfg.get("current").and_then(Value::as_str).unwrap_or("");
    cfg.get("items")
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .map(|it| {
                    let id = it
                        .get("id")
                        .and_then(Value::as_str)
                        .unwrap_or("")
                        .to_string();
                    let name = it
                        .get("name")
                        .and_then(Value::as_str)
                        .filter(|s| !s.is_empty())
                        .unwrap_or(&id)
                        .to_string();
                    let kind = it
                        .get("type")
                        .and_then(Value::as_str)
                        .unwrap_or("remote")
                        .to_string();
                    let is_current = id == current;
                    let extra = it.get("extra");
                    let upload = extra
                        .and_then(|e| e.get("upload"))
                        .and_then(Value::as_u64)
                        .unwrap_or(0);
                    let download = extra
                        .and_then(|e| e.get("download"))
                        .and_then(Value::as_u64)
                        .unwrap_or(0);
                    let total = extra
                        .and_then(|e| e.get("total"))
                        .and_then(Value::as_u64)
                        .unwrap_or(0);
                    let expire = extra
                        .and_then(|e| e.get("expire"))
                        .and_then(Value::as_i64)
                        .unwrap_or(0);
                    ProfileItem {
                        id: id.into(),
                        name: name.into(),
                        kind: kind.into(),
                        is_current,
                        used: upload.saturating_add(download),
                        total,
                        expire,
                    }
                })
                .collect()
        })
        .unwrap_or_default()
}

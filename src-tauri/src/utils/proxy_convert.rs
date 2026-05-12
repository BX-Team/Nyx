const PROXY_SCHEMES: &[&str] = &[
    "vless://",
    "vmess://",
    "ss://",
    "ssr://",
    "trojan://",
    "hysteria://",
    "hysteria2://",
    "hy2://",
    "tuic://",
];

pub fn detect_and_convert_subscription(body: &str) -> String {
    let trimmed = body.trim();

    if looks_like_mihomo_config(trimmed) {
        return body.to_string();
    }

    if let Some(decoded) = try_base64_decode(trimmed) {
        if looks_like_mihomo_config(&decoded) {
            return decoded;
        }
        if looks_like_proxy_list(&decoded) {
            return convert_proxy_list_to_config(&decoded);
        }
    }

    if looks_like_proxy_list(trimmed) {
        return convert_proxy_list_to_config(trimmed);
    }

    body.to_string()
}

fn looks_like_mihomo_config(text: &str) -> bool {
    if let Ok(serde_yaml::Value::Mapping(map)) = serde_yaml::from_str::<serde_yaml::Value>(text) {
        let keys = [
            "proxies",
            "proxy-groups",
            "rules",
            "mixed-port",
            "port",
            "socks-port",
        ];
        return keys
            .iter()
            .any(|k| map.contains_key(serde_yaml::Value::String(k.to_string())));
    }
    false
}

fn is_readable(s: &str) -> bool {
    s.chars()
        .all(|c| !c.is_control() || matches!(c, '\n' | '\r' | '\t'))
}

fn try_b64(cleaned: &str, engine: &impl base64::Engine) -> Option<String> {
    engine
        .decode(cleaned)
        .ok()
        .and_then(|b| String::from_utf8(b).ok())
        .filter(|s| is_readable(s))
}

fn try_base64_decode(s: &str) -> Option<String> {
    use base64::engine::general_purpose::{STANDARD, STANDARD_NO_PAD, URL_SAFE, URL_SAFE_NO_PAD};
    let cleaned = s.replace(['\n', '\r', ' ', '\t'], "");
    try_b64(&cleaned, &STANDARD)
        .or_else(|| try_b64(&cleaned, &URL_SAFE))
        .or_else(|| try_b64(&cleaned, &URL_SAFE_NO_PAD))
        .or_else(|| try_b64(&cleaned, &STANDARD_NO_PAD))
}

fn looks_like_proxy_list(text: &str) -> bool {
    let non_empty: Vec<&str> = text
        .lines()
        .map(|l| l.trim())
        .filter(|l| !l.is_empty() && !l.starts_with('#'))
        .collect();

    if non_empty.is_empty() {
        return false;
    }

    let proxy_count = non_empty
        .iter()
        .filter(|l| PROXY_SCHEMES.iter().any(|s| l.starts_with(s)))
        .count();

    proxy_count > 0 && proxy_count * 2 >= non_empty.len()
}

fn convert_proxy_list_to_config(text: &str) -> String {
    let mut proxies = Vec::new();
    let mut names: Vec<String> = Vec::new();
    let mut seen: std::collections::HashSet<String> = std::collections::HashSet::new();

    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let Some(mut proxy) = parse_proxy_uri(line) else {
            continue;
        };

        let raw_name = proxy
            .get(sv("name"))
            .and_then(|v| v.as_str())
            .unwrap_or("proxy")
            .to_string();

        let unique = if !seen.contains(&raw_name) {
            raw_name.clone()
        } else {
            let mut i = 2u32;
            loop {
                let candidate = format!("{raw_name} {i}");
                if !seen.contains(&candidate) {
                    break candidate;
                }
                i += 1;
            }
        };

        seen.insert(unique.clone());
        proxy.insert(sv("name"), sv(&unique));
        names.push(unique);
        proxies.push(serde_yaml::Value::Mapping(proxy));
    }

    if proxies.is_empty() {
        return text.to_string();
    }

    let mut selector = serde_yaml::Mapping::new();
    selector.insert(sv("name"), sv("Proxy"));
    selector.insert(sv("type"), sv("select"));
    selector.insert(
        sv("proxies"),
        serde_yaml::Value::Sequence(
            std::iter::once(serde_yaml::Value::String("DIRECT".into()))
                .chain(names.iter().map(|n| serde_yaml::Value::String(n.clone())))
                .collect(),
        ),
    );

    let mut config = serde_yaml::Mapping::new();
    config.insert(sv("proxies"), serde_yaml::Value::Sequence(proxies));
    config.insert(
        sv("proxy-groups"),
        serde_yaml::Value::Sequence(vec![serde_yaml::Value::Mapping(selector)]),
    );
    config.insert(
        sv("rules"),
        serde_yaml::Value::Sequence(vec![serde_yaml::Value::String("MATCH,Proxy".into())]),
    );

    serde_yaml::to_string(&serde_yaml::Value::Mapping(config)).unwrap_or_else(|_| text.to_string())
}

// ─── helpers ──────────────────────────────────────────────────────────────

fn sv(s: &str) -> serde_yaml::Value {
    serde_yaml::Value::String(s.to_string())
}

fn percent_decode(s: &str) -> String {
    let mut buf: Vec<u8> = Vec::with_capacity(s.len());
    let src = s.as_bytes();
    let mut i = 0;
    while i < src.len() {
        if src[i] == b'%' && i + 2 < src.len() {
            if let Ok(b) = u8::from_str_radix(&s[i + 1..i + 3], 16) {
                buf.push(b);
                i += 3;
                continue;
            }
        }
        buf.push(src[i]);
        i += 1;
    }
    String::from_utf8_lossy(&buf).into_owned()
}

fn parse_host_port(s: &str) -> Option<(String, u16)> {
    if s.starts_with('[') {
        let end = s.find(']')?;
        let host = s[1..end].to_string();
        let port: u16 = s[end + 1..].strip_prefix(':')?.parse().ok()?;
        return Some((host, port));
    }
    let colon = s.rfind(':')?;
    Some((s[..colon].to_string(), s[colon + 1..].parse().ok()?))
}

fn parse_query(query: &str) -> std::collections::HashMap<String, String> {
    query
        .split('&')
        .filter_map(|pair| pair.split_once('='))
        .map(|(k, v)| (k.trim().to_lowercase(), percent_decode(v)))
        .collect()
}

fn try_b64_any(cleaned: &str, engine: &impl base64::Engine) -> Option<String> {
    engine
        .decode(cleaned)
        .ok()
        .and_then(|b| String::from_utf8(b).ok())
}

fn base64_decode_any(s: &str) -> Option<String> {
    use base64::engine::general_purpose::{STANDARD, STANDARD_NO_PAD, URL_SAFE, URL_SAFE_NO_PAD};
    let cleaned = s.replace(['\n', '\r', ' '], "");
    try_b64_any(&cleaned, &STANDARD)
        .or_else(|| try_b64_any(&cleaned, &URL_SAFE))
        .or_else(|| try_b64_any(&cleaned, &URL_SAFE_NO_PAD))
        .or_else(|| try_b64_any(&cleaned, &STANDARD_NO_PAD))
}

// ─── dispatcher ───────────────────────────────────────────────────────────

fn parse_proxy_uri(uri: &str) -> Option<serde_yaml::Mapping> {
    if uri.starts_with("vless://") {
        parse_vless(uri)
    } else if uri.starts_with("vmess://") {
        parse_vmess(uri)
    } else if uri.starts_with("ss://") {
        parse_ss(uri)
    } else if uri.starts_with("ssr://") {
        parse_ssr(uri)
    } else if uri.starts_with("trojan://") {
        parse_trojan(uri)
    } else if uri.starts_with("hysteria2://") || uri.starts_with("hy2://") {
        parse_hysteria2(uri)
    } else if uri.starts_with("hysteria://") {
        parse_hysteria(uri)
    } else if uri.starts_with("tuic://") {
        parse_tuic(uri)
    } else {
        None
    }
}

// ─── VLESS ────────────────────────────────────────────────────────────────

fn parse_vless(uri: &str) -> Option<serde_yaml::Mapping> {
    let rest = uri.strip_prefix("vless://")?;

    let (before_hash, raw_name) = match rest.rfind('#') {
        Some(i) => (&rest[..i], percent_decode(&rest[i + 1..])),
        None => (rest, String::new()),
    };

    let (before_query, raw_query) = match before_hash.find('?') {
        Some(i) => (&before_hash[..i], &before_hash[i + 1..]),
        None => (before_hash, ""),
    };

    let at = before_query.find('@')?;
    let uuid = &before_query[..at];
    let (host, port) = parse_host_port(&before_query[at + 1..])?;

    let params = parse_query(raw_query);
    let network = params.get("type").map(String::as_str).unwrap_or("tcp");
    let security = params.get("security").map(String::as_str).unwrap_or("none");
    let name = if raw_name.is_empty() {
        format!("{host}:{port}")
    } else {
        raw_name
    };

    let mut p = serde_yaml::Mapping::new();
    p.insert(sv("name"), sv(&name));
    p.insert(sv("type"), sv("vless"));
    p.insert(sv("server"), sv(&host));
    p.insert(sv("port"), serde_yaml::Value::Number(port.into()));
    p.insert(sv("uuid"), sv(uuid));
    p.insert(sv("network"), sv(network));
    p.insert(
        sv("tls"),
        serde_yaml::Value::Bool(matches!(security, "tls" | "reality")),
    );

    if let Some(flow) = params.get("flow").filter(|s| !s.is_empty()) {
        p.insert(sv("flow"), sv(flow));
    }

    if security == "reality" {
        let mut ro = serde_yaml::Mapping::new();
        if let Some(pbk) = params.get("pbk") {
            ro.insert(sv("public-key"), sv(pbk));
        }
        if let Some(sid) = params.get("sid") {
            ro.insert(sv("short-id"), sv(sid));
        }
        if !ro.is_empty() {
            p.insert(sv("reality-opts"), serde_yaml::Value::Mapping(ro));
        }
    }

    if let Some(sni) = params.get("sni").filter(|s| !s.is_empty()) {
        p.insert(sv("servername"), sv(sni));
    }
    if let Some(fp) = params.get("fp").filter(|s| !s.is_empty()) {
        p.insert(sv("client-fingerprint"), sv(fp));
    }
    if let Some(alpn) = params.get("alpn").filter(|s| !s.is_empty()) {
        p.insert(
            sv("alpn"),
            serde_yaml::Value::Sequence(alpn.split(',').map(|a| sv(a.trim())).collect()),
        );
    }

    apply_network_opts(&mut p, network, &params);
    Some(p)
}

// ─── VMESS ────────────────────────────────────────────────────────────────

fn parse_vmess(uri: &str) -> Option<serde_yaml::Mapping> {
    let b64 = uri.strip_prefix("vmess://")?;
    let decoded = base64_decode_any(b64)?;
    let j: serde_json::Value = serde_json::from_str(&decoded).ok()?;

    let name = j["ps"].as_str().unwrap_or("").to_string();
    let host = j["add"].as_str().filter(|s| !s.is_empty())?.to_string();
    let port: u16 = j["port"]
        .as_u64()
        .or_else(|| j["port"].as_str().and_then(|s| s.parse().ok()))
        .unwrap_or(0) as u16;
    let uuid = j["id"].as_str().unwrap_or("").to_string();
    let alter_id = j["aid"]
        .as_u64()
        .or_else(|| j["aid"].as_str().and_then(|s| s.parse().ok()))
        .unwrap_or(0);
    let network = j["net"].as_str().unwrap_or("tcp");
    let tls = j["tls"].as_str().unwrap_or("") == "tls";
    let display = if name.is_empty() {
        format!("{host}:{port}")
    } else {
        name
    };

    let mut p = serde_yaml::Mapping::new();
    p.insert(sv("name"), sv(&display));
    p.insert(sv("type"), sv("vmess"));
    p.insert(sv("server"), sv(&host));
    p.insert(sv("port"), serde_yaml::Value::Number(port.into()));
    p.insert(sv("uuid"), sv(&uuid));
    p.insert(sv("alterId"), serde_yaml::Value::Number(alter_id.into()));
    p.insert(sv("cipher"), sv("auto"));
    p.insert(sv("network"), sv(network));
    p.insert(sv("tls"), serde_yaml::Value::Bool(tls));

    match network {
        "ws" => {
            let path = j["path"].as_str().unwrap_or("/");
            let ws_host = j["host"].as_str().unwrap_or("");
            let mut opts = serde_yaml::Mapping::new();
            opts.insert(sv("path"), sv(path));
            if !ws_host.is_empty() {
                let mut hdrs = serde_yaml::Mapping::new();
                hdrs.insert(sv("Host"), sv(ws_host));
                opts.insert(sv("headers"), serde_yaml::Value::Mapping(hdrs));
            }
            p.insert(sv("ws-opts"), serde_yaml::Value::Mapping(opts));
        }
        "grpc" => {
            let svc = j["path"].as_str().unwrap_or("");
            if !svc.is_empty() {
                let mut opts = serde_yaml::Mapping::new();
                opts.insert(sv("grpc-service-name"), sv(svc));
                p.insert(sv("grpc-opts"), serde_yaml::Value::Mapping(opts));
            }
        }
        "h2" => {
            let path = j["path"].as_str().unwrap_or("/");
            let h2_host = j["host"].as_str().unwrap_or("");
            let mut opts = serde_yaml::Mapping::new();
            opts.insert(sv("path"), sv(path));
            if !h2_host.is_empty() {
                opts.insert(sv("host"), serde_yaml::Value::Sequence(vec![sv(h2_host)]));
            }
            p.insert(sv("h2-opts"), serde_yaml::Value::Mapping(opts));
        }
        _ => {}
    }

    if let Some(sni) = j["sni"].as_str().filter(|s| !s.is_empty()) {
        p.insert(sv("servername"), sv(sni));
    }
    if let Some(fp) = j["fp"].as_str().filter(|s| !s.is_empty()) {
        p.insert(sv("client-fingerprint"), sv(fp));
    }
    if let Some(alpn) = j["alpn"].as_str().filter(|s| !s.is_empty()) {
        p.insert(
            sv("alpn"),
            serde_yaml::Value::Sequence(alpn.split(',').map(|a| sv(a.trim())).collect()),
        );
    }

    Some(p)
}

// ─── SS ───────────────────────────────────────────────────────────────────

fn parse_ss(uri: &str) -> Option<serde_yaml::Mapping> {
    let rest = uri.strip_prefix("ss://")?;

    let (before_hash, raw_name) = match rest.rfind('#') {
        Some(i) => (&rest[..i], percent_decode(&rest[i + 1..])),
        None => (rest, String::new()),
    };

    // Strip plugin params
    let (main, _plugin) = match before_hash.find('?') {
        Some(i) => (&before_hash[..i], &before_hash[i + 1..]),
        None => (before_hash, ""),
    };

    let (method, password, host, port) = if let Some(at) = main.rfind('@') {
        let userinfo = &main[..at];
        let hostport = &main[at + 1..];
        let (host, port) = parse_host_port(hostport)?;
        let (m, pw) = if let Some(c) = userinfo.find(':') {
            (
                percent_decode(&userinfo[..c]),
                percent_decode(&userinfo[c + 1..]),
            )
        } else {
            // base64-encoded userinfo
            let decoded = base64_decode_any(userinfo)?;
            let c = decoded.find(':')?;
            (decoded[..c].to_string(), decoded[c + 1..].to_string())
        };
        (m, pw, host, port)
    } else {
        // Old format: base64(method:password@host:port)
        let decoded = base64_decode_any(main)?;
        let at = decoded.rfind('@')?;
        let (host, port) = parse_host_port(&decoded[at + 1..])?;
        let userinfo = &decoded[..at];
        let c = userinfo.find(':')?;
        (
            userinfo[..c].to_string(),
            userinfo[c + 1..].to_string(),
            host,
            port,
        )
    };

    let name = if raw_name.is_empty() {
        format!("{host}:{port}")
    } else {
        raw_name
    };

    let mut p = serde_yaml::Mapping::new();
    p.insert(sv("name"), sv(&name));
    p.insert(sv("type"), sv("ss"));
    p.insert(sv("server"), sv(&host));
    p.insert(sv("port"), serde_yaml::Value::Number(port.into()));
    p.insert(sv("cipher"), sv(&method));
    p.insert(sv("password"), sv(&password));
    Some(p)
}

// ─── SSR ──────────────────────────────────────────────────────────────────

fn parse_ssr(uri: &str) -> Option<serde_yaml::Mapping> {
    let rest = uri.strip_prefix("ssr://")?;
    let decoded = base64_decode_any(rest)?;

    // server:port:protocol:method:obfs:base64(password)/?params#base64(name)
    let (main, suffix) = match decoded.find("/?") {
        Some(i) => (&decoded[..i], Some(&decoded[i + 2..])),
        None => (decoded.as_str(), None),
    };

    let parts: Vec<&str> = main.splitn(6, ':').collect();
    if parts.len() < 6 {
        return None;
    }
    let host = parts[0].to_string();
    let port: u16 = parts[1].parse().ok()?;
    let protocol = parts[2].to_string();
    let method = parts[3].to_string();
    let obfs = parts[4].to_string();
    let password = base64_decode_any(parts[5]).unwrap_or_else(|| parts[5].to_string());

    let mut raw_name = String::new();
    let mut proto_param = String::new();
    let mut obfs_param = String::new();

    if let Some(sfx) = suffix {
        let (params_str, name_b64) = match sfx.rfind('#') {
            Some(i) => (&sfx[..i], Some(&sfx[i + 1..])),
            None => (sfx, None),
        };
        if let Some(nb) = name_b64 {
            raw_name = base64_decode_any(nb)
                .or_else(|| Some(percent_decode(nb)))
                .unwrap_or_default();
        }
        for pair in params_str.split('&') {
            if let Some((k, v)) = pair.split_once('=') {
                let val = base64_decode_any(v).unwrap_or_else(|| v.to_string());
                match k {
                    "protoparam" => proto_param = val,
                    "obfsparam" => obfs_param = val,
                    _ => {}
                }
            }
        }
    }

    let name = if raw_name.is_empty() {
        format!("{host}:{port}")
    } else {
        raw_name
    };

    let mut p = serde_yaml::Mapping::new();
    p.insert(sv("name"), sv(&name));
    p.insert(sv("type"), sv("ssr"));
    p.insert(sv("server"), sv(&host));
    p.insert(sv("port"), serde_yaml::Value::Number(port.into()));
    p.insert(sv("cipher"), sv(&method));
    p.insert(sv("password"), sv(&password));
    p.insert(sv("protocol"), sv(&protocol));
    p.insert(sv("obfs"), sv(&obfs));
    if !proto_param.is_empty() {
        p.insert(sv("protocol-param"), sv(&proto_param));
    }
    if !obfs_param.is_empty() {
        p.insert(sv("obfs-param"), sv(&obfs_param));
    }
    Some(p)
}

// ─── Trojan ───────────────────────────────────────────────────────────────

fn parse_trojan(uri: &str) -> Option<serde_yaml::Mapping> {
    let rest = uri.strip_prefix("trojan://")?;

    let (before_hash, raw_name) = match rest.rfind('#') {
        Some(i) => (&rest[..i], percent_decode(&rest[i + 1..])),
        None => (rest, String::new()),
    };

    let (before_query, raw_query) = match before_hash.find('?') {
        Some(i) => (&before_hash[..i], &before_hash[i + 1..]),
        None => (before_hash, ""),
    };

    let at = before_query.rfind('@')?;
    let password = percent_decode(&before_query[..at]);
    let (host, port) = parse_host_port(&before_query[at + 1..])?;
    let params = parse_query(raw_query);

    let network = params.get("type").map(String::as_str).unwrap_or("tcp");
    let security = params.get("security").map(String::as_str).unwrap_or("tls");
    let name = if raw_name.is_empty() {
        format!("{host}:{port}")
    } else {
        raw_name
    };

    let mut p = serde_yaml::Mapping::new();
    p.insert(sv("name"), sv(&name));
    p.insert(sv("type"), sv("trojan"));
    p.insert(sv("server"), sv(&host));
    p.insert(sv("port"), serde_yaml::Value::Number(port.into()));
    p.insert(sv("password"), sv(&password));
    p.insert(sv("network"), sv(network));
    p.insert(sv("tls"), serde_yaml::Value::Bool(security != "none"));

    if security == "reality" {
        let mut ro = serde_yaml::Mapping::new();
        if let Some(pbk) = params.get("pbk") {
            ro.insert(sv("public-key"), sv(pbk));
        }
        if let Some(sid) = params.get("sid") {
            ro.insert(sv("short-id"), sv(sid));
        }
        if !ro.is_empty() {
            p.insert(sv("reality-opts"), serde_yaml::Value::Mapping(ro));
        }
    }

    if let Some(sni) = params.get("sni").filter(|s| !s.is_empty()) {
        p.insert(sv("sni"), sv(sni));
    }
    if let Some(fp) = params.get("fp").filter(|s| !s.is_empty()) {
        p.insert(sv("client-fingerprint"), sv(fp));
    }

    apply_network_opts(&mut p, network, &params);
    Some(p)
}

// ─── Hysteria2 ────────────────────────────────────────────────────────────

fn parse_hysteria2(uri: &str) -> Option<serde_yaml::Mapping> {
    let rest = uri
        .strip_prefix("hysteria2://")
        .or_else(|| uri.strip_prefix("hy2://"))?;

    let (before_hash, raw_name) = match rest.rfind('#') {
        Some(i) => (&rest[..i], percent_decode(&rest[i + 1..])),
        None => (rest, String::new()),
    };

    let (before_query, raw_query) = match before_hash.find('?') {
        Some(i) => (&before_hash[..i], &before_hash[i + 1..]),
        None => (before_hash, ""),
    };

    let (password, hostport) = match before_query.find('@') {
        Some(at) => (percent_decode(&before_query[..at]), &before_query[at + 1..]),
        None => (String::new(), before_query),
    };

    let (host, port) = parse_host_port(hostport)?;
    let params = parse_query(raw_query);
    let name = if raw_name.is_empty() {
        format!("{host}:{port}")
    } else {
        raw_name
    };

    let mut p = serde_yaml::Mapping::new();
    p.insert(sv("name"), sv(&name));
    p.insert(sv("type"), sv("hysteria2"));
    p.insert(sv("server"), sv(&host));
    p.insert(sv("port"), serde_yaml::Value::Number(port.into()));
    p.insert(sv("password"), sv(&password));

    if let Some(sni) = params.get("sni").filter(|s| !s.is_empty()) {
        p.insert(sv("sni"), sv(sni));
    }
    if let Some(obfs) = params.get("obfs").filter(|s| !s.is_empty()) {
        p.insert(sv("obfs"), sv(obfs));
        if let Some(op) = params.get("obfs-password").filter(|s| !s.is_empty()) {
            p.insert(sv("obfs-password"), sv(op));
        }
    }

    Some(p)
}

// ─── Hysteria v1 ──────────────────────────────────────────────────────────

fn parse_hysteria(uri: &str) -> Option<serde_yaml::Mapping> {
    let rest = uri.strip_prefix("hysteria://")?;

    let (before_hash, raw_name) = match rest.rfind('#') {
        Some(i) => (&rest[..i], percent_decode(&rest[i + 1..])),
        None => (rest, String::new()),
    };

    let (hostport, raw_query) = match before_hash.find('?') {
        Some(i) => (&before_hash[..i], &before_hash[i + 1..]),
        None => (before_hash, ""),
    };

    let (host, port) = parse_host_port(hostport)?;
    let params = parse_query(raw_query);
    let name = if raw_name.is_empty() {
        format!("{host}:{port}")
    } else {
        raw_name
    };

    let mut p = serde_yaml::Mapping::new();
    p.insert(sv("name"), sv(&name));
    p.insert(sv("type"), sv("hysteria"));
    p.insert(sv("server"), sv(&host));
    p.insert(sv("port"), serde_yaml::Value::Number(port.into()));

    if let Some(auth) = params
        .get("auth")
        .or_else(|| params.get("auth_str"))
        .filter(|s| !s.is_empty())
    {
        p.insert(sv("auth-str"), sv(auth));
    }
    if let Some(up) = params.get("upmbps") {
        p.insert(sv("up"), sv(up));
    }
    if let Some(down) = params.get("downmbps") {
        p.insert(sv("down"), sv(down));
    }
    if let Some(sni) = params.get("peer").filter(|s| !s.is_empty()) {
        p.insert(sv("sni"), sv(sni));
    }
    if let Some(obfs) = params.get("obfs").filter(|s| !s.is_empty()) {
        p.insert(sv("obfs"), sv(obfs));
    }

    Some(p)
}

// ─── TUIC ─────────────────────────────────────────────────────────────────

fn parse_tuic(uri: &str) -> Option<serde_yaml::Mapping> {
    let rest = uri.strip_prefix("tuic://")?;

    let (before_hash, raw_name) = match rest.rfind('#') {
        Some(i) => (&rest[..i], percent_decode(&rest[i + 1..])),
        None => (rest, String::new()),
    };

    let (before_query, raw_query) = match before_hash.find('?') {
        Some(i) => (&before_hash[..i], &before_hash[i + 1..]),
        None => (before_hash, ""),
    };

    let at = before_query.rfind('@')?;
    let userinfo = &before_query[..at];
    let (host, port) = parse_host_port(&before_query[at + 1..])?;
    let params = parse_query(raw_query);

    let (uuid, password) = match userinfo.split_once(':') {
        Some((u, pw)) => (u.to_string(), pw.to_string()),
        None => (userinfo.to_string(), String::new()),
    };

    let name = if raw_name.is_empty() {
        format!("{host}:{port}")
    } else {
        raw_name
    };

    let mut p = serde_yaml::Mapping::new();
    p.insert(sv("name"), sv(&name));
    p.insert(sv("type"), sv("tuic"));
    p.insert(sv("server"), sv(&host));
    p.insert(sv("port"), serde_yaml::Value::Number(port.into()));
    p.insert(sv("uuid"), sv(&uuid));
    p.insert(sv("password"), sv(&password));

    if let Some(sni) = params.get("sni").filter(|s| !s.is_empty()) {
        p.insert(sv("sni"), sv(sni));
    }
    if let Some(alpn) = params.get("alpn").filter(|s| !s.is_empty()) {
        p.insert(
            sv("alpn"),
            serde_yaml::Value::Sequence(alpn.split(',').map(|a| sv(a.trim())).collect()),
        );
    }
    if let Some(cc) = params.get("congestion_control").filter(|s| !s.is_empty()) {
        p.insert(sv("congestion-controller"), sv(cc));
    }

    Some(p)
}

// ─── network transport options ────────────────────────────────────────────

fn apply_network_opts(
    p: &mut serde_yaml::Mapping,
    network: &str,
    params: &std::collections::HashMap<String, String>,
) {
    match network {
        "ws" => {
            let mut opts = serde_yaml::Mapping::new();
            if let Some(path) = params.get("path").filter(|s| !s.is_empty()) {
                opts.insert(sv("path"), sv(path));
            }
            if let Some(host) = params.get("host").filter(|s| !s.is_empty()) {
                let mut hdrs = serde_yaml::Mapping::new();
                hdrs.insert(sv("Host"), sv(host));
                opts.insert(sv("headers"), serde_yaml::Value::Mapping(hdrs));
            }
            if !opts.is_empty() {
                p.insert(sv("ws-opts"), serde_yaml::Value::Mapping(opts));
            }
        }
        "grpc" => {
            let mut opts = serde_yaml::Mapping::new();
            let svc = params
                .get("servicename")
                .or_else(|| params.get("servicename"))
                .filter(|s| !s.is_empty());
            if let Some(svc) = svc {
                opts.insert(sv("grpc-service-name"), sv(svc));
            }
            if !opts.is_empty() {
                p.insert(sv("grpc-opts"), serde_yaml::Value::Mapping(opts));
            }
        }
        "h2" => {
            let mut opts = serde_yaml::Mapping::new();
            if let Some(path) = params.get("path").filter(|s| !s.is_empty()) {
                opts.insert(sv("path"), sv(path));
            }
            if let Some(host) = params.get("host").filter(|s| !s.is_empty()) {
                opts.insert(sv("host"), serde_yaml::Value::Sequence(vec![sv(host)]));
            }
            if !opts.is_empty() {
                p.insert(sv("h2-opts"), serde_yaml::Value::Mapping(opts));
            }
        }
        _ => {}
    }
}

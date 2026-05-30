use axum::{
    extract::Query,
    response::{IntoResponse, Json},
    routing::get,
    Router,
};
use base64::{engine::general_purpose, Engine as _};
use regex::Regex;
use serde::Deserialize;
use serde_json::{json, Value};
use std::net::SocketAddr;
use std::sync::Arc;

#[derive(Deserialize)]
struct SubQuery {
    url: String,
}

#[tokio::main]
async fn main() {
    let app = Router::new().route("/sub", get(handle_sub));

    let port: u16 = std::env::var("PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(3000);

    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    println!("Listening on http://{}", addr);
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

async fn handle_sub(Query(query): Query<SubQuery>) -> impl IntoResponse {
    match fetch_and_build_config(&query.url).await {
        Ok(config) => Json(config).into_response(),
        Err(e) => (axum::http::StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

async fn fetch_and_build_config(url: &str) -> Result<Value, Box<dyn std::error::Error>> {
    // 1. Fetch subscription
    let client = reqwest::Client::builder().user_agent("clash.meta").build()?;
    let resp = client.get(url).send().await?.text().await?;

    let text = match decode_base64(&resp) {
        Ok(decoded) => String::from_utf8_lossy(&decoded).into_owned(),
        Err(_) => resp,
    };

    let mut parsed_nodes = Vec::new();
    let mut node_tags = Vec::new();
    let mut tag_counts = std::collections::HashMap::new();

    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }

        if let Some(mut config) = parse_line(line) {
            if let Some(tag_val) = config.get("tag").and_then(|t| t.as_str()) {
                let base_tag = tag_val.to_string();
                let mut new_tag = base_tag.clone();
                let count = tag_counts.entry(base_tag.clone()).or_insert(0);
                if *count > 0 {
                    new_tag = format!("{} {}", base_tag, *count + 1); // Append number for duplicates
                }
                *count += 1;

                config["tag"] = json!(new_tag);
                node_tags.push(new_tag);
            }
            parsed_nodes.push(config);
        }
    }

    // 2. Read template
    let template_str = std::fs::read_to_string("template.yaml").unwrap_or_else(|_| "{}".to_string());
    let mut config: Value = json5::from_str(&template_str)?;

    // 3. Process outbounds
    if let Some(outbounds) = config.get_mut("outbounds").and_then(|o| o.as_array_mut()) {
        for outbound in outbounds.iter_mut() {
            if let Some(obj) = outbound.as_object_mut() {
                let has_include = obj.contains_key("include");
                let use_all = obj.get("use_all_nodes").and_then(|v| v.as_bool()).unwrap_or(false);

                if has_include || use_all {
                    let include_pattern = obj.get("include").and_then(|v| v.as_str()).unwrap_or(".*");
                    let exclude_pattern = obj.get("exclude").and_then(|v| v.as_str()).unwrap_or("");

                    let inc_regex = Regex::new(include_pattern).unwrap_or_else(|_| Regex::new(".*").unwrap());
                    let exc_regex = if !exclude_pattern.is_empty() {
                        Regex::new(exclude_pattern).ok()
                    } else {
                        None
                    };

                    let mut matched_tags = Vec::new();
                    for tag in &node_tags {
                        if inc_regex.is_match(tag) {
                            let mut excluded = false;
                            if let Some(ref exc) = exc_regex {
                                if exc.is_match(tag) {
                                    excluded = true;
                                }
                            }
                            if !excluded {
                                matched_tags.push(json!(tag));
                            }
                        }
                    }

                    if matched_tags.is_empty() {
                        matched_tags.push(json!("DIRECT"));
                    }

                    // Replace fields with the matched node tags
                    obj.insert("outbounds".to_string(), json!(matched_tags));
                    obj.remove("include");
                    obj.remove("exclude");
                    obj.remove("use_all_nodes");
                }
            }
        }

        // 4. Inject all parsed nodes into outbounds
        for node in parsed_nodes {
            outbounds.push(node);
        }
    }

    Ok(config)
}

fn decode_base64(s: &str) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let mut b64 = s.trim().replace('\n', "").replace('\r', "");
    let pad = b64.len() % 4;
    if pad != 0 {
        b64.push_str(&"=".repeat(4 - pad));
    }
    general_purpose::STANDARD
        .decode(&b64)
        .or_else(|_| general_purpose::URL_SAFE.decode(&b64))
        .map_err(|e| e.into())
}

fn parse_line(line: &str) -> Option<Value> {
    if let Some(rest) = line.strip_prefix("vmess://") {
        parse_vmess(rest)
    } else if let Some(rest) = line.strip_prefix("vless://") {
        parse_vless(rest)
    } else if let Some(rest) = line.strip_prefix("ss://") {
        parse_ss(rest)
    } else if let Some(rest) = line.strip_prefix("trojan://") {
        parse_trojan(rest)
    } else {
        None
    }
}

fn parse_port(v: &Value) -> u16 {
    if let Some(s) = v.as_str() {
        s.parse::<u16>().unwrap_or(0)
    } else if let Some(n) = v.as_u64() {
        n as u16
    } else {
        0
    }
}

fn parse_vmess(b64: &str) -> Option<Value> {
    let decoded = decode_base64(b64).ok()?;
    let text = String::from_utf8_lossy(&decoded);
    let v: Value = serde_json::from_str(&text).ok()?;

    let mut out = json!({
        "type": "vmess",
        "tag": v["ps"].as_str().unwrap_or("vmess"),
        "server": v["add"].as_str().unwrap_or(""),
        "server_port": parse_port(&v["port"]),
        "uuid": v["id"].as_str().unwrap_or(""),
        "security": v["scy"].as_str().unwrap_or("auto"),
        "alter_id": parse_port(&v["aid"]),
    });

    let tls_type = v["tls"].as_str().unwrap_or("");
    if tls_type == "tls" {
        let mut tls = json!({
            "enabled": true,
            "insecure": false
        });
        if let Some(sni) = v["sni"].as_str() {
            if !sni.is_empty() {
                tls["server_name"] = json!(sni);
            }
        }
        if let Some(alpn) = v["alpn"].as_str() {
            if !alpn.is_empty() {
                let alpn_list: Vec<&str> = alpn.split(',').collect();
                tls["alpn"] = json!(alpn_list);
            } else {
                tls["alpn"] = json!(["h2", "http/1.1"]);
            }
        } else {
            tls["alpn"] = json!(["h2", "http/1.1"]);
        }
        out["tls"] = tls;
    }

    let net = v["net"].as_str().unwrap_or("tcp");
    if net == "ws" {
        let mut ws = json!({ "type": "ws" });
        if let Some(path) = v["path"].as_str() {
            if !path.is_empty() {
                ws["path"] = json!(path);
            }
        }
        if let Some(host) = v["host"].as_str() {
            if !host.is_empty() {
                ws["headers"] = json!({ "Host": host });
            }
        }
        out["transport"] = ws;
    } else if net == "grpc" {
        let mut grpc = json!({ "type": "grpc" });
        if let Some(path) = v["path"].as_str() {
            if !path.is_empty() {
                grpc["service_name"] = json!(path);
            }
        }
        out["transport"] = grpc;
    }

    Some(out)
}

fn parse_vless(url_str: &str) -> Option<Value> {
    let full_url = format!("vless://{}", url_str);
    let u = url::Url::parse(&full_url).ok()?;

    let tag = u.fragment().map(|f| urlencoding::decode(f).unwrap_or_else(|_| f.into()).to_string()).unwrap_or_else(|| "vless".to_string());
    let uuid = u.username();
    let server = u.host_str().unwrap_or("");
    let port = u.port().unwrap_or(443);

    let mut out = json!({
        "type": "vless",
        "tag": tag,
        "server": server,
        "server_port": port,
        "uuid": uuid,
    });

    let query_pairs: std::collections::HashMap<_, _> = u.query_pairs().into_owned().collect();

    if let Some(flow) = query_pairs.get("flow") {
        if !flow.is_empty() {
            out["flow"] = json!(flow);
        }
    }

    if let Some(security) = query_pairs.get("security") {
        if security == "tls" || security == "reality" {
            let mut tls = json!({
                "enabled": true,
                "insecure": false
            });
            if let Some(sni) = query_pairs.get("sni") {
                tls["server_name"] = json!(sni);
            }
            if let Some(alpn) = query_pairs.get("alpn") {
                if !alpn.is_empty() {
                    let alpn_list: Vec<&str> = alpn.split(',').collect();
                    tls["alpn"] = json!(alpn_list);
                } else {
                    tls["alpn"] = json!(["h2", "http/1.1"]);
                }
            } else {
                tls["alpn"] = json!(["h2", "http/1.1"]);
            }
            if let Some(fp) = query_pairs.get("fp") {
                tls["utls"] = json!({
                    "enabled": true,
                    "fingerprint": fp
                });
            }
            if security == "reality" {
                let mut reality = json!({
                    "enabled": true,
                });
                if let Some(pbk) = query_pairs.get("pbk") {
                    reality["public_key"] = json!(pbk);
                }
                if let Some(sid) = query_pairs.get("sid") {
                    reality["short_id"] = json!(sid);
                }
                tls["reality"] = reality;
            }
            out["tls"] = tls;
        }
    }

    if let Some(type_) = query_pairs.get("type") {
        if type_ == "ws" {
            let mut ws = json!({ "type": "ws" });
            if let Some(path) = query_pairs.get("path") {
                ws["path"] = json!(path);
            }
            if let Some(host) = query_pairs.get("host") {
                ws["headers"] = json!({ "Host": host });
            }
            out["transport"] = ws;
        } else if type_ == "grpc" {
            let mut grpc = json!({ "type": "grpc" });
            if let Some(service_name) = query_pairs.get("serviceName") {
                grpc["service_name"] = json!(service_name);
            }
            out["transport"] = grpc;
        }
    }

    Some(out)
}

fn parse_trojan(url_str: &str) -> Option<Value> {
    let full_url = format!("trojan://{}", url_str);
    let u = url::Url::parse(&full_url).ok()?;

    let tag = u.fragment().map(|f| urlencoding::decode(f).unwrap_or_else(|_| f.into()).to_string()).unwrap_or_else(|| "trojan".to_string());
    let password = urlencoding::decode(u.username()).unwrap_or_default().to_string();
    let server = u.host_str().unwrap_or("");
    let port = u.port().unwrap_or(443);

    let mut out = json!({
        "type": "trojan",
        "tag": tag,
        "server": server,
        "server_port": port,
        "password": password,
    });

    let query_pairs: std::collections::HashMap<_, _> = u.query_pairs().into_owned().collect();

    if let Some(security) = query_pairs.get("security") {
        if security == "tls" {
            let mut tls = json!({
                "enabled": true,
                "insecure": false
            });
            if let Some(sni) = query_pairs.get("sni") {
                tls["server_name"] = json!(sni);
            }
            if let Some(alpn) = query_pairs.get("alpn") {
                if !alpn.is_empty() {
                    let alpn_list: Vec<&str> = alpn.split(',').collect();
                    tls["alpn"] = json!(alpn_list);
                } else {
                    tls["alpn"] = json!(["h2", "http/1.1"]);
                }
            } else {
                tls["alpn"] = json!(["h2", "http/1.1"]);
            }
            out["tls"] = tls;
        }
    }

    if let Some(type_) = query_pairs.get("type") {
        if type_ == "ws" {
            let mut ws = json!({ "type": "ws" });
            if let Some(path) = query_pairs.get("path") {
                ws["path"] = json!(path);
            }
            if let Some(host) = query_pairs.get("host") {
                ws["headers"] = json!({ "Host": host });
            }
            out["transport"] = ws;
        } else if type_ == "grpc" {
            let mut grpc = json!({ "type": "grpc" });
            if let Some(service_name) = query_pairs.get("serviceName") {
                grpc["service_name"] = json!(service_name);
            }
            out["transport"] = grpc;
        }
    }

    Some(out)
}

fn parse_ss(url_str: &str) -> Option<Value> {
    let full_url = format!("ss://{}", url_str);
    
    if let Ok(u) = url::Url::parse(&full_url) {
        let tag = u.fragment().map(|f| urlencoding::decode(f).unwrap_or_else(|_| f.into()).to_string()).unwrap_or_else(|| "ss".to_string());
        
        let user_info = u.username();
        let mut method = String::new();
        let mut password = String::new();
        
        if let Ok(decoded) = decode_base64(user_info) {
            let decoded_str = String::from_utf8_lossy(&decoded);
            if let Some((m, p)) = decoded_str.split_once(':') {
                method = m.to_string();
                password = p.to_string();
            } else if let Some((m, p)) = user_info.split_once(':') {
                method = m.to_string();
                password = p.to_string();
            }
        } else if let Some((m, p)) = user_info.split_once(':') {
            method = m.to_string();
            password = p.to_string();
        }
        
        let server = u.host_str().unwrap_or("");
        let port = u.port().unwrap_or(0);
        
        if !method.is_empty() && port != 0 {
            return Some(json!({
                "type": "shadowsocks",
                "tag": tag,
                "server": server,
                "server_port": port,
                "method": method,
                "password": password,
            }));
        }
    }
    
    let hash_idx = url_str.find('#').unwrap_or(url_str.len());
    let b64_part = &url_str[..hash_idx];
    let tag = if hash_idx < url_str.len() {
        urlencoding::decode(&url_str[hash_idx + 1..]).unwrap_or_else(|_| url_str[hash_idx + 1..].into()).to_string()
    } else {
        "ss".to_string()
    };
    
    if let Ok(decoded) = decode_base64(b64_part) {
        let decoded_str = String::from_utf8_lossy(&decoded);
        if let Some((auth, addr)) = decoded_str.split_once('@') {
            if let Some((m, p)) = auth.split_once(':') {
                if let Some((server, port_str)) = addr.split_once(':') {
                    if let Ok(port) = port_str.parse::<u16>() {
                        return Some(json!({
                            "type": "shadowsocks",
                            "tag": tag,
                            "server": server,
                            "server_port": port,
                            "method": m,
                            "password": p,
                        }));
                    }
                }
            }
        }
    }
    
    None
}

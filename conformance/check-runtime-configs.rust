#![forbid(unsafe_code)]

use std::collections::BTreeMap;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};

fn fail(message: impl AsRef<str>) -> ! {
    eprintln!("runtime-config conformance failed: {}", message.as_ref());
    std::process::exit(1);
}

fn require(condition: bool, message: impl AsRef<str>) {
    if !condition { fail(message); }
}

fn repo_root() -> PathBuf {
    env::current_dir().unwrap_or_else(|error| fail(format!("cannot resolve current directory: {error}")))
}

fn read_real_file(root: &Path, name: &str) -> String {
    let path = root.join(name);
    let metadata = fs::symlink_metadata(&path)
        .unwrap_or_else(|error| fail(format!("missing {name}: {error}")));
    require(!metadata.file_type().is_symlink(), format!("{name} must not be a symlink"));
    require(metadata.is_file(), format!("{name} must be a regular file"));
    fs::read_to_string(path).unwrap_or_else(|error| fail(format!("cannot read {name}: {error}")))
}

fn strip_comment(line: &str) -> &str {
    let mut in_string = false;
    let mut escaped = false;
    for (index, ch) in line.char_indices() {
        if in_string {
            if escaped { escaped = false; }
            else if ch == '\\' { escaped = true; }
            else if ch == '"' { in_string = false; }
        } else if ch == '"' { in_string = true; }
        else if ch == '#' { return &line[..index]; }
    }
    line
}

fn parse_toml_subset(raw: &str) -> BTreeMap<String, String> {
    let mut out = BTreeMap::new();
    let mut section = String::new();
    let mut array_counts: BTreeMap<String, usize> = BTreeMap::new();
    for (line_number, raw_line) in raw.lines().enumerate() {
        let line = strip_comment(raw_line).trim();
        if line.is_empty() { continue; }
        if line.starts_with("[[") && line.ends_with("]]" ) {
            let base = line[2..line.len() - 2].trim();
            require(!base.is_empty(), format!("empty TOML array section on line {}", line_number + 1));
            let index = array_counts.entry(base.to_owned()).or_insert(0);
            section = format!("{base}#{index}");
            *index += 1;
            continue;
        }
        if line.starts_with('[') && line.ends_with(']') {
            section = line[1..line.len() - 1].trim().to_owned();
            require(!section.is_empty(), format!("empty TOML section on line {}", line_number + 1));
            continue;
        }
        let (key, value) = line.split_once('=')
            .unwrap_or_else(|| fail(format!("unsupported TOML line {}: {line}", line_number + 1)));
        let key = key.trim();
        let value = value.trim();
        require(!key.is_empty() && !value.is_empty(), format!("invalid TOML assignment on line {}", line_number + 1));
        let full = if section.is_empty() { key.to_owned() } else { format!("{section}.{key}") };
        require(out.insert(full.clone(), value.to_owned()).is_none(), format!("duplicate TOML key {full}"));
    }
    out
}

fn toml_string<'a>(map: &'a BTreeMap<String, String>, key: &str) -> &'a str {
    let value = map.get(key).unwrap_or_else(|| fail(format!("missing TOML key {key}")));
    require(value.starts_with('"') && value.ends_with('"') && value.len() >= 2, format!("{key} must be a basic string"));
    &value[1..value.len() - 1]
}

fn toml_bool(map: &BTreeMap<String, String>, key: &str) -> bool {
    match map.get(key).map(String::as_str) {
        Some("true") => true,
        Some("false") => false,
        _ => fail(format!("{key} must be a boolean")),
    }
}

fn toml_i64(map: &BTreeMap<String, String>, key: &str) -> i64 {
    map.get(key).unwrap_or_else(|| fail(format!("missing TOML key {key}")))
        .replace('_', "").parse::<i64>()
        .unwrap_or_else(|error| fail(format!("{key} must be an integer: {error}")))
}

fn toml_string_array(map: &BTreeMap<String, String>, key: &str) -> Vec<String> {
    let value = map.get(key).unwrap_or_else(|| fail(format!("missing TOML key {key}"))).trim();
    require(value.starts_with('[') && value.ends_with(']'), format!("{key} must be an array"));
    let body = value[1..value.len() - 1].trim();
    if body.is_empty() { return Vec::new(); }
    body.split(',').map(|part| {
        let part = part.trim();
        require(part.starts_with('"') && part.ends_with('"') && part.len() >= 2, format!("{key} array values must be strings"));
        part[1..part.len() - 1].to_owned()
    }).collect()
}

fn find_toml_array_entry(map: &BTreeMap<String, String>, base: &str, key: &str, expected: &str) -> String {
    for index in 0..128 {
        let prefix = format!("{base}#{index}");
        let probe = format!("{prefix}.{key}");
        match map.get(&probe) {
            Some(value) if value == &format!("\"{expected}\"") => return prefix,
            Some(_) => continue,
            None => { if index == 0 { fail(format!("missing TOML array section [[{base}]]")); } break; }
        }
    }
    fail(format!("no [[{base}]] entry has {key}={expected:?}"))
}

#[derive(Debug, Clone)]
enum Json { Null, Bool(bool), Number(f64), String(String), Array(Vec<Json>), Object(BTreeMap<String, Json>) }

struct JsonParser<'a> { bytes: &'a [u8], index: usize }

impl<'a> JsonParser<'a> {
    fn new(raw: &'a str) -> Self { Self { bytes: raw.as_bytes(), index: 0 } }
    fn parse(mut self) -> Result<Json, String> {
        let value = self.value()?; self.ws();
        if self.index != self.bytes.len() { return Err(format!("trailing JSON at byte {}", self.index)); }
        Ok(value)
    }
    fn ws(&mut self) { while matches!(self.bytes.get(self.index), Some(b' ' | b'\n' | b'\r' | b'\t')) { self.index += 1; } }
    fn value(&mut self) -> Result<Json, String> {
        self.ws();
        match self.bytes.get(self.index).copied() {
            Some(b'{') => self.object(), Some(b'[') => self.array(), Some(b'"') => self.string().map(Json::String),
            Some(b't') => { self.literal(b"true")?; Ok(Json::Bool(true)) },
            Some(b'f') => { self.literal(b"false")?; Ok(Json::Bool(false)) },
            Some(b'n') => { self.literal(b"null")?; Ok(Json::Null) },
            Some(b'-' | b'0'..=b'9') => self.number().map(Json::Number),
            Some(other) => Err(format!("unexpected JSON byte {other:?} at {}", self.index)), None => Err("unexpected end of JSON".to_owned()),
        }
    }
    fn literal(&mut self, literal: &[u8]) -> Result<(), String> {
        if self.bytes.get(self.index..self.index + literal.len()) == Some(literal) { self.index += literal.len(); Ok(()) }
        else { Err(format!("invalid JSON literal at {}", self.index)) }
    }
    fn object(&mut self) -> Result<Json, String> {
        self.index += 1; let mut out = BTreeMap::new(); self.ws();
        if self.bytes.get(self.index) == Some(&b'}') { self.index += 1; return Ok(Json::Object(out)); }
        loop {
            self.ws(); let key = self.string()?; self.ws();
            if self.bytes.get(self.index) != Some(&b':') { return Err(format!("expected ':' at {}", self.index)); }
            self.index += 1; let value = self.value()?;
            if out.insert(key.clone(), value).is_some() { return Err(format!("duplicate JSON object key {key:?}")); }
            self.ws(); match self.bytes.get(self.index) { Some(b',') => self.index += 1, Some(b'}') => { self.index += 1; break; }, _ => return Err(format!("expected ',' or '}}' at {}", self.index)) }
        }
        Ok(Json::Object(out))
    }
    fn array(&mut self) -> Result<Json, String> {
        self.index += 1; let mut out = Vec::new(); self.ws();
        if self.bytes.get(self.index) == Some(&b']') { self.index += 1; return Ok(Json::Array(out)); }
        loop { out.push(self.value()?); self.ws(); match self.bytes.get(self.index) { Some(b',') => self.index += 1, Some(b']') => { self.index += 1; break; }, _ => return Err(format!("expected ',' or ']' at {}", self.index)) } }
        Ok(Json::Array(out))
    }
    fn string(&mut self) -> Result<String, String> {
        if self.bytes.get(self.index) != Some(&b'"') { return Err(format!("expected JSON string at {}", self.index)); }
        self.index += 1; let mut out = String::new();
        while let Some(byte) = self.bytes.get(self.index).copied() {
            self.index += 1;
            match byte {
                b'"' => return Ok(out),
                b'\\' => {
                    let escape = *self.bytes.get(self.index).ok_or_else(|| "unterminated JSON escape".to_owned())?; self.index += 1;
                    match escape { b'"' => out.push('"'), b'\\' => out.push('\\'), b'/' => out.push('/'), b'b' => out.push('\u{0008}'), b'f' => out.push('\u{000c}'), b'n' => out.push('\n'), b'r' => out.push('\r'), b't' => out.push('\t'), _ => return Err(format!("unsupported JSON escape at {}", self.index - 1)) }
                }
                0x00..=0x1f => return Err("control character in JSON string".to_owned()),
                0x20..=0x7f => out.push(char::from(byte)),
                _ => { let start = self.index - 1; let tail = std::str::from_utf8(&self.bytes[start..]).map_err(|error| error.to_string())?; let ch = tail.chars().next().ok_or_else(|| "invalid UTF-8 in JSON string".to_owned())?; out.push(ch); self.index = start + ch.len_utf8(); }
            }
        }
        Err("unterminated JSON string".to_owned())
    }
    fn number(&mut self) -> Result<f64, String> {
        let start = self.index; while matches!(self.bytes.get(self.index), Some(b'-' | b'+' | b'.' | b'e' | b'E' | b'0'..=b'9')) { self.index += 1; }
        let text = std::str::from_utf8(&self.bytes[start..self.index]).map_err(|error| error.to_string())?;
        text.parse::<f64>().map_err(|error| format!("invalid JSON number {text:?}: {error}"))
    }
}

fn json_at<'a>(value: &'a Json, path: &[&str]) -> &'a Json {
    let mut current = value;
    for key in path { current = match current { Json::Object(object) => object.get(*key).unwrap_or_else(|| fail(format!("missing JSON path {}", path.join(".")))), _ => fail(format!("JSON path {} crosses a non-object", path.join("."))) }; }
    current
}
fn json_string<'a>(value: &'a Json, path: &[&str]) -> &'a str { match json_at(value, path) { Json::String(value) => value, _ => fail(format!("JSON path {} must be a string", path.join("."))) } }
fn json_bool(value: &Json, path: &[&str]) -> bool { match json_at(value, path) { Json::Bool(value) => *value, _ => fail(format!("JSON path {} must be a boolean", path.join("."))) } }
fn json_number(value: &Json, path: &[&str]) -> f64 { match json_at(value, path) { Json::Number(value) => *value, _ => fail(format!("JSON path {} must be a number", path.join("."))) } }
fn json_string_array(value: &Json, path: &[&str]) -> Vec<String> { match json_at(value, path) { Json::Array(values) => values.iter().map(|value| match value { Json::String(value) => value.clone(), _ => fail(format!("JSON path {} must contain only strings", path.join("."))) }).collect(), _ => fail(format!("JSON path {} must be an array", path.join("."))) } }

#[derive(Clone, Copy)]
struct Role { target: &'static str, policy: &'static str, capacity: i64, client_visible: bool, lru_capacity: i64, two_factor_required: bool, write_api: bool, service_name: &'static str }
fn role_for(target: &str) -> Role {
    match target {
        "web" => Role { target: "web", policy: "web-default", capacity: 120, client_visible: true, lru_capacity: 512, two_factor_required: false, write_api: false, service_name: "ores-otel-web-server" },
        "api" => Role { target: "api", policy: "public-default", capacity: 120, client_visible: true, lru_capacity: 512, two_factor_required: false, write_api: true, service_name: "ores-otel-api-server" },
        "admin-web" => Role { target: "admin-web", policy: "admin-default", capacity: 60, client_visible: false, lru_capacity: 256, two_factor_required: true, write_api: false, service_name: "ores-otel-admin-web-server" },
        "admin-api" => Role { target: "admin-api", policy: "admin-default", capacity: 60, client_visible: false, lru_capacity: 256, two_factor_required: true, write_api: true, service_name: "ores-otel-admin-api-server" },
        other => fail(format!("unsupported middleware target role {other:?}")),
    }
}

fn main() {
    let root = repo_root();
    require(!root.join(".auth-shared.toml").exists(), "legacy .auth-shared.toml must not coexist with canonical .shared-auth.toml");
    let mw = parse_toml_subset(&read_real_file(&root, ".ores-mw.toml"));
    let otel = parse_toml_subset(&read_real_file(&root, ".ores-otel.toml"));
    let rate = parse_toml_subset(&read_real_file(&root, ".ores-rl.toml"));
    let lru = parse_toml_subset(&read_real_file(&root, ".ores-lru.toml"));
    let auth = parse_toml_subset(&read_real_file(&root, ".shared-auth.toml"));
    let stack_raw = read_real_file(&root, "config/ores-middleware-stack.json");
    let stack = JsonParser::new(&stack_raw).parse().unwrap_or_else(|error| fail(format!("invalid config/ores-middleware-stack.json: {error}")));

    require(toml_i64(&mw, "schema_version") == 1, ".ores-mw.toml schema_version drift");
    require(toml_string(&mw, "repository_mode") == "server-only", ".ores-mw.toml must remain server-only");
    let role = role_for(toml_string(&mw, "default_target"));
    let target_prefix = find_toml_array_entry(&mw, "targets", "name", role.target);
    require(toml_string(&mw, &format!("{target_prefix}.role")) == "server", "middleware target role must be server");
    require(toml_string(&mw, &format!("{target_prefix}.stack_config")) == "config/ores-middleware-stack.json", "middleware target must bind canonical stack config");

    require(toml_string(&rate, "schemaVersion") == "ores.rate-limit.config.v1", "rate-limit schema drift");
    require(toml_string(&rate, "layout") == "server-only", "rate-limit layout must be server-only");
    require(toml_string(&rate, "defaultPolicyId") == role.policy, "default rate-limit policy drift");
    let policy_prefix = find_toml_array_entry(&rate, "policies", "policyId", role.policy);
    require(toml_bool(&rate, &format!("{policy_prefix}.clientVisible")) == role.client_visible, "rate-limit client visibility drift");
    require(toml_string(&rate, &format!("{policy_prefix}.algorithm")) == "token-bucket", "rate-limit algorithm must be token-bucket");
    require(toml_i64(&rate, &format!("{policy_prefix}.capacity")) == role.capacity, "rate-limit capacity drift");
    require(toml_i64(&rate, &format!("{policy_prefix}.windowMs")) == 0, "token-bucket contract windowMs must remain zero");
    let refill_tokens = toml_i64(&rate, &format!("{policy_prefix}.refillTokens"));
    let refill_interval_ms = toml_i64(&rate, &format!("{policy_prefix}.refillIntervalMs"));
    require(refill_tokens > 0 && refill_interval_ms > 0, "token-bucket refill parameters must be positive");
    require(toml_i64(&rate, &format!("{policy_prefix}.requestCost")) == 1, "request cost must remain one token");
    require(toml_string(&rate, &format!("{policy_prefix}.enforcementMode")) == "enforce", "rate-limit policy must enforce");
    require(toml_string(&rate, &format!("{policy_prefix}.backendFailureMode")) == "fail-closed", "rate-limit backend failure mode must fail closed");
    require(toml_i64(&rate, &format!("{policy_prefix}.maxOvershoot")) == 0, "rate-limit overshoot must remain zero");
    require(toml_string(&rate, &format!("{policy_prefix}.keyVersion")) == "v1", "rate-limit key version drift");

    require(toml_string(&lru, "protocol") == "ores.lru-config.v1", "LRU protocol drift");
    require(toml_i64(&lru, "defaults.capacity") == role.lru_capacity, "LRU capacity drift");
    require(toml_string(&lru, "defaults.syncMode") == "read_only", "LRU sync mode must remain read_only");
    require(!toml_bool(&lru, "defaults.failOpenOnStartup"), "LRU startup must fail closed");

    require(toml_i64(&auth, "schema_version") == 1, "shared-auth schema_version drift");
    require(toml_bool(&auth, "factors.two_factor.required") == role.two_factor_required, "shared-auth 2FA posture drift");
    require(toml_bool(&auth, "factors.three_factor.enabled"), "shared-auth three-factor support must remain enabled");

    require(toml_i64(&otel, "version") == 1, "OTEL config version drift");
    require(toml_bool(&otel, "common.enabled"), "OTEL must remain enabled");
    require(toml_bool(&otel, "common.logging.enabled"), "OTEL logging must remain enabled");
    require(toml_bool(&otel, "common.tracing.enabled"), "OTEL tracing must remain enabled");
    require(toml_bool(&otel, "common.metrics.enabled"), "OTEL metrics must remain enabled");
    require(toml_string(&otel, "server.service_name") == role.service_name, "OTEL service_name drift");
    require(toml_string_array(&otel, "common.tracing.propagators") == vec!["tracecontext".to_owned(), "baggage".to_owned()], "OTEL propagator contract drift");

    require(json_string(&stack, &["contractVersion"]) == "1.0.0", "middleware contract version drift");
    require(json_string(&stack, &["environment"]) == "production", "middleware environment must remain production");
    require(json_string(&stack, &["settings", "requestIdHeader"]) == "x-request-id", "request-id header drift");
    require(json_string(&stack, &["settings", "traceHeader"]) == "traceparent", "trace header drift");
    require(json_bool(&stack, &["settings", "rateLimit", "enabled"]), "middleware rate limiting must remain enabled");
    require(json_string(&stack, &["settings", "rateLimit", "policyId"]) == role.policy, "middleware policyId drift");
    require(json_string(&stack, &["settings", "rateLimit", "algorithm"]) == "token-bucket", "middleware rate-limit algorithm drift");
    require(json_number(&stack, &["settings", "rateLimit", "capacity"]) == role.capacity as f64, "middleware rate-limit capacity drift");
    require(json_string(&stack, &["settings", "rateLimit", "failureMode"]) == "fail-closed", "middleware rate-limit failure mode must fail closed");
    require(json_string(&stack, &["settings", "rateLimit", "keyVersion"]) == toml_string(&rate, &format!("{policy_prefix}.keyVersion")), "middleware/rate-limit keyVersion drift");
    let expected_refill = refill_tokens as f64 * 1000.0 / refill_interval_ms as f64;
    let actual_refill = json_number(&stack, &["settings", "rateLimit", "refillPerSecond"]);
    require((actual_refill - expected_refill).abs() < 1e-12, format!("derived refill rate drift: expected {expected_refill}, got {actual_refill}"));

    require(!json_bool(&stack, &["settings", "testAuthBypass", "enabled"]), "test auth bypass must remain disabled");
    let bypass_header = json_string(&stack, &["settings", "testAuthBypass", "headerName"]);
    require(bypass_header.starts_with("x-ores-") && bypass_header == bypass_header.to_ascii_lowercase(), "test auth bypass header must use canonical lowercase x-ores-* namespace");
    for integration in ["sharedAuth", "optoSync"] {
        if json_string(&stack, &["integrations", integration, "mode"]) == "disabled" {
            require(!json_bool(&stack, &["integrations", integration, "failOpen"]), format!("disabled {integration} integration must fail closed"));
        }
    }
    require(json_bool(&stack, &["integrations", "oresOtel", "enabled"]), "middleware OTEL integration must remain enabled");
    require(json_string(&stack, &["integrations", "oresOtel", "serviceName"]) == role.service_name, "middleware/OTEL service identity drift");
    require(json_string_array(&stack, &["integrations", "oresOtel", "propagators"]) == vec!["tracecontext".to_owned(), "baggage".to_owned()], "middleware OTEL propagator drift");

    require(json_bool(&stack, &["settings", "idempotency", "enabled"]) == role.write_api, "web/API idempotency role drift");
    let required_methods = json_string_array(&stack, &["settings", "idempotency", "requiredMethods"]);
    if role.write_api { require(required_methods == vec!["POST", "PUT", "PATCH"], "API idempotency methods must be POST/PUT/PATCH"); }
    else { require(required_methods.is_empty(), "web idempotency requiredMethods must be empty"); }

    let csp = json_string(&stack, &["settings", "securityHeaders", "contentSecurityPolicy"]);
    require(csp.contains("default-src 'none'"), "CSP must default deny");
    require(csp.contains("frame-ancestors 'none'"), "CSP must deny framing");
    require(csp.contains("base-uri 'none'"), "CSP must deny base-uri");
    if role.write_api { require(!csp.contains("style-src") && !csp.contains("script-src"), "API CSP must not grant browser style/script sources"); }
    else { require(csp.contains("style-src 'self'") && csp.contains("script-src 'self'"), "web CSP must permit only self-hosted style/script"); }

    let representations = json_string_array(&stack, &["settings", "contentRepresentations"]);
    if role.write_api { require(!representations.iter().any(|value| value == "text/html"), "API content representations must not include text/html"); }
    else { require(representations.iter().any(|value| value == "text/html"), "web content representations must include text/html"); }

    println!("runtime-config conformance: ok ({})", role.target);
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn parses_toml_sections_and_array_tables() {
        let parsed = parse_toml_subset("x = 1\n[a]\ny = \"z\"\n[[p]]\nid = \"one\"\n[[p]]\nid = \"two\"\n");
        assert_eq!(toml_i64(&parsed, "x"), 1); assert_eq!(toml_string(&parsed, "a.y"), "z"); assert_eq!(toml_string(&parsed, "p#1.id"), "two");
    }
    #[test]
    fn parses_json_objects_arrays_and_escapes() {
        let parsed = JsonParser::new(r#"{"a":{"b":["x","y"]},"ok":true,"n":2.5}"#).parse().unwrap();
        assert!(json_bool(&parsed, &["ok"])); assert_eq!(json_number(&parsed, &["n"]), 2.5); assert_eq!(json_string_array(&parsed, &["a", "b"]), vec!["x", "y"]);
    }
}

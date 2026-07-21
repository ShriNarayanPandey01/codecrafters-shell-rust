use std::collections::{HashMap, VecDeque};
use std::io::{self, BufRead, BufReader, Read, Write};
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::sync::Mutex;
use std::time::{Duration, Instant};

use crate::engine::ShellEngine;
use crate::shell::shell_context::ShellContext;

pub fn run_server(host: &str, port: u16) -> io::Result<()> {
    let listener = TcpListener::bind((host, port))?;
    let engine = ShellEngine::new();
    let sessions = Mutex::new(HashMap::<String, ShellContext>::new());
    let security = ServerSecurity::from_env()?;

    println!("BYOShell API listening on http://{host}:{port}");

    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                if let Err(error) = handle_connection(stream, &engine, &sessions, &security) {
                    eprintln!("{error}");
                }
            }
            Err(error) => eprintln!("{error}"),
        }
    }

    Ok(())
}

fn handle_connection(
    mut stream: TcpStream,
    engine: &ShellEngine,
    sessions: &Mutex<HashMap<String, ShellContext>>,
    security: &ServerSecurity,
) -> io::Result<()> {
    let mut reader = BufReader::new(stream.try_clone()?);

    let mut request_line = String::new();
    if reader.read_line(&mut request_line)? == 0 {
        return Ok(());
    }

    let request_line = request_line.trim_end().to_string();
    let mut parts = request_line.split_whitespace();
    let method = parts.next().unwrap_or_default();
    let path = parts.next().unwrap_or("/");

    let mut headers = HashMap::new();
    let mut content_length = 0usize;
    loop {
        let mut header = String::new();
        reader.read_line(&mut header)?;
        let header = header.trim_end();

        if header.is_empty() {
            break;
        }

        if let Some((name, value)) = header.split_once(':') {
            let name = name.trim().to_ascii_lowercase();
            let value = value.trim().to_string();

            if name == "content-length" {
                content_length = value.parse::<usize>().unwrap_or(0);
            }

            headers.insert(name, value);
        }
    }

    let mut body = vec![0; content_length];
    if content_length > 0 {
        reader.read_exact(&mut body)?;
    }

    if !(method == "GET" && path == "/health") {
        if !is_authorized(&headers, &security.api_key) {
            return write_response_with_headers(
                &mut stream,
                401,
                &[("WWW-Authenticate", "Bearer")],
                r#"{"error":"unauthorized"}"#.to_string(),
            );
        }

        let client_id = client_identifier(&headers, stream.peer_addr().ok());
        if !security.check_rate_limit(&client_id)? {
            return write_response_with_headers(
                &mut stream,
                429,
                &[("Retry-After", &security.rate_limit_window_secs.to_string())],
                format!(
                    "{{\"error\":\"rate limit exceeded\",\"limit\":{},\"window_seconds\":{}}}",
                    security.rate_limit_max_requests, security.rate_limit_window_secs
                ),
            );
        }
    }

    match (method, path) {
        ("GET", "/health") => write_response(&mut stream, 200, r#"{"status":"ok"}"#.to_string()),
        ("POST", "/execute") => {
            let body = String::from_utf8_lossy(&body).into_owned();
            let (session_id, command) = parse_execute_request(&body);

            if command.trim().is_empty() {
                return write_response(
                    &mut stream,
                    400,
                    r#"{"error":"missing command"}"#.to_string(),
                );
            }

            let mut sessions = sessions
                .lock()
                .map_err(|_| io::Error::other("failed to acquire session lock"))?;
            let session = sessions
                .entry(session_id.clone())
                .or_insert_with(|| engine.new_context());
            let result = engine.execute_line(session, &command);
            let response = format!(
                "{{\"session_id\":\"{}\",\"command\":\"{}\",\"stdout\":\"{}\",\"stderr\":\"{}\",\"exit_code\":{},\"current_dir\":\"{}\",\"should_exit\":{}}}",
                escape_json(&session_id),
                escape_json(&command),
                escape_json(&result.stdout),
                escape_json(&result.stderr),
                result.exit_code,
                escape_json(&result.current_dir),
                if result.should_exit { "true" } else { "false" }
            );

            if result.should_exit {
                sessions.remove(&session_id);
            }

            write_response(&mut stream, 200, response)
        }
        _ => write_response(&mut stream, 404, r#"{"error":"not found"}"#.to_string()),
    }
}

fn parse_execute_request(body: &str) -> (String, String) {
    let trimmed = body.trim();
    if trimmed.starts_with('{') {
        let session_id = extract_json_string_field(trimmed, "session_id")
            .unwrap_or_else(|| "default".to_string());
        let command = extract_json_string_field(trimmed, "command").unwrap_or_default();
        (session_id, command)
    } else {
        ("default".to_string(), body.to_string())
    }
}

fn extract_json_string_field(body: &str, field: &str) -> Option<String> {
    let pattern = format!("\"{field}\"");
    let start = body.find(&pattern)?;
    let after_key = &body[start + pattern.len()..];
    let colon = after_key.find(':')?;
    let after_colon = after_key[colon + 1..].trim_start();
    let quoted = after_colon.strip_prefix('"')?;
    let mut result = String::new();
    let mut chars = quoted.chars();

    while let Some(ch) = chars.next() {
        match ch {
            '"' => return Some(result),
            '\\' => {
                let escaped = chars.next()?;
                match escaped {
                    '"' => result.push('"'),
                    '\\' => result.push('\\'),
                    '/' => result.push('/'),
                    'b' => result.push('\u{0008}'),
                    'f' => result.push('\u{000C}'),
                    'n' => result.push('\n'),
                    'r' => result.push('\r'),
                    't' => result.push('\t'),
                    'u' => {
                        let code: String = chars.by_ref().take(4).collect();
                        let value = u16::from_str_radix(&code, 16).ok()?;
                        result.push(char::from_u32(value as u32)?);
                    }
                    _ => return None,
                }
            }
            _ => result.push(ch),
        }
    }

    None
}

fn write_response(stream: &mut TcpStream, status: u16, body: String) -> io::Result<()> {
    write_response_with_headers(stream, status, &[], body)
}

fn write_response_with_headers(
    stream: &mut TcpStream,
    status: u16,
    headers: &[(&str, &str)],
    body: String,
) -> io::Result<()> {
    let reason = match status {
        200 => "OK",
        401 => "Unauthorized",
        429 => "Too Many Requests",
        400 => "Bad Request",
        404 => "Not Found",
        _ => "Internal Server Error",
    };

    let extra_headers = headers
        .iter()
        .map(|(name, value)| format!("{name}: {value}\r\n"))
        .collect::<String>();
    let response = format!(
        "HTTP/1.1 {status} {reason}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n{}\r\n{}",
        body.len(),
        extra_headers,
        body
    );
    stream.write_all(response.as_bytes())
}

fn escape_json(value: &str) -> String {
    let mut escaped = String::with_capacity(value.len());
    for ch in value.chars() {
        match ch {
            '"' => escaped.push_str("\\\""),
            '\\' => escaped.push_str("\\\\"),
            '\n' => escaped.push_str("\\n"),
            '\r' => escaped.push_str("\\r"),
            '\t' => escaped.push_str("\\t"),
            ch if ch.is_control() => escaped.push_str(&format!("\\u{:04x}", ch as u32)),
            _ => escaped.push(ch),
        }
    }
    escaped
}

struct ServerSecurity {
    api_key: String,
    rate_limit_max_requests: usize,
    rate_limit_window_secs: u64,
    rate_limiter: Mutex<RateLimiter>,
}

impl ServerSecurity {
    fn from_env() -> io::Result<Self> {
        let api_key = std::env::var("BYOSHELL_API_KEY")
            .map_err(|_| io::Error::other("missing BYOSHELL_API_KEY environment variable"))?;
        if api_key.trim().is_empty() {
            return Err(io::Error::other("BYOSHELL_API_KEY cannot be empty"));
        }

        let rate_limit_max_requests = parse_env_usize("BYOSHELL_RATE_LIMIT_MAX_REQUESTS", 60)?;
        let rate_limit_window_secs = parse_env_u64("BYOSHELL_RATE_LIMIT_WINDOW_SECS", 60)?;

        if rate_limit_max_requests == 0 {
            return Err(io::Error::other(
                "BYOSHELL_RATE_LIMIT_MAX_REQUESTS must be greater than 0",
            ));
        }

        if rate_limit_window_secs == 0 {
            return Err(io::Error::other(
                "BYOSHELL_RATE_LIMIT_WINDOW_SECS must be greater than 0",
            ));
        }

        Ok(Self {
            api_key,
            rate_limit_max_requests,
            rate_limit_window_secs,
            rate_limiter: Mutex::new(RateLimiter::new(
                rate_limit_max_requests,
                Duration::from_secs(rate_limit_window_secs),
            )),
        })
    }

    fn check_rate_limit(&self, client_id: &str) -> io::Result<bool> {
        let mut limiter = self
            .rate_limiter
            .lock()
            .map_err(|_| io::Error::other("failed to acquire rate limiter lock"))?;
        Ok(limiter.allow(client_id))
    }
}

struct RateLimiter {
    max_requests: usize,
    window: Duration,
    requests: HashMap<String, VecDeque<Instant>>,
}

impl RateLimiter {
    fn new(max_requests: usize, window: Duration) -> Self {
        Self {
            max_requests,
            window,
            requests: HashMap::new(),
        }
    }

    fn allow(&mut self, key: &str) -> bool {
        self.allow_at(key, Instant::now())
    }

    fn allow_at(&mut self, key: &str, now: Instant) -> bool {
        let cutoff = now.checked_sub(self.window).unwrap_or(now);
        let entries = self.requests.entry(key.to_string()).or_default();

        while matches!(entries.front(), Some(timestamp) if *timestamp <= cutoff) {
            entries.pop_front();
        }

        if entries.len() >= self.max_requests {
            return false;
        }

        entries.push_back(now);
        true
    }
}

fn is_authorized(headers: &HashMap<String, String>, expected_api_key: &str) -> bool {
    if let Some(auth_header) = headers.get("authorization") {
        let mut parts = auth_header.splitn(2, ' ');
        if let (Some(scheme), Some(token)) = (parts.next(), parts.next()) {
            if scheme.eq_ignore_ascii_case("bearer") && token.trim() == expected_api_key {
                return true;
            }
        }
    }

    headers
        .get("x-api-key")
        .is_some_and(|value| value == expected_api_key)
}

fn client_identifier(headers: &HashMap<String, String>, peer_addr: Option<SocketAddr>) -> String {
    if let Some(forwarded_for) = headers.get("x-forwarded-for") {
        if let Some(client_ip) = forwarded_for
            .split(',')
            .map(str::trim)
            .find(|value| !value.is_empty())
        {
            return client_ip.to_string();
        }
    }

    peer_addr
        .map(|addr| addr.ip().to_string())
        .unwrap_or_else(|| "unknown".to_string())
}

fn parse_env_usize(name: &str, default: usize) -> io::Result<usize> {
    match std::env::var(name) {
        Ok(value) => value.parse::<usize>().map_err(|_| {
            io::Error::other(format!("{name} must be a positive integer, got '{value}'"))
        }),
        Err(_) => Ok(default),
    }
}

fn parse_env_u64(name: &str, default: u64) -> io::Result<u64> {
    match std::env::var(name) {
        Ok(value) => value.parse::<u64>().map_err(|_| {
            io::Error::other(format!("{name} must be a positive integer, got '{value}'"))
        }),
        Err(_) => Ok(default),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bearer_auth_is_accepted() {
        let mut headers = HashMap::new();
        headers.insert("authorization".to_string(), "Bearer secret-key".to_string());

        assert!(is_authorized(&headers, "secret-key"));
    }

    #[test]
    fn x_api_key_auth_is_accepted() {
        let mut headers = HashMap::new();
        headers.insert("x-api-key".to_string(), "secret-key".to_string());

        assert!(is_authorized(&headers, "secret-key"));
    }

    #[test]
    fn rate_limiter_blocks_after_limit() {
        let mut limiter = RateLimiter::new(2, Duration::from_secs(60));
        let now = Instant::now();

        assert!(limiter.allow_at("client-1", now));
        assert!(limiter.allow_at("client-1", now + Duration::from_secs(1)));
        assert!(!limiter.allow_at("client-1", now + Duration::from_secs(2)));
        assert!(limiter.allow_at("client-1", now + Duration::from_secs(61)));
    }
}

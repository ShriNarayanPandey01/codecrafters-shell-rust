use std::collections::HashMap;
use std::io::{self, BufRead, BufReader, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::Mutex;

use crate::engine::ShellEngine;
use crate::shell::shell_context::ShellContext;

pub fn run_server(host: &str, port: u16) -> io::Result<()> {
    let listener = TcpListener::bind((host, port))?;
    let engine = ShellEngine::new();
    let sessions = Mutex::new(HashMap::<String, ShellContext>::new());

    println!("BYOShell API listening on http://{host}:{port}");

    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                if let Err(error) = handle_connection(stream, &engine, &sessions) {
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

    let mut content_length = 0usize;
    loop {
        let mut header = String::new();
        reader.read_line(&mut header)?;
        let header = header.trim_end();

        if header.is_empty() {
            break;
        }

        if let Some(value) = header.strip_prefix("Content-Length:") {
            content_length = value.trim().parse::<usize>().unwrap_or(0);
        }
    }

    let mut body = vec![0; content_length];
    if content_length > 0 {
        reader.read_exact(&mut body)?;
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
    let reason = match status {
        200 => "OK",
        400 => "Bad Request",
        404 => "Not Found",
        _ => "Internal Server Error",
    };

    let response = format!(
        "HTTP/1.1 {status} {reason}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
        body.len(),
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

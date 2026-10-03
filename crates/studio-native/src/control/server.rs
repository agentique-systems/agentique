//! The control interface's local endpoint (C-53, ROADMAP §4.16): one JSON
//! object per line over TCP on 127.0.0.1, on a port the system picks. Each
//! request carries the token written, with the port, to the endpoint file
//! (`--control <file>`, in the instance's app data); a request without it
//! is refused. The Orchestrator drives a test instance through it; tests and
//! tools can drive any Studio started with `--control`. A line longer than
//! 1 MB, a connection idle for longer than an answer may take, or more than
//! eight connections at once are refused, so a local process cannot exhaust
//! the Studio's memory or threads before it shows a token.
//!
//! ```text
//! → {"token": "…", "op": "observe", "detail": "full"}
//! ← {"ok": true, "identity": {…}, "screen": "surface", "controls": [ … ], …}
//! → {"token": "…", "op": "act", "agent": "evaluator", "why": "…",
//!    "expect": {"instance": "…"}, "observed": 7,
//!    "action": {"kind": "click", "control": "dialog-confirm"}}
//! ← {"ok": true, "did": "click “dialog-confirm”", "screen": "surface", …}
//! ```

use super::{Reply, Request};
use serde_json::{Value, json};
use std::io::{BufRead, BufReader, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::mpsc::Sender;
use std::time::Duration;

/// How long one request may take (a wait can last up to ten minutes).
const ANSWER: Duration = Duration::from_secs(660);
/// The longest request line.
const LINE: u64 = 1 << 20;
/// How long a connection may wait before its first request with the token.
const FIRST: Duration = Duration::from_secs(10);
/// Connections at once.
const CONNECTIONS: usize = 8;

/// A running endpoint.
#[derive(Clone, Debug)]
pub struct Endpoint {
    pub port: u16,
    pub file: PathBuf,
}

/// `bytes` random bytes as hex, from the operating system's randomness.
pub fn random_hex(bytes: usize) -> String {
    let mut buffer = vec![0u8; bytes];
    getrandom::fill(&mut buffer).expect("the operating system provides randomness");
    buffer.iter().map(|b| format!("{b:02x}")).collect()
}

/// Compares two tokens in time that does not depend on where they differ.
fn same(a: &str, b: &str) -> bool {
    a.len() == b.len()
        && a.bytes()
            .zip(b.bytes())
            .fold(0u8, |acc, (x, y)| acc | (x ^ y))
            == 0
}

/// Starts the endpoint and writes its file (port, token, process, instance).
pub fn start(file: &Path, instance: &str, sender: Sender<Request>) -> Result<Endpoint, String> {
    let listener = TcpListener::bind(("127.0.0.1", 0))
        .map_err(|e| format!("the control endpoint could not start: {e}"))?;
    let port = listener.local_addr().map_err(|e| e.to_string())?.port();
    let token = random_hex(32);
    let text = json!({
        "port": port,
        "token": token,
        "pid": std::process::id(),
        "instance": instance,
        "version": env!("CARGO_PKG_VERSION"),
    })
    .to_string();
    if let Some(parent) = file.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    agq_launcher::write_atomically(file, text.as_bytes())?;
    // The token is for this user only (Windows: the app data's own
    // permissions).
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = std::fs::set_permissions(file, std::fs::Permissions::from_mode(0o600));
    }
    let accepted = token.clone();
    let open = Arc::new(AtomicUsize::new(0));
    std::thread::Builder::new()
        .name("agentique-control".into())
        .spawn(move || {
            for stream in listener.incoming().flatten() {
                if open.load(Ordering::SeqCst) >= CONNECTIONS {
                    let mut stream = stream;
                    let _ = writeln!(
                        stream,
                        "{}",
                        json!({ "ok": false, "error": "refused: too many connections" })
                    );
                    continue;
                }
                open.fetch_add(1, Ordering::SeqCst);
                let sender = sender.clone();
                let token = accepted.clone();
                let held = open.clone();
                let started = std::thread::Builder::new()
                    .name("agentique-control-connection".into())
                    .spawn(move || {
                        serve(stream, &token, sender);
                        held.fetch_sub(1, Ordering::SeqCst);
                    });
                if started.is_err() {
                    open.fetch_sub(1, Ordering::SeqCst);
                }
            }
        })
        .map_err(|e| e.to_string())?;
    Ok(Endpoint {
        port,
        file: file.to_path_buf(),
    })
}

fn serve(stream: TcpStream, token: &str, sender: Sender<Request>) {
    let Ok(mut writer) = stream.try_clone() else {
        return;
    };
    if stream.set_read_timeout(Some(FIRST)).is_err() {
        return;
    }
    let Ok(timeouts) = stream.try_clone() else {
        return;
    };
    let mut trusted = false;
    let mut reader = BufReader::new(stream);
    loop {
        let mut line = String::new();
        match (&mut reader).take(LINE).read_line(&mut line) {
            Ok(0) | Err(_) => return,
            Ok(_) if !line.ends_with('\n') && line.len() as u64 >= LINE => {
                let _ = writeln!(
                    writer,
                    "{}",
                    json!({ "ok": false, "error": "refused: the request is longer than 1 MB" })
                );
                return;
            }
            Ok(_) => {}
        }
        if line.trim().is_empty() {
            continue;
        }
        let answer = match serde_json::from_str::<Value>(&line) {
            Err(error) => json!({ "ok": false, "error": format!("not JSON: {error}") }),
            Ok(body) if !body["token"].as_str().is_some_and(|t| same(t, token)) => {
                json!({ "ok": false, "error": "refused: wrong or missing token" })
            }
            Ok(mut body) => {
                if !trusted {
                    // Shown the token: it may now wait as long as an answer.
                    trusted = true;
                    let _ = timeouts.set_read_timeout(Some(ANSWER));
                }
                if let Some(object) = body.as_object_mut() {
                    object.remove("token");
                }
                // The in-Studio Assistant's name is its own: an endpoint
                // client cannot cancel or speak as it.
                if body["agent"] == "Assistant" {
                    body["agent"] = json!("Assistant (endpoint)");
                }
                let id = body["id"].clone();
                let (reply, answer) = std::sync::mpsc::channel();
                let request =
                    Request::new(body, Reply::Channel(reply), ANSWER - Duration::from_secs(5));
                if sender.send(request).is_err() {
                    return;
                }
                let mut answer = answer.recv_timeout(ANSWER).unwrap_or_else(
                    |_| json!({ "ok": false, "error": "the Studio did not answer in time" }),
                );
                if !id.is_null() {
                    answer["id"] = id;
                }
                answer
            }
        };
        if writeln!(writer, "{answer}")
            .and_then(|_| writer.flush())
            .is_err()
        {
            return;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_endpoint_answers_only_with_its_token() {
        let dir = std::env::temp_dir().join(format!("agq-control-{}", std::process::id()));
        let file = dir.join("control.json");
        let (sender, requests) = std::sync::mpsc::channel::<Request>();
        let endpoint = start(&file, "test-instance", sender).unwrap();
        let written: Value =
            serde_json::from_str(&std::fs::read_to_string(&file).unwrap()).unwrap();
        assert_eq!(written["port"], endpoint.port);
        let token = written["token"].as_str().unwrap().to_string();
        assert_eq!(token.len(), 64);
        // A stand-in Studio: answers each request with what it was asked.
        std::thread::spawn(move || {
            for request in requests {
                let op = request.body["op"].clone();
                let token = request.body.get("token").cloned();
                request
                    .reply
                    .send(json!({ "ok": true, "op": op, "sawToken": token.is_some() }));
            }
        });
        let ask = |line: Value| {
            let mut stream = TcpStream::connect(("127.0.0.1", endpoint.port)).unwrap();
            writeln!(stream, "{line}").unwrap();
            let mut answer = String::new();
            BufReader::new(stream).read_line(&mut answer).unwrap();
            serde_json::from_str::<Value>(&answer).unwrap()
        };
        let answer = ask(json!({ "op": "observe", "id": 7, "token": token }));
        // The token never reaches the Studio's handlers.
        assert_eq!(
            answer,
            json!({ "ok": true, "op": "observe", "sawToken": false, "id": 7 })
        );
        // Without the token, nothing reaches the Studio.
        let refused = ask(json!({ "op": "act", "token": "guess" }));
        assert_eq!(refused["ok"], false);
        assert!(refused["error"].as_str().unwrap().contains("refused"));
        assert_ne!(random_hex(32), random_hex(32));
        // A line without end is refused at 1 MB, not read into memory
        // (exactly 1 MB is sent, so nothing unread makes the connection
        // reset before the answer arrives).
        let mut stream = TcpStream::connect(("127.0.0.1", endpoint.port)).unwrap();
        let chunk = vec![b'x'; 64 * 1024];
        for _ in 0..16 {
            stream.write_all(&chunk).unwrap();
        }
        let mut answer = String::new();
        let _ = BufReader::new(stream).read_line(&mut answer);
        assert!(answer.contains("longer than 1 MB"), "{answer}");
        let _ = std::fs::remove_dir_all(dir);
    }
}

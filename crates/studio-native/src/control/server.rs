//! The control interface's local endpoint (C-53, ROADMAP §4.16): one JSON
//! object per line over TCP on 127.0.0.1, on a port the system picks. Each
//! request carries the token written, with the port, to the endpoint file
//! (`--control <file>`, in the instance's app data); a request without it
//! is refused. The Orchestrator drives a test instance through it; tests and
//! tools can drive any Studio started with `--control`.
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
use std::io::{BufRead, BufReader, Write};
use std::net::{TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::sync::mpsc::Sender;
use std::time::Duration;

/// How long one request may take (a wait can last up to ten minutes).
const ANSWER: Duration = Duration::from_secs(660);

/// A running endpoint.
#[derive(Clone, Debug)]
pub struct Endpoint {
    pub port: u16,
    pub file: PathBuf,
}

/// `bytes` random bytes as hex, from the operating system's randomness
/// (each `RandomState` is seeded by it).
pub fn random_hex(bytes: usize) -> String {
    use std::collections::hash_map::RandomState;
    use std::hash::{BuildHasher, Hasher};
    let mut out = String::new();
    while out.len() < bytes * 2 {
        let mut hasher = RandomState::new().build_hasher();
        hasher.write_u128(
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0),
        );
        out.push_str(&format!("{:016x}", hasher.finish()));
    }
    out.truncate(bytes * 2);
    out
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
    let accepted = token.clone();
    std::thread::Builder::new()
        .name("agentique-control".into())
        .spawn(move || {
            for stream in listener.incoming().flatten() {
                let sender = sender.clone();
                let token = accepted.clone();
                let _ = std::thread::Builder::new()
                    .name("agentique-control-connection".into())
                    .spawn(move || serve(stream, &token, sender));
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
    for line in BufReader::new(stream).lines() {
        let Ok(line) = line else { return };
        if line.trim().is_empty() {
            continue;
        }
        let answer = match serde_json::from_str::<Value>(&line) {
            Err(error) => json!({ "ok": false, "error": format!("not JSON: {error}") }),
            Ok(body) if !body["token"].as_str().is_some_and(|t| same(t, token)) => {
                json!({ "ok": false, "error": "refused: wrong or missing token" })
            }
            Ok(mut body) => {
                if let Some(object) = body.as_object_mut() {
                    object.remove("token");
                }
                let id = body["id"].clone();
                let (reply, answer) = std::sync::mpsc::channel();
                if sender
                    .send(Request {
                        body,
                        reply: Reply::Channel(reply),
                    })
                    .is_err()
                {
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
        let _ = std::fs::remove_dir_all(dir);
    }
}

//! Running allow-listed commands: to completion with a timeout, or
//! interactively (the harness). Cancelling or timing out ends the whole
//! process tree. Output is captured, capped, and never contains the
//! environment's secrets because the environment never had them.

use crate::Refusal;
use std::io::{BufRead, BufReader, Read, Write};
use std::path::Path;
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{Receiver, RecvTimeoutError, channel};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

/// Captured output beyond this is cut, with a note.
const OUTPUT_LIMIT: usize = 2 * 1024 * 1024;

/// A command: a program and its arguments.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Program {
    pub name: String,
    pub args: Vec<String>,
}

/// Cargo subcommands a task may run: build, check, test, run (the harness),
/// metadata, and the formatter and linter. Nothing that publishes, installs
/// or changes the toolchain.
const CARGO_ALLOWED: [&str; 8] = [
    "build",
    "check",
    "test",
    "run",
    "metadata",
    "fmt",
    "clippy",
    "--version",
];

impl Program {
    pub fn new(name: &str, args: &[&str]) -> Program {
        Program {
            name: name.to_string(),
            args: args.iter().map(|a| a.to_string()).collect(),
        }
    }

    pub fn cargo(args: &[&str]) -> Program {
        Program::new("cargo", args)
    }

    /// A command written as a list (from `links.json`).
    pub fn from_list(list: &[String]) -> Option<Program> {
        let (name, args) = list.split_first()?;
        Some(Program {
            name: name.clone(),
            args: args.to_vec(),
        })
    }

    /// The command as the Operator reads it.
    pub fn display(&self) -> String {
        std::iter::once(self.name.as_str())
            .chain(self.args.iter().map(String::as_str))
            .collect::<Vec<_>>()
            .join(" ")
    }

    pub(crate) fn check(&self) -> Result<(), Refusal> {
        let allowed = self.name == "cargo"
            && self
                .args
                .first()
                .is_some_and(|sub| CARGO_ALLOWED.contains(&sub.as_str()))
            && !self.args.iter().any(|a| {
                a.starts_with("--config")
                    || a.starts_with("-Z")
                    || a.starts_with("+")
                    || a.starts_with("--manifest-path")
                    || a == "--fix"
            });
        if allowed {
            Ok(())
        } else {
            Err(Refusal::NotAllowed(self.display()))
        }
    }
}

/// How a command ended.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Finished {
    pub code: Option<i32>,
    pub success: bool,
    pub stdout: String,
    pub stderr: String,
    pub timed_out: bool,
    pub cancelled: bool,
    pub duration: Duration,
}

impl Finished {
    /// One line for the Operator.
    pub fn summary(&self) -> String {
        if self.cancelled {
            "cancelled".into()
        } else if self.timed_out {
            format!("stopped after {:?} (timeout)", self.duration)
        } else if self.success {
            format!("succeeded in {:.1} s", self.duration.as_secs_f64())
        } else {
            format!(
                "failed (exit code {}) in {:.1} s",
                self.code.map_or("none".into(), |c| c.to_string()),
                self.duration.as_secs_f64()
            )
        }
    }
}

fn command(program: &Program, cwd: &Path, env: &[(String, String)]) -> Command {
    let mut command = Command::new(&program.name);
    command
        .args(&program.args)
        .current_dir(cwd)
        .env_clear()
        .envs(env.iter().map(|(k, v)| (k.as_str(), v.as_str())));
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        command.creation_flags(CREATE_NO_WINDOW);
    }
    command
}

/// Ends a process and every process it started.
pub fn kill_tree(child: &mut Child) {
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        let _ = Command::new("taskkill")
            .args(["/T", "/F", "/PID", &child.id().to_string()])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .creation_flags(0x0800_0000)
            .status();
    }
    let _ = child.kill();
    let _ = child.wait();
}

fn capture(mut reader: impl Read + Send + 'static) -> Arc<Mutex<Vec<u8>>> {
    let buffer = Arc::new(Mutex::new(Vec::new()));
    let sink = buffer.clone();
    std::thread::spawn(move || {
        let mut chunk = [0u8; 8192];
        while let Ok(n) = reader.read(&mut chunk) {
            if n == 0 {
                break;
            }
            let mut out = sink.lock().expect("output buffer");
            if out.len() < OUTPUT_LIMIT {
                let room = OUTPUT_LIMIT - out.len();
                out.extend_from_slice(&chunk[..n.min(room)]);
            }
        }
    });
    buffer
}

fn text(buffer: &Arc<Mutex<Vec<u8>>>) -> String {
    let bytes = buffer.lock().expect("output buffer").clone();
    let mut text = String::from_utf8_lossy(&bytes).into_owned();
    if bytes.len() >= OUTPUT_LIMIT {
        text.push_str("\n[output cut at 2 MB]");
    }
    text
}

/// Runs to completion, the timeout or cancellation.
pub(crate) fn run(
    program: &Program,
    cwd: &Path,
    env: &[(String, String)],
    timeout: Duration,
    cancel: &AtomicBool,
) -> Result<Finished, Refusal> {
    if cancel.load(Ordering::SeqCst) {
        return Ok(Finished {
            code: None,
            success: false,
            stdout: String::new(),
            stderr: String::new(),
            timed_out: false,
            cancelled: true,
            duration: Duration::ZERO,
        });
    }
    let started = Instant::now();
    let mut child = command(program, cwd, env)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| Refusal::Io(format!("`{}` could not start: {e}", program.display())))?;
    let stdout = capture(child.stdout.take().expect("piped"));
    let stderr = capture(child.stderr.take().expect("piped"));
    let (mut timed_out, mut cancelled) = (false, false);
    let status = loop {
        match child.try_wait() {
            Ok(Some(status)) => break Some(status),
            Ok(None) => {}
            Err(e) => return Err(Refusal::Io(e.to_string())),
        }
        if cancel.load(Ordering::SeqCst) {
            cancelled = true;
            kill_tree(&mut child);
            break None;
        }
        if started.elapsed() > timeout {
            timed_out = true;
            kill_tree(&mut child);
            break None;
        }
        std::thread::sleep(Duration::from_millis(15));
    };
    // Let the readers drain what is left.
    std::thread::sleep(Duration::from_millis(20));
    Ok(Finished {
        code: status.and_then(|s| s.code()),
        success: status.is_some_and(|s| s.success()),
        stdout: text(&stdout),
        stderr: text(&stderr),
        timed_out,
        cancelled,
        duration: started.elapsed(),
    })
}

/// A command that talks over its standard input and output, one line at a
/// time. Dropping it ends the process tree.
pub struct Interactive {
    child: Child,
    stdin: Option<ChildStdin>,
    lines: Receiver<String>,
    stderr: Arc<Mutex<Vec<u8>>>,
    cancel: Arc<AtomicBool>,
}

pub(crate) fn spawn(
    program: &Program,
    cwd: &Path,
    env: &[(String, String)],
    cancel: Arc<AtomicBool>,
) -> Result<Interactive, Refusal> {
    let mut child = command(program, cwd, env)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| Refusal::Io(format!("`{}` could not start: {e}", program.display())))?;
    let stdout = child.stdout.take().expect("piped");
    let stderr = capture(child.stderr.take().expect("piped"));
    let (sender, lines) = channel();
    std::thread::spawn(move || {
        for line in BufReader::new(stdout).lines() {
            let Ok(line) = line else { break };
            if sender.send(line).is_err() {
                break;
            }
        }
    });
    Ok(Interactive {
        stdin: child.stdin.take(),
        child,
        lines,
        stderr,
        cancel,
    })
}

impl Interactive {
    /// Sends one line.
    pub fn send(&mut self, line: &str) -> Result<(), String> {
        let stdin = self.stdin.as_mut().ok_or("the process's input is closed")?;
        writeln!(stdin, "{line}")
            .and_then(|_| stdin.flush())
            .map_err(|e| format!("the process stopped reading: {e}"))
    }

    /// The next line, within `timeout`. `Err` when the process ended, the
    /// time ran out or the run was cancelled.
    pub fn read(&mut self, timeout: Duration) -> Result<String, String> {
        let started = Instant::now();
        loop {
            if self.cancel.load(Ordering::SeqCst) {
                return Err("cancelled".into());
            }
            match self.lines.recv_timeout(Duration::from_millis(25)) {
                Ok(line) => return Ok(line),
                Err(RecvTimeoutError::Timeout) if started.elapsed() < timeout => {}
                Err(RecvTimeoutError::Timeout) => {
                    return Err(format!("no answer within {:?}", timeout));
                }
                Err(RecvTimeoutError::Disconnected) => {
                    std::thread::sleep(Duration::from_millis(20));
                    return Err(format!(
                        "the process ended{}",
                        match self.error_output().trim() {
                            "" => String::new(),
                            e => format!(": {}", last_lines(e, 12)),
                        }
                    ));
                }
            }
        }
    }

    /// What the process wrote to its error output so far.
    pub fn error_output(&self) -> String {
        text(&self.stderr)
    }

    /// Ends the process tree.
    pub fn kill(&mut self) {
        self.stdin = None;
        kill_tree(&mut self.child);
    }
}

impl Drop for Interactive {
    fn drop(&mut self) {
        self.kill();
    }
}

/// The last `n` lines of a text.
pub fn last_lines(text: &str, n: usize) -> String {
    let lines: Vec<&str> = text.lines().collect();
    lines[lines.len().saturating_sub(n)..].join("\n")
}

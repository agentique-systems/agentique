//! Keys (R-25, ROADMAP §4.9): stored in the Windows Credential Manager, one
//! generic credential per provider (target `agentique:<provider>`,
//! persistence Local) and one for the Operator's Claude subscription token
//! (`agentique:anthropic-subscription`, C-54), never in a file, a log, the project or the System
//! State. A non-empty environment variable wins over the stored key. One
//! thread owns all credential access, with a timeout per call, because the
//! store does not reliably order operations on one entry from different
//! threads [46]. Without a credential store (not Windows) keys come only
//! from the environment; there is no plain-text fallback.

use crate::Credential;
use std::sync::mpsc::{self, Sender};
use std::sync::{Mutex, OnceLock};
use std::time::Duration;

/// How long one credential operation may take.
const TIMEOUT: Duration = Duration::from_secs(5);

/// Why a key could not be stored, read or removed, in plain words.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KeyError(pub String);

impl std::fmt::Display for KeyError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

enum Command {
    Get(Credential, Sender<Result<Option<String>, KeyError>>),
    Set(Credential, String, Sender<Result<(), KeyError>>),
    Delete(Credential, Sender<Result<(), KeyError>>),
}

/// The credential thread's inbox.
fn inbox() -> &'static Mutex<Sender<Command>> {
    static INBOX: OnceLock<Mutex<Sender<Command>>> = OnceLock::new();
    INBOX.get_or_init(|| {
        let (sender, commands) = mpsc::channel::<Command>();
        std::thread::Builder::new()
            .name("agq-credentials".into())
            .spawn(move || {
                let store = open_store();
                for command in commands {
                    match command {
                        Command::Get(credential, reply) => {
                            let _ = reply.send(
                                store
                                    .as_ref()
                                    .map_err(Clone::clone)
                                    .and_then(|_| get(credential)),
                            );
                        }
                        Command::Set(credential, key, reply) => {
                            let _ = reply.send(
                                store
                                    .as_ref()
                                    .map_err(Clone::clone)
                                    .and_then(|_| set(credential, &key)),
                            );
                        }
                        Command::Delete(credential, reply) => {
                            let _ = reply.send(
                                store
                                    .as_ref()
                                    .map_err(Clone::clone)
                                    .and_then(|_| delete(credential)),
                            );
                        }
                    }
                }
            })
            .expect("the credential thread starts");
        Mutex::new(sender)
    })
}

fn ask<T>(command: impl FnOnce(Sender<Result<T, KeyError>>) -> Command) -> Result<T, KeyError> {
    let (reply, answer) = mpsc::channel();
    inbox()
        .lock()
        .map_err(|_| KeyError("The credential store is not available.".into()))?
        .send(command(reply))
        .map_err(|_| KeyError("The credential store is not available.".into()))?;
    answer.recv_timeout(TIMEOUT).unwrap_or_else(|_| {
        Err(KeyError(
            "The Windows Credential Manager did not answer in time. Try again, or set the key's environment variable.".into(),
        ))
    })
}

/// The stored key (or token), if any; an error when the store cannot be
/// read (R-25 point 5: say so, never guess).
pub(crate) fn stored(credential: impl Into<Credential>) -> Result<Option<String>, KeyError> {
    let credential = credential.into();
    ask(|reply| Command::Get(credential, reply))
}

/// Stores `key` for a provider (or the subscription token) in the Windows
/// Credential Manager.
pub fn store(credential: impl Into<Credential>, key: &str) -> Result<(), KeyError> {
    let credential = credential.into();
    let key = key.trim();
    if key.is_empty() {
        return Err(KeyError("The key is empty.".into()));
    }
    // CREDENTIALW holds at most 2,560 bytes of secret [47]; the store
    // writes UTF-16.
    if key.encode_utf16().count() * 2 > 2_560 {
        return Err(KeyError(
            "The key is longer than the Credential Manager holds (2,560 bytes).".into(),
        ));
    }
    ask(|reply| Command::Set(credential, key.to_string(), reply))
}

/// Removes a stored key (or token); removing one that is not there is not
/// an error.
pub fn remove(credential: impl Into<Credential>) -> Result<(), KeyError> {
    let credential = credential.into();
    ask(|reply| Command::Delete(credential, reply))
}

/// What Settings shows instead of a key: its fixed prefix and last four
/// characters (`sk-ant-…a1B2`, R-25); a short key shows nothing of itself.
pub fn hint(key: &str) -> String {
    let key = key.trim();
    if key.chars().count() < 16 {
        return "…".to_string();
    }
    let prefix: String = key
        .split_inclusive('-')
        .take_while(|part| part.ends_with('-') && part.len() <= 5)
        .collect();
    let tail: String = key
        .chars()
        .rev()
        .take(4)
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect();
    format!("{prefix}…{tail}")
}

#[cfg(windows)]
fn open_store() -> Result<(), KeyError> {
    let store = windows_native_keyring_store::Store::new().map_err(|error| {
        KeyError(format!(
            "The Windows Credential Manager could not be opened ({error}). Set the key's environment variable instead."
        ))
    })?;
    keyring_core::set_default_store(store);
    Ok(())
}

#[cfg(not(windows))]
fn open_store() -> Result<(), KeyError> {
    Err(KeyError(
        "Keys are stored in the Windows Credential Manager, which this system does not have. Set the key's environment variable instead.".into(),
    ))
}

fn entry(credential: Credential) -> Result<keyring_core::Entry, KeyError> {
    let user = credential.id();
    let target = format!("agentique:{user}");
    let modifiers =
        std::collections::HashMap::from([("target", target.as_str()), ("persistence", "Local")]);
    keyring_core::Entry::new_with_modifiers("agentique", &user, &modifiers).map_err(|error| {
        KeyError(format!(
            "The credential for the {} could not be addressed ({error}).",
            credential.name()
        ))
    })
}

fn get(credential: Credential) -> Result<Option<String>, KeyError> {
    match entry(credential)?.get_password() {
        Ok(key) => Ok(Some(key)),
        Err(keyring_core::Error::NoEntry) => Ok(None),
        Err(error) => Err(KeyError(format!(
            "The stored {} could not be read ({error}).",
            credential.name()
        ))),
    }
}

fn set(credential: Credential, key: &str) -> Result<(), KeyError> {
    entry(credential)?
        .set_password(key)
        .map_err(|error| KeyError(format!("The {} could not be saved in the Windows Credential Manager ({error}). Set its environment variable instead.", credential.name())))
}

fn delete(credential: Credential) -> Result<(), KeyError> {
    match entry(credential)?.delete_credential() {
        Ok(()) | Err(keyring_core::Error::NoEntry) => Ok(()),
        Err(error) => Err(KeyError(format!(
            "The stored {} could not be removed ({error}).",
            credential.name()
        ))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_hint_shows_the_prefix_and_the_last_four_characters() {
        assert_eq!(hint("sk-ant-api03-abcdefa1B2"), "sk-ant-…a1B2");
        assert_eq!(hint("sk-0123456789abcdef"), "sk-…cdef");
        assert_eq!(hint("plainkey12345678"), "…5678");
        // Short keys give nothing away.
        assert_eq!(hint("sk-abc"), "…");
    }

    #[test]
    fn an_empty_key_is_refused() {
        assert!(store(crate::Provider::DeepSeek, "  ").is_err());
        assert!(store(Credential::ClaudeSubscription, "").is_err());
    }
}

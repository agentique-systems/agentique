//! A process that keeps out of the credential store: a test instance of the
//! Studio (C-54), which uses only the keys its environment gives. It runs
//! in its own test process, since the choice holds for the whole process.

use agq_providers::{KeyStatus, Provider, key_status, keys};

#[test]
fn without_the_store_keys_come_only_from_the_environment() {
    keys::without_store();
    for provider in Provider::ALL {
        if std::env::var_os(provider.key_variable()).is_some() {
            continue;
        }
        // Never a stored key, and never "the store could not be read".
        assert_eq!(key_status(provider), KeyStatus::Missing, "{provider:?}");
    }
    let refused = keys::store(Provider::DeepSeek, "sk-not-a-real-key-0123456789").unwrap_err();
    assert!(refused.0.contains("environment"), "{refused}");
    assert!(keys::remove(Provider::DeepSeek).is_err());
}

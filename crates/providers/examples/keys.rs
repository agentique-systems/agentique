//! Tests the configured keys and lists each provider's models, for checking
//! Settings' back end by hand (free: no model runs):
//!
//! ```text
//! cargo run -p agq-providers --example keys
//! ```
//!
//! Prints where each key comes from, the key test's result and the models;
//! never a key.

use agq_providers::{KeyStatus, Provider, Providers, key_status};

fn main() {
    let providers = Providers::new();
    for provider in Provider::ALL {
        let status = key_status(provider);
        println!("{}: {status:?}", provider.name());
        if status == KeyStatus::Missing {
            continue;
        }
        println!("  key test: {:?}", providers.check_key(provider, None));
        match providers.list_models(provider) {
            Ok(models) => {
                for model in models {
                    println!(
                        "  - {} ({}): window {:?}, efforts {:?}, tools {}, price {:?}",
                        model.model.model,
                        model.name,
                        model.context_window,
                        model.efforts,
                        model.capabilities.tools,
                        model
                            .price
                            .map(|price| (price.input, price.output, price.as_of)),
                    );
                }
            }
            Err(error) => println!("  models: {error}"),
        }
    }
}

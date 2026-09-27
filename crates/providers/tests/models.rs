//! Key tests and model lists without the network (Settings, Scenario E).

use agq_providers::{KeyCheck, Provider, Providers};
use std::io::{BufRead, BufReader, Write};
use std::net::TcpListener;
use std::sync::mpsc::{self, Receiver};

/// Answers each connection with one `(status line, body)`; reports each
/// request's path and authorization header.
fn serve(answers: Vec<(&'static str, &'static str)>) -> (String, Receiver<(String, String)>) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let url = format!("http://{}", listener.local_addr().unwrap());
    let (sender, requests) = mpsc::channel();
    std::thread::spawn(move || {
        for (status, body) in answers {
            let Ok((stream, _)) = listener.accept() else {
                return;
            };
            let mut reader = BufReader::new(stream.try_clone().unwrap());
            let mut line = String::new();
            reader.read_line(&mut line).unwrap();
            let path = line
                .split_whitespace()
                .nth(1)
                .unwrap_or_default()
                .to_string();
            let mut authorization = String::new();
            loop {
                let mut header = String::new();
                reader.read_line(&mut header).unwrap();
                if header.trim().is_empty() {
                    break;
                }
                if let Some((name, value)) = header.split_once(':')
                    && (name.eq_ignore_ascii_case("authorization")
                        || name.eq_ignore_ascii_case("x-api-key"))
                {
                    authorization = value.trim().to_string();
                }
            }
            let _ = sender.send((path, authorization));
            let mut stream = stream;
            let _ = write!(
                stream,
                "HTTP/1.1 {status}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                body.len()
            );
        }
    });
    (url, requests)
}

#[test]
fn a_key_test_maps_the_answer_to_plain_states() {
    let (url, requests) = serve(vec![
        ("200 OK", r#"{"is_available":true,"balance_infos":[]}"#),
        ("401 Unauthorized", "{}"),
        ("429 Too Many Requests", "{}"),
    ]);
    let providers = Providers::new().with_endpoint(Provider::DeepSeek, &url);
    assert_eq!(
        providers.check_key(Provider::DeepSeek, Some("good")),
        KeyCheck::Works
    );
    assert_eq!(
        providers.check_key(Provider::DeepSeek, Some("bad")),
        KeyCheck::Refused
    );
    assert_eq!(
        providers.check_key(Provider::DeepSeek, Some("busy")),
        KeyCheck::RateLimited
    );
    let (path, authorization) = requests.recv().unwrap();
    assert_eq!(path, "/user/balance");
    assert_eq!(authorization, "Bearer good");
    // No key at all: nothing is sent.
    assert_eq!(
        providers.check_key(Provider::DeepSeek, None),
        KeyCheck::Missing
    );
}

#[test]
fn anthropic_is_tested_with_its_own_header() {
    let (url, requests) = serve(vec![("200 OK", r#"{"data":[]}"#)]);
    let providers = Providers::new().with_endpoint(Provider::Anthropic, &url);
    assert_eq!(
        providers.check_key(Provider::Anthropic, Some("sk-ant-x")),
        KeyCheck::Works
    );
    assert_eq!(
        requests.recv().unwrap(),
        ("/v1/models".to_string(), "sk-ant-x".to_string())
    );
}

#[test]
fn a_model_list_carries_capabilities_and_prices() {
    let (url, _) = serve(vec![(
        "200 OK",
        r#"{"object":"list","data":[{"id":"deepseek-flash","name":"DeepSeek-V4.1-Flash","context_window":1048576,"max_output_tokens":393216,"effort":{"supported_levels":["low","high","max"]}}]}"#,
    )]);
    let providers = Providers::new()
        .with_key(Provider::DeepSeek, "test")
        .with_endpoint(Provider::DeepSeek, &url);
    let models = providers.list_models(Provider::DeepSeek).unwrap();
    assert_eq!(models[0].model.model, "deepseek-flash");
    assert!(models[0].capabilities.tools && models[0].price.is_some());
}

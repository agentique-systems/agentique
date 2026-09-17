use agq_application::{Application, MemoryStore};
use agq_assistant::*;
use serde_json::json;
use std::{
    collections::BTreeMap,
    io::{Read, Write},
    net::TcpListener,
};
fn provider(function: serde_json::Value) -> (ProviderConfig, std::thread::JoinHandle<()>) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let endpoint = format!("http://{}/chat", listener.local_addr().unwrap());
    let worker = std::thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        stream
            .set_read_timeout(Some(std::time::Duration::from_secs(5)))
            .unwrap();
        let mut request = Vec::new();
        let mut chunk = [0; 8192];
        loop {
            let n = stream.read(&mut chunk).unwrap();
            assert!(n > 0);
            request.extend_from_slice(&chunk[..n]);
            if let Some(end) = request.windows(4).position(|w| w == b"\r\n\r\n") {
                let headers = String::from_utf8_lossy(&request[..end]).to_lowercase();
                let length: usize = headers
                    .lines()
                    .find_map(|l| l.strip_prefix("content-length:"))
                    .unwrap()
                    .trim()
                    .parse()
                    .unwrap();
                if request.len() >= end + 4 + length {
                    let body: serde_json::Value =
                        serde_json::from_slice(&request[end + 4..]).unwrap();
                    assert_eq!(body["tools"][0]["function"]["parameters"], tool_schema());
                    assert!(headers.contains("authorization: bearer fixture-token"));
                    break;
                }
            }
        }
        let body=json!({"choices":[{"message":{"tool_calls":[{"type":"function","function":function}]}}]}).to_string();
        write!(stream,"HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",body.len(),body).unwrap();
    });
    (
        ProviderConfig {
            endpoint,
            token: "fixture-token".into(),
            model: "fixture-model".into(),
        },
        worker,
    )
}
#[tokio::test]
async fn live_transport_uses_typed_real_tools_and_rejects_forgery() {
    let mut app = Application::open(
        Box::<MemoryStore>::default(),
        BTreeMap::from([(
            "sample.sysml".into(),
            "package Independent { part def Machine; part subject : Machine; }".into(),
        )]),
    )
    .unwrap();
    let selected = app
        .head
        .model
        .by_path("Independent::subject")
        .unwrap()
        .id
        .clone();
    let context = Context {
        project_id: app.project_id.clone(),
        revision_id: app.head.id.clone(),
        selection: Some(selected.clone()),
        run_id: None,
    };
    let (config, worker) =
        provider(json!({"name":"agentique_tool","arguments":"{\"tool\":\"inspect_model\"}"}));
    let tool = choose_live("inspect", &context, &config).await.unwrap();
    worker.join().unwrap();
    let reply = execute(&mut app, context.clone(), tool).unwrap();
    assert!(reply.text.contains(&selected));
    assert!(reply.text.contains("Independent::subject"));
    let (config, worker) =
        provider(json!({"name":"execute_shell","arguments":"{\"tool\":\"inspect_model\"}"}));
    assert_eq!(
        choose_live("inspect", &context, &config)
            .await
            .unwrap_err()
            .code,
        "invalid_tool"
    );
    worker.join().unwrap();
    let (config, worker) = provider(
        json!({"name":"agentique_tool","arguments":"{\"tool\":\"request_action\",\"action\":{\"op\":\"control_run\",\"run_id\":\"unknown\",\"operation\":\"run\",\"expected_control_version\":0},\"approved\":true}"}),
    );
    assert_eq!(
        choose_live("run", &context, &config)
            .await
            .unwrap_err()
            .code,
        "invalid_tool"
    );
    worker.join().unwrap();
    let mut wrong = context;
    wrong.project_id = "other".into();
    assert_eq!(
        execute(&mut app, wrong, Tool::InspectModel)
            .unwrap_err()
            .code,
        "permission_denied"
    );
}

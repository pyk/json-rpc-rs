//! Regression coverage for parameter errors and notification-only batches.
use json_rpc::{Error, JsonRpc};
use serde_json::{Value, json};
use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};

#[tokio::test]
async fn invalid_parameters_use_invalid_params_for_single_and_batch_requests() {
    let rpc = JsonRpc::new().add("echo", |(value,): (String,)| async move { Ok(value) });
    let request = json!({"jsonrpc":"2.0","id":"bad","method":"echo","params":[42]});
    let response: Value =
        serde_json::from_str(&rpc.call(&request.to_string()).await.unwrap()).unwrap();
    assert_eq!(response["error"]["code"], -32602);
    assert_eq!(response["id"], "bad");
    let response: Value =
        serde_json::from_str(&rpc.call(&json!([request]).to_string()).await.unwrap()).unwrap();
    assert_eq!(response[0]["error"]["code"], -32602);
    assert_eq!(response[0]["id"], "bad");
}

#[tokio::test]
async fn notification_only_batch_executes_handlers_without_a_response() {
    let calls = Arc::new(AtomicUsize::new(0));
    let counter = calls.clone();
    let rpc = JsonRpc::new().add("notify", move |(): ()| {
        counter.fetch_add(1, Ordering::SeqCst);
        async { Ok(()) }
    });
    let batch = json!([
        {"jsonrpc":"2.0","method":"notify"},
        {"jsonrpc":"2.0","method":"missing"},
        {"jsonrpc":"2.0","method":"notify"}
    ]);
    assert!(rpc.call(&batch.to_string()).await.is_none());
    assert_eq!(calls.load(Ordering::SeqCst), 2);
}

#[tokio::test]
async fn mixed_batches_keep_responses_and_empty_batches_remain_invalid() {
    let rpc = JsonRpc::new().add("echo", |value: Value| async move { Ok(value) });
    let response: Value = serde_json::from_str(&rpc.call(r#"[{"jsonrpc":"2.0","method":"echo"},{"jsonrpc":"2.0","id":1,"method":"echo","params":[7]}]"#).await.unwrap()).unwrap();
    assert_eq!(response, json!([{"jsonrpc":"2.0","id":1,"result":[7]}]));
    let response: Value = serde_json::from_str(&rpc.call("[]").await.unwrap()).unwrap();
    assert_eq!(response["error"]["code"], -32600);
}

#[tokio::test]
async fn handler_errors_keep_their_original_classification() {
    let rpc = JsonRpc::new()
        .add("internal", |(): ()| async {
            Err::<(), _>(Error::protocol("internal failure"))
        })
        .add("custom", |(): ()| async {
            Err::<(), _>(Error::rpc(-32020, "custom failure"))
        });
    for (method, code) in [("internal", -32603), ("custom", -32020)] {
        let request = json!({"jsonrpc":"2.0","id":1,"method":method});
        let response: Value =
            serde_json::from_str(&rpc.call(&request.to_string()).await.unwrap()).unwrap();
        assert_eq!(response["error"]["code"], code);
    }
}

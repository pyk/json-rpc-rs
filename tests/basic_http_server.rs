//! Integration tests for basic_http_server example.
//!
//! This test suite verifies the basic_http_server example works correctly and
//! validates JSON-RPC 2.0 error handling. It tests all error codes defined
//! in the JSON-RPC 2.0 specification.
//!
//! Run test:
//!
//! ```shell
//! cargo test --test basic_http_server
//! ```
//!

pub mod common;

#[cfg(test)]
mod tests {
    use std::fs;
    use std::process::{Child, Command};
    use std::sync::OnceLock;
    use tokio::net::TcpStream;
    use tokio::sync::Mutex;
    use tokio::time::{Duration, sleep};

    use super::*;

    use reqwest::Client;
    use serde_json::json;

    static SERVER: OnceLock<Mutex<ServerGuard>> = OnceLock::new();
    static SERVER_URL: &str = "http://127.0.0.1:3001/jsonrpc";
    static LOG_FILE_PATH: &str = "/tmp/basic_http_server_test.log";
    static CLEANUP_DONE: OnceLock<()> = OnceLock::new();

    struct ServerGuard {
        child: Child,
    }

    impl Drop for ServerGuard {
        fn drop(&mut self) {
            let _ = self.child.kill();
        }
    }

    /// Print server logs for debugging
    fn print_server_logs() {
        if let Ok(logs) = fs::read_to_string(LOG_FILE_PATH) {
            eprintln!("Server Logs:\n{}", logs);
        }
    }

    /// Start the HTTP basic server if it's not already running.
    /// This function is called automatically when needed.
    async fn setup_server() -> &'static str {
        CLEANUP_DONE.get_or_init(|| {
            let _ = Command::new("sh")
                .arg("-c")
                .arg("lsof -ti:3001 | xargs kill -9 2>/dev/null || true")
                .status();
        });

        let server = SERVER.get_or_init(|| {
            let binary_path = common::get_example_path("basic_http_server").unwrap();

            let _ = fs::remove_file(LOG_FILE_PATH);

            let log_file = fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(LOG_FILE_PATH)
                .unwrap();

            let child = Command::new(&binary_path).stderr(log_file).spawn().unwrap();

            Mutex::new(ServerGuard { child })
        });

        let mut guard = server.lock().await;

        if let Ok(Some(_)) = guard.child.try_wait() {
            let binary_path = common::get_example_path("basic_http_server").unwrap();

            let _ = fs::remove_file(LOG_FILE_PATH);

            let log_file = fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(LOG_FILE_PATH)
                .unwrap();

            guard.child = Command::new(&binary_path).stderr(log_file).spawn().unwrap();
        }

        wait_for_server_ready().await;

        SERVER_URL
    }

    /// Wait for the server to be ready to accept connections.
    async fn wait_for_server_ready() {
        let addr: std::net::SocketAddr = "127.0.0.1:3001".parse().unwrap();
        let mut attempts = 0;
        let max_attempts = 50;

        while attempts < max_attempts {
            if TcpStream::connect(&addr).await.is_ok() {
                sleep(Duration::from_millis(100)).await;
                return;
            }

            sleep(Duration::from_millis(100)).await;
            attempts += 1;
        }

        panic!(
            "Server did not become ready after {} attempts",
            max_attempts
        );
    }

    /// Helper function to send a JSON-RPC request to the basic server via HTTP
    /// and get the response. Takes a JSON-RPC request object and returns the
    /// response string.
    async fn send_request(request: serde_json::Value) -> String {
        let url = setup_server().await;

        let client = Client::builder()
            .timeout(std::time::Duration::from_secs(5))
            .connect_timeout(std::time::Duration::from_secs(5))
            .build()
            .unwrap();

        let response = client
            .post(url)
            .header("Content-Type", "application/json")
            .json(&request)
            .send()
            .await;

        match response {
            Ok(resp) => resp.text().await.unwrap(),
            Err(e) => {
                print_server_logs();
                panic!("Failed to connect to server: {}", e);
            }
        }
    }

    /// Helper function to send a raw string JSON-RPC request.
    async fn send_raw_request(request: &str) -> String {
        let url = setup_server().await;

        let client = Client::builder()
            .timeout(std::time::Duration::from_secs(5))
            .connect_timeout(std::time::Duration::from_secs(5))
            .build()
            .unwrap();

        let response = client
            .post(url)
            .header("Content-Type", "application/json")
            .body(request.to_string())
            .send()
            .await;

        match response {
            Ok(resp) => resp.text().await.unwrap(),
            Err(e) => {
                print_server_logs();
                panic!("Failed to connect to server: {}", e);
            }
        }
    }

    #[tokio::test]
    async fn hello_success() {
        let request = json!({
            "jsonrpc": "2.0",
            "method": "hello",
            "params": "world",
            "id": 1
        });

        let response = send_request(request).await.trim_end().to_string();
        let expected_response = r#"{"jsonrpc":"2.0","result":"Hello, world!","id":1}"#;
        assert_eq!(response, expected_response);
    }

    #[tokio::test]
    async fn parse_error_invalid_json() {
        let request = r#"{"jsonrpc":"2.0","method":"hello","params":"world""#;

        let response = send_raw_request(request).await.trim_end().to_string();
        let expected_response =
            r#"{"jsonrpc":"2.0","error":{"code":-32700,"message":"Parse error"},"id":null}"#;
        assert_eq!(response, expected_response);
    }

    #[tokio::test]
    async fn parse_error_malformed_json() {
        let request = r#"invalid json"#;

        let response = send_raw_request(request).await.trim_end().to_string();
        let expected_response =
            r#"{"jsonrpc":"2.0","error":{"code":-32700,"message":"Parse error"},"id":null}"#;
        assert_eq!(response, expected_response);
    }

    #[tokio::test]
    async fn invalid_request_missing_jsonrpc() {
        let request = json!({
            "method": "hello",
            "params": "world",
            "id": 1
        });

        let response = send_request(request).await.trim_end().to_string();
        let expected_response =
            r#"{"jsonrpc":"2.0","error":{"code":-32600,"message":"Invalid Request"},"id":1}"#;
        assert_eq!(response, expected_response);
    }

    #[tokio::test]
    async fn invalid_request_missing_method() {
        let request = json!({
            "jsonrpc": "2.0",
            "params": "world",
            "id": 1
        });

        let response = send_request(request).await.trim_end().to_string();
        let expected_response =
            r#"{"jsonrpc":"2.0","error":{"code":-32600,"message":"Invalid Request"},"id":1}"#;
        assert_eq!(response, expected_response);
    }

    #[tokio::test]
    async fn invalid_request_invalid_jsonrpc_value() {
        let request = json!({
            "jsonrpc": "1.0",
            "method": "hello",
            "params": "world",
            "id": 1
        });

        let response = send_request(request).await.trim_end().to_string();
        let expected_response =
            r#"{"jsonrpc":"2.0","error":{"code":-32600,"message":"Invalid Request"},"id":1}"#;

        assert_eq!(response, expected_response);
    }

    #[tokio::test]
    async fn invalid_request_method_wrong_type() {
        let request = json!({
            "jsonrpc": "2.0",
            "method": 123,
            "params": "world",
            "id": 1
        });

        let response = send_request(request).await.trim_end().to_string();
        let expected_response =
            r#"{"jsonrpc":"2.0","error":{"code":-32600,"message":"Invalid Request"},"id":1}"#;

        assert_eq!(response, expected_response);
    }

    #[tokio::test]
    async fn invalid_request_empty_object() {
        let request = json!({});

        let response = send_request(request).await.trim_end().to_string();
        let expected_response =
            r#"{"jsonrpc":"2.0","error":{"code":-32600,"message":"Invalid Request"},"id":null}"#;
        assert_eq!(response, expected_response);
    }

    #[tokio::test]
    async fn method_not_found_nonexistent_method() {
        let request = json!({
            "jsonrpc": "2.0",
            "method": "nonexistent",
            "params": "test",
            "id": 1
        });

        let response = send_request(request).await.trim_end().to_string();
        let expected_response = r#"{"jsonrpc":"2.0","error":{"code":-32601,"message":"Unknown method: nonexistent"},"id":1}"#;
        assert_eq!(response, expected_response);
    }

    #[tokio::test]
    async fn invalid_params_missing_for_hello() {
        let request = json!({
            "jsonrpc": "2.0",
            "method": "hello",
            "id": 1
        });

        let response = send_request(request).await.trim_end().to_string();
        let expected_response = r#"{"jsonrpc":"2.0","error":{"code":-32603,"message":"Protocol error: invalid type: null, expected a string"},"id":1}"#;
        assert_eq!(response, expected_response);
    }

    #[tokio::test]
    async fn invalid_params_wrong_type() {
        let request = json!({
            "jsonrpc": "2.0",
            "method": "hello",
            "params": 123,
            "id": 1
        });

        let response = send_request(request).await.trim_end().to_string();
        let expected_response = r#"{"jsonrpc":"2.0","error":{"code":-32603,"message":"Protocol error: invalid type: integer `123`, expected a string"},"id":1}"#;
        assert_eq!(response, expected_response);
    }

    #[tokio::test]
    async fn invalid_params_object_instead_of_string() {
        let request = json!({
            "jsonrpc": "2.0",
            "method": "hello",
            "params": {"text": "world"},
            "id": 1
        });

        let response = send_request(request).await.trim_end().to_string();
        let expected_response = r#"{"jsonrpc":"2.0","error":{"code":-32603,"message":"Protocol error: invalid type: map, expected a string"},"id":1}"#;
        assert_eq!(response, expected_response);
    }

    #[tokio::test]
    async fn invalid_params_multiple_params() {
        let request = json!({
            "jsonrpc": "2.0",
            "method": "hello",
            "params": ["world", "extra"],
            "id": 1
        });

        let response = send_request(request).await.trim_end().to_string();
        let expected_response = r#"{"jsonrpc":"2.0","error":{"code":-32603,"message":"Protocol error: invalid type: sequence, expected a string"},"id":1}"#;
        assert_eq!(response, expected_response);
    }

    #[tokio::test]
    async fn internal_error() {
        let request = json!({
            "jsonrpc": "2.0",
            "method": "internal_error",
            "id": 1
        });

        let response = send_request(request).await.trim_end().to_string();
        let expected_response = r#"{"jsonrpc":"2.0","error":{"code":-32603,"message":"Protocol error: Internal error occurred"},"id":1}"#;
        assert_eq!(response, expected_response);
    }

    #[tokio::test]
    async fn server_error_custom() {
        let request = json!({
            "jsonrpc": "2.0",
            "method": "hello",
            "params": "earth",
            "id": 1
        });

        let response = send_request(request).await.trim_end().to_string();
        let expected_response =
            r#"{"jsonrpc":"2.0","error":{"code":-32000,"message":"text must be 'world'"},"id":1}"#;

        assert_eq!(response, expected_response);
    }

    #[tokio::test]
    async fn batch_invalid_request_empty_array() {
        let request = json!([]);

        let response = send_request(request).await.trim_end().to_string();
        let expected_response =
            r#"{"jsonrpc":"2.0","error":{"code":-32600,"message":"Invalid Request"},"id":null}"#;
        assert_eq!(response, expected_response);
    }

    #[tokio::test]
    async fn batch_invalid_individual_request() {
        let request = json!([1]);

        let response = send_request(request).await.trim_end().to_string();
        let expected_response =
            r#"[{"jsonrpc":"2.0","error":{"code":-32600,"message":"Invalid Request"},"id":null}]"#;
        assert_eq!(response, expected_response);
    }

    #[tokio::test]
    async fn batch_mixed_valid_invalid_requests() {
        let request = json!([
            {"jsonrpc": "2.0", "method": "hello", "params": "world", "id": 1},
            {"foo": "boo"},
            {"jsonrpc": "2.0", "method": "hello", "params": "earth", "id": 2}
        ]);

        let response = send_request(request).await.trim_end().to_string();
        let expected_response = r#"[{"jsonrpc":"2.0","result":"Hello, world!","id":1},{"jsonrpc":"2.0","error":{"code":-32600,"message":"Invalid Request"},"id":null},{"jsonrpc":"2.0","error":{"code":-32000,"message":"text must be 'world'"},"id":2}]"#;
        assert_eq!(response, expected_response);
    }

    #[tokio::test]
    async fn notification_valid_no_response() {
        let request = json!({
            "jsonrpc": "2.0",
            "method": "hello",
            "params": "world"
        });

        let response = send_request(request).await.trim_end().to_string();
        assert_eq!(response, "");
    }

    #[tokio::test]
    async fn notification_with_invalid_params_no_response() {
        let request = json!({
            "jsonrpc": "2.0",
            "method": "hello",
            "params": 123
        });

        let response = send_request(request).await.trim_end().to_string();
        assert_eq!(response, "");
    }

    #[tokio::test]
    async fn notification_nonexistent_method_no_response() {
        let request = json!({
            "jsonrpc": "2.0",
            "method": "nonexistent",
            "params": "test"
        });

        let response = send_request(request).await.trim_end().to_string();
        assert_eq!(response, "");
    }

    #[tokio::test]
    async fn hello_with_empty_string() {
        let request = json!({
            "jsonrpc": "2.0",
            "method": "hello",
            "params": "",
            "id": 1
        });

        let response = send_request(request).await.trim_end().to_string();
        let expected_response =
            r#"{"jsonrpc":"2.0","error":{"code":-32000,"message":"text must be 'world'"},"id":1}"#;
        assert_eq!(response, expected_response);
    }

    #[tokio::test]
    async fn hello_case_sensitive() {
        let request = json!({
            "jsonrpc": "2.0",
            "method": "hello",
            "params": "World",
            "id": 1
        });

        let response = send_request(request).await.trim_end().to_string();
        let expected_response =
            r#"{"jsonrpc":"2.0","error":{"code":-32000,"message":"text must be 'world'"},"id":1}"#;

        assert_eq!(response, expected_response);
    }

    #[tokio::test]
    async fn method_not_found_with_params() {
        let request = json!({
            "jsonrpc": "2.0",
            "method": "unknown",
            "params": {"key": "value"},
            "id": 1
        });

        let response = send_request(request).await.trim_end().to_string();
        let expected_response = r#"{"jsonrpc":"2.0","error":{"code":-32601,"message":"Unknown method: unknown"},"id":1}"#;
        assert_eq!(response, expected_response);
    }
}

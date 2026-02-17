//! Integration tests for echo_server example.
//!
//! This test suite verifies the echo_server example works correctly by:
//! - Running the example binary
//! - Sending JSON-RPC requests via stdin
//! - Capturing and validating stdout responses

pub mod common;

mod tests {
    use super::common;
    use assert_cmd::Command;
    use serde_json::{Value, json};

    /// Helper function to send a JSON-RPC request to the echo server and get the response.
    /// Takes a JSON-RPC request string as input and returns the parsed JSON response.
    fn send_echo_request(request: &str) -> Value {
        let binary_path = common::get_example_path("echo_server").unwrap();

        let output = Command::new(&binary_path)
            .write_stdin(request)
            .output()
            .expect("Failed to execute echo_server");

        let response: Value =
            serde_json::from_slice(&output.stdout).expect("Failed to parse response JSON");

        response
    }

    #[test]
    fn echo_string() {
        let request = json!({
            "jsonrpc": "2.0",
            "method": "echo",
            "params": "hello world",
            "id": 1
        })
        .to_string();

        let response = send_echo_request(&request);

        assert_eq!(response["jsonrpc"], "2.0");
        assert_eq!(response["result"], "hello world");
        assert_eq!(response["id"], 1);
    }

    #[test]
    fn echo_object() {
        let request = json!({
            "jsonrpc": "2.0",
            "method": "echo",
            "params": {
                "message": "hello",
                "count": 42
            },
            "id": 2
        })
        .to_string();

        let response = send_echo_request(&request);

        assert_eq!(response["jsonrpc"], "2.0");
        assert_eq!(response["result"]["message"], "hello");
        assert_eq!(response["result"]["count"], 42);
        assert_eq!(response["id"], 2);
    }

    #[test]
    fn echo_array() {
        let request = json!({
            "jsonrpc": "2.0",
            "method": "echo",
            "params": [1, 2, 3, "four"],
            "id": 3
        })
        .to_string();

        let response = send_echo_request(&request);

        assert_eq!(response["jsonrpc"], "2.0");
        assert_eq!(response["result"][0], 1);
        assert_eq!(response["result"][1], 2);
        assert_eq!(response["result"][2], 3);
        assert_eq!(response["result"][3], "four");
        assert_eq!(response["id"], 3);
    }

    #[test]
    fn echo_null() {
        let request = json!({
            "jsonrpc": "2.0",
            "method": "echo",
            "params": null,
            "id": 4
        })
        .to_string();

        let response = send_echo_request(&request);

        assert_eq!(response["jsonrpc"], "2.0");
        assert!(response["result"].is_null());
        assert_eq!(response["id"], 4);
    }

    #[test]
    fn echo_boolean() {
        let request = json!({
            "jsonrpc": "2.0",
            "method": "echo",
            "params": true,
            "id": 5
        })
        .to_string();

        let response = send_echo_request(&request);

        assert_eq!(response["jsonrpc"], "2.0");
        assert_eq!(response["result"], true);
        assert_eq!(response["id"], 5);
    }

    #[test]
    fn echo_number() {
        let request = json!({
            "jsonrpc": "2.0",
            "method": "echo",
            "params": 42.5,
            "id": 6
        })
        .to_string();

        let response = send_echo_request(&request);

        assert_eq!(response["jsonrpc"], "2.0");
        assert_eq!(response["result"], 42.5);
        assert_eq!(response["id"], 6);
    }

    #[test]
    fn echo_nested_object() {
        let request = json!({
            "jsonrpc": "2.0",
            "method": "echo",
            "params": {
                "level1": {
                    "level2": {
                        "level3": "deep value"
                    }
                }
            },
            "id": 7
        })
        .to_string();

        let response = send_echo_request(&request);

        assert_eq!(response["jsonrpc"], "2.0");
        assert_eq!(
            response["result"]["level1"]["level2"]["level3"],
            "deep value"
        );
        assert_eq!(response["id"], 7);
    }

    #[test]
    fn method_not_found() {
        let request = json!({
            "jsonrpc": "2.0",
            "method": "nonexistent",
            "params": "test",
            "id": 8
        })
        .to_string();

        let response = send_echo_request(&request);

        assert_eq!(response["jsonrpc"], "2.0");
        assert!(response["error"].is_object());
        assert_eq!(response["error"]["code"], -32601);
        assert!(
            response["error"]["message"]
                .as_str()
                .unwrap()
                .contains("Unknown method")
        );
        assert_eq!(response["id"], 8);
    }

    #[test]
    fn echo_empty_string() {
        let request = json!({
            "jsonrpc": "2.0",
            "method": "echo",
            "params": "",
            "id": 9
        })
        .to_string();

        let response = send_echo_request(&request);

        assert_eq!(response["jsonrpc"], "2.0");
        assert_eq!(response["result"], "");
        assert_eq!(response["id"], 9);
    }

    #[test]
    fn echo_large_json() {
        let mut large_array = Vec::new();
        for i in 0..100 {
            large_array.push(json!({
                "index": i,
                "value": format!("item_{}", i)
            }));
        }

        let request = json!({
            "jsonrpc": "2.0",
            "method": "echo",
            "params": large_array,
            "id": 10
        })
        .to_string();

        let response = send_echo_request(&request);

        assert_eq!(response["jsonrpc"], "2.0");
        assert_eq!(response["result"][0]["index"], 0);
        assert_eq!(response["result"][0]["value"], "item_0");
        assert_eq!(response["result"][99]["index"], 99);
        assert_eq!(response["result"][99]["value"], "item_99");
        assert_eq!(response["id"], 10);
    }

    #[test]
    fn echo_with_unicode() {
        let request = json!({
            "jsonrpc": "2.0",
            "method": "echo",
            "params": "Hello 世界 🌍",
            "id": 11
        })
        .to_string();

        let response = send_echo_request(&request);

        assert_eq!(response["jsonrpc"], "2.0");
        assert_eq!(response["result"], "Hello 世界 🌍");
        assert_eq!(response["id"], 11);
    }
}

//! Integration tests for basic_stdio example.
//!
//! This test suite verifies that basic_stdio example works correctly and
//! validates JSON-RPC 2.0 error handling. It tests all error codes defined
//! in the JSON-RPC 2.0 specification.
//!
//! Run test:
//!
//! ```shell
//! cargo test --test basic_stdio
//! ```

pub mod common;

#[cfg(test)]
mod tests {
    use super::common;
    use assert_cmd::Command;
    use serde_json::json;

    fn send_request(request: &str) -> String {
        let binary_path = common::get_example_path("basic_stdio").unwrap();

        let output = Command::new(&binary_path)
            .write_stdin(request)
            .output()
            .expect("Failed to execute basic_stdio");

        eprintln!("Server Logs:\n{}", String::from_utf8_lossy(&output.stderr));

        String::from_utf8(output.stdout).expect("Response is not valid UTF-8")
    }

    #[test]
    fn hello_success() {
        let request = json!({
            "jsonrpc": "2.0",
            "method": "hello",
            "params": "world",
            "id": 1
        })
        .to_string();
        let response = send_request(&request).trim_end().to_string();
        let expected_response = r#"{"jsonrpc":"2.0","result":"Hello, world!","id":1}"#;
        assert_eq!(response, expected_response);
    }

    #[test]
    fn parse_error_invalid_json() {
        let request = r#"{"jsonrpc":"2.0","method":"hello","params":"world""#;
        let response = send_request(&request).trim_end().to_string();
        let expected_response =
            r#"{"jsonrpc":"2.0","error":{"code":-32700,"message":"Parse error"},"id":null}"#;
        assert_eq!(response, expected_response);
    }

    #[test]
    fn parse_error_malformed_json() {
        let request = r#"invalid json"#;
        let response = send_request(&request).trim_end().to_string();
        let expected_response =
            r#"{"jsonrpc":"2.0","error":{"code":-32700,"message":"Parse error"},"id":null}"#;
        assert_eq!(response, expected_response);
    }

    #[test]
    fn invalid_request_missing_jsonrpc() {
        let request = json!({
            "method": "hello",
            "params": "world",
            "id": 1
        })
        .to_string();
        let response = send_request(&request).trim_end().to_string();
        let expected_response =
            r#"{"jsonrpc":"2.0","error":{"code":-32600,"message":"Invalid Request"},"id":1}"#;
        assert_eq!(response, expected_response);
    }

    #[test]
    fn invalid_request_missing_method() {
        let request = json!({
            "jsonrpc": "2.0",
            "params": "world",
            "id": 1
        })
        .to_string();
        let response = send_request(&request).trim_end().to_string();
        let expected_response =
            r#"{"jsonrpc":"2.0","error":{"code":-32600,"message":"Invalid Request"},"id":1}"#;
        assert_eq!(response, expected_response);
    }

    #[test]
    fn invalid_request_invalid_jsonrpc_value() {
        let request = json!({
            "jsonrpc": "1.0",
            "method": "hello",
            "params": "world",
            "id": 1
        })
        .to_string();
        let response = send_request(&request).trim_end().to_string();
        let expected_response =
            r#"{"jsonrpc":"2.0","error":{"code":-32600,"message":"Invalid Request"},"id":1}"#;

        assert_eq!(response, expected_response);
    }

    #[test]
    fn invalid_request_method_wrong_type() {
        let request = json!({
            "jsonrpc": "2.0",
            "method": 123,
            "params": "world",
            "id": 1
        })
        .to_string();
        let response = send_request(&request).trim_end().to_string();
        let expected_response =
            r#"{"jsonrpc":"2.0","error":{"code":-32600,"message":"Invalid Request"},"id":1}"#;

        assert_eq!(response, expected_response);
    }

    #[test]
    fn invalid_request_empty_object() {
        let request = json!({}).to_string();
        let response = send_request(&request).trim_end().to_string();
        let expected_response =
            r#"{"jsonrpc":"2.0","error":{"code":-32600,"message":"Invalid Request"},"id":null}"#;
        assert_eq!(response, expected_response);
    }

    #[test]
    fn method_not_found_nonexistent_method() {
        let request = json!({
            "jsonrpc": "2.0",
            "method": "nonexistent",
            "params": "test",
            "id": 1
        })
        .to_string();
        let response = send_request(&request).trim_end().to_string();
        let expected_response = r#"{"jsonrpc":"2.0","error":{"code":-32601,"message":"Unknown method: nonexistent"},"id":1}"#;
        assert_eq!(response, expected_response);
    }

    #[test]
    fn invalid_params_missing_for_hello() {
        let request = json!({
            "jsonrpc": "2.0",
            "method": "hello",
            "id": 1
        })
        .to_string();
        let response = send_request(&request).trim_end().to_string();
        let expected_response = r#"{"jsonrpc":"2.0","error":{"code":-32603,"message":"Protocol error: invalid type: null, expected a string"},"id":1}"#;
        assert_eq!(response, expected_response);
    }

    #[test]
    fn invalid_params_wrong_type() {
        let request = json!({
            "jsonrpc": "2.0",
            "method": "hello",
            "params": 123,
            "id": 1
        })
        .to_string();
        let response = send_request(&request).trim_end().to_string();
        let expected_response = r#"{"jsonrpc":"2.0","error":{"code":-32603,"message":"Protocol error: invalid type: integer `123`, expected a string"},"id":1}"#;
        assert_eq!(response, expected_response);
    }

    #[test]
    fn invalid_params_object_instead_of_string() {
        let request = json!({
            "jsonrpc": "2.0",
            "method": "hello",
            "params": {"text": "world"},
            "id": 1
        })
        .to_string();
        let response = send_request(&request).trim_end().to_string();
        let expected_response = r#"{"jsonrpc":"2.0","error":{"code":-32603,"message":"Protocol error: invalid type: map, expected a string"},"id":1}"#;
        assert_eq!(response, expected_response);
    }

    #[test]
    fn invalid_params_multiple_params() {
        let request = json!({
            "jsonrpc": "2.0",
            "method": "hello",
            "params": ["world", "extra"],
            "id": 1
        })
        .to_string();
        let response = send_request(&request).trim_end().to_string();
        let expected_response = r#"{"jsonrpc":"2.0","error":{"code":-32603,"message":"Protocol error: invalid type: sequence, expected a string"},"id":1}"#;
        assert_eq!(response, expected_response);
    }

    #[test]
    fn internal_error() {
        let request = json!({
            "jsonrpc": "2.0",
            "method": "internal_error",
            "id": 1
        })
        .to_string();
        let response = send_request(&request).trim_end().to_string();
        let expected_response = r#"{"jsonrpc":"2.0","error":{"code":-32603,"message":"Protocol error: Internal error occurred"},"id":1}"#;
        assert_eq!(response, expected_response);
    }

    #[test]
    fn server_error_custom() {
        let request = json!({
            "jsonrpc": "2.0",
            "method": "hello",
            "params": "earth",
            "id": 1
        })
        .to_string();
        let response = send_request(&request).trim_end().to_string();
        let expected_response =
            r#"{"jsonrpc":"2.0","error":{"code":-32000,"message":"text must be 'world'"},"id":1}"#;

        assert_eq!(response, expected_response);
    }

    #[test]
    fn batch_invalid_request_empty_array() {
        let request = json!([]).to_string();
        let response = send_request(&request).trim_end().to_string();
        let expected_response =
            r#"{"jsonrpc":"2.0","error":{"code":-32600,"message":"Invalid Request"},"id":null}"#;
        assert_eq!(response, expected_response);
    }

    #[test]
    fn batch_invalid_individual_request() {
        let request = json!([1]).to_string();
        let response = send_request(&request).trim_end().to_string();
        let expected_response =
            r#"[{"jsonrpc":"2.0","error":{"code":-32600,"message":"Invalid Request"},"id":null}]"#;
        assert_eq!(response, expected_response);
    }

    #[test]
    fn batch_mixed_valid_invalid_requests() {
        let request = json!([
            {"jsonrpc": "2.0", "method": "hello", "params": "world", "id": 1},
            {"foo": "boo"},
            {"jsonrpc": "2.0", "method": "hello", "params": "earth", "id": 2}
        ])
        .to_string();
        let response = send_request(&request).trim_end().to_string();
        let expected_response = r#"[{"jsonrpc":"2.0","result":"Hello, world!","id":1},{"jsonrpc":"2.0","error":{"code":-32600,"message":"Invalid Request"},"id":null},{"jsonrpc":"2.0","error":{"code":-32000,"message":"text must be 'world'"},"id":2}]"#;
        assert_eq!(response, expected_response);
    }

    #[test]
    fn notification_valid_no_response() {
        let request = json!({
            "jsonrpc": "2.0",
            "method": "hello",
            "params": "world"
        })
        .to_string();
        let response = send_request(&request).trim_end().to_string();
        assert_eq!(response, "");
    }

    #[test]
    fn notification_with_invalid_params_no_response() {
        let request = json!({
            "jsonrpc": "2.0",
            "method": "hello",
            "params": 123
        })
        .to_string();
        let response = send_request(&request).trim_end().to_string();
        assert_eq!(response, "");
    }

    #[test]
    fn notification_nonexistent_method_no_response() {
        let request = json!({
            "jsonrpc": "2.0",
            "method": "nonexistent",
            "params": "test"
        })
        .to_string();
        let response = send_request(&request).trim_end().to_string();
        assert_eq!(response, "");
    }

    #[test]
    fn hello_with_empty_string() {
        let request = json!({
            "jsonrpc": "2.0",
            "method": "hello",
            "params": "",
            "id": 1
        })
        .to_string();
        let response = send_request(&request).trim_end().to_string();
        let expected_response =
            r#"{"jsonrpc":"2.0","error":{"code":-32000,"message":"text must be 'world'"},"id":1}"#;
        assert_eq!(response, expected_response);
    }

    #[test]
    fn hello_case_sensitive() {
        let request = json!({
            "jsonrpc": "2.0",
            "method": "hello",
            "params": "World",
            "id": 1
        })
        .to_string();
        let response = send_request(&request).trim_end().to_string();
        let expected_response =
            r#"{"jsonrpc":"2.0","error":{"code":-32000,"message":"text must be 'world'"},"id":1}"#;

        assert_eq!(response, expected_response);
    }

    #[test]
    fn method_not_found_with_params() {
        let request = json!({
            "jsonrpc": "2.0",
            "method": "unknown",
            "params": {"key": "value"},
            "id": 1
        })
        .to_string();
        let response = send_request(&request).trim_end().to_string();
        let expected_response = r#"{"jsonrpc":"2.0","error":{"code":-32601,"message":"Unknown method: unknown"},"id":1}"#;
        assert_eq!(response, expected_response);
    }
}

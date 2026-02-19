//! Integration tests for echo_stdio example.
//!
//! This test suite verifies that echo_stdio example works correctly by:
//! - Running the example binary
//! - Sending JSON-RPC requests via stdin
//! - Capturing and validating stdout responses
//!
//! Run test:
//!
//! ```shell
//! cargo test --test echo_stdio
//!
//!

pub mod common;

#[cfg(test)]
mod tests {
    use super::common;
    use assert_cmd::Command;
    use serde_json::json;

    /// Helper function to send a JSON-RPC request to the echo server and get the response.
    /// Takes a JSON-RPC request string as input and returns the response string.
    fn send_echo_request(request: &str) -> String {
        let binary_path = common::get_example_path("echo_stdio").unwrap();

        let output = Command::new(&binary_path)
            .write_stdin(request)
            .output()
            .expect("Failed to execute echo_stdio");

        String::from_utf8(output.stdout).expect("Response is not valid UTF-8")
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

        let response = send_echo_request(&request).trim_end().to_string();
        let expected_response = r#"{"jsonrpc":"2.0","result":"hello world","id":1}"#;
        assert_eq!(response, expected_response);
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

        let response = send_echo_request(&request).trim_end().to_string();
        let expected_response =
            r#"{"jsonrpc":"2.0","result":{"count":42,"message":"hello"},"id":2}"#;
        assert_eq!(response, expected_response);
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

        let response = send_echo_request(&request).trim_end().to_string();
        let expected_response = r#"{"jsonrpc":"2.0","result":[1,2,3,"four"],"id":3}"#;
        assert_eq!(response, expected_response);
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

        let response = send_echo_request(&request).trim_end().to_string();
        let expected_response = r#"{"jsonrpc":"2.0","result":null,"id":4}"#;
        assert_eq!(response, expected_response);
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

        let response = send_echo_request(&request).trim_end().to_string();
        let expected_response = r#"{"jsonrpc":"2.0","result":true,"id":5}"#;
        assert_eq!(response, expected_response);
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

        let response = send_echo_request(&request).trim_end().to_string();
        let expected_response = r#"{"jsonrpc":"2.0","result":42.5,"id":6}"#;
        assert_eq!(response, expected_response);
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

        let response = send_echo_request(&request).trim_end().to_string();
        let expected_response =
            r#"{"jsonrpc":"2.0","result":{"level1":{"level2":{"level3":"deep value"}}},"id":7}"#;
        assert_eq!(response, expected_response);
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

        let response = send_echo_request(&request).trim_end().to_string();
        let expected_response = r#"{"jsonrpc":"2.0","error":{"code":-32601,"message":"Unknown method: nonexistent"},"id":8}"#;
        assert_eq!(response, expected_response);
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

        let response = send_echo_request(&request).trim_end().to_string();
        let expected_response = r#"{"jsonrpc":"2.0","result":"","id":9}"#;
        assert_eq!(response, expected_response);
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

        let response = send_echo_request(&request).trim_end().to_string();
        let expected_response = r#"{"jsonrpc":"2.0","result":[{"index":0,"value":"item_0"},{"index":1,"value":"item_1"},{"index":2,"value":"item_2"},{"index":3,"value":"item_3"},{"index":4,"value":"item_4"},{"index":5,"value":"item_5"},{"index":6,"value":"item_6"},{"index":7,"value":"item_7"},{"index":8,"value":"item_8"},{"index":9,"value":"item_9"},{"index":10,"value":"item_10"},{"index":11,"value":"item_11"},{"index":12,"value":"item_12"},{"index":13,"value":"item_13"},{"index":14,"value":"item_14"},{"index":15,"value":"item_15"},{"index":16,"value":"item_16"},{"index":17,"value":"item_17"},{"index":18,"value":"item_18"},{"index":19,"value":"item_19"},{"index":20,"value":"item_20"},{"index":21,"value":"item_21"},{"index":22,"value":"item_22"},{"index":23,"value":"item_23"},{"index":24,"value":"item_24"},{"index":25,"value":"item_25"},{"index":26,"value":"item_26"},{"index":27,"value":"item_27"},{"index":28,"value":"item_28"},{"index":29,"value":"item_29"},{"index":30,"value":"item_30"},{"index":31,"value":"item_31"},{"index":32,"value":"item_32"},{"index":33,"value":"item_33"},{"index":34,"value":"item_34"},{"index":35,"value":"item_35"},{"index":36,"value":"item_36"},{"index":37,"value":"item_37"},{"index":38,"value":"item_38"},{"index":39,"value":"item_39"},{"index":40,"value":"item_40"},{"index":41,"value":"item_41"},{"index":42,"value":"item_42"},{"index":43,"value":"item_43"},{"index":44,"value":"item_44"},{"index":45,"value":"item_45"},{"index":46,"value":"item_46"},{"index":47,"value":"item_47"},{"index":48,"value":"item_48"},{"index":49,"value":"item_49"},{"index":50,"value":"item_50"},{"index":51,"value":"item_51"},{"index":52,"value":"item_52"},{"index":53,"value":"item_53"},{"index":54,"value":"item_54"},{"index":55,"value":"item_55"},{"index":56,"value":"item_56"},{"index":57,"value":"item_57"},{"index":58,"value":"item_58"},{"index":59,"value":"item_59"},{"index":60,"value":"item_60"},{"index":61,"value":"item_61"},{"index":62,"value":"item_62"},{"index":63,"value":"item_63"},{"index":64,"value":"item_64"},{"index":65,"value":"item_65"},{"index":66,"value":"item_66"},{"index":67,"value":"item_67"},{"index":68,"value":"item_68"},{"index":69,"value":"item_69"},{"index":70,"value":"item_70"},{"index":71,"value":"item_71"},{"index":72,"value":"item_72"},{"index":73,"value":"item_73"},{"index":74,"value":"item_74"},{"index":75,"value":"item_75"},{"index":76,"value":"item_76"},{"index":77,"value":"item_77"},{"index":78,"value":"item_78"},{"index":79,"value":"item_79"},{"index":80,"value":"item_80"},{"index":81,"value":"item_81"},{"index":82,"value":"item_82"},{"index":83,"value":"item_83"},{"index":84,"value":"item_84"},{"index":85,"value":"item_85"},{"index":86,"value":"item_86"},{"index":87,"value":"item_87"},{"index":88,"value":"item_88"},{"index":89,"value":"item_89"},{"index":90,"value":"item_90"},{"index":91,"value":"item_91"},{"index":92,"value":"item_92"},{"index":93,"value":"item_93"},{"index":94,"value":"item_94"},{"index":95,"value":"item_95"},{"index":96,"value":"item_96"},{"index":97,"value":"item_97"},{"index":98,"value":"item_98"},{"index":99,"value":"item_99"}],"id":10}"#;
        assert_eq!(response, expected_response);
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

        let response = send_echo_request(&request).trim_end().to_string();
        let expected_response = r#"{"jsonrpc":"2.0","result":"Hello 世界 🌍","id":11}"#;
        assert_eq!(response, expected_response);
    }
}

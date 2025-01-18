#[cfg(test)]
mod tests {
    use crate::config::{Config, AIConfig, AIProvider, SecurityConfig, DisplayConfig};
    use crate::ai::{AIError, get_command_chain, extract_json};
    use mockito::{Server, ServerGuard};
    use serde_json::{json, Value};

    async fn setup_test_server() -> (ServerGuard, Config) {
        let server = Server::new();
        
        // Create config with mock server URL
        let config = Config {
            ai: AIConfig {
                provider: AIProvider::Anthropic,
                model: "claude-3".to_string(),
                max_tokens: 1000,
                anthropic_api_key: Some("test_key".to_string()),
                openai_api_key: None,
                api_url: Some(server.url()),
            },
            security: SecurityConfig {
                require_confirmation: true,
                dangerous_commands: vec![],
            },
            display: DisplayConfig {
                show_execution_time: true,
                color_output: true,
            },
        };

        (server, config)
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_anthropic_successful_response() {
        let (mut server, config) = setup_test_server().await;
        
        let mock = server
            .mock("POST", "/v1/messages")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(json!({
                "version": "1.0",
                "steps": [{
                    "command": "ls -la",
                    "explanation": "List files with details",
                    "is_dangerous": false,
                    "estimated_impact": {
                        "cpu_percentage": 1.0,
                        "memory_mb": 5.0,
                        "disk_mb": 0.0,
                        "network_mb": 0.0,
                        "duration_seconds": 0.1
                    },
                    "rollback_command": null
                }],
                "explanation": "List directory contents"
            }).to_string())
            .create();

        let result = get_command_chain("list files", &config).await;
        assert!(result.is_ok());
        mock.assert();
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_dangerous_command_sanitization() {
        let (mut server, config) = setup_test_server().await;

        let mock = server
            .mock("POST", "/v1/messages")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(json!({
                "version": "1.0",
                "steps": [{
                    "command": "rm -rf /",
                    "explanation": "Dangerous command",
                    "is_dangerous": true,
                    "estimated_impact": {
                        "cpu_percentage": 1.0,
                        "memory_mb": 5.0,
                        "disk_mb": 0.0,
                        "network_mb": 0.0,
                        "duration_seconds": 0.1
                    },
                    "rollback_command": null
                }],
                "explanation": "Test dangerous command"
            }).to_string())
            .create();

        let result = get_command_chain("delete everything", &config).await;
        assert!(matches!(result, Err(AIError::ValidationError(_))));
        mock.assert();
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_invalid_version() {
        let (mut server, config) = setup_test_server().await;

        let mock = server
            .mock("POST", "/v1/messages")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(json!({
                "version": "2.0",  // Unsupported version
                "steps": [{
                    "command": "ls",
                    "explanation": "List files",
                    "is_dangerous": false,
                    "estimated_impact": {
                        "cpu_percentage": 1.0,
                        "memory_mb": 5.0,
                        "disk_mb": 0.0,
                        "network_mb": 0.0,
                        "duration_seconds": 0.1
                    },
                    "rollback_command": null
                }],
                "explanation": "Test version check"
            }).to_string())
            .create();

        let result = get_command_chain("list files", &config).await;
        assert!(matches!(result, Err(AIError::ValidationError(_))));
        mock.assert();
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_json_extraction() {
        let text = r#"Here's your command: {
            "version": "1.0",
            "steps": [],
            "explanation": "test"
        }"#;
        let json = extract_json(text).unwrap();
        assert!(json.starts_with('{'));
        assert!(json.contains("version"));
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_invalid_json_extraction() {
        let text = "No JSON here at all";
        assert!(extract_json(text).is_err());

        let text = "Here's invalid JSON: { not valid }";
        assert!(extract_json(text).is_err());

        let text = r#"Here's incomplete JSON: {"version": "1.0"}"#;
        assert!(extract_json(text).is_err());
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_anthropic_response_parsing() {
        let (mut server, config) = setup_test_server().await;
        
        // Test successful response
        let mock = server
            .mock("POST", "/v1/messages")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(json!({
                "content": [{
                    "text": r#"Here's the command: {
                        "version": "1.0",
                        "steps": [{
                            "command": "echo test",
                            "explanation": "test command",
                            "is_dangerous": false,
                            "estimated_impact": {
                                "cpu_percentage": 0.1,
                                "memory_mb": 1.0,
                                "disk_mb": 0.0,
                                "network_mb": 0.0,
                                "duration_seconds": 0.1
                            },
                            "rollback_command": null
                        }],
                        "explanation": "test"
                    }"#
                }]
            }).to_string())
            .create();

        let result = get_command_chain("test command", &config).await;
        assert!(result.is_ok());
        let chain = result.unwrap();
        assert_eq!(chain.steps.len(), 1);
        assert_eq!(chain.steps[0].command, "echo test");
        mock.assert();
    }

    #[test]
    fn test_json_extraction_with_powershell_heredoc() {
        let text = r#"Some text before {
            "version": "1.0",
            "steps": [{
                "command": "Set-Content -Path 'test.py' -Value @'
def test():
    print('hello')
'@",
                "explanation": "test"
            }]
        }"#;
        let json = extract_json(text).unwrap();
        let parsed: Value = serde_json::from_str(&json).unwrap();
        assert!(parsed.get("version").is_some());
    }

    #[test]
    fn test_json_extraction_with_nested_quotes() {
        let text = r#"Response: {
            "version": "1.0",
            "steps": [{
                "command": "echo \"Hello, world!\"",
                "explanation": "test"
            }]
        }"#;
        let json = extract_json(text).unwrap();
        let parsed: Value = serde_json::from_str(&json).unwrap();
        assert!(parsed.get("version").is_some());
    }

    #[test]
    fn test_powershell_heredoc_extraction() {
        let text = r#"Some text {
            "version": "1.0",
            "steps": [{
                "command": "Set-Content -Path 'test.py' -Value @\"
def test():
    print('hello')
\"@",
                "explanation": "test"
            }]
        }"#;
        let json = extract_json(text).unwrap();
        let parsed: Value = serde_json::from_str(&json).unwrap();
        assert!(parsed.get("version").is_some());
        assert!(parsed.get("steps").is_some());
    }

    #[test]
    fn test_json_extraction_all_shells() {
        // Test PowerShell heredoc with double quotes
        let powershell = r#"Response: {
            "version": "1.0",
            "steps": [{
                "command": "Set-Content -Path 'test.py' -Value @\"
def test():
    print('hello')
\"@",
                "explanation": "test"
            }]
        }"#;
        println!("Input JSON:\n{}", powershell);
        let result = extract_json(powershell);
        println!("PowerShell test result: {:?}", result);
        
        // If it fails, let's try to parse it manually to see what's wrong
        if let Ok(json_str) = &result {
            println!("Preprocessed JSON:\n{}", json_str);
            let parse_result = serde_json::from_str::<Value>(json_str);
            println!("Parse result: {:?}", parse_result);
        }
        
        assert!(result.is_ok());

        // Test PowerShell heredoc with single quotes
        let powershell_single = r#"Response: {
            "version": "1.0",
            "steps": [{
                "command": "Set-Content -Path 'test.py' -Value @'
def test():
    print('hello')
'@",
                "explanation": "test"
            }]
        }"#;
        let result = extract_json(powershell_single);
        println!("PowerShell single quote test result: {:?}", result);
        assert!(result.is_ok());

        // Test Bash heredoc
        let bash = r#"Response: {
            "version": "1.0",
            "steps": [{
                "command": "cat << 'EOF' > test.py\ndef test():\n    print('hello')\nEOF",
                "explanation": "test"
            }]
        }"#;
        let result = extract_json(bash);
        println!("Bash test result: {:?}", result);
        assert!(result.is_ok());

        // Test simple command
        let simple = r#"Response: {
            "version": "1.0",
            "steps": [{
                "command": "mkdir test_dir",
                "explanation": "test"
            }]
        }"#;
        let result = extract_json(simple);
        println!("Simple command test result: {:?}", result);
        assert!(result.is_ok());

        // Test command with quotes
        let quoted = r#"Response: {
            "version": "1.0",
            "steps": [{
                "command": "echo \"Hello, world!\"",
                "explanation": "test"
            }]
        }"#;
        let result = extract_json(quoted);
        println!("Quoted command test result: {:?}", result);
        assert!(result.is_ok());
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_shell_specific_responses() {
        let (mut server, config) = setup_test_server().await;

        // Test PowerShell response
        let powershell_response = json!({
            "content": [{
                "text": r#"Here's the command: {
                    "version": "1.0",
                    "steps": [{
                        "command": "Set-Content -Path 'test.py' -Value @\"
def test():
    print('hello')
\"@",
                        "explanation": "test command",
                        "is_dangerous": false,
                        "estimated_impact": {
                            "cpu_percentage": 0.1,
                            "memory_mb": 1.0,
                            "disk_mb": 0.0,
                            "network_mb": 0.0,
                            "duration_seconds": 0.1
                        },
                        "rollback_command": null
                    }],
                    "explanation": "test"
                }"#
            }]
        });

        // Test Bash response
        let bash_response = json!({
            "content": [{
                "text": r#"Here's the command: {
                    "version": "1.0",
                    "steps": [{
                        "command": "cat << 'EOF' > test.py\ndef test():\n    print('hello')\nEOF",
                        "explanation": "test command",
                        "is_dangerous": false,
                        "estimated_impact": {
                            "cpu_percentage": 0.1,
                            "memory_mb": 1.0,
                            "disk_mb": 0.0,
                            "network_mb": 0.0,
                            "duration_seconds": 0.1
                        },
                        "rollback_command": null
                    }],
                    "explanation": "test"
                }"#
            }]
        });

        // Test PowerShell
        let mock = server
            .mock("POST", "/v1/messages")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(powershell_response.to_string())
            .create();

        let result = get_command_chain("test command", &config).await;
        assert!(result.is_ok());
        mock.assert();

        // Test Bash
        let mock = server
            .mock("POST", "/v1/messages")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(bash_response.to_string())
            .create();

        let result = get_command_chain("test command", &config).await;
        assert!(result.is_ok());
        mock.assert();
    }
} 
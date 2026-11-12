use std::{
    io::Write,
    process::{Command, Stdio},
};

use serde_json::{Value, json};

fn meta() -> Value {
    json!({
        "io.modelcontextprotocol/protocolVersion":"2026-07-28",
        "io.modelcontextprotocol/clientCapabilities":{},
        "io.modelcontextprotocol/clientInfo":{"name":"process-test","version":"1"}
    })
}

#[test]
fn default_startup_refuses_the_missing_production_composition() {
    let output = Command::new(env!("CARGO_BIN_EXE_vellum-semantic-mcp"))
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(2));
    assert!(output.stdout.is_empty());
    assert!(
        String::from_utf8(output.stderr)
            .unwrap()
            .contains("no semantic composition selected")
    );
}

#[test]
fn unavailable_peer_serves_discovery_and_refuses_a_query_without_claiming_readiness() {
    let mut child = Command::new(env!("CARGO_BIN_EXE_vellum-semantic-mcp"))
        .args(["--unavailable", "--metrics-stderr"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let discover = json!({
        "jsonrpc":"2.0","id":1,"method":"server/discover","params":{"_meta":meta()}
    });
    let capabilities = json!({
        "jsonrpc":"2.0","id":2,"method":"resources/read",
        "params":{"_meta":meta(),"uri":"vellum-semantic://capabilities"}
    });
    let query = json!({
        "jsonrpc":"2.0","id":3,"method":"tools/call",
        "params":{
            "_meta":meta(),
            "name":"vellum.semantic.diagnostics",
            "arguments":{
                "request":"01010101010101010101010101010101",
                "document":"02020202020202020202020202020202",
                "source_revision":1,
                "index_generation":1,
                "language":"rust"
            }
        }
    });
    {
        let input = child.stdin.as_mut().unwrap();
        writeln!(input, "{discover}").unwrap();
        writeln!(input, "{capabilities}").unwrap();
        writeln!(input, "{query}").unwrap();
    }
    drop(child.stdin.take());
    let output = child.wait_with_output().unwrap();
    assert!(output.status.success());
    let replies: Vec<Value> = String::from_utf8(output.stdout)
        .unwrap()
        .lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect();
    assert_eq!(replies.len(), 3);
    assert_eq!(replies[0]["result"]["supportedVersions"][0], "2026-07-28");
    let capability_text = replies[1]["result"]["contents"][0]["text"]
        .as_str()
        .unwrap();
    let capability: Value = serde_json::from_str(capability_text).unwrap();
    assert_eq!(capability["readiness"]["state"], "unavailable");
    assert_eq!(capability["read_index"], false);
    assert_eq!(capability["plan_rename"], false);
    assert_eq!(
        replies[2]["result"]["structuredContent"],
        json!({"status":"refused","reason":"unknown_document"})
    );
    let stderr = String::from_utf8(output.stderr).unwrap();
    let metric: Value = serde_json::from_str(stderr.trim()).unwrap();
    assert_eq!(metric["event"], "vellum_semantic_mcp_exit");
    assert_eq!(metric["backend"], "unavailable");
    assert_eq!(metric["stdio"]["frames"], 3);
    assert_eq!(metric["mcp"]["tool_calls"], 1);
}

#[test]
fn nonempty_eof_fragment_is_rejected_and_never_executed() {
    let mut child = Command::new(env!("CARGO_BIN_EXE_vellum-semantic-mcp"))
        .arg("--unavailable")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    child
        .stdin
        .as_mut()
        .unwrap()
        .write_all(br#"{"jsonrpc":"2.0","id":9,"method":"server/discover","params":{}}"#)
        .unwrap();
    drop(child.stdin.take());
    let output = child.wait_with_output().unwrap();
    assert!(output.status.success());
    let reply: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(reply["id"], Value::Null);
    assert_eq!(reply["error"]["code"], -32700);
    assert_eq!(reply["error"]["message"], "truncated request");
}

#[test]
fn oversized_frame_is_drained_before_the_next_request() {
    let mut child = Command::new(env!("CARGO_BIN_EXE_vellum-semantic-mcp"))
        .arg("--unavailable")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let discover = json!({
        "jsonrpc":"2.0","id":7,"method":"server/discover","params":{"_meta":meta()}
    });
    {
        let input = child.stdin.as_mut().unwrap();
        input.write_all(&vec![b'x'; 256 * 1024 + 1]).unwrap();
        writeln!(input).unwrap();
        writeln!(input, "{discover}").unwrap();
    }
    drop(child.stdin.take());
    let output = child.wait_with_output().unwrap();
    assert!(output.status.success());
    let replies: Vec<Value> = String::from_utf8(output.stdout)
        .unwrap()
        .lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect();
    assert_eq!(replies.len(), 2);
    assert_eq!(replies[0]["error"]["code"], -32600);
    assert_eq!(replies[0]["error"]["message"], "request exceeds limit");
    assert_eq!(replies[1]["id"], 7);
    assert_eq!(replies[1]["result"]["supportedVersions"][0], "2026-07-28");
}

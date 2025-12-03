use component_script_rhai::{ComponentResult, describe_payload, handle_invocation};
use serde_json::json;

fn invoke(invocation: serde_json::Value) -> ComponentResult {
    let input = invocation.to_string();
    handle_invocation(&input).expect("invocation should succeed")
}

#[test]
fn describe_mentions_world() {
    let payload = describe_payload();
    let json: serde_json::Value = serde_json::from_str(&payload).expect("describe should be json");
    assert_eq!(
        json["component"]["world"],
        "greentic:component/component@0.4.0"
    );
}

#[test]
fn basic_transform_wraps_output() {
    let invocation = json!({
        "config": { "script": "let name = state.user.name; #{ greeting: \"Hello \" + name }" },
        "payload": {},
        "state": { "user": { "name": "Alice" } },
        "connections": ["next"]
    });

    let result = invoke(invocation);
    assert_eq!(result.payload["output"]["greeting"], "Hello Alice");
    assert!(result.error.is_none());
}

#[test]
fn structured_greentic_routing() {
    let invocation = json!({
        "config": { "script": r#"
            let res = #{};
            res.__greentic = #{ payload: #{ confirmed: true }, out: ["next_node"], err: ["error_node"] };
            res
        "# },
        "payload": {},
        "state": {},
        "connections": ["next_node", "error_node"]
    });

    let result = invoke(invocation);

    let payload = result.payload.clone();
    assert!(result.error.is_none(), "error: {:?}", result.error);
    assert_eq!(
        payload,
        json!({"confirmed": true}),
        "payload: {:?}",
        payload
    );
    let control = result.control.expect("control should be present");
    assert_eq!(control.out.unwrap(), vec!["next_node"]);
    assert_eq!(control.err.unwrap(), vec!["error_node"]);
    assert!(result.error.is_none());
}

#[test]
fn state_mutation_persists_on_error() {
    let invocation = json!({
        "config": { "script": r#"
            state.counter += 1;
            throw "boom";
        "# },
        "payload": {},
        "state": { "counter": 0 },
        "connections": []
    });

    let result =
        handle_invocation(&invocation.to_string()).expect("invocation should return result");

    assert_eq!(result.state_updates["counter"], 1);
    let error = result.error.expect("error expected");
    assert_eq!(error.kind, "ScriptError");
}

#[test]
fn non_serializable_return_falls_back() {
    let invocation = json!({
        "config": { "script": "return || true;" },
        "payload": {},
        "state": {},
        "connections": []
    });

    let result =
        handle_invocation(&invocation.to_string()).expect("invocation should return result");

    let error = result.error.expect("conversion error expected");
    assert_eq!(error.kind, "SerializationError");
    assert_eq!(
        result.payload["error"],
        "Failed to convert Rhai result to JSON"
    );
}

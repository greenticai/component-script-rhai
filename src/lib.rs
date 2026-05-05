use serde_json::{Value, json};

mod model;
mod script_engine;

use crate::model::{
    ComponentControl, GreenticDirectives, InvocationEnvelope, OnError, ResultMode, ScriptConfig,
};
use crate::script_engine::{ScriptOutcome, execute_script};

pub use crate::model::{ComponentError, ComponentResult};

#[cfg(target_arch = "wasm32")]
#[used]
#[unsafe(link_section = ".greentic.wasi")]
static WASI_TARGET_MARKER: [u8; 13] = *b"wasm32-wasip2";

/// Returns the component manifest payload.
pub fn describe_payload() -> String {
    serde_json::json!({
        "component": {
            "name": "component-script-rhai",
            "org": "ai.greentic",
            "version": "0.1.0",
            "world": "greentic:component/component@0.4.0",
            "schemas": {
                "component": "schemas/component.schema.json",
                "input": "schemas/io/input.schema.json",
                "output": "schemas/io/output.schema.json"
            }
        }
    })
    .to_string()
}

/// Handles a component invocation represented as JSON.
pub fn handle_invocation(raw_input: &str) -> Result<ComponentResult, ComponentError> {
    let mut envelope: InvocationEnvelope =
        serde_json::from_str(raw_input).map_err(|err| ComponentError {
            kind: "InvalidInput".to_string(),
            message: format!("Invalid invocation envelope: {err}"),
            details: None,
        })?;

    envelope.normalize();

    let config =
        ScriptConfig::from_value(envelope.config.clone()).map_err(|message| ComponentError {
            kind: "InvalidConfig".to_string(),
            message,
            details: Some(envelope.config.clone()),
        })?;

    let ScriptOutcome {
        state_out,
        script_value,
        directives,
        script_error,
        conversion_error,
    } = execute_script(&envelope, &config);

    let state_updates = compute_state_updates(&envelope.state, &state_out);

    let (payload, control, error) = build_component_result(
        &config,
        script_value,
        directives,
        script_error,
        conversion_error,
    );

    Ok(ComponentResult {
        payload,
        state_updates,
        control,
        error,
    })
}

fn compute_state_updates(state_in: &Value, state_out: &Value) -> Value {
    if state_in == state_out {
        return json!({});
    }

    match (state_in, state_out) {
        (Value::Object(old), Value::Object(new)) => {
            let mut diff = serde_json::Map::new();
            for (key, new_val) in new {
                let entry = old.get(key);
                if entry.is_none() {
                    diff.insert(key.clone(), new_val.clone());
                    continue;
                }
                let old_val = entry.unwrap();
                if old_val == new_val {
                    continue;
                }
                let child = compute_state_updates(old_val, new_val);
                if !child.is_null() && child != json!({}) {
                    diff.insert(key.clone(), child);
                } else if old_val != new_val {
                    diff.insert(key.clone(), new_val.clone());
                }
            }
            Value::Object(diff)
        }
        _ => state_out.clone(),
    }
}

fn build_component_result(
    config: &ScriptConfig,
    script_value: Option<Value>,
    directives: Option<GreenticDirectives>,
    script_error: Option<String>,
    conversion_error: Option<String>,
) -> (Value, Option<ComponentControl>, Option<ComponentError>) {
    let mut control = None;
    let mut payload = Value::Null;
    let mut error = script_error.map(|msg| ComponentError {
        kind: "ScriptError".to_string(),
        message: format!("Script error: {msg}"),
        details: None,
    });

    if let Some(conv_err) = conversion_error {
        error = Some(ComponentError {
            kind: "SerializationError".to_string(),
            message: conv_err,
            details: None,
        });
        payload = json!({ "error": "Failed to convert Rhai result to JSON" });
        return (payload, control, error);
    }

    if let Some(directives) = directives {
        if let Some(out) = directives.out {
            control.get_or_insert_with(Default::default).out = Some(out);
        }

        if let Some(err) = directives.err {
            control.get_or_insert_with(Default::default).err = Some(err);
        }

        payload = directives.payload.unwrap_or(Value::Null);
    } else if let Some(value) = script_value {
        payload = match config.result_mode {
            ResultMode::Wrap => json!({ "output": value }),
            ResultMode::Raw => value,
        };
    }

    if error.is_some() && payload.is_null() && config.on_error == OnError::Fail {
        payload = json!({ "error": error.as_ref().map(|e| e.message.clone()).unwrap_or_default() });
    }

    (payload, control, error)
}

#[cfg(target_arch = "wasm32")]
mod component {
    use super::handle_invocation;
    use greentic_interfaces_guest::component_v0_6::{component_i18n, component_qa, node};
    use greentic_types::cbor::canonical;

    pub(super) struct Component;

    impl node::Guest for Component {
        fn describe() -> node::ComponentDescriptor {
            node::ComponentDescriptor {
                name: "component-script-rhai".to_string(),
                version: env!("CARGO_PKG_VERSION").to_string(),
                summary: Some("Execute Rhai scripts over invocation envelopes".to_string()),
                capabilities: Vec::new(),
                ops: Vec::new(),
                schemas: Vec::new(),
                setup: None,
            }
        }

        fn invoke(
            _op: String,
            envelope: node::InvocationEnvelope,
        ) -> Result<node::InvocationResult, node::NodeError> {
            // Decode the CBOR payload into a JSON Value, then delegate to the
            // existing JSON-based handle_invocation logic.
            let input_json: serde_json::Value = canonical::from_cbor(&envelope.payload_cbor)
                .map_err(|err| node::NodeError {
                    code: "CborDecodeError".to_string(),
                    message: format!("Failed to decode payload CBOR: {err}"),
                    retryable: false,
                    backoff_ms: None,
                    details: None,
                })?;

            let input_str = serde_json::to_string(&input_json).map_err(|err| node::NodeError {
                code: "SerializationError".to_string(),
                message: format!("Failed to serialize input to JSON: {err}"),
                retryable: false,
                backoff_ms: None,
                details: None,
            })?;

            match handle_invocation(&input_str) {
                Ok(result) => {
                    let result_json =
                        serde_json::to_value(&result).map_err(|err| node::NodeError {
                            code: "SerializationError".to_string(),
                            message: format!("Failed to serialize result: {err}"),
                            retryable: false,
                            backoff_ms: None,
                            details: None,
                        })?;

                    let output_cbor = canonical::to_canonical_cbor_allow_floats(&result_json)
                        .map_err(|err| node::NodeError {
                            code: "CborEncodeError".to_string(),
                            message: format!("Failed to encode result as CBOR: {err}"),
                            retryable: false,
                            backoff_ms: None,
                            details: None,
                        })?;

                    Ok(node::InvocationResult {
                        ok: result.error.is_none(),
                        output_cbor,
                        output_metadata_cbor: None,
                    })
                }
                Err(err) => Err(node::NodeError {
                    code: err.kind,
                    message: err.message,
                    retryable: false,
                    backoff_ms: None,
                    details: None,
                }),
            }
        }
    }

    impl component_qa::Guest for Component {
        fn qa_spec(_mode: component_qa::QaMode) -> Vec<u8> {
            // No QA wizard for this component — return empty CBOR map.
            vec![0xa0]
        }

        fn apply_answers(
            _mode: component_qa::QaMode,
            _current_config: Vec<u8>,
            _answers: Vec<u8>,
        ) -> Vec<u8> {
            // No QA wizard — return empty CBOR map.
            vec![0xa0]
        }
    }

    impl component_i18n::Guest for Component {
        fn i18n_keys() -> Vec<String> {
            Vec::new()
        }
    }
}

#[cfg(target_arch = "wasm32")]
greentic_interfaces_guest::export_component_v060!(
    component::Component,
    component_qa: component::Component,
    component_i18n: component::Component,
);

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn describe_payload_is_json() {
        let payload = describe_payload();
        let json: Value = serde_json::from_str(&payload).expect("valid json");
        assert_eq!(json["component"]["name"], "component-script-rhai");
    }

    #[test]
    fn handle_invocation_wraps_output() {
        let invocation = json!({
            "config": { "script": "let name = state.user.name; #{ greeting: \"Hello \" + name }" },
            "payload": {},
            "state": { "user": { "name": "Alice" } },
            "connections": ["next"]
        });

        let result = handle_invocation(&invocation.to_string()).expect("invocation ok");
        assert_eq!(result.payload["output"]["greeting"], "Hello Alice");
        assert!(result.error.is_none());
    }
}

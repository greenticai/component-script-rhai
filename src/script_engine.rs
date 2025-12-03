use rhai::{Array, Dynamic, Engine, FLOAT, INT, ImmutableString, Map, Scope};
use serde_json::{Number, Value};

use crate::model::{GreenticDirectives, InvocationEnvelope, ScriptConfig, directives_from_value};

#[derive(Debug)]
pub struct ScriptOutcome {
    pub state_out: Value,
    pub script_value: Option<Value>,
    pub directives: Option<GreenticDirectives>,
    pub script_error: Option<String>,
    pub conversion_error: Option<String>,
}

pub fn execute_script(invocation: &InvocationEnvelope, config: &ScriptConfig) -> ScriptOutcome {
    let engine = Engine::new();
    let mut scope = Scope::new();

    let msg_value = invocation
        .msg
        .as_ref()
        .and_then(|m| serde_json::to_value(m).ok())
        .unwrap_or(Value::Null);
    let connections_value = Value::Array(
        invocation
            .connections
            .iter()
            .map(|c| Value::String(c.clone()))
            .collect(),
    );

    let msg_dynamic = json_to_dynamic(&msg_value).unwrap_or(Dynamic::UNIT);
    let payload_dynamic = json_to_dynamic(&invocation.payload).unwrap_or(Dynamic::UNIT);
    let state_dynamic = json_to_dynamic(&invocation.state).unwrap_or(Dynamic::UNIT);
    let connections_dynamic = json_to_dynamic(&connections_value).unwrap_or(Dynamic::UNIT);

    scope.push_dynamic("msg", msg_dynamic);
    scope.push_dynamic("payload", payload_dynamic);
    scope.push_dynamic("state", state_dynamic);
    scope.push_dynamic("connections", connections_dynamic);

    let eval_result = engine.eval_with_scope::<Dynamic>(&mut scope, &config.script);

    let mut script_error = None;
    let mut script_value = None;

    match eval_result {
        Ok(value) => script_value = Some(value),
        Err(err) => script_error = Some(err.to_string()),
    }

    let state_dynamic = scope.get_value::<Dynamic>("state").unwrap_or(Dynamic::UNIT);

    let mut conversion_error = None;
    let state_out = match dynamic_to_json(&state_dynamic) {
        Ok(value) => value,
        Err(err) => {
            conversion_error = Some(format!("Failed to convert state to JSON: {err}"));
            invocation.state.clone()
        }
    };

    let script_value = match script_value {
        Some(value) => match dynamic_to_json(&value) {
            Ok(json) => Some(json),
            Err(err) => {
                conversion_error = Some(format!("Failed to convert Rhai result to JSON: {err}"));
                None
            }
        },
        None => None,
    };

    let directives = script_value.as_ref().and_then(directives_from_value);

    ScriptOutcome {
        state_out,
        script_value,
        directives,
        script_error,
        conversion_error,
    }
}

fn json_to_dynamic(value: &Value) -> Result<Dynamic, String> {
    match value {
        Value::Null => Ok(Dynamic::UNIT),
        Value::Bool(b) => Ok(Dynamic::from_bool(*b)),
        Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                Ok(Dynamic::from_int(i))
            } else if let Some(u) = n.as_u64() {
                Ok(Dynamic::from_int(u as INT))
            } else if let Some(f) = n.as_f64() {
                Ok(Dynamic::from_float(f))
            } else {
                Err("Unsupported number type".to_string())
            }
        }
        Value::String(s) => Ok(Dynamic::from(s.clone())),
        Value::Array(items) => {
            let mut arr = Array::new();
            for item in items {
                arr.push(json_to_dynamic(item)?);
            }
            Ok(Dynamic::from_array(arr))
        }
        Value::Object(map) => {
            let mut out = Map::new();
            for (k, v) in map {
                out.insert(k.into(), json_to_dynamic(v)?);
            }
            Ok(Dynamic::from_map(out))
        }
    }
}

fn dynamic_to_json(value: &Dynamic) -> Result<Value, String> {
    if value.is_unit() {
        return Ok(Value::Null);
    }

    if let Some(b) = value.clone().try_cast::<bool>() {
        return Ok(Value::Bool(b));
    }

    if let Some(i) = value.clone().try_cast::<INT>() {
        return Ok(Value::Number(Number::from(i)));
    }

    if let Some(f) = value.clone().try_cast::<FLOAT>() {
        return Number::from_f64(f)
            .map(Value::Number)
            .ok_or_else(|| "Invalid float".to_string());
    }

    if let Some(s) = value.clone().try_cast::<ImmutableString>() {
        return Ok(Value::String(s.into()));
    }

    if let Some(arr) = value.clone().try_cast::<Array>() {
        let mut out = Vec::with_capacity(arr.len());
        for item in arr {
            out.push(dynamic_to_json(&item)?);
        }
        return Ok(Value::Array(out));
    }

    if let Some(map) = value.clone().try_cast::<Map>() {
        let mut out = serde_json::Map::new();
        for (k, v) in map {
            out.insert(k.to_string(), dynamic_to_json(&v)?);
        }
        return Ok(Value::Object(out));
    }

    Err("Unsupported Rhai value".to_string())
}

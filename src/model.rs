use greentic_types::ChannelMessageEnvelope;
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum ResultMode {
    #[default]
    Wrap,
    Raw,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum OnError {
    #[default]
    Fail,
    Continue,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct ScriptConfig {
    pub script: String,
    pub result_mode: ResultMode,
    pub on_error: OnError,
}

impl Default for ScriptConfig {
    fn default() -> Self {
        Self {
            script: String::new(),
            result_mode: ResultMode::Wrap,
            on_error: OnError::Fail,
        }
    }
}

impl ScriptConfig {
    pub fn from_value(value: Value) -> Result<Self, String> {
        let cfg: ScriptConfig =
            serde_json::from_value(value).map_err(|err| format!("Invalid config: {err}"))?;

        if cfg.script.trim().is_empty() {
            return Err("config.script is required".to_string());
        }

        Ok(cfg)
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct InvocationEnvelope {
    pub config: Value,
    pub msg: Option<ChannelMessageEnvelope>,
    pub payload: Value,
    pub state: Value,
    pub connections: Vec<String>,
}

impl Default for InvocationEnvelope {
    fn default() -> Self {
        Self {
            config: Value::Object(Default::default()),
            msg: None,
            payload: Value::Null,
            state: Value::Object(Default::default()),
            connections: Vec::new(),
        }
    }
}

impl InvocationEnvelope {
    pub fn normalize(&mut self) {
        if self.config.is_null() {
            self.config = Value::Object(Default::default());
        }

        if self.payload.is_null() {
            self.payload = Value::Object(Default::default());
        }

        if self.state.is_null() {
            self.state = Value::Object(Default::default());
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct ComponentControl {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub out: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub err: Option<Vec<String>>,
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct ComponentError {
    pub kind: String,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<Value>,
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct ComponentResult {
    pub payload: Value,
    #[serde(default)]
    pub state_updates: Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub control: Option<ComponentControl>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<ComponentError>,
}

#[derive(Clone, Debug, Default)]
pub struct GreenticDirectives {
    pub payload: Option<Value>,
    pub out: Option<Vec<String>>,
    pub err: Option<Vec<String>>,
}

pub fn directives_from_value(value: &Value) -> Option<GreenticDirectives> {
    let obj = value.as_object()?;
    let directives = obj.get("__greentic")?.as_object()?;

    let payload = directives.get("payload").cloned();
    let out = parse_connections(directives.get("out"));
    let err = parse_connections(directives.get("err"));

    Some(GreenticDirectives { payload, out, err })
}

fn parse_connections(value: Option<&Value>) -> Option<Vec<String>> {
    match value {
        Some(Value::String(item)) => Some(vec![item.to_string()]),
        Some(Value::Array(items)) => {
            let mut out = Vec::new();
            for item in items {
                if let Some(s) = item.as_str() {
                    out.push(s.to_string());
                }
            }
            if out.is_empty() { None } else { Some(out) }
        }
        _ => None,
    }
}

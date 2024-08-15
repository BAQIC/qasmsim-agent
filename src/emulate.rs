use axum::Json;
use serde::Deserialize;
use serde_json::{json, Value};
use std::{collections::HashMap, fmt};

#[derive(Deserialize, Debug, Clone)]
pub enum EmulateMode {
    #[serde(rename = "sequence")]
    Sequence,
    #[serde(rename = "aggregation")]
    Aggregation,
    #[serde(rename = "max")]
    Max,
    #[serde(rename = "min")]
    Min,
}

/// For the `EmulateMode` enum, we need to implement the `FromStr` trait to
/// convert the string to the enum. This is used when the user sends the status
/// in the form of a string.
#[derive(Debug, PartialEq, Eq)]
pub struct ParseEmulateModeError;

impl fmt::Display for ParseEmulateModeError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Invalid emulate mode")
    }
}

impl fmt::Display for EmulateMode {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            EmulateMode::Sequence => write!(f, "sequence"),
            EmulateMode::Aggregation => write!(f, "aggregation"),
            EmulateMode::Max => write!(f, "max"),
            EmulateMode::Min => write!(f, "min"),
        }
    }
}

#[derive(Deserialize, Debug)]
pub struct EmulateMessage {
    pub qasm: String,
    pub shots: usize,
    pub mode: Option<EmulateMode>,
    pub vars: Option<HashMap<String, f32>>,
}

pub fn post_process_msg_agg(seq: Vec<String>) -> Json<Value> {
    let mut mem = HashMap::new();
    for s in seq {
        let count = mem.entry(s).or_insert(0);
        *count += 1;
    }

    Json(json!({
        "Memory": mem,
    }))
}

pub fn post_process_msg_minmax(seq: Vec<String>, is_max: bool) -> Json<Value> {
    let mut mem = HashMap::new();
    for s in seq {
        let count = mem.entry(s).or_insert(0);
        *count += 1;
    }

    if is_max {
        let max = mem.iter().max_by_key(|&(_, count)| count).unwrap();
        Json(json!({
            "Memory": {
                max.0: max.1,
            },
        }))
    } else {
        let min = mem.iter().min_by_key(|&(_, count)| count).unwrap();
        Json(json!({
            "Memory": {
                min.0: min.1,
            },
        }))
    }
}

pub fn post_process_msg(seq: Vec<String>, mode: String) -> Result<Json<Value>, String> {
    match mode.as_str() {
        "sequence" => Ok(Json(json!({
            "Memory": seq,
        }))),
        "aggregation" => Ok(post_process_msg_agg(seq)),
        "max" => Ok(post_process_msg_minmax(seq, true)),
        "min" => Ok(post_process_msg_minmax(seq, false)),
        _ => Err("Invalid mode".to_string()),
    }
}

pub fn pre_process_msg(mut msg: EmulateMessage) -> EmulateMessage {
    if msg.vars.is_some() {
        for (key, value) in msg.vars.as_ref().unwrap() {
            msg.qasm = msg.qasm.replace(key, &value.to_string());
        }
    }

    msg
}

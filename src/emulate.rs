use axum::Json;
use serde::Deserialize;
use serde_json::{json, Value};
use std::{collections::HashMap, fmt};

use crate::SharedState;

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
    #[serde(rename = "expectation")]
    Expectation,
    #[serde(rename = "vqe")]
    Vqe,
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
            EmulateMode::Expectation => write!(f, "expectation"),
            EmulateMode::Vqe => write!(f, "vqe"),
        }
    }
}

/// Only for deserialize the post message
#[derive(Deserialize, Debug, Clone)]
pub struct EmulateMessage {
    pub qasm: String,
    pub shots: usize,
    pub mode: Option<EmulateMode>,
    // only when the mode is vqe, this field is required
    pub iterations: Option<usize>,
    pub vars: Option<String>,
    pub vars_range: Option<String>,
}

/// For simulator use
#[derive(Deserialize, Debug)]
pub struct EmulateInfo {
    pub qasm: String,
    pub shots: Option<usize>,
    pub mode: Option<EmulateMode>,
}

pub fn post_process_msg_agg(seq: Vec<String>) -> Json<Value> {
    let mut mem = HashMap::new();
    for s in seq {
        let count = mem.entry(s).or_insert(0);
        *count += 1;
    }

    Json(json!({
        "Result": mem,
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
            "Result": {
                max.0: max.1,
            },
        }))
    } else {
        let min = mem.iter().min_by_key(|&(_, count)| count).unwrap();
        Json(json!({
            "Result": {
                min.0: min.1,
            },
        }))
    }
}

/// current for z expectation
pub fn post_process_msg_expe(seq: Vec<String>) -> Json<Value> {
    let len = seq.len();
    let mut exp: Vec<f32> = if len != 0 {
        vec![0.0; seq[0].len()]
    } else {
        Vec::new()
    };

    for s in seq {
        let char = s.chars();
        for (i, c) in char.enumerate() {
            if c == '1' {
                exp[i] -= 1.0;
            } else {
                exp[i] += 1.0;
            }
        }
    }

    exp = exp.into_iter().map(|x| x / len as f32).collect();

    Json(json!({"Result": [exp]}))
}

pub fn post_process_msg_vqe(seq: Vec<f64>) -> Result<Json<Value>, String> {
    println!("{:?}", seq);
    Ok(Json(json!({})))
}

pub async fn post_process_msg(
    state: SharedState,
    seq: Vec<String>,
    mode: String,
) -> Result<Json<Value>, String> {
    let mut state_w = state.write().await;
    for s in seq.iter() {
        state_w.results.update_results(s);
    }

    match mode.as_str() {
        "sequence" => Ok(Json(json!({
            "Result": [seq],
        }))),
        "aggregation" => Ok(post_process_msg_agg(seq)),
        "max" => Ok(post_process_msg_minmax(seq, true)),
        "min" => Ok(post_process_msg_minmax(seq, false)),
        "expectation" => Ok(post_process_msg_expe(seq)),
        _ => Err("Invalid mode".to_string()),
    }
}

pub fn pre_process_msg(msg: EmulateMessage) -> EmulateInfo {
    let vars = serde_json::from_str::<HashMap<String, f32>>(
        msg.vars.clone().unwrap_or("{}".to_string()).as_str(),
    )
    .unwrap();

    let mut qasm_ = msg.qasm.clone();

    if msg.vars.is_some() {
        for (key, value) in vars.iter() {
            qasm_ = qasm_.replace(key, &value.to_string());
        }
    }

    EmulateInfo {
        qasm: qasm_,
        shots: if msg.shots == 0 {
            Some(1)
        } else {
            Some(msg.shots)
        },
        mode: msg.mode,
    }
}

pub fn pre_process_msg_vqe(
    msg: EmulateMessage,
    vars_range: HashMap<String, (f32, f32)>,
    iteration: usize,
    iterations: usize,
) -> EmulateInfo {
    let mut vars: HashMap<String, f32> = HashMap::new();

    for (key, value) in vars_range {
        vars.insert(
            key,
            value.0 + (value.1 - value.0) * iteration as f32 / (iterations - 1) as f32,
        );
    }

    let mut qasm_ = msg.qasm.clone();
    if !vars.is_empty() {
        for (key, value) in vars.iter() {
            qasm_ = qasm_.replace(key, &value.to_string());
        }
    }

    EmulateInfo {
        qasm: qasm_,
        shots: None,
        mode: msg.mode,
    }
}

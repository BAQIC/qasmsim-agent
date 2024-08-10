use axum::Json;
use serde::{de, Deserialize, Deserializer};
use serde_json::{json, Value};
use std::{collections::HashMap, fmt, str::FromStr};

#[derive(Deserialize, Debug)]
pub enum EmulateMode {
    Sequence,
    Aggregation,
    Max,
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

impl FromStr for EmulateMode {
    type Err = ParseEmulateModeError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "sequence" => Ok(EmulateMode::Sequence),
            "aggregation" => Ok(EmulateMode::Aggregation),
            "max" => Ok(EmulateMode::Max),
            "min" => Ok(EmulateMode::Min),
            _ => Err(ParseEmulateModeError),
        }
    }
}

#[derive(Deserialize, Debug)]
pub struct EmulateMessage {
    pub qasm: String,
    pub shots: usize,
    #[serde(default, deserialize_with = "empty_string_as_none")]
    pub mode: Option<EmulateMode>,
}

/// The function that converts an empty string to `None` when deserializing the
/// optional field.
fn empty_string_as_none<'de, D, T>(de: D) -> Result<Option<T>, D::Error>
where
    D: Deserializer<'de>,
    T: FromStr,
    T::Err: fmt::Display,
{
    let opt = Option::<String>::deserialize(de)?;
    match opt.as_deref() {
        None | Some("") => Ok(None),
        Some(s) => FromStr::from_str(s).map_err(de::Error::custom).map(Some),
    }
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

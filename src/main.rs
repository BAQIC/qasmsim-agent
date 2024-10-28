use std::{collections::HashMap, sync::Arc};

use axum::{
    extract::{Query, Request, State},
    http::{header, StatusCode},
    routing, Form, Json, RequestExt, Router,
};
use emulate::{EmulateMessage, EmulateMode};
use serde::Deserialize;
use serde_json::{json, Value};
use tokio::sync::{oneshot, RwLock};
pub mod emulate;
pub mod optimizer;
pub mod qubits;
pub mod thread;

#[derive(Debug, Clone)]
pub struct ServerState {
    pub measure_path: String,
    pub qmem: qubits::QMemory,
    pub qreg: qubits::QResgister,
}

type SharedState = Arc<RwLock<ServerState>>;

/// For classical storage initialize and update
#[derive(Deserialize, Debug, Clone)]
pub struct ClassicalInfo {
    pub qbits: Option<usize>,
    pub capacity: Option<usize>,
}

/// For classical storage query
#[derive(Deserialize, Debug, Clone)]
pub struct MeasurePos {
    pub pos: usize,
}

/// consume_task is the main function to consume the task
/// it will spawn the quantum_thread and classical_thread execept for VQE
/// for VQE, it will spawn multiple classical_thread_vqe amd quantum_thread_vqe
pub async fn consume_task(
    state: SharedState,
    Form(mut message): Form<emulate::EmulateMessage>,
) -> (StatusCode, Json<Value>) {
    message.mode = Some(message.mode.unwrap_or(EmulateMode::Aggregation));

    match message.mode {
        Some(EmulateMode::Aggregation)
        | Some(EmulateMode::Max)
        | Some(EmulateMode::Min)
        | Some(EmulateMode::Expectation)
        | Some(EmulateMode::Sequence) => {
            let (msg_tx, msg_rx) = oneshot::channel();
            let (res_tx, res_rx) = oneshot::channel();

            tokio::spawn(thread::quantum_thread(msg_rx, res_tx));
            tokio::spawn(thread::classical_thread(state, message, msg_tx, res_rx))
                .await
                .unwrap()
        }
        Some(EmulateMode::Vqe) => {
            let vars_range = match serde_json::from_str::<HashMap<String, (f32, f32)>>(
                message.vars.clone().unwrap_or("{}".to_string()).as_str(),
            ) {
                Ok(vars_range) => vars_range,
                Err(_) => {
                    return (
                        StatusCode::BAD_REQUEST,
                        Json(json!({"Error": "Invalid vars range"})),
                    )
                }
            };

            // if default value is 1, then the variable will NAN
            let iterations = message.iterations.unwrap_or(2);
            let mut results = Vec::new();

            for index in 0..iterations {
                let (msg_tx, msg_rx) = oneshot::channel();
                let (res_tx, res_rx) = oneshot::channel();
                tokio::spawn(thread::quantum_thread_vqe(msg_rx, res_tx));
                match tokio::spawn(thread::classical_thread_vqe(
                    message.clone(),
                    vars_range.clone(),
                    index,
                    iterations,
                    msg_tx,
                    res_rx,
                ))
                .await
                {
                    Ok((status, json)) => {
                        if status != StatusCode::OK {
                            return (status, json);
                        }
                        results.push(json);
                    }
                    Err(err) => {
                        return (
                            StatusCode::BAD_REQUEST,
                            Json(json!({"Error": format!("{}", err)})),
                        )
                    }
                };
            }
            (StatusCode::OK, Json(json!({"Result": "Success"})))
        }
        _ => unreachable!(),
    }
}

/// endpoint to submit the task
pub async fn submit(
    State(state): State<SharedState>,
    request: Request,
) -> (StatusCode, Json<Value>) {
    match request.headers().get(header::CONTENT_TYPE) {
        Some(content_type) => match content_type.to_str().unwrap() {
            "application/x-www-form-urlencoded" => {
                let Form(message) = request.extract().await.unwrap();
                consume_task(state, Form(message)).await
            }
            "application/json" => {
                let Json::<EmulateMessage>(message) = request.extract().await.unwrap();
                consume_task(state, Form(message)).await
            }
            _ => (
                StatusCode::BAD_REQUEST,
                Json(json!({"Error": format!("content type {:?} not support", content_type)})),
            ),
        },
        _ => (
            StatusCode::BAD_REQUEST,
            Json(json!({"Error": format!("content type not specified")})),
        ),
    }
}

pub async fn update_classical(
    State(state): State<SharedState>,
    request: Request,
) -> (StatusCode, Json<Value>) {
    match request.headers().get(header::CONTENT_TYPE) {
        Some(content_type) => match content_type.to_str().unwrap() {
            "application/x-www-form-urlencoded" => {
                let Form(message): Form<ClassicalInfo> = request.extract().await.unwrap();
                let mut state_w = state.write().await;

                if message.qbits.is_some() {
                    state_w.qmem.update_qubits(message.qbits.unwrap());
                    state_w.qreg.update_qubits(message.qbits.unwrap());
                }

                if message.capacity.is_some() {
                    state_w.qmem.update_capacity(message.capacity.unwrap());
                }

                state_w.qmem.dump_file(&state_w.measure_path);

                (
                    StatusCode::OK,
                    Json(json!({"Result": format!("Update classical info with {:?}", message)})),
                )
            }
            "application/json" => {
                let Json::<ClassicalInfo>(message) = request.extract().await.unwrap();
                let mut state_w = state.write().await;

                if message.qbits.is_some() {
                    state_w.qmem.update_qubits(message.qbits.unwrap());
                    state_w.qreg.update_qubits(message.qbits.unwrap());
                }

                if message.capacity.is_some() {
                    state_w.qmem.update_capacity(message.capacity.unwrap());
                }

                state_w.qmem.dump_file(&state_w.measure_path);

                (
                    StatusCode::OK,
                    Json(json!({"Result": format!("Update classical info with {:?}", message)})),
                )
            }
            _ => (
                StatusCode::BAD_REQUEST,
                Json(json!({"Error": format!("content type {:?} not support", content_type)})),
            ),
        },
        _ => (
            StatusCode::BAD_REQUEST,
            Json(json!({"Error": format!("content type not specified")})),
        ),
    }
}

pub async fn get_measure(
    State(state): State<SharedState>,
    Query(pos): Query<MeasurePos>,
) -> (StatusCode, Json<Value>) {
    let state_r = state.read().await;
    if pos.pos > state_r.qmem.capacity {
        (
            StatusCode::BAD_REQUEST,
            Json(
                json!({"Error": format!("Quert position {} is larger than capacity {}", pos.pos, state_r.qmem.capacity)}),
            ),
        )
    } else {
        (
            StatusCode::OK,
            Json(json!({"Results": state_r.qmem.mem[pos.pos]})),
        )
    }
}

#[tokio::main]
async fn main() {
    if std::path::Path::new(".env").exists() {
        dotenv::dotenv().ok();
    }

    let measure_path =
        std::env::var("MEASURE_PATH").unwrap_or_else(|_| "./measure.pkl".to_string());

    let qmem = if std::path::Path::new(&measure_path).exists() {
        qubits::QMemory::read_file(&measure_path)
    } else {
        qubits::QMemory::default()
    };

    let state = Arc::new(RwLock::new(ServerState {
        measure_path: measure_path.clone(),
        qreg: qubits::QResgister::new(qmem.qubits),
        qmem,
    }));

    let listener_addr = std::env::var("LISTENER_ADDR").unwrap_or("0.0.0.0:3003".to_string());
    let qpp_router = Router::new()
        .route("/submit", routing::post(submit))
        .route("/update", routing::post(update_classical))
        .route("/get_measure", routing::get(get_measure))
        .with_state(state);
    let listener = tokio::net::TcpListener::bind(listener_addr).await.unwrap();
    axum::serve(listener, qpp_router).await.unwrap();
}

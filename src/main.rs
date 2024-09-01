use std::collections::HashMap;

use axum::{
    extract::Request,
    http::{header, StatusCode},
    routing, Form, Json, RequestExt, Router,
};
use emulate::{pre_process_msg, pre_process_msg_vqe, EmulateInfo, EmulateMessage, EmulateMode};
use serde_json::{json, Value};
use tokio::sync::oneshot;
pub mod emulate;
pub mod optimizer;

/// TODO: merge quantum_thread and quantum_thread_vqe
/// quantum thread for aggregation, max, min, expectation, and sequence
pub async fn quantum_thread(
    msg_rx: oneshot::Receiver<EmulateInfo>,
    res_tx: oneshot::Sender<Result<qasmsim::Execution, String>>,
) {
    let msg = msg_rx.await.unwrap();
    match qasmsim::run_mode(&msg.qasm, msg.shots, "sequence".to_string()) {
        Ok(result) => {
            // send the result to the classical_thread
            res_tx.send(Ok(result)).unwrap()
        }
        Err(err) => {
            // if there are some errors, send the error message to the classical_thread
            res_tx.send(Err(err.to_string())).unwrap()
        }
    }
}

/// quantum thread for VQE
pub async fn quantum_thread_vqe(
    msg_rx: oneshot::Receiver<EmulateInfo>,
    res_tx: oneshot::Sender<Result<qasmsim::Execution, String>>,
) {
    let msg = msg_rx.await.unwrap();
    match qasmsim::run(&msg.qasm, msg.shots) {
        Ok(result) => {
            // send the result to the classical_thread
            res_tx.send(Ok(result)).unwrap()
        }
        Err(err) => {
            // if there are some errors, send the error message to the classical_thread
            res_tx.send(Err(err.to_string())).unwrap()
        }
    }
}

/// TODO: merge classical_thread and classical_thread_vqe
/// classical thread for aggregation, max, min, expectation, and sequence
pub async fn classical_thread(
    msg: EmulateMessage,
    msg_tx: oneshot::Sender<EmulateInfo>,
    res_rx: oneshot::Receiver<Result<qasmsim::Execution, String>>,
) -> (StatusCode, Json<Value>) {
    let mode = msg.mode.clone().unwrap();

    // send the message to the quantum_thread
    msg_tx.send(pre_process_msg(msg)).unwrap();

    // use res_rx to receive the result from the quantum_thread
    match res_rx.await {
        Ok(Ok(result)) => {
            // post process message
            match emulate::post_process_msg(result.sequences().clone().unwrap(), mode.to_string()) {
                Ok(json) => return (StatusCode::OK, json),
                Err(err) => {
                    return (
                        StatusCode::BAD_REQUEST,
                        Json(json!({"Error": format!("{}", err)})),
                    )
                }
            }
        }
        Ok(Err(err)) => (
            // quantum thread error
            StatusCode::BAD_REQUEST,
            Json(json!({"Error": format!("{}", err)})),
        ),
        Err(_) => (
            // receiver error
            StatusCode::BAD_REQUEST,
            Json(json!({"Error": "Internal server error"})),
        ),
    }
}

/// classical thread for VQE
pub async fn classical_thread_vqe(
    msg: EmulateMessage,
    vars_range: HashMap<String, (f32, f32)>,
    iteration: usize,
    iterations: usize,
    msg_tx: oneshot::Sender<EmulateInfo>,
    res_rx: oneshot::Receiver<Result<qasmsim::Execution, String>>,
) -> (StatusCode, Json<Value>) {
    // send the message to the quantum_thread
    msg_tx
        .send(pre_process_msg_vqe(msg, vars_range, iteration, iterations))
        .unwrap();

    // use res_rx to receive the result from the quantum_thread
    match res_rx.await {
        Ok(Ok(result)) => {
            // post process message
            match emulate::post_process_msg_vqe(result.expectation().clone()) {
                Ok(json) => return (StatusCode::OK, json),
                Err(err) => {
                    return (
                        StatusCode::BAD_REQUEST,
                        Json(json!({"Error": format!("{}", err)})),
                    )
                }
            }
        }
        Ok(Err(err)) => (
            // quantum thread error
            StatusCode::BAD_REQUEST,
            Json(json!({"Error": format!("{}", err)})),
        ),
        Err(_) => (
            // receiver error
            StatusCode::BAD_REQUEST,
            Json(json!({"Error": "Internal server error"})),
        ),
    }
}

/// consume_task is the main function to consume the task
/// it will spawn the quantum_thread and classical_thread execept for VQE
/// for VQE, it will spawn multiple classical_thread_vqe amd quantum_thread_vqe
pub async fn consume_task(
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

            tokio::spawn(quantum_thread(msg_rx, res_tx));
            tokio::spawn(classical_thread(message, msg_tx, res_rx))
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
                tokio::spawn(quantum_thread_vqe(msg_rx, res_tx));
                match tokio::spawn(classical_thread_vqe(
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
pub async fn submit(request: Request) -> (StatusCode, Json<Value>) {
    match request.headers().get(header::CONTENT_TYPE) {
        Some(content_type) => match content_type.to_str().unwrap() {
            "application/x-www-form-urlencoded" => {
                let Form(message) = request.extract().await.unwrap();
                consume_task(Form(message)).await
            }
            "application/json" => {
                let Json::<EmulateMessage>(message) = request.extract().await.unwrap();
                consume_task(Form(message)).await
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

#[tokio::main]
async fn main() {
    if std::path::Path::new(".env").exists() {
        dotenv::dotenv().ok();
    }
    let listener_addr = std::env::var("LISTENER_ADDR").unwrap_or("0.0.0.0:3003".to_string());
    let qpp_router = Router::new().route("/submit", routing::post(submit));
    let listener = tokio::net::TcpListener::bind(listener_addr).await.unwrap();
    axum::serve(listener, qpp_router).await.unwrap();
}

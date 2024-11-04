use crate::SharedState;

use super::emulate::{
    post_process_msg, post_process_msg_vqe, pre_process_msg, pre_process_msg_vqe, EmulateInfo,
    EmulateMessage,
};
use axum::{http::StatusCode, Json};
use serde_json::{json, Value};
use std::collections::HashMap;
use tokio::sync::oneshot;

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
    state: SharedState,
    msg: EmulateMessage,
    msg_tx: oneshot::Sender<EmulateInfo>,
    res_rx: oneshot::Receiver<Result<qasmsim::Execution, String>>,
) -> (StatusCode, Json<Value>) {
    let mode = msg.mode.clone().unwrap();

    let idle_qubits = state.read().await.qreg.idle;
    if idle_qubits < 1 || idle_qubits < msg.qubits {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({"Error": "No enough qubits"})),
        );
    }

    let qubits = msg.qubits;

    state.write().await.qreg.idle -= qubits;

    // send the message to the quantum_thread
    msg_tx.send(pre_process_msg(msg)).unwrap();

    // use res_rx to receive the result from the quantum_thread
    match res_rx.await {
        Ok(Ok(result)) => {
            state.write().await.qreg.idle += qubits;
            // post process message
            match post_process_msg(state, result.sequences().clone().unwrap(), mode.to_string())
                .await
            {
                Ok(json) => return (StatusCode::OK, json),
                Err(err) => {
                    return (
                        StatusCode::BAD_REQUEST,
                        Json(json!({"Error": format!("{}", err)})),
                    )
                }
            }
        }
        Ok(Err(err)) => {
            state.write().await.qreg.idle += qubits;
            (
                // quantum thread error
                StatusCode::BAD_REQUEST,
                Json(json!({"Error": format!("{}", err)})),
            )
        }
        Err(_) => {
            state.write().await.qreg.idle += qubits;
            (
                // receiver error
                StatusCode::BAD_REQUEST,
                Json(json!({"Error": "Internal server error"})),
            )
        }
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
            match post_process_msg_vqe(result.expectation().clone()) {
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

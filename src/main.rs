use axum::{
    extract::Request,
    http::{header, StatusCode},
    routing, Form, Json, RequestExt, Router,
};
use emulate::{pre_process_msg, EmulateMessage, EmulateMode};
use serde_json::{json, Value};
use tokio::sync::oneshot;
pub mod emulate;

pub async fn quantum_thread(
    msg_rx: oneshot::Receiver<EmulateMessage>,
    res_tx: oneshot::Sender<Result<qasmsim::Execution, String>>,
) {
    let msg = msg_rx.await.unwrap();
    let shots = if msg.shots == 0 {
        res_tx
            .send(Err("shots must be greater than 0".to_string()))
            .unwrap();
        return;
    } else {
        Some(msg.shots)
    };

    match qasmsim::run_mode(&msg.qasm, shots, "sequence".to_string()) {
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

pub async fn classical_thread(
    msg: EmulateMessage,
    msg_tx: oneshot::Sender<EmulateMessage>,
    res_rx: oneshot::Receiver<Result<qasmsim::Execution, String>>,
) -> (StatusCode, Json<Value>) {
    let mode = msg.mode.clone().unwrap_or(EmulateMode::Aggregation);

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

pub async fn consume_task(
    Form(message): Form<emulate::EmulateMessage>,
) -> (StatusCode, Json<Value>) {
    let (msg_tx, msg_rx) = oneshot::channel();
    let (res_tx, res_rx) = oneshot::channel();

    tokio::spawn(quantum_thread(msg_rx, res_tx));
    tokio::spawn(classical_thread(message, msg_tx, res_rx))
        .await
        .unwrap()
}

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

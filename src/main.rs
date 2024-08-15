use axum::{
    extract::Request,
    http::{header, StatusCode},
    routing, Form, Json, RequestExt, Router,
};
use emulate::EmulateMessage;
use serde_json::{json, Value};
use tokio::sync::oneshot;
pub mod emulate;

pub async fn quantum_thread(
    qasm: String,
    shots: Option<usize>,
    tx: oneshot::Sender<Result<qasmsim::Execution, String>>,
) {
    match qasmsim::run_mode(&qasm, shots, "sequence".to_string()) {
        Ok(result) => {
            // send the result to the classical_thread
            tx.send(Ok(result)).unwrap()
        }
        Err(err) => {
            // if there are some errors, send the error message to the classical_thread
            tx.send(Err(err.to_string())).unwrap()
        }
    }
}

pub async fn classical_thread(
    mode: String,
    rx: oneshot::Receiver<Result<qasmsim::Execution, String>>,
) -> (StatusCode, Json<Value>) {
    // use rx to receive the result from the quantum_thread
    match rx.await {
        Ok(Ok(result)) => {
            // post process message
            match emulate::post_process_msg(result.sequences().clone().unwrap(), mode.clone()) {
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
    let qasm = message.qasm;
    let shots = if message.shots == 0 {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({"Error": "shots must be greater than 0"})),
        );
    } else {
        Some(message.shots)
    };

    let mode = if message.mode.is_none() {
        "aggregation".to_string()
    } else {
        message.mode.unwrap().to_string()
    };

    let (tx, rx) = oneshot::channel();
    tokio::spawn(quantum_thread(qasm, shots, tx));
    tokio::spawn(classical_thread(mode, rx)).await.unwrap()
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

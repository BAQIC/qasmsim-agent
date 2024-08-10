use axum::{
    extract::Request,
    http::{header, StatusCode},
    routing, Form, Json, RequestExt, Router,
};
use emulate::EmulateMessage;
use serde_json::{json, Value};
pub mod emulate;

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

    // Currently, we don't need another thread to run the simulation
    match qasmsim::run_mode(&qasm, shots, "sequence".to_string()) {
        Ok(result) => {
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
        Err(err) => (
            StatusCode::BAD_REQUEST,
            Json(json!({"Error": format!("{}", err)})),
        ),
    }
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

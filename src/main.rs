use axum::{
    extract::Request,
    http::{header, StatusCode},
    routing, Form, Json, RequestExt, Router,
};
use serde::Deserialize;
use serde_json::{json, Value};

#[derive(Deserialize, Debug)]
pub struct EmulateMessage {
    qasm: String,
    shots: usize,
}

pub async fn consume_task(Form(message): Form<EmulateMessage>) -> (StatusCode, Json<Value>) {
    let qasm = message.qasm;
    let shots = if message.shots == 0 {
        None
    } else {
        Some(message.shots)
    };

    match qasmsim::run(&qasm, shots) {
        Ok(result) => {
            let options = qasmsim::options::Options {
                shots,
                format: qasmsim::options::Format::Json,
                ..Default::default()
            };

            (
                StatusCode::OK,
                Json(
                    serde_json::from_str::<Value>(&qasmsim::print_result(&result, &options))
                        .unwrap(),
                ),
            )
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
    let qpp_router = Router::new().route("/submit", routing::post(submit));
    let listener = tokio::net::TcpListener::bind("0.0.0.0:3003").await.unwrap();
    axum::serve(listener, qpp_router).await.unwrap();
}

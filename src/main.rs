use serde_json::json;
use std::env;
use tiny_http::{Header, Method, Response, Server, StatusCode};

fn required_env(key: &str) -> String {
    env::var(key).unwrap_or_else(|_| panic!("Missing required environment variable: {}", key))
}

fn json_response(status: u16, payload: serde_json::Value) -> Response<std::io::Cursor<Vec<u8>>> {
    let body = serde_json::to_vec_pretty(&payload).expect("JSON serialization should not fail");
    let content_type = Header::from_bytes("Content-Type", "application/json")
        .expect("content type header should be valid");

    Response::from_data(body)
        .with_status_code(StatusCode(status))
        .with_header(content_type)
}

fn main() {
    let app_name = required_env("APP_NAME");
    let api_key = required_env("API_KEY");
    let rust_env = required_env("RUST_ENV");
    let release = required_env("RELEASE");
    let port = env::var("PORT").unwrap_or_else(|_| "3000".to_string());
    let address = format!("0.0.0.0:{}", port);
    let server = Server::http(&address).expect("server should bind to requested address");

    println!("{} listening on {}", app_name, address);

    for request in server.incoming_requests() {
        let method = request.method().clone();
        let path = request.url().split('?').next().unwrap_or("/");

        let response = match (method, path) {
            (Method::Get, "/") => json_response(200, json!({
                "app": app_name,
                "release": release,
                "message": "Portiq Rust test API is running with Cargo dependencies",
                "env": {
                    "apiKeyConfigured": !api_key.is_empty(),
                    "rustEnv": rust_env,
                    "port": port.parse::<u16>().unwrap_or(3000)
                },
                "routes": [
                    "GET /health",
                    "GET /secure",
                    "POST /echo"
                ]
            })),
            (Method::Get, "/health") => json_response(200, json!({
                "status": "ok",
                "release": release
            })),
            (Method::Get, "/secure") => {
                if !has_valid_api_key(&request, &api_key) {
                    json_response(401, json!({
                        "error": "Invalid or missing x-api-key header"
                    }))
                } else {
                    json_response(200, json!({
                        "message": "Protected Rust route reached",
                        "release": release
                    }))
                }
            }
            (Method::Post, "/echo") => {
                if !has_valid_api_key(&request, &api_key) {
                    json_response(401, json!({
                        "error": "Invalid or missing x-api-key header"
                    }))
                } else {
                    json_response(200, json!({
                        "message": "Echo route reached",
                        "release": release
                    }))
                }
            }
            _ => json_response(404, json!({
                "error": "Route not found"
            })),
        };

        if let Err(error) = request.respond(response) {
            eprintln!("Failed to send response: {}", error);
        }
    }
}

fn has_valid_api_key(request: &tiny_http::Request, api_key: &str) -> bool {
    request
        .headers()
        .iter()
        .any(|header| header.field.equiv("x-api-key") && header.value.as_str() == api_key)
}

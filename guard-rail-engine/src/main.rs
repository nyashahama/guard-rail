use axum::{Router, routing::get};
use tokio::net::TcpListener;

#[tokio::main]
async fn main() {
    let app = Router::new().route("/health", get(|| async { "ok" }));
    let listener = TcpListener::bind("0.0.0.0:8080").await.unwrap();
    println!("Guard Rail Engine starting on :8080");
    axum::serve(listener, app).await.unwrap();
}

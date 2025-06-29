use axum::{Router, routing::get, response::Html};
use std::net::SocketAddr;
use tokio::net::TcpListener;

async fn home() -> Html<&'static str> {
    Html("<h1>Hello from Axum on Railway!</h1>")
}

#[tokio::main]
async fn main() {
    let app = Router::new().route("/", get(home));
    let addr = SocketAddr::from(([0, 0, 0, 0], 8080));
    println!("Listening on {}", addr);
    axum::serve(TcpListener::bind(addr).await.unwrap(), app)
        .await
        .unwrap();
}

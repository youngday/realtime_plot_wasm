use axum::{
    extract::ws::{WebSocket, WebSocketUpgrade},
    response::Response,
    routing::get,
    Router,
};
use chrono::Utc;
use futures_util::{SinkExt, StreamExt}; // StreamExt 用于读取客户端消息（可选）
use log::error;
use rand::{distributions::Uniform, rngs::StdRng, Rng, SeedableRng};
use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};
use tokio::net::TcpListener;

#[derive(Clone, Serialize, Deserialize)]
pub struct MyData {
    time: chrono::DateTime<Utc>,
    y1: f64,
    y2: f64,
}

// 新的 WebSocket 处理函数
async fn ws_handler(
    ws: WebSocketUpgrade,
    rng: axum::extract::Extension<Arc<Mutex<StdRng>>>,
) -> Response {
    ws.on_upgrade(move |socket| handle_socket(socket, rng.0))
}

async fn handle_socket(mut socket: WebSocket, rng: Arc<Mutex<StdRng>>) {
    let noise_range = Uniform::new(-0.2, 0.2);
    let noise_range_y2 = Uniform::new(-0.1, 0.1);
    let mut counter = 0.0;
    let start_time = Utc::now() - chrono::Duration::days(7);

    // 可选：忽略客户端发来的消息
    let _ = socket.next().await;

    loop {
        let mut data = Vec::new();
        for i in 0..100 {
            let mut rng = rng.lock().unwrap();
            data.push(MyData {
                time: start_time + chrono::Duration::hours(i * 2),
                y1: (i as f64 / 10.0 + counter).sin() + rng.sample(noise_range),
                y2: (i as f64 / 5.0 + counter).sin() * 0.8 + rng.sample(noise_range_y2),
            });
        }
        counter += 0.1;
        if counter > 2.0 {
            counter = 0.0;
        }

        if let Err(e) = socket
            .send(axum::extract::ws::Message::Text(
                serde_json::to_string(&data).unwrap().into(),//into: to Utf8Bytes
            ))
            .await
        {
            error!("Send error: {}", e);
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    }
}

#[tokio::main]
async fn main() {
    // 初始化随机数生成器
    let rng = Arc::new(Mutex::new(StdRng::from_entropy()));

    // 构建路由
    let app = Router::new()
        .route("/ws", get(ws_handler))
        .layer(axum::extract::Extension(rng));

    // 绑定端口
    let listener = TcpListener::bind("127.0.0.1:8080").await.unwrap();
    println!("WebSocket server listening on ws://127.0.0.1:8080/ws");

    // 启动服务
    axum::serve(listener, app).await.unwrap();
}
use axum::{
    Router,
    extract::ws::{WebSocket, WebSocketUpgrade},
    response::Response,
    routing::get,
};
use chrono::{Duration, Utc};
use log::{error, info};
use rand::{Rng, SeedableRng, rngs::StdRng};
use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};

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
    // Using gen_range instead of Uniform for simpler rand 0.9 usage
    let mut counter = 0.0;
    let base_time = Utc::now() - Duration::days(7);

    // 删掉下面这一行，让服务器连接后立刻开始发送
    // let _ = socket.next().await;
    let mut buf = Vec::with_capacity(100);
    let mut loop_cnt = 0;
    loop {
        buf.clear();
        {
            let mut rng = rng.lock().unwrap();
            for i in 0..100 {
                let t = base_time + Duration::hours(i * 2);
                let y1 = (i as f64 / 10.0 + counter).sin() + rng.random_range(-0.2..0.2);
                let y2 = (i as f64 / 5.0 + counter).sin() * 0.8 + rng.random_range(-0.1..0.1);
                buf.push(MyData { time: t, y1, y2 });
            }
        }

        counter += 0.1;
        if counter > 2.0 {
            counter = 0.0;
        }

        let json = serde_json::to_string(&buf).unwrap();
        if let Err(e) = socket
            .send(axum::extract::ws::Message::Text(json.into()))
            .await
        {
            error!("Send error: {}", e);
            break;
        }
        info!("loop_cnt: {} ,Sent {} items", loop_cnt, buf.len());
        loop_cnt += 1;
        if loop_cnt > 1000 {
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    }
}

#[tokio::main]
async fn main() {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    let rng = Arc::new(Mutex::new(StdRng::seed_from_u64(42)));
    let app = Router::new()
        .route("/ws", get(ws_handler))
        .layer(axum::extract::Extension(rng));

    let addr = "127.0.0.1:8080";
    info!("WebSocket server listening on ws://{}/ws", addr);
    axum::serve(tokio::net::TcpListener::bind(addr).await.unwrap(), app)
        .await
        .unwrap();
}

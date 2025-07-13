use tokio::net::{TcpListener, TcpStream};
use tokio_tungstenite::accept_async;
use futures_util::SinkExt;
use chrono::Utc;
use rand::{Rng, distributions::Uniform, rngs::StdRng, SeedableRng};
use serde::{Serialize, Deserialize};
use log::error;
use std::sync::{Arc, Mutex};

#[derive(Clone, Serialize, Deserialize)]
pub struct MyData {
    time: chrono::DateTime<Utc>,
    y1: f64,
    y2: f64,
}

// use rand::rngs::StdRng;
// use rand::SeedableRng;
// use std::sync::Arc;
// use tokio::sync::Mutex;

async fn handle_connection(stream: TcpStream, rng: Arc<Mutex<StdRng>>) {
    let mut ws_stream = accept_async(stream).await.unwrap();
    let noise_range = Uniform::new(-0.2, 0.2);
    let noise_range_y2 = Uniform::new(-0.1, 0.1);
    let mut counter = 0.0;
     let start_time = Utc::now() - chrono::Duration::days(7);
    loop {
        let mut data = Vec::new();
        for i in 0..100 {
            let mut rng = rng.lock().unwrap();
            data.push(MyData {
                time: start_time + chrono::Duration::hours(i*2),
                y1: (i as f64 / 10.0 + counter).sin() + rng.sample(noise_range),
                y2: (i as f64 / 5.0 + counter).sin() * 0.8 + rng.sample(noise_range_y2),
            });
        }
        counter+=0.1;
        if counter>2.0 {
            counter=0.0;
        }
        
           if let Err(e) = ws_stream.send(serde_json::to_string(&data).unwrap().into()).await {
            error!("Send error: {}", e);
            break;
        }
 // Change from 1 second to 100 milliseconds
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    }
}

#[tokio::main]
async fn main() {
    let listener = TcpListener::bind("127.0.0.1:8080").await.unwrap();
    let rng = Arc::new(Mutex::new(StdRng::from_entropy()));
    
    while let Ok((stream, _)) = listener.accept().await {
        let rng_clone = rng.clone();
        tokio::spawn(handle_connection(stream, rng_clone));
    }
}
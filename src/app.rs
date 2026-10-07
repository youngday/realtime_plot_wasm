use chrono::{DateTime, Duration, Utc};
use leptos::ev::MessageEvent;
use leptos::prelude::*;
use leptos_chartistry::*;
use rand::RngExt;
use wasm_bindgen::JsCast;
use wasm_bindgen::closure::Closure;
use web_sys::WebSocket;

/// Data feed endpoint. Matches `examples/server.rs` and `server.py`.
const WS_URL: &str = "ws://127.0.0.1:8080/ws";

#[derive(Clone, serde::Serialize, serde::Deserialize)]
pub struct MyData {
    time: DateTime<Utc>,
    y1: f64,
    y2: f64,
}

impl MyData {
    fn new(time: DateTime<Utc>, y1: f64, y2: f64) -> Self {
        Self { time, y1, y2 }
    }
}

pub fn load_data() -> Vec<MyData> {
    let mut rng = rand::rng();
    let start_time = Utc::now() - Duration::days(7);
    (0..=100)
        .map(|i| {
            let time = start_time + Duration::hours(i * 2);
            let rand_offset = rng.random_range(-0.5..0.5);
            MyData::new(
                time,
                (i as f64 * 0.1).sin() + rand_offset,
                (i as f64 * 0.2).sin() * 0.8 + rand_offset,
            )
        })
        .collect()
}

/// Owns the WebSocket and its message callback.
///
/// Storing this in a `StoredValue::new_local` ties both to the reactive owner, so
/// they are torn down when the component is disposed. The previous code used
/// `Closure::forget()`, which leaked the callback (and everything it captured)
/// permanently, and never closed the socket.
struct WsFeed {
    ws: WebSocket,
    _on_message: Closure<dyn FnMut(MessageEvent)>,
}

impl Drop for WsFeed {
    fn drop(&mut self) {
        // Clear the handler before the closure is released, so the browser can
        // never call into a closure that wasm-bindgen has already invalidated.
        self.ws.set_onmessage(None);
        let _ = self.ws.close();
    }
}

#[component]
pub fn App() -> impl IntoView {
    let series = Series::new(|data: &MyData| data.time)
        .line(Line::new(|data: &MyData| data.y1).with_name("y1"))
        .line(Line::new(|data: &MyData| data.y2).with_name("y2"));

    let data: RwSignal<Vec<MyData>> = RwSignal::new(load_data());
    let (is_paused, set_paused) = signal(false);

    // One effect owns the WebSocket for the lifetime of the component.
    Effect::new(move |_| {
        let ws = match WebSocket::new(WS_URL) {
            Ok(ws) => ws,
            Err(err) => {
                // Previously `.expect(..)`, which took the whole page down when the
                // feed was not running.
                log::error!("WebSocket connect to {WS_URL} failed: {err:?}");
                return;
            }
        };

        let on_message = Closure::<dyn FnMut(MessageEvent)>::new(move |e: MessageEvent| {
            // `is_paused` is `Copy`; reading it untracked keeps this callback out of
            // the reactive graph (it is driven by the browser, not by a scope).
            if is_paused.get_untracked() {
                return;
            }
            let Some(frame) = e.data().as_string() else {
                return;
            };
            match serde_json::from_str::<Vec<MyData>>(&frame) {
                Ok(new_data) => {
                    // Was a `console_log(format!(..))` on every frame; `trace!` is
                    // below the logger level in `hydrate()`, so it costs nothing by
                    // default but stays available when debugging.
                    log::trace!("received {} points", new_data.len());
                    data.set(new_data);
                }
                Err(err) => log::warn!("dropping malformed frame: {err}"),
            }
        });

        ws.set_onmessage(Some(on_message.as_ref().unchecked_ref()));

        // Hand ownership to the reactive arena: `WsFeed::drop` runs on disposal.
        // (`on_cleanup` is not usable here -- it requires `Send + Sync`, and browser
        // handles are neither.)
        let _feed = StoredValue::new_local(WsFeed {
            ws,
            _on_message: on_message,
        });
    });

    view! {
        <h1>"时间序列图表"</h1>
        <button on:click=move |_| set_paused.update(|p| *p = !*p)>
            {move || if is_paused.get() { "继续" } else { "暂停" }}
        </button>
        <Chart
            aspect_ratio=AspectRatio::from_outer_height(300.0, 1.2)
            series=series
            // `RwSignal` converts straight into the `Signal` prop, so the extra
            // `Signal::derive(move || data.get())` memo (one full Vec clone per
            // frame) is gone.
            data=data
            top=RotatedLabel::middle("时间序列数据")
            left=TickLabels::aligned_floats()
            bottom=Legend::end()
            inner=[
                AxisMarker::left_edge().into_inner(),
                AxisMarker::bottom_edge().into_inner(),
                XGridLine::default().into_inner(),
                YGridLine::default().into_inner(),
                YGuideLine::over_mouse().into_inner(),
                XGuideLine::over_data().into_inner(),
            ]
            tooltip=Tooltip::left_cursor().show_x_ticks(false)
        />
    }
}

pub fn shell(options: LeptosOptions) -> impl IntoView {
    use leptos_meta::MetaTags;
    view! {
        <!DOCTYPE html>
        <html lang="en">
            <head>
                <meta charset="utf-8" />
                <meta name="viewport" content="width=device-width, initial-scale=1" />
                <MetaTags />
                <HydrationScripts options />
            </head>
            <body>
                <App/>
            </body>
        </html>
    }
}

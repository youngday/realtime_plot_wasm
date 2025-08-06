use chrono::{DateTime, Duration, Utc};
use leptos::ev::MessageEvent;
use leptos::{leptos_dom::logging::console_log, prelude::*};
use leptos_chartistry::*;
use rand::Rng;
use wasm_bindgen::closure::Closure;
use wasm_bindgen::JsCast;
use web_sys::WebSocket;
#[derive(Clone, serde::Serialize, serde::Deserialize)] // Add serde traits
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
    // Changed from Signal to Vec
    let mut rng = rand::thread_rng();
    let start_time = Utc::now() - Duration::days(7);
    (0..=100)
        .map(|i| {
            let time = start_time + Duration::hours(i * 2);
            let rand_offset = rng.gen_range(-0.5..0.5); // Larger random variation
            // let rand_offset = 0.0; // Larger random variation
            MyData::new(
                time,
                (i as f64 * 0.1).sin() + rand_offset, // y1 with noise
                (i as f64 * 0.2).sin() * 0.8 + rand_offset, // y2 with different amplitude
            )
        })
        .collect()
}

#[component]
pub fn App() -> impl IntoView {
    let series = Series::new(|data: &MyData| data.time)
        .line(Line::new(|data: &MyData| data.y1).with_name("y1"))
        .line(Line::new(|data: &MyData| data.y2).with_name("y2"));

    let data: RwSignal<Vec<MyData>> = RwSignal::new(load_data());
    let (is_paused, set_paused) = signal(false);
    // 添加WebSocket连接
    Effect::new(move |_| {
        let ws: WebSocket =
            WebSocket::new("ws://localhost:8080/ws").expect("Failed to connect to WebSocket");

        let is_paused_clone = is_paused.clone();
        let on_message = Closure::wrap(Box::new(move |e: MessageEvent| {
            if is_paused_clone.get_untracked() {  // Use get_untracked() since we don't need reactivity here
                return;
            }
            
            if let Some(serialized) = e.data().as_string() {
                if let Ok(new_data) = serde_json::from_str::<Vec<MyData>>(&serialized) {
                    console_log(&format!("Parsed {} items", new_data.len()));
                    data.set(new_data);
                } else {
                    console_log("Failed to parse data");
                }
            }
        }) as Box<dyn FnMut(MessageEvent)>);

        Effect::new(move |_| {
            if !is_paused.get() {
                let _handle = set_interval_with_handle(
                    move || {
                        // This can be removed since we're using WebSocket updates
                    },
                    std::time::Duration::from_millis(3000),
                )
                .expect("Could not set interval");
            }
        });

        ws.set_onmessage(Some(on_message.as_ref().unchecked_ref()));
        on_message.forget();
    });

    Effect::new(move |_| {
        if !is_paused.get() {
            // 只在未暂停时执行刷新
            let handle = set_interval_with_handle(
                move || {
                    let start = js_sys::Date::now();
                    // data.set(load_data());
                    let duration = js_sys::Date::now() - start;
                    console_log(&format!("Refresh took {:.2}ms", duration));
                    console_log(&format!("now time: {:.2}ms", start));
                },
                std::time::Duration::from_millis(30), // Convert to milliseconds
            )
            .expect("Could not create interval");

            on_cleanup(move || {
                handle.clear();
            });
        }
    });

    view! {
        <h1>"时间序列图表"</h1>
        <button on:click=move |_| set_paused.update(|p| *p = !*p)>
            {move || if is_paused.get(){ "继续" } else { "暂停" }}
        </button>
        <Chart
            aspect_ratio=AspectRatio::from_outer_height(300.0, 1.2)
            series=series
            data=Signal::derive(move || data.get())
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

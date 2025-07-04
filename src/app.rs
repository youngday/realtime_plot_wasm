use leptos::{leptos_dom::logging::console_log, prelude::*};
use leptos_chartistry::*;
use chrono::{DateTime, Utc, Duration};
use rand::Rng;
use web_sys::{console, js_sys};

#[derive(Clone)]  // Add this attribute
pub struct MyData {
    time: DateTime<Utc>, // 使用DateTime类型作为时间
    y1: f64,
    y2: f64,
}

impl MyData {
    fn new(time: DateTime<Utc>, y1: f64, y2: f64) -> Self {
        Self { time, y1, y2 }
    }
}

pub fn load_data() -> Vec<MyData> {  // Changed from Signal to Vec
    let mut rng = rand::thread_rng();
    let start_time = Utc::now() - Duration::days(7);
    (0..=100).map(|i| {
        let time = start_time + Duration::hours(i * 2);
        let rand_offset = rng.gen_range(-0.5..0.5);  // Larger random variation
        MyData::new(
            time,
            (i as f64 * 0.1).sin() + rand_offset,  // y1 with noise
            (i as f64 * 0.2).sin() * 0.8 + rand_offset  // y2 with different amplitude
        )
    }).collect()
}

#[component]
pub fn App() -> impl IntoView {
    let series = Series::new(|data: &MyData| data.time)
        .line(Line::new(|data: &MyData| data.y1).with_name("y1"))
        .line(Line::new(|data: &MyData| data.y2).with_name("y2"));
    
    let data = RwSignal::new(load_data());
    let (is_paused, set_paused) = signal(false); // 新增暂停状态
    
    Effect::new(move |_| {
        if !is_paused.get() { // 只在未暂停时执行刷新
            let handle = set_interval_with_handle(
                move || {
                    let start = js_sys::Date::now();
                    data.set(load_data());
                    let duration = js_sys::Date::now() - start;
                    console_log(&format!("Refresh took {:.2}ms", duration));
                    console_log(&format!("now time: {:.2}ms", start));
                },
            std::time::Duration::from_millis(1), // Convert to milliseconds
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
            {move || if is_paused.get() { "继续" } else { "暂停" }}
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
                <AutoReload options=options.clone() />
                <HydrationScripts options />
                <MetaTags />
            </head>
            <body>
                <App/>
            </body>
        </html>
    }
}

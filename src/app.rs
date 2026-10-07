use std::cell::{Cell, RefCell};
use std::rc::Rc;

use chrono::{DateTime, Duration, Utc};
use leptos::ev::MessageEvent;
use leptos::prelude::*;
use leptos_chartistry::*;
use rand::RngExt;
use wasm_bindgen::JsCast;
use wasm_bindgen::closure::Closure;
use web_sys::{CloseEvent, Event, WebSocket};

/// Data feed endpoint. Matches `examples/server.rs` and `server.py`.
const WS_URL: &str = "ws://127.0.0.1:8080/ws";

/// First reconnect waits ~250 ms, then the delay doubles per failed attempt up
/// to a 5 s ceiling.
fn backoff_ms(attempt: u32) -> i32 {
    (250u32 << attempt.min(5)).min(5_000) as i32
}

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
    // 100 points, matching the frame size both feeds send
    // (`examples/server.rs` and `server.py`), so the pre-hydration render lines
    // up with the first WebSocket frame instead of briefly showing one extra point.
    (0..100)
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

/// A connected `WebSocket` together with every JS callback registered on it.
///
/// `Drop` detaches the handlers from the socket *before* the closures are
/// released, so the browser can never call into wasm-bindgen closures that have
/// already been freed, and then closes the socket.
struct Connection {
    ws: WebSocket,
    _on_open: Closure<dyn FnMut(Event)>,
    _on_message: Closure<dyn FnMut(MessageEvent)>,
    _on_error: Closure<dyn FnMut(Event)>,
    _on_close: Closure<dyn FnMut(CloseEvent)>,
}

impl Drop for Connection {
    fn drop(&mut self) {
        self.ws.set_onopen(None);
        self.ws.set_onmessage(None);
        self.ws.set_onerror(None);
        self.ws.set_onclose(None);
        let _ = self.ws.close();
    }
}

/// Everything the feed owns for the life of the `App` component.
///
/// The callbacks below hold a `Weak` back-reference, so the only strong `Rc` is
/// the one parked in the reactive arena. Disposing the component therefore
/// releases the socket, both timers and the pending frame in one step.
struct Feed {
    data: RwSignal<Vec<MyData>>,
    is_paused: ReadSignal<bool>,
    conn: RefCell<Option<Connection>>,
    /// Newest frame not yet painted; newer frames overwrite it rather than
    /// queueing, so a burst collapses into a single redraw.
    pending: RefCell<Option<Vec<MyData>>>,
    raf_scheduled: Cell<bool>,
    raf_handle: Cell<Option<i32>>,
    retry_handle: Cell<Option<i32>>,
    attempt: Cell<u32>,
    disposed: Cell<bool>,
}

impl Feed {
    /// Open (or reopen) the socket. Safe to call repeatedly: the previous
    /// connection, if any, is dropped first.
    fn start(self: &Rc<Self>) {
        if self.disposed.get() {
            return;
        }

        let ws = match WebSocket::new(WS_URL) {
            Ok(ws) => ws,
            Err(err) => {
                log::error!("WebSocket connect to {WS_URL} failed: {err:?}");
                self.schedule_retry();
                return;
            }
        };

        let weak = Rc::downgrade(self);

        let on_open = {
            let weak = weak.clone();
            Closure::<dyn FnMut(Event)>::new(move |_| {
                if let Some(feed) = weak.upgrade() {
                    feed.attempt.set(0);
                }
            })
        };

        let on_message = {
            let weak = weak.clone();
            Closure::<dyn FnMut(MessageEvent)>::new(move |e: MessageEvent| {
                let Some(feed) = weak.upgrade() else {
                    return;
                };
                // `is_paused` is `Copy`; reading it untracked keeps this callback
                // out of the reactive graph (the browser drives it, not a scope).
                if feed.is_paused.get_untracked() {
                    return;
                }
                let Some(frame) = e.data().as_string() else {
                    return;
                };
                match serde_json::from_str::<Vec<MyData>>(&frame) {
                    Ok(new_data) => feed.push_frame(new_data),
                    Err(err) => log::warn!("dropping malformed frame: {err}"),
                }
            })
        };

        let on_error = {
            let weak = weak.clone();
            Closure::<dyn FnMut(Event)>::new(move |_| {
                if let Some(feed) = weak.upgrade() {
                    feed.on_disconnect("error");
                }
            })
        };

        let on_close = {
            let weak = weak.clone();
            Closure::<dyn FnMut(CloseEvent)>::new(move |_| {
                if let Some(feed) = weak.upgrade() {
                    feed.on_disconnect("close");
                }
            })
        };

        ws.set_onopen(Some(on_open.as_ref().unchecked_ref()));
        ws.set_onmessage(Some(on_message.as_ref().unchecked_ref()));
        ws.set_onerror(Some(on_error.as_ref().unchecked_ref()));
        ws.set_onclose(Some(on_close.as_ref().unchecked_ref()));

        // Replacing the slot drops the previous `Connection`, detaching its
        // handlers; by now the browser is not executing them any more.
        self.conn.replace(Some(Connection {
            ws,
            _on_open: on_open,
            _on_message: on_message,
            _on_error: on_error,
            _on_close: on_close,
        }));
    }

    /// Publish a frame on the next animation frame.
    ///
    /// Frames arrive faster than the display refreshes, and every `data.set`
    /// redraws the whole chart, so only the newest frame is kept and at most one
    /// redraw is scheduled per paint.
    fn push_frame(self: &Rc<Self>, frame: Vec<MyData>) {
        *self.pending.borrow_mut() = Some(frame);
        if self.raf_scheduled.get() {
            return;
        }
        self.raf_scheduled.set(true);

        let weak = Rc::downgrade(self);
        let paint = Closure::once_into_js(move || {
            if let Some(feed) = weak.upgrade() {
                feed.raf_scheduled.set(false);
                feed.raf_handle.set(None);
                if let Some(frame) = feed.pending.borrow_mut().take() {
                    feed.data.set(frame);
                }
            }
        });
        let handle = web_sys::window()
            .and_then(|win| win.request_animation_frame(paint.unchecked_ref()).ok());
        self.raf_handle.set(handle);
    }

    /// The socket died: schedule one retry. The retry runs in a fresh task, so
    /// the dead `Connection` is dropped outside its own callback.
    fn on_disconnect(self: &Rc<Self>, reason: &str) {
        if self.disposed.get() || self.retry_handle.get().is_some() {
            return;
        }
        log::warn!("WebSocket {reason}; reconnecting");
        self.schedule_retry();
    }

    fn schedule_retry(self: &Rc<Self>) {
        if self.disposed.get() || self.retry_handle.get().is_some() {
            return;
        }
        let attempt = self.attempt.get();
        self.attempt.set(attempt + 1);

        let weak = Rc::downgrade(self);
        let retry = Closure::once_into_js(move || {
            if let Some(feed) = weak.upgrade() {
                feed.retry_handle.set(None);
                feed.start();
            }
        });
        let handle = web_sys::window().and_then(|win| {
            win.set_timeout_with_callback_and_timeout_and_arguments_0(
                retry.unchecked_ref(),
                backoff_ms(attempt),
            )
            .ok()
        });
        self.retry_handle.set(handle);
    }
}

impl Drop for Feed {
    fn drop(&mut self) {
        self.disposed.set(true);
        if let Some(win) = web_sys::window() {
            if let Some(handle) = self.raf_handle.take() {
                let _ = win.cancel_animation_frame(handle);
            }
            if let Some(handle) = self.retry_handle.take() {
                win.clear_timeout_with_handle(handle);
            }
        }
        // Dropping the `Connection` detaches the socket handlers.
        self.conn.replace(None);
    }
}

#[component]
pub fn App() -> impl IntoView {
    let series = Series::new(|data: &MyData| data.time)
        .line(Line::new(|data: &MyData| data.y1).with_name("y1"))
        .line(Line::new(|data: &MyData| data.y2).with_name("y2"));

    let data: RwSignal<Vec<MyData>> = RwSignal::new(load_data());
    let (is_paused, set_paused) = signal(false);

    // One effect owns the whole feed. `StoredValue::new_local` ties it to the
    // reactive owner, so disposal runs `Feed::drop`. (`on_cleanup` is not usable
    // here -- it requires `Send + Sync`, and browser handles are neither.)
    Effect::new(move |_| {
        let feed = Rc::new(Feed {
            data,
            is_paused,
            conn: RefCell::new(None),
            pending: RefCell::new(None),
            raf_scheduled: Cell::new(false),
            raf_handle: Cell::new(None),
            retry_handle: Cell::new(None),
            attempt: Cell::new(0),
            disposed: Cell::new(false),
        });
        feed.start();
        let _keep_alive = StoredValue::new_local(feed);
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

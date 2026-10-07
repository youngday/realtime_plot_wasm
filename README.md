# realtime plot

## fun
An example showcasing how to use Chartistry with Leptos and SSR. It borrows heavily from [Leptos' SSR mode axum example](https://github.com/leptos-rs/leptos/tree/main/examples/ssr_modes_axum).
more info and start demo ,please check "leptos" and "leptos-chartistry". 
## run
Run `cargo-leptos watch` (note the '-').
## out
The chart redraws once per WebSocket frame, ~10 fps with the bundled feeds.
![alt text](demo.png)

## plot
## NOTE:
we can use websocket client to refresh data and plot in real time.

The page is served on `127.0.0.1:3000`, and the feed endpoint is
`ws://127.0.0.1:8080/ws` (see `WS_URL` in `src/app.rs`), so run the client and one
of the servers below side by side.

### client
websocket client  refresh data 
```sh
cargo leptos watch
```
### python server
```sh
uv run server.py
```
### rust axum websocket server
```sh
cargo run --example server
```



# issued


## at console.log js

```log
my_example_ssr.js:967 At src/app.rs:53:26, you access a reactive_graph::signal::read::ReadSignal<bool> (defined at src/app.rs:46:35) outside a reactive tracking context. This might mean your app is not responding to changes in signal values in the way you expect.

Here’s how to fix it:

1. If this is inside a `view!` macro, make sure you are passing a function, not a value.
  ❌ NO  <p>{x.get() * 2}</p>
  ✅ YES <p>{move || x.get() * 2}</p>

2. If it’s in the body of a component, try wrapping this access in a closure: 
  ❌ NO  let y = x.get() * 2
  ✅ YES let y = move || x.get() * 2.

3. If you’re *trying* to access the value without tracking, use `.get_untracked()` or `.with_untracked()` instead.

```

```rust
   let is_paused_clone = is_paused.clone();
        let on_message = Closure::wrap(Box::new(move |e: MessageEvent| {
            if is_paused_clone.get_untracked() {  // Use get_untracked() since we don't need reactivity here
                return;
            }
```


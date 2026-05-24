# Animations

`eddacraft-tui` uses the [`animate`](https://crates.io/crates/animate) crate to
smooth transitions on `ProgressBar` and `ParallelProgress`. Animation is opt-in
at the frame level: the library tracks targets and eases toward them, but your
event loop has to advance the clock and trigger redraws. Without that wiring the
widgets still render correctly, they just snap to the latest value instead of
animating.

## Required wiring

Call `animate_tick` once per frame with the elapsed time in milliseconds, then
render as normal. Both symbols come through the prelude.

```rust,no_run
use std::time::Instant;
use eddacraft_tui::prelude::*;

let mut last = Instant::now();
loop {
    let now = Instant::now();
    let delta_ms = now.duration_since(last).as_millis() as usize;
    animate_tick(delta_ms);
    last = now;

    terminal.draw(|f| {
        // render ProgressBar / ParallelProgress here
    })?;

    // Keep pumping frames while an animation is in flight, even if no input
    // arrives. Otherwise fall back to your normal blocking input wait.
    if is_animating() {
        // e.g. poll crossterm events with a ~16ms timeout for ~60fps
    }
}
```

## What's animated

| Widget             | Animated property               | Duration | Easing   |
| ------------------ | ------------------------------- | -------- | -------- |
| `ProgressBar`      | `display_fraction` → `fraction` | 250 ms   | quad-out |
| `ParallelProgress` | overall progress bar            | 250 ms   | quad-out |

Each animated value eases toward its target whenever the target changes. The
interpolated value is read during `render`; you don't need to poll it manually.

## Driving redraws

Ratatui only draws when you call `terminal.draw`. If your app blocks on input
during an animation it will skip frames and the transition will look choppy or
incomplete. A common pattern:

- Poll input with a short timeout (e.g. 16 ms ≈ 60 fps) while `is_animating()`
  is true.
- Block indefinitely on input when it returns false.

This keeps the main loop responsive without burning CPU when nothing is moving.

## Prelude exports

The prelude re-exports the two functions the host application needs:

- `animate_tick(delta: usize)` — advance the global animation clock by `delta`
  milliseconds.
- `is_animating() -> bool` — returns `true` while any animated value is still
  transitioning.

Internal widget state (easing curve, duration, interpolation) is not part of the
public API and may change between minor versions.

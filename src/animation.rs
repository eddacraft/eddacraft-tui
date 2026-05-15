//! Thin shim over the underlying animation runtime so callers depend on
//! eddacraft-tui rather than the third-party crate. This lets us swap the
//! engine without breaking downstream API.

/// Advance the animation clock by `delta_ms`. Call once per frame.
pub fn animate_tick(delta_ms: usize) {
    animate::tick(delta_ms);
}

/// Whether any tracked value is still easing toward its target.
#[must_use]
pub fn is_animating() -> bool {
    animate::is_animating()
}

// Internal re-exports so widget code inside this crate names animation
// runtime types through the shim. If we swap the `animate` dep, only this
// module changes; widget modules already route through `crate::animation`.
pub(crate) use animate::easing;
pub(crate) use animate::{Lerp, Once};

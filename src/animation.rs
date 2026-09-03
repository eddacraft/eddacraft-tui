//! Thin shim over the underlying animation runtime so callers depend on
//! eddacraft-tui rather than the third-party crate. This lets us swap the
//! engine without breaking downstream API.

use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::time::Duration;

use animate_core::{Activity, Interpolate, Time, Tween};

static ELAPSED_MS: AtomicU64 = AtomicU64::new(0);
static LAST_DELTA_MS: AtomicU64 = AtomicU64::new(0);
static ANIMATED_PREVIOUS_FRAME: AtomicBool = AtomicBool::new(false);
static ANIMATED_THIS_FRAME: AtomicBool = AtomicBool::new(false);

/// Advance the animation clock by `delta_ms`. Call once per frame.
pub fn animate_tick(delta_ms: usize) {
    let delta_ms = u64::try_from(delta_ms).unwrap_or(u64::MAX);
    LAST_DELTA_MS.store(delta_ms, Ordering::Relaxed);
    let _ = ELAPSED_MS.fetch_update(Ordering::Relaxed, Ordering::Relaxed, |elapsed| {
        Some(elapsed.saturating_add(delta_ms))
    });
    ANIMATED_PREVIOUS_FRAME.store(
        ANIMATED_THIS_FRAME.swap(false, Ordering::Relaxed),
        Ordering::Relaxed,
    );
}

/// Advance one tween using the shared frame clock.
///
/// The returned activity is also folded into the frame-level
/// [`is_animating`] signal consumed by downstream event loops.
pub fn advance<T>(tween: &mut Tween<T>) -> Activity
where
    T: Interpolate + Clone + PartialEq,
{
    let activity = tween.advance(Time::new(
        Duration::from_millis(ELAPSED_MS.load(Ordering::Relaxed)),
        Duration::from_millis(LAST_DELTA_MS.load(Ordering::Relaxed)),
    ));
    if activity.running() {
        ANIMATED_THIS_FRAME.store(true, Ordering::Relaxed);
    }
    activity
}

/// Whether any tracked value is still easing toward its target.
#[must_use]
pub fn is_animating() -> bool {
    ANIMATED_PREVIOUS_FRAME.load(Ordering::Relaxed) || ANIMATED_THIS_FRAME.load(Ordering::Relaxed)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn running_tween_keeps_frame_scheduler_active() {
        let mut tween = Tween::new(0.0).duration(Duration::from_millis(100));
        tween.to(1.0);
        advance(&mut tween);
        assert!(is_animating());

        animate_tick(16);
        assert!(is_animating());
    }

    #[test]
    fn tween_converges_on_shared_clock() {
        let mut tween = Tween::new(0.0).duration(Duration::from_millis(100));
        tween.to(1.0);
        advance(&mut tween);

        animate_tick(101);
        let activity = advance(&mut tween);

        assert_eq!(*tween, 1.0);
        assert!(activity.finished());
        assert!(!activity.running());
    }
}

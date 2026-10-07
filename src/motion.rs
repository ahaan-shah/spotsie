//! Motion primitives. Every animation in the app goes through these, so
//! timings and curves stay consistent and repaints are requested only while
//! something is actually moving (idle costs no CPU).
//!
//! The durations and curves follow Magpie's: quick hovers, gentle toggles,
//! and slightly longer entrances, all easing out.

use std::cell::Cell;

use egui::{Color32, Context, Id};

thread_local! {
    /// Whether things move on this thread. Tests and `--demo-shot` run on a
    /// clock that never advances, so there everything lands at once.
    static ENABLED: Cell<bool> = const { Cell::new(!cfg!(test)) };
}

/// Whether animations run on this thread. See [`set_enabled`].
pub fn enabled() -> bool {
    ENABLED.with(Cell::get)
}

/// Turns animations on or off for the calling thread: off for screenshots
/// and tests, which need every frame settled.
pub fn set_enabled(on: bool) {
    ENABLED.with(|enabled| enabled.set(on));
}

/// Hovers, focus rings and other small responses.
pub const MICRO: f32 = 0.12;
/// Toggles, sliding highlights, panels opening and closing.
pub const STANDARD: f32 = 0.22;
/// Entrances worth noticing: a page arriving, a dialog landing.
pub const EMPHASIS: f32 = 0.42;
/// Values gliding to a new place, such as the equalizer's curve.
pub const GLIDE: f32 = 0.32;

pub fn ease_out(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    1.0 - (1.0 - t).powi(3)
}

pub fn ease_in_out(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    if t < 0.5 {
        4.0 * t * t * t
    } else {
        1.0 - (-2.0 * t + 2.0).powi(3) / 2.0
    }
}

/// A slight overshoot, for things that land: dialogs and toasts.
pub fn ease_out_back(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    let c1 = 1.4;
    let c3 = c1 + 1.0;
    1.0 + c3 * (t - 1.0).powi(3) + c1 * (t - 1.0).powi(2)
}

pub fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}

/// Mixes two colours in sRGB, alpha included.
pub fn lerp_color(a: Color32, b: Color32, t: f32) -> Color32 {
    let t = t.clamp(0.0, 1.0);
    let mix = |x: u8, y: u8| (f32::from(x) + (f32::from(y) - f32::from(x)) * t).round() as u8;
    Color32::from_rgba_premultiplied(
        mix(a.r(), b.r()),
        mix(a.g(), b.g()),
        mix(a.b(), b.b()),
        mix(a.a(), b.a()),
    )
}

pub fn with_alpha(color: Color32, alpha: f32) -> Color32 {
    color.gamma_multiply(alpha.clamp(0.0, 1.0))
}

#[derive(Clone, Copy)]
struct Tween {
    from: f32,
    to: f32,
    start: f64,
    duration: f32,
}

/// Moves smoothly towards `target` with an ease-out curve. Retargeting
/// mid-flight continues from where the value is, so a number that keeps
/// changing never jumps.
pub fn tween(ctx: &Context, id: Id, target: f32, duration: f32) -> f32 {
    if !enabled() {
        return target;
    }
    let now = ctx.input(|input| input.time);
    let id = id.with("tween");
    let Some(mut tween) = ctx.data(|data| data.get_temp::<Tween>(id)) else {
        ctx.data_mut(|data| {
            data.insert_temp(
                id,
                Tween {
                    from: target,
                    to: target,
                    start: now,
                    duration,
                },
            );
        });
        return target;
    };
    let progress = if tween.duration <= 0.0 {
        1.0
    } else {
        ((now - tween.start) as f32 / tween.duration).clamp(0.0, 1.0)
    };
    let value = lerp(tween.from, tween.to, ease_out(progress));
    if (target - tween.to).abs() > f32::EPSILON {
        tween = Tween {
            from: value,
            to: target,
            start: now,
            duration,
        };
        ctx.data_mut(|data| data.insert_temp(id, tween));
        ctx.request_repaint();
        return value;
    }
    if progress < 1.0 {
        ctx.request_repaint();
    }
    value
}

/// Like [`tween`], but the first time `id` is seen it starts at `from`:
/// for things that animate in when they first appear.
pub fn tween_from(ctx: &Context, id: Id, from: f32, target: f32, duration: f32) -> f32 {
    if !enabled() {
        return target;
    }
    let key = id.with("tween");
    if ctx.data(|data| data.get_temp::<Tween>(key).is_none()) {
        let now = ctx.input(|input| input.time);
        ctx.data_mut(|data| {
            data.insert_temp(
                key,
                Tween {
                    from,
                    to: target,
                    start: now,
                    duration,
                },
            );
        });
        ctx.request_repaint();
        return from;
    }
    tween(ctx, id, target, duration)
}

/// Forgets a [`tween`] or [`toggle`], so its next value is the target at
/// once: for changes made out of sight.
pub fn snap(ctx: &Context, id: Id) {
    ctx.data_mut(|data| data.remove::<Tween>(id.with("tween")));
}

/// An animated bool: 0.0 for false, 1.0 for true, eased.
pub fn toggle(ctx: &Context, id: Id, on: bool, duration: f32) -> f32 {
    tween(ctx, id, if on { 1.0 } else { 0.0 }, duration)
}

/// Progress from 0 to 1 of an entrance that began at `shown_at` (egui time)
/// plus `delay` seconds, eased.
pub fn appear(ctx: &Context, shown_at: f64, delay: f32, duration: f32) -> f32 {
    if !enabled() {
        return 1.0;
    }
    let now = ctx.input(|input| input.time);
    let progress = (((now - shown_at) as f32 - delay) / duration).clamp(0.0, 1.0);
    if progress < 1.0 {
        ctx.request_repaint();
    }
    ease_out(progress)
}

/// A staggered entrance, as Magpie's cards make: how opaque the `index`th
/// item is and how far below its place it still sits.
pub fn reveal(ctx: &Context, shown_at: f64, index: usize) -> (f32, f32) {
    let progress = appear(ctx, shown_at, index.min(12) as f32 * 0.045, 0.38);
    (progress, (1.0 - progress) * 14.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn curves_start_at_zero_and_end_at_one() {
        for curve in [ease_out, ease_in_out, ease_out_back] {
            assert!(curve(0.0).abs() < 1e-6);
            assert!((curve(1.0) - 1.0).abs() < 1e-6);
        }
        assert!(ease_out_back(0.7) > 1.0, "lands with a little overshoot");
    }

    #[test]
    fn without_motion_everything_lands_at_once() {
        let ctx = Context::default();
        assert!(!enabled(), "tests run settled");
        assert_eq!(tween(&ctx, Id::new("a"), 3.0, 1.0), 3.0);
        assert_eq!(appear(&ctx, 100.0, 1.0, 1.0), 1.0);
    }

    #[test]
    fn a_tween_starts_where_it_is_and_eases_to_the_target() {
        set_enabled(true);
        let ctx = Context::default();
        let id = Id::new("test");
        let at = |time: f64, target: f32| {
            let mut value = 0.0;
            let mut output = ctx.run_ui(
                egui::RawInput {
                    time: Some(time),
                    ..Default::default()
                },
                |ui| value = tween(ui.ctx(), id, target, 1.0),
            );
            output.textures_delta.clear();
            value
        };
        assert_eq!(at(0.0, 0.0), 0.0, "first sight is the target");
        assert_eq!(at(0.0, 10.0), 0.0, "a new target starts from here");
        let halfway = at(0.5, 10.0);
        assert!(halfway > 5.0 && halfway < 10.0, "{halfway}");
        assert_eq!(at(2.0, 10.0), 10.0);
    }

    #[test]
    fn colours_mix_end_to_end() {
        let (a, b) = (Color32::BLACK, Color32::WHITE);
        assert_eq!(lerp_color(a, b, 0.0), a);
        assert_eq!(lerp_color(a, b, 1.0), b);
        assert_eq!(lerp_color(a, b, 0.5), Color32::from_gray(128));
    }
}

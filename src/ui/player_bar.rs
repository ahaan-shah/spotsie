//! The now-playing bar along the bottom of the window.

use egui::{Align, Color32, Frame, Layout, Margin, Rect, Sense, UiBuilder, Vec2, pos2, vec2};

use crate::app::{App, NowPlaying};
use crate::i18n::gettext;
use crate::model::{Action, DragTrack, Page};
use crate::player::RepeatMode;
use crate::theme::{self, Icon};
use crate::util;

use super::widgets::{SliderEvent, thin_slider};

/// How much of the playing art's tint the bar's fill carries.
const TINT_STRENGTH: f32 = 0.12;
/// How long the bar takes to cross over to a new song's tint.
const TINT_FADE_SECONDS: f32 = 0.45;
const TINT_SESSION_ID: &str = "player-bar-tint-session";
/// Forget this bar's animation session while the sign-in screen is shown.
pub(crate) fn end_tint_session(ctx: &egui::Context) {
    ctx.data_mut(|data| data.remove::<u64>(egui::Id::new(TINT_SESSION_ID)));
}

/// How long the bar stays up over the cover or the lyrics after the pointer
/// last moved over it.
const REVEAL_SECONDS: f64 = 1.5;

/// Whether the bar shows over the cover or the lyrics: for a moment after
/// the pointer moves where the bar sits, and while a control is held, the
/// device list is open, or a menu opened from the bar (or anywhere) is up,
/// so choosing from it never pulls the bar out from under the pointer.
fn revealed(app: &App, ctx: &egui::Context) -> bool {
    let id = egui::Id::new("player-bar-revealed-at");
    let (now, active) = ctx.input(|input| {
        let window = input.content_rect();
        let near = input.pointer.hover_pos().is_some_and(|pos| {
            pos.y >= window.bottom() - theme::PLAYER_BAR_HEIGHT && window.contains(pos)
        });
        let moved = input.pointer.delta() != Vec2::ZERO;
        (input.time, near && (moved || input.pointer.any_down()))
    });
    if active || app.show_devices || egui::Popup::is_any_open(ctx) {
        ctx.data_mut(|data| data.insert_temp(id, now));
    }
    let Some(at) = ctx.data(|data| data.get_temp::<f64>(id)) else {
        return false;
    };
    let left = REVEAL_SECONDS - (now - at);
    if left > 0.0 {
        ctx.request_repaint_after(std::time::Duration::from_secs_f64(left));
    }
    left > 0.0
}

pub fn show(app: &mut App, ui: &mut egui::Ui) {
    let palette = app.palette;
    let fill = eased_fill(ui.ctx(), palette.panel, app.now_playing_tint());
    let mut open = !super::immersive(app) || revealed(app, ui.ctx());
    let panel = egui::Panel::bottom("player-bar")
        .exact_size(theme::PLAYER_BAR_HEIGHT)
        .resizable(false)
        .show_separator_line(false)
        .frame(
            Frame::new()
                .fill(fill)
                .inner_margin(Margin::symmetric(16, 0)),
        );
    super::sliding_panel(ui, panel, "player-bar", &mut open, |ui| {
        let rect = ui.max_rect();
        let now = app.now_playing();
        ui.painter().hline(
            rect.x_range(),
            rect.top() + 0.5,
            egui::Stroke::new(1.0, palette.outline),
        );
        let width = rect.width();
        let fit = Fit::for_width(width);
        let shown = Shown::ease(ui.ctx(), fit);
        // The side columns ease to their new width as controls come and go.
        let side = crate::motion::tween(
            ui.ctx(),
            egui::Id::new("player-bar-side"),
            fit.side(width),
            crate::motion::STANDARD,
        )
        .min(width / 2.0);
        let cy = rect.center().y;
        let left = Rect::from_min_max(rect.min, pos2(rect.left() + side, rect.bottom()));
        let center = Rect::from_min_max(
            pos2(rect.left() + side, rect.top()),
            pos2(rect.right() - side, rect.bottom()),
        );

        // egui's cross-axis centring is unreliable across nested layouts of
        // mixed heights, so each region is placed in an explicit band that
        // is sized to its content and centred on the bar's midline.
        if shown.art > 0.01 {
            let mut left_ui = ui.new_child(UiBuilder::new().max_rect(left));
            left_ui.set_clip_rect(left.intersect(ui.clip_rect()));
            left_ui.set_opacity(shown.art);
            now_playing_block(app, &mut left_ui, left, now.as_ref(), shown.title);
        }

        transport(app, ui, now.as_ref(), center, shown);

        extras(app, ui, now.as_ref(), pos2(rect.right(), cy), shown);
    });
}

/// What the bar has room for. As the window narrows the controls go one
/// at a time, the least used first, until only Play is left.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Fit {
    cover_view: bool,
    lyrics: bool,
    volume_slider: bool,
    queue: bool,
    times: bool,
    shuffle_repeat: bool,
    devices: bool,
    mute: bool,
    title: bool,
    progress: bool,
    skips: bool,
    art: bool,
}

/// Room around the transport, so it never touches the side columns.
const CENTER_PAD: f32 = 24.0;
const TRANSPORT_GAP: f32 = 10.0;
const EXTRAS_GAP: f32 = 6.0;
const ICON_EDGE: f32 = 30.0;
const VOLUME_WIDTH: f32 = 92.0;
/// The song's name beside the cover needs at least this much.
const TITLE_COLUMN: f32 = 200.0;
/// The cover on its own.
const ART_COLUMN: f32 = 68.0;
/// The seek bar's shortest length, and the space its times take.
const PROGRESS_MIN: f32 = 96.0;
const TIMES_WIDTH: f32 = 96.0;
/// The least the window narrows between one control going and the next.
const STEP_SPACING: f32 = 24.0;

impl Fit {
    const ALL: Fit = Fit {
        cover_view: true,
        lyrics: true,
        volume_slider: true,
        queue: true,
        times: true,
        shuffle_repeat: true,
        devices: true,
        mute: true,
        title: true,
        progress: true,
        skips: true,
        art: true,
    };

    /// Each step drops one more control, in the order they go.
    fn steps() -> impl Iterator<Item = Fit> {
        let drops: [fn(&mut Fit); 12] = [
            |f| f.cover_view = false,
            |f| f.lyrics = false,
            |f| f.volume_slider = false,
            |f| f.queue = false,
            |f| f.times = false,
            |f| f.shuffle_repeat = false,
            |f| f.devices = false,
            |f| f.mute = false,
            |f| f.title = false,
            |f| f.progress = false,
            |f| f.skips = false,
            |f| f.art = false,
        ];
        let mut fit = Fit::ALL;
        std::iter::once(fit).chain(drops.into_iter().map(move |drop| {
            drop(&mut fit);
            fit
        }))
    }

    /// The most the bar can show in `width`. Each step comes in at least
    /// `STEP_SPACING` wider than the one after it, even where dropping its
    /// control frees no room, so they go and return one at a time.
    fn for_width(width: f32) -> Fit {
        let steps: Vec<Fit> = Fit::steps().collect();
        let mut from = vec![0.0; steps.len()];
        for i in (0..steps.len() - 1).rev() {
            from[i] = steps[i].needs().max(from[i + 1] + STEP_SPACING);
        }
        steps
            .iter()
            .zip(from)
            .find(|(_, from)| *from <= width)
            .map(|(fit, _)| *fit)
            .unwrap_or_else(|| *steps.last().expect("steps"))
    }

    fn left(&self) -> f32 {
        if self.title {
            TITLE_COLUMN
        } else if self.art {
            ART_COLUMN
        } else {
            0.0
        }
    }

    fn right(&self) -> f32 {
        let mut items = Vec::new();
        for (on, width) in [
            (self.volume_slider, VOLUME_WIDTH),
            (self.mute, ICON_EDGE),
            (self.devices, ICON_EDGE + 4.0),
            (self.queue, ICON_EDGE),
            (self.lyrics, ICON_EDGE),
            (self.cover_view, ICON_EDGE),
        ] {
            if on {
                items.push(width);
            }
        }
        items.iter().sum::<f32>() + EXTRAS_GAP * items.len().saturating_sub(1) as f32
    }

    fn transport(&self) -> f32 {
        let mut width = 36.0;
        if self.skips {
            width += 2.0 * (30.0 + TRANSPORT_GAP);
        }
        if self.shuffle_repeat {
            width += 2.0 * (29.0 + TRANSPORT_GAP);
        }
        let progress = if self.progress {
            PROGRESS_MIN + if self.times { TIMES_WIDTH } else { 0.0 }
        } else {
            0.0
        };
        width.max(progress) + CENTER_PAD
    }

    fn needs(&self) -> f32 {
        2.0 * self.left().max(self.right()) + self.transport()
    }

    /// How wide each side column is: a share of the bar, never less than
    /// either side needs, never so much that the transport is squeezed.
    fn side(&self, width: f32) -> f32 {
        let least = self.left().max(self.right());
        let most = ((width - self.transport()) / 2.0).max(least);
        (width * 0.3).clamp(least, most.min(420.0).max(least))
    }
}

/// How far each control has eased in, 0 gone to 1 shown.
#[derive(Clone, Copy)]
struct Shown {
    cover_view: f32,
    lyrics: f32,
    volume_slider: f32,
    queue: f32,
    times: f32,
    shuffle_repeat: f32,
    devices: f32,
    mute: f32,
    title: f32,
    progress: f32,
    skips: f32,
    art: f32,
}

impl Shown {
    fn ease(ctx: &egui::Context, fit: Fit) -> Shown {
        let k = |name: &'static str, on: bool| {
            crate::motion::toggle(
                ctx,
                egui::Id::new(("player-bar-fit", name)),
                on,
                crate::motion::STANDARD,
            )
        };
        Shown {
            cover_view: k("cover-view", fit.cover_view),
            lyrics: k("lyrics", fit.lyrics),
            volume_slider: k("volume", fit.volume_slider),
            queue: k("queue", fit.queue),
            times: k("times", fit.times),
            shuffle_repeat: k("shuffle-repeat", fit.shuffle_repeat),
            devices: k("devices", fit.devices),
            mute: k("mute", fit.mute),
            title: k("title", fit.title),
            progress: k("progress", fit.progress),
            skips: k("skips", fit.skips),
            art: k("art", fit.art),
        }
    }
}

/// A control's own band, faded to how far it has eased in. `None` once it
/// has gone.
fn faded(ui: &mut egui::Ui, rect: Rect, shown: f32) -> Option<egui::Ui> {
    (shown > 0.01).then(|| {
        let mut child = ui.new_child(
            UiBuilder::new()
                .max_rect(rect)
                .layout(Layout::centered_and_justified(egui::Direction::LeftToRight)),
        );
        child.set_opacity(shown);
        child
    })
}

/// Ease the final fill's RGB. Untinted custom panels keep their alpha while
/// the colour returns to the panel colour.
fn eased_fill(ctx: &egui::Context, panel: Color32, tint: Option<Color32>) -> Color32 {
    let target = tint.map_or(panel, |tint| super::blend(panel, tint, TINT_STRENGTH));
    // Color32 stores premultiplied RGB, so interpolate unmultiplied channels.
    let [r, g, b, _] = target.to_srgba_unmultiplied();
    // A new pass after sign-out gets new ids without clearing other animations.
    let pass = ctx.cumulative_pass_nr();
    let session = ctx.data_mut(|data| {
        *data.get_temp_mut_or_insert_with(egui::Id::new(TINT_SESSION_ID), || pass)
    });
    let channel = |axis: &'static str, value: u8| {
        ctx.animate_value_with_time(
            egui::Id::new(("player-bar-tint", session, axis)),
            f32::from(value),
            TINT_FADE_SECONDS,
        )
        .round() as u8
    };
    let eased = [channel("r", r), channel("g", g), channel("b", b)];
    if eased == [r, g, b] {
        target
    } else {
        Color32::from_rgba_unmultiplied(eased[0], eased[1], eased[2], target.a())
    }
}

fn now_playing_block(
    app: &mut App,
    ui: &mut egui::Ui,
    region: Rect,
    now: Option<&NowPlaying>,
    title_shown: f32,
) {
    let palette = app.palette;
    let cy = region.center().y;
    let cover_rect = Rect::from_min_size(pos2(region.left() + 4.0, cy - 28.0), Vec2::splat(56.0));

    let Some(now) = now else {
        super::widgets::paint_cover(ui, &palette, None, cover_rect, 6.0, Icon::Music, None);
        if title_shown <= 0.01 {
            return;
        }
        let text_left = cover_rect.right() + 12.0;
        let text_rect = Rect::from_min_size(
            pos2(text_left, cy - 17.0),
            vec2((region.right() - text_left - 8.0).max(40.0), 34.0),
        );
        let mut text_ui = ui.new_child(
            UiBuilder::new()
                .max_rect(text_rect)
                .layout(Layout::top_down(Align::Min)),
        );
        text_ui.spacing_mut().item_spacing.y = 2.0;
        text_ui.multiply_opacity(title_shown);
        theme::text(
            &mut text_ui,
            gettext(app.locale, "Nothing playing"),
            theme::medium(14.0),
            palette.secondary,
        );
        theme::text(
            &mut text_ui,
            gettext(app.locale, "Pick a song, album, or playlist"),
            theme::regular(12.0),
            palette.dim,
        );
        return;
    };

    super::widgets::paint_cover(
        ui,
        &palette,
        now.art_small.as_deref().or(now.art_url.as_deref()),
        cover_rect,
        6.0,
        Icon::Music,
        Some(app.backend.art()),
    );
    let song = app.now_playing_item();
    let drag_sense = if song.is_some() {
        Sense::click_and_drag()
    } else {
        Sense::click()
    };
    let cover_response = ui
        .interact(cover_rect, egui::Id::new("now-playing-cover"), drag_sense)
        .on_hover_cursor(if song.is_some() {
            egui::CursorIcon::Grab
        } else {
            egui::CursorIcon::Default
        });
    // Hovering the cover offers to dock the art large at the sidebar's
    // bottom, the way Spotify expands it. (#92)
    let art_available = now.art_url.is_some() || now.art_small.is_some();
    let expand_rect = Rect::from_center_size(
        pos2(cover_rect.right() - 10.0, cover_rect.top() + 10.0),
        Vec2::splat(18.0),
    );
    let offer_expand = art_available && !app.settings.art_expanded && app.settings.sidebar_visible;
    let over_expand = offer_expand && ui.rect_contains_pointer(expand_rect);
    if cover_response.clicked() && !over_expand {
        if let Some(id) = &now.album_id {
            app.actions.push(Action::Open(Page::Album(id.clone())));
        } else if let Some(id) = &now.show_id {
            app.actions.push(Action::Open(Page::Show(id.clone())));
        }
    }
    if offer_expand && (cover_response.hovered() || over_expand) {
        let expand = ui.interact(
            expand_rect,
            egui::Id::new("now-playing-art-expand"),
            Sense::click(),
        );
        ui.painter()
            .circle_filled(expand_rect.center(), 9.0, palette.panel.gamma_multiply(0.9));
        Icon::ChevronUp.image(palette.text, 12.0).paint_at(
            ui,
            Rect::from_center_size(expand_rect.center(), Vec2::splat(12.0)),
        );
        if expand.clicked() {
            app.settings.art_expanded = true;
            app.actions.push(Action::SettingsChanged);
        }
    }
    if title_shown <= 0.01 {
        return;
    }
    ui.multiply_opacity(title_shown);
    let heart_width = if now.is_episode { 0.0 } else { 42.0 };
    let text_left = cover_rect.right() + 12.0;
    let text_width = (region.right() - text_left - heart_width).max(40.0);
    let text_rect = Rect::from_min_size(pos2(text_left, cy - 18.0), vec2(text_width, 36.0));
    let info_response = ui.interact(text_rect, egui::Id::new("now-playing-info"), drag_sense);
    let mut text_ui = ui.new_child(
        UiBuilder::new()
            .max_rect(text_rect)
            .layout(Layout::top_down(Align::Min)),
    );
    text_ui.set_clip_rect(text_rect.intersect(ui.clip_rect()));
    text_ui.spacing_mut().item_spacing.y = 2.0;
    let title_response = theme::link(&mut text_ui, &now.title, theme::medium(14.0), palette.text);
    if title_response.clicked() {
        if let Some(id) = &now.album_id {
            app.actions.push(Action::Open(Page::Album(id.clone())));
        } else if let Some(id) = &now.show_id {
            app.actions.push(Action::Open(Page::Show(id.clone())));
        }
    }
    text_ui.horizontal_top(|ui| {
        if now.artists.is_empty() {
            if theme::link(ui, &now.subtitle, theme::regular(12.0), palette.secondary).clicked()
                && let Some(id) = &now.show_id
            {
                app.actions.push(Action::Open(Page::Show(id.clone())));
            }
        } else {
            super::widgets::artist_links(
                ui,
                app,
                &now.artists,
                theme::regular(12.0),
                palette.secondary,
            );
        }
    });
    if (cover_response.drag_started_by(egui::PointerButton::Primary)
        || info_response.drag_started_by(egui::PointerButton::Primary))
        && let Some(item) = &song
    {
        egui::DragAndDrop::set_payload(
            ui.ctx(),
            DragTrack {
                title: item.name().to_string(),
                image: item.image(64).map(str::to_string),
                items: vec![item.clone()],
                from: None,
            },
        );
    }

    // The playing thing answers the same right-click menu as a table row,
    // from the cover, the empty space around the words, or the words.
    if let Some(item) = song {
        let context = app.editable_context_playlist();
        for response in [&cover_response, &info_response, &title_response] {
            egui::Popup::context_menu(response)
                .frame(super::widgets::menu_frame(&palette))
                .show(|ui| super::widgets::item_menu(ui, app, &item, context.as_ref(), None));
        }
    }

    if !now.is_episode {
        let saved = app.is_saved(&now.uri).unwrap_or(false);
        let (icon, color, tooltip) = if saved {
            (
                Icon::HeartFilled,
                palette.accent,
                gettext(app.locale, "Remove from Liked Songs"),
            )
        } else {
            (
                Icon::Heart,
                palette.secondary,
                gettext(app.locale, "Save to Liked Songs"),
            )
        };
        // Sit the heart just past the actual text, not at the region's far
        // edge, so it stays visually attached to the title.
        let natural = {
            let title =
                ui.painter()
                    .layout_no_wrap(now.title.clone(), theme::medium(14.0), palette.text);
            let subtitle = ui.painter().layout_no_wrap(
                now.subtitle.clone(),
                theme::regular(12.0),
                palette.secondary,
            );
            title.size().x.max(subtitle.size().x).min(text_width)
        };
        let heart_x = (text_left + natural + 21.0).min(region.right() - 21.0);
        let heart_rect = Rect::from_center_size(pos2(heart_x, cy), Vec2::splat(30.0));
        let mut heart_ui = ui.new_child(
            UiBuilder::new()
                .max_rect(heart_rect)
                .layout(Layout::centered_and_justified(egui::Direction::LeftToRight)),
        );
        if theme::icon_button(&mut heart_ui, icon, 17.0, color, palette.text, &tooltip).clicked() {
            app.actions.push(Action::ToggleSaved(now.uri.clone()));
        }
    }
}

fn transport(
    app: &mut App,
    ui: &mut egui::Ui,
    now: Option<&NowPlaying>,
    region: Rect,
    shown: Shown,
) {
    let palette = app.palette;
    // Everything here is placed with explicit rects: egui's implicit rows
    // centre each widget in the row height known when it is added, which
    // left earlier icons riding high next to the play disc.
    //
    // The buttons row (36) and the progress row (~15, after a 6px gap) form
    // one cluster, centred as a group in the 88px bar: the buttons sit 8px
    // above the bar's midline and the progress row 23px below it. Measured
    // on screen this puts equal breathing room above and beneath the
    // cluster. Without the progress row the buttons settle on the midline.
    let cy = region.center().y - 8.0 * shown.progress;
    let enabled = now.is_some_and(|now| now.can_control) || app.is_connected();
    let playing = now.is_some_and(|now| now.playing);
    let loading = now.is_some_and(|now| now.loading);
    let shuffle = now.map_or_else(|| app.playing_context_shuffle(), |now| now.shuffle);
    let repeat = now.map(|now| now.repeat).unwrap_or_default();
    let dim = if enabled {
        palette.secondary
    } else {
        palette.dim
    };

    // Button widths: icon buttons occupy icon size + 12; the disc is 36.
    // A control making way narrows its slot as it fades, so the rest close
    // in around Play.
    let widths = [29.0, 30.0, 36.0, 30.0, 29.0];
    let eased = [
        shown.shuffle_repeat,
        shown.skips,
        1.0,
        shown.skips,
        shown.shuffle_repeat,
    ];
    let slots: [Rect; 5] = {
        let total: f32 = widths
            .iter()
            .zip(eased)
            .map(|(width, k)| (width + TRANSPORT_GAP) * k)
            .sum::<f32>()
            - TRANSPORT_GAP;
        let mut x = region.center().x - total / 2.0;
        std::array::from_fn(|i| {
            let room = widths[i] * eased[i];
            let rect = Rect::from_center_size(pos2(x + room / 2.0, cy), vec2(widths[i], 36.0));
            x += (widths[i] + TRANSPORT_GAP) * eased[i];
            rect
        })
    };
    let centered = |ui: &mut egui::Ui, rect: Rect| {
        ui.new_child(
            UiBuilder::new()
                .max_rect(rect)
                .layout(Layout::centered_and_justified(egui::Direction::LeftToRight)),
        )
    };

    let shuffle_color = if shuffle { palette.accent } else { dim };
    if let Some(mut cell) = faded(ui, slots[0], shown.shuffle_repeat) {
        let shuffle_button = theme::icon_button(
            &mut cell,
            Icon::Shuffle,
            17.0,
            shuffle_color,
            if shuffle {
                palette.accent_hover
            } else {
                palette.text
            },
            &gettext(app.locale, "Shuffle"),
        );
        shuffle_button.widget_info(|| {
            egui::WidgetInfo::selected(
                egui::WidgetType::Checkbox,
                cell.is_enabled(),
                shuffle,
                gettext(app.locale, "Shuffle"),
            )
        });
        if shuffle_button.clicked() {
            app.actions.push(Action::ToggleShuffle);
        }
    }

    if let Some(mut cell) = faded(ui, slots[1], shown.skips)
        && theme::icon_button(
            &mut cell,
            Icon::SkipBackFilled,
            18.0,
            dim,
            palette.text,
            &gettext(app.locale, "Previous"),
        )
        .clicked()
    {
        app.actions.push(Action::Previous);
    }

    let disc = slots[2];
    if loading || app.any_play_pending() {
        ui.painter()
            .circle_filled(disc.center(), 18.0, palette.text);
        let mut cell = centered(ui, disc);
        theme::spinner(&mut cell, 22.0, palette.window);
    } else {
        let icon = if playing {
            Icon::PauseFilled
        } else {
            Icon::PlayFilled
        };
        let hover = if palette.dark {
            egui::Color32::WHITE
        } else {
            palette.text
        };
        let mut cell = centered(ui, disc);
        if theme::circle_button(
            &mut cell,
            icon,
            36.0,
            palette.text,
            hover,
            palette.window,
            &if playing {
                gettext(app.locale, "Pause")
            } else {
                gettext(app.locale, "Play")
            },
        )
        .clicked()
        {
            app.actions.push(Action::TogglePlay);
        }
    }

    if let Some(mut cell) = faded(ui, slots[3], shown.skips)
        && theme::icon_button(
            &mut cell,
            Icon::SkipForwardFilled,
            18.0,
            dim,
            palette.text,
            &gettext(app.locale, "Next"),
        )
        .clicked()
    {
        app.actions.push(Action::Next);
    }

    let (repeat_icon, repeat_color, tooltip) = match repeat {
        RepeatMode::Off => (Icon::Repeat, dim, gettext(app.locale, "Repeat")),
        RepeatMode::Context => (
            Icon::Repeat,
            palette.accent,
            gettext(app.locale, "Repeat one"),
        ),
        RepeatMode::Track => (
            Icon::Repeat1,
            palette.accent,
            gettext(app.locale, "Repeat off"),
        ),
    };
    if let Some(mut cell) = faded(ui, slots[4], shown.shuffle_repeat)
        && theme::icon_button(
            &mut cell,
            repeat_icon,
            17.0,
            repeat_color,
            if repeat == RepeatMode::Off {
                palette.text
            } else {
                palette.accent_hover
            },
            &tooltip,
        )
        .clicked()
    {
        app.actions.push(Action::CycleRepeat);
    }

    if shown.progress <= 0.01 {
        return;
    }
    // Progress row, just below the buttons (disc bottom + 6px gap + half of
    // the time text's line height). It shortens with the window, and its
    // times make way before it does.
    let row_cy = cy + 31.0;
    let slider_width = (region.width() - CENTER_PAD - TIMES_WIDTH * shown.times).clamp(0.0, 620.0);
    let times_color = |color: Color32| color.gamma_multiply(shown.times * shown.progress);
    let (position, duration) = now
        .map(|now| (now.position_ms, now.duration_ms))
        .unwrap_or((0, 0));
    let shown_position = match app.seek_preview {
        Some(fraction) => (fraction * duration as f32) as u32,
        None => position,
    };
    let time_color = if now.is_some() {
        palette.secondary
    } else {
        palette.dim
    };
    let slider_left = region.center().x - slider_width / 2.0;
    ui.painter().text(
        pos2(slider_left - 8.0, row_cy),
        egui::Align2::RIGHT_CENTER,
        util::format_duration_ms(shown_position),
        theme::regular(11.5),
        times_color(time_color),
    );
    let slider_rect =
        Rect::from_center_size(pos2(region.center().x, row_cy), vec2(slider_width, 16.0));
    let mut slider_ui = ui.new_child(
        UiBuilder::new()
            .max_rect(slider_rect)
            .layout(Layout::left_to_right(Align::Center)),
    );
    slider_ui.set_opacity(shown.progress);
    let fraction = if duration > 0 {
        position as f32 / duration as f32
    } else {
        0.0
    };
    match thin_slider(
        &mut slider_ui,
        &palette,
        egui::Id::new("seek-slider"),
        &gettext(app.locale, "Playback position (%)"),
        fraction,
        slider_width,
        None,
    ) {
        SliderEvent::Dragging(value) => app.seek_preview = Some(value),
        SliderEvent::Committed(value) => {
            app.seek_preview = None;
            if duration > 0 {
                app.actions
                    .push(Action::Seek((value * duration as f32) as u32));
            }
        }
        SliderEvent::None => {}
    }
    ui.painter().text(
        pos2(slider_left + slider_width + 8.0, row_cy),
        egui::Align2::LEFT_CENTER,
        util::format_duration_ms(duration),
        theme::regular(11.5),
        times_color(time_color),
    );
}

/// Volume, devices, queue, lyrics and cover view, laid out leftwards from
/// `right`. A control making way narrows its slot as it fades.
fn extras(
    app: &mut App,
    ui: &mut egui::Ui,
    now: Option<&NowPlaying>,
    right: egui::Pos2,
    shown: Shown,
) {
    let palette = app.palette;
    let mut x = right.x;
    let mut slot = |width: f32, k: f32| {
        let rect =
            Rect::from_center_size(pos2(x - width * k / 2.0, right.y), vec2(width, ICON_EDGE));
        x -= (width + EXTRAS_GAP) * k;
        rect
    };
    let volume = now
        .map(|now| now.volume_percent)
        .unwrap_or_else(|| crate::app::volume_to_percent(app.local.volume));
    let shown_volume = match app.volume_preview {
        Some(fraction) => (fraction * 100.0).round() as u8,
        None => volume,
    };
    // Some remote devices, such as a phone playing over Bluetooth, refuse
    // volume changes from other apps: show their volume but don't offer to
    // change it.
    let adjustable = now.is_none_or(|now| now.can_set_volume);
    let mut volume_rect = Rect::NOTHING;
    let slider = slot(VOLUME_WIDTH, shown.volume_slider);
    if let Some(mut cell) = faded(ui, slider, shown.volume_slider) {
        if !adjustable {
            cell.disable();
        }
        volume_rect = volume_rect.union(slider);
        match thin_slider(
            &mut cell,
            &palette,
            egui::Id::new("volume-slider"),
            &gettext(app.locale, "Volume (%)"),
            shown_volume as f32 / 100.0,
            VOLUME_WIDTH,
            Some(0.05),
        ) {
            SliderEvent::Dragging(value) => {
                app.volume_preview = Some(value);
                // Local volume is cheap to apply continuously; remote goes on release.
                if now.is_none_or(|now| now.local) {
                    app.actions
                        .push(Action::PreviewVolume((value * 100.0).round() as u8));
                }
            }
            SliderEvent::Committed(value) => {
                app.volume_preview = None;
                app.actions
                    .push(Action::SetVolume((value * 100.0).round() as u8));
            }
            SliderEvent::None => {}
        }
    }
    let mute = slot(ICON_EDGE, shown.mute);
    if let Some(mut cell) = faded(ui, mute, shown.mute) {
        if !adjustable {
            cell.disable();
        }
        volume_rect = volume_rect.union(mute);
        let volume_icon = match shown_volume {
            0 => Icon::VolumeX,
            1..=33 => Icon::Volume,
            34..=66 => Icon::Volume1,
            _ => Icon::Volume2,
        };
        if theme::icon_button(
            &mut cell,
            volume_icon,
            18.0,
            palette.secondary,
            palette.text,
            &if shown_volume == 0 {
                gettext(app.locale, "Unmute")
            } else {
                gettext(app.locale, "Mute")
            },
        )
        .clicked()
        {
            app.actions.push(Action::ToggleMute);
        }
    }
    if !adjustable && volume_rect.is_positive() {
        ui.interact(
            volume_rect,
            egui::Id::new("volume-fixed"),
            egui::Sense::hover(),
        )
        .on_hover_text(gettext(
            app.locale,
            "This device's volume can't be changed from Spotsie",
        ));
    }
    let remote = now.is_some_and(|now| !now.local);
    // The devices button keeps a little more room from the volume.
    let devices = slot(ICON_EDGE + 4.0, shown.devices);
    let devices = Rect::from_center_size(devices.center() - vec2(2.0, 0.0), Vec2::splat(ICON_EDGE));
    if let Some(mut cell) = faded(ui, devices, shown.devices) {
        let response = theme::icon_button(
            &mut cell,
            Icon::Speaker,
            18.0,
            if remote {
                palette.accent
            } else {
                palette.secondary
            },
            palette.text,
            &gettext(app.locale, "Connect to a device"),
        );
        ui.ctx().data_mut(|data| {
            data.insert_temp(egui::Id::new(super::devices::BUTTON_RECT_ID), response.rect)
        });
        if response.clicked() {
            app.actions.push(Action::ToggleDevicesPopup);
        }
    }
    let queue_open = app.show_queue_panel || matches!(app.page(), Page::Queue);
    if let Some(mut cell) = faded(ui, slot(ICON_EDGE, shown.queue), shown.queue) {
        let queue_button = theme::icon_button(
            &mut cell,
            Icon::ListVideo,
            18.0,
            if queue_open {
                palette.accent
            } else {
                palette.secondary
            },
            palette.text,
            &gettext(app.locale, "Queue"),
        );
        if queue_button.clicked() {
            app.actions.push(Action::ToggleQueuePanel);
        }
        // A dragged song dropped on the queue button queues it, same as the
        // "Add to queue" menu item.
        if let Some(track) = queue_button.dnd_release_payload::<DragTrack>() {
            app.actions.push(Action::QueueMany {
                songs: track
                    .items
                    .iter()
                    .map(|item| (item.uri().to_string(), item.name().to_string()))
                    .collect(),
            });
        }
    }
    if let Some(mut cell) = faded(ui, slot(ICON_EDGE, shown.lyrics), shown.lyrics)
        && theme::icon_button(
            &mut cell,
            Icon::Lyrics,
            18.0,
            if app.show_lyrics_panel {
                palette.accent
            } else {
                palette.secondary
            },
            palette.text,
            &gettext(app.locale, "Lyrics"),
        )
        .clicked()
    {
        app.actions.push(Action::ToggleLyricsPanel);
    }
    if let Some(mut cell) = faded(ui, slot(ICON_EDGE, shown.cover_view), shown.cover_view)
        && theme::icon_button(
            &mut cell,
            Icon::CoverView,
            18.0,
            if app.show_cover_view {
                palette.accent
            } else {
                palette.secondary
            },
            palette.text,
            &gettext(app.locale, "Cover view"),
        )
        .clicked()
    {
        app.actions.push(Action::ToggleCoverView);
    }
}

#[cfg(test)]
mod player_bar_tint_tests {
    use super::*;
    use crate::theme::Palette;

    /// Run one frame at `time` and report the bar's actual fill.
    fn frame(ctx: &egui::Context, time: f64, panel: Color32, tint: Option<Color32>) -> Color32 {
        let mut fill = Color32::PLACEHOLDER;
        let mut output = ctx.run_ui(
            egui::RawInput {
                time: Some(time),
                ..Default::default()
            },
            |ui| fill = eased_fill(ui.ctx(), panel, tint),
        );
        output.textures_delta.clear();
        fill
    }

    #[test]
    fn a_song_without_art_preserves_translucent_panel() {
        for panel in [
            Palette::dark().panel,
            Palette::light().panel,
            Color32::from_rgba_unmultiplied(80, 120, 180, 128),
        ] {
            let ctx = egui::Context::default();
            assert_eq!(frame(&ctx, 0.0, panel, None), panel);
            let art = Color32::from_rgb(200, 40, 90);
            frame(&ctx, 0.1, panel, Some(art));
            frame(&ctx, 0.6, panel, Some(art));
            assert_eq!(frame(&ctx, 0.7, panel, None).a(), panel.a());
            assert_eq!(frame(&ctx, 1.2, panel, None), panel);
        }
    }

    #[test]
    fn the_first_frame_shows_the_tint_without_fading_in() {
        let ctx = egui::Context::default();
        let panel = Palette::dark().panel;
        let art = Color32::from_rgb(200, 40, 90);
        assert_eq!(
            frame(&ctx, 0.0, panel, Some(art)),
            super::super::blend(panel, art, TINT_STRENGTH)
        );
    }

    #[test]
    fn first_art_after_an_untinted_frame_fades_in() {
        let ctx = egui::Context::default();
        let panel = Palette::dark().panel;
        let art = Color32::from_rgb(220, 140, 160);
        let target = super::super::blend(panel, art, TINT_STRENGTH);
        let fade = f64::from(TINT_FADE_SECONDS);

        assert_eq!(frame(&ctx, 0.0, panel, None), panel);
        assert_eq!(frame(&ctx, 0.1, panel, Some(art)), panel);
        let middle = frame(&ctx, 0.1 + fade / 2.0, panel, Some(art));
        assert_ne!(middle, panel);
        assert_ne!(middle, target);
        assert_eq!(frame(&ctx, 0.1 + fade, panel, Some(art)), target);
    }

    #[test]
    fn changing_songs_mid_fade_continues_from_the_visible_colour() {
        let ctx = egui::Context::default();
        let panel = Palette::dark().panel;
        let first = Color32::from_rgb(20, 40, 60);
        let second = Color32::from_rgb(220, 140, 160);
        let third = Color32::from_rgb(40, 230, 30);
        let first_fill = super::super::blend(panel, first, TINT_STRENGTH);
        let second_fill = super::super::blend(panel, second, TINT_STRENGTH);
        let third_fill = super::super::blend(panel, third, TINT_STRENGTH);
        let fade = f64::from(TINT_FADE_SECONDS);

        assert_eq!(frame(&ctx, 0.0, panel, Some(first)), first_fill);
        assert_eq!(frame(&ctx, 0.05, panel, Some(second)), first_fill);
        let middle = frame(&ctx, 0.2, panel, Some(second));
        assert_ne!(middle, first_fill);
        assert_ne!(middle, second_fill);
        assert_eq!(frame(&ctx, 0.2, panel, Some(third)), middle);
        assert_ne!(frame(&ctx, 0.2 + fade / 2.0, panel, Some(third)), middle);
        assert_eq!(frame(&ctx, 0.2 + fade, panel, Some(third)), third_fill);
    }

    #[test]
    fn a_new_session_does_not_reuse_the_previous_tint() {
        let ctx = egui::Context::default();
        let panel = Palette::dark().panel;
        let first = Color32::from_rgb(20, 40, 60);
        let second = Color32::from_rgb(220, 140, 160);
        let next_session = Color32::from_rgb(40, 230, 30);

        frame(&ctx, 0.0, panel, Some(first));
        frame(&ctx, 0.1, panel, Some(second));
        assert_ne!(
            frame(&ctx, 0.2, panel, Some(second)),
            super::super::blend(panel, second, TINT_STRENGTH)
        );

        end_tint_session(&ctx);
        assert_eq!(
            frame(&ctx, 0.21, panel, Some(next_session)),
            super::super::blend(panel, next_session, TINT_STRENGTH)
        );

        end_tint_session(&ctx);
        assert_eq!(frame(&ctx, 0.22, panel, None), panel);
    }
}

#[cfg(test)]
mod player_bar_fit_tests {
    use super::*;

    /// At any width the side columns hold what they show and the transport
    /// keeps its room between them, so nothing overlaps.
    #[test]
    fn nothing_overlaps_at_any_width() {
        for width in (40..=2400).step_by(5) {
            let width = width as f32;
            let fit = Fit::for_width(width);
            if fit == Fit::steps().last().unwrap() {
                continue;
            }
            let side = fit.side(width);
            assert!(
                side >= fit.left() && side >= fit.right(),
                "{width}: {fit:?}"
            );
            assert!(
                width - 2.0 * side + 0.01 >= fit.transport(),
                "{width}: {fit:?} side {side}"
            );
        }
    }

    /// Narrowing takes controls away one at a time and widening brings
    /// them back, down to Play alone.
    #[test]
    fn controls_go_one_at_a_time_down_to_play() {
        assert_eq!(Fit::for_width(1400.0), Fit::ALL);
        let tiny = Fit::for_width(60.0);
        assert_eq!(tiny, Fit::steps().last().unwrap());
        assert!(!tiny.skips && !tiny.art && !tiny.progress && !tiny.mute);
        let step = |fit: Fit| Fit::steps().position(|s| s == fit).unwrap();
        let mut last = 0;
        for width in (60..=1400).rev().step_by(2) {
            let now = step(Fit::for_width(width as f32));
            assert!(now <= last + 1, "two controls went at once at {width}");
            assert!(
                now >= last,
                "a control came back while narrowing at {width}"
            );
            last = now;
        }
    }
}

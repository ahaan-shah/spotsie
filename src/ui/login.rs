//! Onboarding and sign-in: a welcome, the steps for making the Spotify app
//! Spotsie signs in through, its Client ID, and the wait for the browser.
//!
//! Spotify no longer lets apps like Spotsie share one registered app, so
//! every listener makes their own (free, a few minutes). The steps here
//! walk them through it. Motion follows Magpie's and Opentern's onboarding:
//! each step rises into place and leaves upwards, over a soft glow in the
//! theme's accent that drifts as they go.

use egui::{
    Align, Align2, Color32, CornerRadius, Id, Layout, Pos2, Rect, Sense, Stroke, Vec2, pos2, vec2,
};

use crate::app::App;
use crate::backend::{AuthStatus, SignInProblem};
use crate::i18n::{Locale, gettext};
use crate::model::Action;
use crate::motion;
use crate::settings::ProxyMode;
use crate::theme::{self, Icon, Palette};

/// Where listeners create their app. Opened only when they ask.
pub const DASHBOARD_URL: &str = "https://developer.spotify.com/dashboard";

/// Where onboarding is. Lives on the app so it survives a failed sign-in
/// and can be set by `--demo-show`.
#[derive(Clone, Debug, Default)]
pub struct Onboarding {
    /// Which step shows, once decided on the first frame.
    step: Option<Step>,
    shown_at: Option<f64>,
    /// The setup steps ticked off, in order.
    done: [bool; 4],
    /// The problem last explained, and when it arrived, so a new one lands.
    problem: Option<(String, f64)>,
}

impl Onboarding {
    /// Opens at a given step, for `--demo-show`.
    pub fn at(step: Step) -> Self {
        Self {
            step: Some(step),
            ..Self::default()
        }
    }

    /// With the first `count` setup steps ticked off, for `--demo-show`.
    pub fn ticked(mut self, count: usize) -> Self {
        for done in self.done.iter_mut().take(count) {
            *done = true;
        }
        self
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Step {
    Welcome,
    MakeApp,
    ClientId,
}

/// Which pane shows: the listener's step, or the wait while Spotify answers.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Pane {
    Welcome,
    MakeApp,
    ClientId,
    Waiting,
}

impl Pane {
    const ALL: [Pane; 4] = [Pane::Welcome, Pane::MakeApp, Pane::ClientId, Pane::Waiting];

    fn index(self) -> usize {
        self as usize
    }
}

/// Space kept free at the top for the window's controls and the progress
/// dots, and at the bottom for the version line.
const TOP: f32 = 72.0;
const BOTTOM: f32 = 48.0;

fn saved_app(app: &App) -> bool {
    app.settings
        .web_client_id
        .as_deref()
        .is_some_and(crate::auth::looks_like_client_id)
}

/// The first step: straight to the Client ID for someone who has set up an
/// app before, to the setup when Spotify retired the shared sign-in they
/// were on, and the welcome otherwise.
fn first_step(app: &App) -> Step {
    if matches!(app.auth, AuthStatus::Refused(SignInProblem::SharedRetired)) {
        Step::MakeApp
    } else if saved_app(app) {
        Step::ClientId
    } else {
        Step::Welcome
    }
}

pub fn show(app: &mut App, ui: &mut egui::Ui, connecting: bool) {
    let palette = app.palette;
    let ctx = ui.ctx().clone();
    let now = ctx.input(|input| input.time);
    if app.onboarding.step.is_none() {
        app.onboarding.step = Some(first_step(app));
    }
    let shown = *app.onboarding.shown_at.get_or_insert(now);
    let waiting = connecting || matches!(app.auth, AuthStatus::WaitingForBrowser { .. });
    let step = app.onboarding.step.unwrap_or(Step::Welcome);
    let pane = match step {
        _ if waiting => Pane::Waiting,
        Step::Welcome => Pane::Welcome,
        Step::MakeApp => Pane::MakeApp,
        Step::ClientId => Pane::ClientId,
    };
    egui::CentralPanel::default()
        .frame(egui::Frame::new().fill(palette.window))
        .show(ui, |ui| {
            let rect = ui.max_rect();
            // The native double-click action belongs to the top bar, not the
            // empty background.
            let drag_rect = if cfg!(target_os = "macos") {
                Rect::from_min_size(
                    rect.min,
                    Vec2::new(
                        rect.width(),
                        theme::TOP_BAR_HEIGHT + theme::titlebar_inset(ui.ctx()),
                    ),
                )
            } else {
                rect
            };
            super::titlebar_drag(ui, drag_rect);
            backdrop(ui, &palette, rect, shown, pane);
            progress_dots(ui, &palette, rect, pane);
            for each in Pane::ALL {
                let k = motion::toggle(
                    &ctx,
                    Id::new(("onboarding-pane", each.index())),
                    pane == each,
                    motion::EMPHASIS,
                );
                if k < 0.001 {
                    continue;
                }
                // Steps ahead rise in from below; steps behind leave upwards.
                let dy = if pane.index() >= each.index() {
                    (1.0 - k) * 36.0
                } else {
                    -(1.0 - k) * 36.0
                };
                let mut child = ui.new_child(
                    egui::UiBuilder::new()
                        .max_rect(rect.translate(vec2(0.0, dy)))
                        .layout(Layout::top_down(Align::Center)),
                );
                child.set_opacity(k);
                if pane != each {
                    child.disable();
                }
                scrolled(&mut child, each, |ui| match each {
                    Pane::Welcome => welcome(app, ui, shown),
                    Pane::MakeApp => make_app(app, ui),
                    Pane::ClientId => client_id(app, ui),
                    Pane::Waiting => waiting_pane(app, ui, connecting),
                });
            }
            ui.painter().text(
                pos2(rect.center().x, rect.bottom() - 20.0),
                Align2::CENTER_BOTTOM,
                // Translators: {version} is the app's version number, such as 1.2.0.
                gettext(
                    app.locale,
                    "Spotsie {version} • not affiliated with Spotify",
                )
                .replace("{version}", env!("CARGO_PKG_VERSION")),
                theme::regular(11.5),
                palette.dim,
            );
        });
}

/// The window's colour with a soft glow in the accent, which drifts a little
/// with each step, as Opentern's does.
fn backdrop(ui: &egui::Ui, palette: &Palette, rect: Rect, shown: f64, pane: Pane) {
    let ctx = ui.ctx();
    let glow = motion::appear(ctx, shown, 0.0, 1.2);
    let drift = motion::tween(
        ctx,
        Id::new("onboarding-drift"),
        pane.index() as f32 - 1.5,
        motion::EMPHASIS,
    );
    let strength = if palette.dark { 0.16 } else { 0.11 };
    let radius = rect.width().max(rect.height()) * 0.55;
    radial_glow(
        ui.painter(),
        pos2(
            rect.center().x + drift * 70.0,
            rect.top() + rect.height() * 0.28,
        ),
        radius,
        motion::with_alpha(palette.accent, strength * glow),
    );
    // A fainter, wider one low on the other side gives the page depth.
    radial_glow(
        ui.painter(),
        pos2(
            rect.center().x - drift * 120.0 - rect.width() * 0.3,
            rect.bottom(),
        ),
        radius * 0.9,
        motion::with_alpha(palette.accent, strength * 0.35 * glow),
    );
}

/// Three dots for the setup, the current one drawn out into a pill.
fn progress_dots(ui: &egui::Ui, palette: &Palette, rect: Rect, pane: Pane) {
    let ctx = ui.ctx();
    let visible = motion::toggle(
        ctx,
        Id::new("onboarding-dots"),
        pane != Pane::Welcome,
        motion::STANDARD,
    );
    if visible < 0.01 {
        return;
    }
    let y = rect.top() + 36.0;
    let widths: Vec<f32> = (0..3)
        .map(|i| {
            let on = pane.index() == i + 1;
            6.0 + 18.0 * motion::toggle(ctx, Id::new(("onboarding-dot", i)), on, motion::STANDARD)
        })
        .collect();
    let total = widths.iter().sum::<f32>() + 8.0 * 2.0;
    let mut x = rect.center().x - total / 2.0;
    for (i, width) in widths.into_iter().enumerate() {
        let reached = pane.index() > i;
        let color = if reached {
            palette.accent
        } else {
            palette.dim.gamma_multiply(0.6)
        };
        ui.painter().rect_filled(
            Rect::from_min_size(pos2(x, y - 3.0), vec2(width, 6.0)),
            3.0,
            motion::with_alpha(color, visible),
        );
        x += width + 8.0;
    }
}

/// A pane's content, centred in the window when it fits and scrolling when
/// it doesn't. Its height is measured each frame to centre the next.
fn scrolled(ui: &mut egui::Ui, pane: Pane, add: impl FnOnce(&mut egui::Ui)) {
    let full = ui.max_rect();
    let area = Rect::from_min_max(
        pos2(full.left(), full.top() + TOP),
        pos2(full.right(), full.bottom() - BOTTOM),
    );
    let height_id = Id::new(("onboarding-height", pane.index()));
    let measured = ui.ctx().data(|data| data.get_temp::<f32>(height_id));
    let mut child = ui.new_child(
        egui::UiBuilder::new()
            .max_rect(area)
            .layout(Layout::top_down(Align::Center)),
    );
    let output = egui::ScrollArea::vertical()
        .id_salt(("onboarding-scroll", pane.index()))
        .auto_shrink([false, false])
        .show(&mut child, |ui| {
            let top = measured.map_or(0.0, |height| {
                ((area.height() - height) / 2.0 - 12.0).max(0.0)
            });
            ui.add_space(top);
            let width = ui.available_width();
            ui.allocate_ui_with_layout(vec2(width, 0.0), Layout::top_down(Align::Center), add)
                .response
                .rect
                .height()
        });
    if measured != Some(output.inner) {
        ui.ctx()
            .data_mut(|data| data.insert_temp(height_id, output.inner));
        ui.ctx().request_repaint();
    }
}

fn wrapped(
    ui: &mut egui::Ui,
    text: impl Into<String>,
    font: egui::FontId,
    color: Color32,
    width: f32,
) {
    ui.scope(|ui| {
        ui.set_max_width(width);
        ui.add(
            egui::Label::new(egui::RichText::new(text.into()).font(font).color(color))
                .wrap()
                .halign(Align::Center),
        );
    });
}

/// How wide a pane's column is in this window.
fn column(ui: &egui::Ui, most: f32) -> f32 {
    (ui.available_width() - 48.0).clamp(240.0, most)
}

/// How far the record has turned: once every nine seconds, as on the
/// website, and quicker while the pointer is over it. The speed eases
/// between the two so it never jumps.
fn record_angle(ctx: &egui::Context, salt: &str, hovered: bool) -> f32 {
    if !motion::enabled() {
        return 0.0;
    }
    let id = Id::new(("record-spin", salt));
    let turns_per_second = motion::tween(
        ctx,
        id.with("speed"),
        if hovered { 1.0 / 2.4 } else { 1.0 / 9.0 },
        0.8,
    );
    let dt = ctx.input(|input| input.stable_dt).min(0.1);
    let angle = ctx.data_mut(|data| {
        let angle = data.get_temp_mut_or_default::<f32>(id);
        *angle = (*angle + dt * turns_per_second * std::f32::consts::TAU) % std::f32::consts::TAU;
        *angle
    });
    ctx.request_repaint();
    angle
}

/// The record, spinning in a soft halo of the accent.
fn spinning_record(
    ui: &mut egui::Ui,
    palette: &Palette,
    salt: &str,
    rect: Rect,
    size: f32,
    glow: f32,
) {
    let hovered =
        ui.rect_contains_pointer(Rect::from_center_size(rect.center(), Vec2::splat(size)));
    radial_glow(
        ui.painter(),
        rect.center(),
        size * 1.25,
        motion::with_alpha(
            palette.accent,
            if palette.dark { 0.30 } else { 0.22 } * glow,
        ),
    );
    let angle = record_angle(ui.ctx(), salt, hovered);
    theme::logo_turned(ui, rect.center(), size, angle);
}

/// Step 1: the record, the name and Get started.
fn welcome(app: &mut App, ui: &mut egui::Ui, shown: f64) {
    let palette = app.palette;
    let locale = app.locale;
    let ctx = ui.ctx().clone();
    let logo = motion::appear(&ctx, shown, 0.1, 0.9);
    let words = motion::appear(&ctx, shown, 0.45, 0.8);
    let button = motion::appear(&ctx, shown, 0.8, 0.6);
    let width = column(ui, 520.0);
    let (rect, _) = ui.allocate_exact_size(vec2(width, 156.0), Sense::hover());
    let size = 128.0 * (0.86 + 0.14 * motion::ease_out_back(logo));
    let mut logo_ui = ui.new_child(egui::UiBuilder::new().max_rect(rect));
    logo_ui.multiply_opacity(logo);
    spinning_record(&mut logo_ui, &palette, "welcome", rect, size, logo);
    ui.add_space(22.0 + (1.0 - words) * 12.0);
    ui.scope(|ui| {
        ui.multiply_opacity(words);
        // The name in the brand's italic and the accent, as the website
        // stresses its words.
        // Translators: {name} is Spotsie, set in italics in the accent colour.
        let title = gettext(locale, "Welcome to {name}");
        let (before, after) = title.split_once("{name}").unwrap_or((&title, ""));
        let size = if width < 360.0 { 34.0 } else { 46.0 };
        let mut job = egui::text::LayoutJob::default();
        let plain = egui::TextFormat::simple(theme::brand(size), palette.text);
        job.append(before, 0.0, plain.clone());
        job.append(
            "Spotsie",
            0.0,
            egui::TextFormat::simple(theme::brand_italic(size), palette.accent),
        );
        job.append(after, 0.0, plain);
        job.wrap.max_width = width;
        job.halign = Align::Center;
        ui.add(egui::Label::new(job).selectable(false));
        ui.add_space(10.0);
        wrapped(
            ui,
            gettext(locale, "Your Spotify, without the weight."),
            theme::regular(17.0),
            palette.secondary,
            width,
        );
    });
    ui.add_space(36.0 + (1.0 - button) * 8.0);
    let go = ui
        .scope(|ui| {
            ui.multiply_opacity(button);
            primary_button(
                ui,
                &palette,
                "onboarding-begin",
                &gettext(locale, "Get started"),
                220.0,
                true,
            )
        })
        .inner;
    ui.add_space(16.0);
    ui.scope(|ui| {
        ui.multiply_opacity(button);
        wrapped(
            ui,
            gettext(locale, "Playing music in Spotsie needs Spotify Premium."),
            theme::regular(12.5),
            palette.dim,
            width,
        );
        ui.add_space(10.0);
        proxy_link(app, ui);
    });
    let enter =
        ui.is_enabled() && button > 0.5 && ui.input(|input| input.key_pressed(egui::Key::Enter));
    if go || enter {
        app.onboarding.step = Some(Step::MakeApp);
    }
}

/// One setup step, said in a line, and how Spotsie can help with it.
struct SetupStep {
    title: String,
    action: Option<SetupAction>,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum SetupAction {
    OpenDashboard,
    CopyRedirect,
}

fn setup_steps(locale: Locale) -> [SetupStep; 4] {
    [
        SetupStep {
            title: gettext(locale, "Open Spotify's developer dashboard").into_owned(),
            action: Some(SetupAction::OpenDashboard),
        },
        SetupStep {
            title: gettext(locale, "Click Create app. Any name and description works.")
                .into_owned(),
            action: None,
        },
        SetupStep {
            title: gettext(locale, "Add this redirect URI").into_owned(),
            action: Some(SetupAction::CopyRedirect),
        },
        SetupStep {
            title: gettext(locale, "Tick Web API, then click Save").into_owned(),
            action: None,
        },
    ]
}

/// Step 2: making the app, four short steps ticked off in turn.
fn make_app(app: &mut App, ui: &mut egui::Ui) {
    let palette = app.palette;
    let locale = app.locale;
    let width = column(ui, 580.0);
    theme::text(
        ui,
        gettext(locale, "Make your Spotify app"),
        theme::brand(30.0),
        palette.text,
    );
    ui.add_space(8.0);
    let intro = if matches!(app.auth, AuthStatus::Refused(SignInProblem::SharedRetired)) {
        gettext(
            locale,
            "Spotify no longer accepts the shared sign-in Spotsie used before. Make your own app to keep listening.",
        )
    } else {
        gettext(locale, "It's free and takes about two minutes.")
    };
    wrapped(ui, intro, theme::regular(15.0), palette.secondary, width);
    ui.add_space(26.0);
    for (index, step) in setup_steps(locale).iter().enumerate() {
        let height = card_height(ui, locale, step, width);
        let (rect, _) = ui.allocate_exact_size(vec2(width, height), Sense::hover());
        setup_card(app, ui, index, step, rect);
        ui.add_space(12.0);
    }
    ui.add_space(18.0);
    let mut back = false;
    let mut next = false;
    let (row, _) = ui.allocate_exact_size(vec2(width, 48.0), Sense::hover());
    let mut row_ui = ui.new_child(
        egui::UiBuilder::new()
            .max_rect(row)
            .layout(Layout::left_to_right(Align::Center)),
    );
    if back_link(&mut row_ui, &palette, &gettext(locale, "Back")) {
        back = true;
    }
    row_ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
        let label = gettext(locale, "Continue");
        let width = button_width(ui, &label).max(160.0);
        if primary_button(ui, &palette, "onboarding-next", &label, width, true) {
            next = true;
        }
    });
    if ui.is_enabled() && ui.input(|input| input.key_pressed(egui::Key::Enter)) && !text_focused(ui)
    {
        next = true;
    }
    if back {
        app.onboarding.step = Some(Step::Welcome);
    } else if next {
        app.onboarding.step = Some(Step::ClientId);
    }
}

const CARD_PAD: f32 = 16.0;
const BADGE: f32 = 30.0;
const CARD_ROW: f32 = 64.0;

/// How wide a step's helper is: the dashboard button, or the redirect URI
/// and its copy button.
fn action_width(ui: &egui::Ui, locale: Locale, action: SetupAction) -> f32 {
    match action {
        SetupAction::OpenDashboard => soft_pill_width(ui, true, &gettext(locale, "Open")),
        SetupAction::CopyRedirect => {
            ui.painter()
                .layout_no_wrap(
                    crate::auth::personal_redirect_uri(),
                    egui::FontId::monospace(12.5),
                    Color32::WHITE,
                )
                .size()
                .x
                + 22.0
                + 8.0
                + 32.0
        }
    }
}

/// Whether a step's helper sits beside its words, or below them in a
/// narrow window.
fn action_beside(ui: &egui::Ui, locale: Locale, step: &SetupStep, width: f32) -> bool {
    let Some(action) = step.action else {
        return true;
    };
    let title = ui
        .painter()
        .layout_no_wrap(step.title.clone(), theme::semibold(15.0), Color32::WHITE)
        .size()
        .x;
    CARD_PAD * 2.0 + BADGE + 14.0 + title + 20.0 + action_width(ui, locale, action) <= width
}

fn card_height(ui: &egui::Ui, locale: Locale, step: &SetupStep, width: f32) -> f32 {
    if action_beside(ui, locale, step, width) {
        CARD_ROW
    } else {
        CARD_ROW + 40.0
    }
}

fn setup_card(app: &mut App, ui: &mut egui::Ui, index: usize, step: &SetupStep, rect: Rect) {
    let palette = app.palette;
    let locale = app.locale;
    let ctx = ui.ctx().clone();
    let id = Id::new(("onboarding-card", index));
    let response = ui.interact(rect, id, Sense::click());
    let hover = motion::toggle(&ctx, id.with("hover"), response.hovered(), motion::STANDARD);
    let done = app.onboarding.done[index];
    let tick = motion::toggle(&ctx, id.with("done"), done, motion::STANDARD);
    // Cards come in one after another, as Magpie's do.
    let entrance = motion::reveal(&ctx, app.onboarding.shown_at.unwrap_or(0.0), index);
    let rect = rect.translate(vec2(0.0, entrance.1 - 1.5 * hover));
    let mut painter = ui.painter().clone();
    painter.multiply_opacity(entrance.0);
    let radius = CornerRadius::same(theme::RADIUS_CARD);
    // Just the border: a thin edge in the accent with a soft line outside
    // it, barely there on hover and a little clearer once the step is
    // ticked off. Dark mode needs less of it to read the same.
    let strength = if palette.dark { 0.55 } else { 0.8 };
    let lit = tick.max(hover * 0.6);
    painter.rect(
        rect,
        radius,
        palette.panel,
        Stroke::new(
            1.0,
            motion::lerp_color(
                palette.outline,
                palette.accent.gamma_multiply(strength),
                lit,
            ),
        ),
        egui::StrokeKind::Inside,
    );
    if lit > 0.01 {
        painter.rect_stroke(
            rect.expand(1.5),
            CornerRadius::same(theme::RADIUS_CARD + 2),
            Stroke::new(
                2.0,
                motion::with_alpha(palette.accent, 0.12 * strength * lit),
            ),
            egui::StrokeKind::Outside,
        );
    }
    // The number, which turns into a check once the step is done.
    let badge = Rect::from_center_size(
        pos2(
            rect.left() + CARD_PAD + BADGE / 2.0,
            rect.top() + CARD_ROW / 2.0,
        ),
        Vec2::splat(BADGE),
    );
    painter.circle_filled(
        badge.center(),
        BADGE / 2.0,
        motion::lerp_color(
            super::blend(palette.panel, palette.accent, 0.16),
            palette.accent,
            tick,
        ),
    );
    if tick < 0.99 {
        painter.text(
            badge.center(),
            Align2::CENTER_CENTER,
            (index + 1).to_string(),
            theme::bold(13.0),
            motion::with_alpha(palette.accent, 1.0 - tick),
        );
    }
    if tick > 0.01 {
        let size = 15.0 * (0.6 + 0.4 * motion::ease_out_back(tick));
        let mut icon_ui = ui.new_child(egui::UiBuilder::new().max_rect(badge));
        icon_ui.set_opacity(tick * entrance.0);
        theme::paint_icon(&icon_ui, Icon::Check, badge, size, palette.on_accent);
    }
    let text_left = badge.right() + 14.0;
    painter.text(
        pos2(text_left, badge.center().y),
        Align2::LEFT_CENTER,
        &step.title,
        theme::semibold(15.0),
        palette.text,
    );
    let mut acted = false;
    if let Some(action) = step.action {
        let width = action_width(ui, locale, action);
        let row = if action_beside(ui, locale, step, rect.width()) {
            Rect::from_min_size(
                pos2(rect.right() - CARD_PAD - width, badge.center().y - 17.0),
                vec2(width, 34.0),
            )
        } else {
            Rect::from_min_size(
                pos2(text_left, rect.top() + CARD_ROW - 8.0),
                vec2(width, 34.0),
            )
        };
        let mut row_ui = ui.new_child(
            egui::UiBuilder::new()
                .max_rect(row)
                .layout(Layout::left_to_right(Align::Center)),
        );
        row_ui.multiply_opacity(entrance.0);
        match action {
            SetupAction::OpenDashboard => {
                if soft_pill(
                    &mut row_ui,
                    &palette,
                    Some(Icon::ExternalLink),
                    &gettext(locale, "Open"),
                )
                .on_hover_text(DASHBOARD_URL)
                .clicked()
                {
                    ctx.open_url(egui::OpenUrl::new_tab(DASHBOARD_URL));
                    acted = true;
                }
            }
            SetupAction::CopyRedirect => {
                acted = redirect_chip(
                    &mut row_ui,
                    &palette,
                    locale,
                    "onboarding-copy-redirect",
                    true,
                );
            }
        }
    }
    if acted {
        app.onboarding.done[index] = true;
    } else if response
        .on_hover_cursor(egui::CursorIcon::PointingHand)
        .clicked()
    {
        app.onboarding.done[index] = !done;
    }
}

/// The redirect URI in a chip, with a button that copies it, just its icon
/// when `compact`. True when copied this frame.
fn redirect_chip(
    ui: &mut egui::Ui,
    palette: &Palette,
    locale: Locale,
    salt: &str,
    compact: bool,
) -> bool {
    let uri = crate::auth::personal_redirect_uri();
    let copied_id = Id::new((salt, "copied"));
    let now = ui.input(|input| input.time);
    let copied_at = ui.data(|data| data.get_temp::<f64>(copied_id));
    let copied = copied_at.is_some_and(|at| now - at < 2.0);
    ui.spacing_mut().item_spacing.x = 8.0;
    let galley =
        ui.painter()
            .layout_no_wrap(uri.clone(), egui::FontId::monospace(12.5), palette.text);
    let (chip, _) = ui.allocate_exact_size(galley.size() + vec2(22.0, 14.0), Sense::hover());
    ui.painter().rect_filled(chip, 8.0, palette.surface);
    ui.painter()
        .galley(chip.center() - galley.size() / 2.0, galley, palette.text);
    let (icon, label) = if copied {
        // Translators: shown on the Copy button for a moment after it copied.
        (Icon::Check, gettext(locale, "Copied"))
    } else {
        (Icon::Copy, gettext(locale, "Copy"))
    };
    let clicked = if compact {
        icon_pill(ui, palette, icon, &label).clicked()
    } else {
        soft_pill(ui, palette, Some(icon), &label).clicked()
    };
    if clicked {
        ui.ctx().copy_text(uri);
        ui.data_mut(|data| data.insert_temp(copied_id, now));
    }
    if copied {
        ui.ctx()
            .request_repaint_after(std::time::Duration::from_millis(500));
    }
    clicked
}

/// What Spotify's refusal means, and what to do about it.
fn problem_text(locale: Locale, problem: SignInProblem) -> String {
    match problem {
        SignInProblem::MalformedClientId => gettext(
            locale,
            "That doesn't look like a Client ID. It is 32 letters and digits, on your app's page in the dashboard.",
        ),
        SignInProblem::UnknownClient => gettext(
            locale,
            "Spotify doesn't know this Client ID. Copy it again from your app's page in the dashboard.",
        ),
        SignInProblem::AccountNotAllowed => gettext(
            locale,
            "Spotify didn't let this account use the app. Sign in with the Spotify account that made it, which needs Premium.",
        ),
        SignInProblem::SharedRetired => gettext(
            locale,
            "Spotify no longer accepts the shared sign-in Spotsie used before. Make your own app to keep listening.",
        ),
        SignInProblem::Expired => gettext(locale, "Your Spotify sign-in expired. Please sign in again."),
    }
    .into_owned()
}

/// Step 3: the Client ID and Sign in. Someone back with a saved app starts
/// here.
fn client_id(app: &mut App, ui: &mut egui::Ui) {
    let palette = app.palette;
    let locale = app.locale;
    let width = column(ui, 440.0);
    let returning = saved_app(app) && !app.onboarding.done.iter().any(|done| *done);
    let (title, subtitle) = if returning {
        (
            gettext(locale, "Welcome back"),
            gettext(
                locale,
                "Sign in through your Spotify app to pick up where you left off.",
            ),
        )
    } else {
        (
            gettext(locale, "Paste your Client ID"),
            gettext(
                locale,
                "It's on your app's page in the dashboard, under Basic Information.",
            ),
        )
    };
    if returning {
        let (rect, _) = ui.allocate_exact_size(vec2(width, 96.0), Sense::hover());
        spinning_record(ui, &palette, "returning", rect, 80.0, 1.0);
        ui.add_space(14.0);
    }
    theme::text(ui, title, theme::brand(30.0), palette.text);
    ui.add_space(6.0);
    wrapped(ui, subtitle, theme::regular(14.5), palette.secondary, width);
    ui.add_space(22.0);
    let problem = match &app.auth {
        AuthStatus::Failed(message) => Some(message.clone()),
        AuthStatus::Refused(problem) => Some(problem_text(locale, *problem)),
        _ => None,
    };
    let now = ui.input(|input| input.time);
    match (&problem, &app.onboarding.problem) {
        (Some(text), Some((shown, _))) if text == shown => {}
        (Some(text), _) => app.onboarding.problem = Some((text.clone(), now)),
        (None, _) => app.onboarding.problem = None,
    }
    if let Some((text, at)) = app.onboarding.problem.clone() {
        let arrive = motion::appear(ui.ctx(), at, 0.0, motion::EMPHASIS);
        ui.scope(|ui| {
            ui.multiply_opacity(arrive);
            callout(ui, &palette, &text, width);
        });
        ui.add_space(14.0 + (1.0 - arrive) * 6.0);
    }
    let typed = client_id_field(app, ui, width);
    let ready = crate::auth::looks_like_client_id(&typed);
    ui.add_space(18.0);
    let sign_in = primary_button(
        ui,
        &palette,
        "onboarding-sign-in",
        &gettext(locale, "Sign in with Spotify"),
        width.min(320.0),
        ready,
    );
    let enter = ready && ui.is_enabled() && ui.input(|input| input.key_pressed(egui::Key::Enter));
    if sign_in || enter {
        app.actions.push(Action::SignIn);
    }
    ui.add_space(12.0);
    wrapped(
        ui,
        gettext(
            locale,
            "Spotify opens in your browser to sign you in. Spotsie never sees your password.",
        ),
        theme::regular(12.5),
        palette.dim,
        width,
    );
    ui.add_space(18.0);
    let back_label = if returning {
        gettext(locale, "Set up a different app")
    } else {
        gettext(locale, "Back to the steps")
    };
    ui.horizontal(|ui| {
        let text_width = ui
            .painter()
            .layout_no_wrap(back_label.to_string(), theme::medium(13.0), palette.text)
            .size()
            .x;
        ui.add_space(((ui.available_width() - text_width - 22.0) / 2.0).max(0.0));
        if back_link(ui, &palette, &back_label) {
            app.onboarding.step = Some(Step::MakeApp);
        }
    });
    ui.add_space(8.0);
    proxy_link(app, ui);
}

/// The Client ID field: what is typed is saved as Settings saves it, and it
/// tells as it is typed whether it looks right. Returns what it holds.
fn client_id_field(app: &mut App, ui: &mut egui::Ui, width: f32) -> String {
    let palette = app.palette;
    let locale = app.locale;
    let ctx = ui.ctx().clone();
    let mut client_id = app.settings.web_client_id.clone().unwrap_or_default();
    let (rect, _) = ui.allocate_exact_size(vec2(width, 52.0), Sense::hover());
    let edit_id = Id::new("login-client-id");
    let focused = ctx.memory(|memory| memory.has_focus(edit_id));
    let focus = motion::toggle(&ctx, edit_id.with("focus"), focused, motion::STANDARD);
    let typed = client_id.trim().to_string();
    let valid = crate::auth::looks_like_client_id(&typed);
    let wrong = !typed.is_empty() && !valid;
    let ok = motion::toggle(&ctx, edit_id.with("ok"), valid, motion::STANDARD);
    let bad = motion::toggle(
        &ctx,
        edit_id.with("bad"),
        wrong && !focused,
        motion::STANDARD,
    );
    let edge = motion::lerp_color(
        motion::lerp_color(palette.outline, palette.accent, focus.max(ok)),
        palette.danger,
        bad,
    );
    ui.painter().rect(
        rect,
        CornerRadius::same(12),
        palette.surface,
        Stroke::new(1.0 + 0.5 * focus, edge),
        egui::StrokeKind::Inside,
    );
    let icon_rect =
        Rect::from_center_size(pos2(rect.left() + 24.0, rect.center().y), Vec2::splat(18.0));
    theme::paint_icon(
        ui,
        Icon::Lock,
        icon_rect,
        16.0,
        motion::lerp_color(palette.dim, palette.accent, focus),
    );
    let edit_rect = Rect::from_min_max(
        pos2(rect.left() + 44.0, rect.top()),
        pos2(rect.right() - 44.0, rect.bottom()),
    );
    let mut edit_ui = ui.new_child(
        egui::UiBuilder::new()
            .max_rect(edit_rect)
            .layout(Layout::left_to_right(Align::Center)),
    );
    let response = super::widgets::text_edit(
        &mut edit_ui,
        locale,
        egui::TextEdit::singleline(&mut client_id)
            .id(edit_id)
            .hint_text(
                egui::RichText::new(gettext(locale, "Client ID"))
                    .font(theme::regular(15.0))
                    .color(palette.dim),
            )
            .font(egui::FontId::monospace(15.0))
            .text_color(palette.text)
            .frame(egui::Frame::NONE)
            .desired_width(edit_rect.width()),
    );
    if response.changed() {
        let trimmed = client_id.trim().to_string();
        app.settings.web_client_id = (!trimmed.is_empty()).then_some(trimmed);
        app.mark_settings_dirty();
    }
    // A check grows in once it looks like a Client ID.
    if ok > 0.01 {
        let center = pos2(rect.right() - 24.0, rect.center().y);
        ui.painter().circle_filled(
            center,
            10.0 * (0.6 + 0.4 * motion::ease_out_back(ok)),
            motion::with_alpha(palette.accent, ok),
        );
        let mut icon_ui = ui.new_child(
            egui::UiBuilder::new().max_rect(Rect::from_center_size(center, Vec2::splat(20.0))),
        );
        icon_ui.set_opacity(ok);
        theme::paint_icon(
            &icon_ui,
            Icon::Check,
            Rect::from_center_size(center, Vec2::splat(20.0)),
            12.0,
            palette.on_accent,
        );
    }
    let note = motion::toggle(&ctx, edit_id.with("note"), wrong, motion::STANDARD);
    if note > 0.01 {
        ui.add_space(8.0 * note);
        ui.scope(|ui| {
            ui.multiply_opacity(note);
            wrapped(
                ui,
                gettext(
                    locale,
                    "A Client ID is 32 letters and digits. Copy it from your app's page.",
                ),
                theme::regular(12.5),
                palette.danger,
                width,
            );
        });
    }
    client_id.trim().to_string()
}

/// The browser has Spotify's page, or Spotsie is connecting.
fn waiting_pane(app: &mut App, ui: &mut egui::Ui, connecting: bool) {
    let palette = app.palette;
    let locale = app.locale;
    let ctx = ui.ctx().clone();
    let width = column(ui, 460.0);
    let (rect, _) = ui.allocate_exact_size(vec2(width, 150.0), Sense::hover());
    listening(ui, &palette, rect.center());
    ui.add_space(16.0);
    if connecting {
        theme::text(
            ui,
            gettext(locale, "Connecting to Spotify…"),
            theme::brand(28.0),
            palette.text,
        );
        ui.add_space(6.0);
        wrapped(
            ui,
            gettext(locale, "Getting your music ready."),
            theme::regular(14.5),
            palette.secondary,
            width,
        );
        return;
    }
    theme::text(
        ui,
        gettext(locale, "Approve Spotsie in your browser"),
        theme::brand(28.0),
        palette.text,
    );
    ui.add_space(6.0);
    wrapped(
        ui,
        gettext(
            locale,
            "Agree on Spotify's page, and Spotsie carries on by itself.",
        ),
        theme::regular(14.5),
        palette.secondary,
        width,
    );
    ui.add_space(10.0);
    if let AuthStatus::WaitingForBrowser { url } = &app.auth {
        let url = url.clone();
        if theme::link(
            ui,
            gettext(locale, "Didn't open? Open the sign-in page again"),
            theme::medium(13.0),
            palette.accent,
        )
        .clicked()
        {
            ctx.open_url(egui::OpenUrl::new_tab(url));
        }
    }
    ui.add_space(20.0);
    // Spotify stops on its own page for this and never tells the app. An
    // unknown Client ID was ruled out before the browser opened.
    let note = gettext(
        locale,
        "If Spotify says “Invalid redirect URI”, add the address below to your app exactly, save, and sign in again.",
    );
    let note_width = width.min(420.0);
    egui::Frame::new()
        .fill(motion::with_alpha(palette.panel, 0.9))
        .stroke(Stroke::new(1.0, palette.outline))
        .corner_radius(CornerRadius::same(theme::RADIUS_CARD))
        .inner_margin(egui::Margin::same(16))
        .show(ui, |ui| {
            ui.set_width(note_width - 32.0);
            ui.vertical_centered(|ui| {
                wrapped(
                    ui,
                    note,
                    theme::regular(13.0),
                    palette.secondary,
                    note_width - 32.0,
                );
                ui.add_space(10.0);
                ui.horizontal(|ui| {
                    let uri = crate::auth::personal_redirect_uri();
                    let chip = ui
                        .painter()
                        .layout_no_wrap(uri, egui::FontId::monospace(12.5), palette.text)
                        .size()
                        .x
                        + 22.0;
                    let copy = button_width(ui, &gettext(locale, "Copied")) + 18.0;
                    ui.add_space(((ui.available_width() - chip - copy - 8.0) / 2.0).max(0.0));
                    redirect_chip(
                        ui,
                        &palette,
                        locale,
                        "onboarding-copy-redirect-waiting",
                        false,
                    );
                });
            });
        });
    ui.add_space(20.0);
    if theme::pill_button(ui, &palette, &gettext(locale, "Cancel"), false).clicked() {
        app.actions.push(Action::CancelSignIn);
    }
}

/// The logo with rings spreading out from it, slowly, while Spotify answers.
fn listening(ui: &egui::Ui, palette: &Palette, center: Pos2) {
    let time = if motion::enabled() {
        ui.ctx().request_repaint();
        ui.input(|input| input.time) as f32
    } else {
        0.6
    };
    for ring in 0..3 {
        let phase = (time * 0.45 + ring as f32 / 3.0).fract();
        let radius = 40.0 + 34.0 * motion::ease_out(phase);
        ui.painter().circle_stroke(
            center,
            radius,
            Stroke::new(
                1.5,
                motion::with_alpha(palette.accent, 0.45 * (1.0 - phase)),
            ),
        );
    }
    let hovered = ui.rect_contains_pointer(Rect::from_center_size(center, Vec2::splat(72.0)));
    theme::logo_turned(ui, center, 72.0, record_angle(ui.ctx(), "waiting", hovered));
}

/// A problem Spotify reported, in a soft red box.
fn callout(ui: &mut egui::Ui, palette: &Palette, text: &str, width: f32) {
    egui::Frame::new()
        .fill(motion::with_alpha(palette.danger, 0.12))
        .stroke(Stroke::new(1.0, motion::with_alpha(palette.danger, 0.4)))
        .corner_radius(CornerRadius::same(12))
        .inner_margin(egui::Margin::symmetric(14, 12))
        .show(ui, |ui| {
            ui.set_width(width - 28.0);
            ui.horizontal_top(|ui| {
                ui.spacing_mut().item_spacing.x = 10.0;
                let (icon, _) = ui.allocate_exact_size(Vec2::splat(18.0), Sense::hover());
                theme::paint_icon(ui, Icon::CircleAlert, icon, 16.0, palette.danger);
                ui.add(
                    egui::Label::new(
                        egui::RichText::new(text)
                            .font(theme::regular(13.0))
                            .color(palette.text),
                    )
                    .wrap(),
                );
            });
        });
}

fn text_focused(ui: &egui::Ui) -> bool {
    ui.ctx().memory(|memory| memory.focused().is_some())
}

/// The big accent button, lit on hover with a soft glow. Dimmed and inert
/// when `enabled` is false.
fn primary_button(
    ui: &mut egui::Ui,
    palette: &Palette,
    salt: &str,
    label: &str,
    width: f32,
    enabled: bool,
) -> bool {
    let size = vec2(width, 48.0);
    let (rect, response) = ui.allocate_exact_size(
        size,
        if enabled {
            Sense::click()
        } else {
            Sense::hover()
        },
    );
    response.widget_info(|| {
        egui::WidgetInfo::labeled(egui::WidgetType::Button, ui.is_enabled() && enabled, label)
    });
    let ctx = ui.ctx();
    let id = Id::new(salt);
    let hover = motion::toggle(
        ctx,
        id.with("hover"),
        enabled && response.hovered(),
        motion::MICRO,
    );
    let press = motion::toggle(
        ctx,
        id.with("press"),
        enabled && response.is_pointer_button_down_on(),
        0.08,
    );
    let live = motion::toggle(ctx, id.with("enabled"), enabled, motion::STANDARD);
    let rect = Rect::from_center_size(rect.center(), rect.size() * (1.0 - 0.02 * press));
    if hover > 0.01 {
        ui.painter().add(
            egui::epaint::Shadow {
                offset: [0, 6],
                blur: 22,
                spread: 0,
                color: motion::with_alpha(palette.accent, 0.35 * hover),
            }
            .as_shape(rect, CornerRadius::same(24)),
        );
    }
    let fill = motion::lerp_color(palette.accent, palette.accent_hover, hover);
    // Waiting for input it is a quiet outline, so it reads as not yet.
    let idle = super::blend(palette.surface, palette.accent, 0.08);
    let fill = motion::lerp_color(idle, fill, live);
    ui.painter().rect_filled(rect, 24.0, fill);
    if live < 0.99 {
        ui.painter().rect_stroke(
            rect,
            24.0,
            Stroke::new(1.0, motion::with_alpha(palette.outline, 1.0 - live)),
            egui::StrokeKind::Inside,
        );
    }
    let text = motion::lerp_color(palette.dim, palette.on_accent, live);
    let galley = ui
        .painter()
        .layout_no_wrap(label.to_string(), theme::bold(15.0), text);
    ui.painter()
        .galley(rect.center() - galley.size() / 2.0, galley, text);
    theme::focus_ring(ui, &response);
    enabled
        && response
            .on_hover_cursor(egui::CursorIcon::PointingHand)
            .clicked()
}

fn button_width(ui: &egui::Ui, label: &str) -> f32 {
    ui.painter()
        .layout_no_wrap(label.to_string(), theme::bold(15.0), Color32::WHITE)
        .size()
        .x
        + 56.0
}

/// A small rounded button in the surface colour, with an optional icon.
/// How wide a [`soft_pill`] with this label is.
fn soft_pill_width(ui: &egui::Ui, icon: bool, label: &str) -> f32 {
    ui.painter()
        .layout_no_wrap(label.to_string(), theme::semibold(13.0), Color32::WHITE)
        .size()
        .x
        + if icon { 20.0 } else { 0.0 }
        + 28.0
}

fn soft_pill(
    ui: &mut egui::Ui,
    palette: &Palette,
    icon: Option<Icon>,
    label: &str,
) -> egui::Response {
    let font = theme::semibold(13.0);
    let galley = ui
        .painter()
        .layout_no_wrap(label.to_string(), font, palette.text);
    let icon_room = if icon.is_some() { 20.0 } else { 0.0 };
    let size = vec2(galley.size().x + icon_room + 28.0, 32.0);
    let (rect, response) = ui.allocate_exact_size(size, Sense::click());
    response.widget_info(|| {
        egui::WidgetInfo::labeled(egui::WidgetType::Button, ui.is_enabled(), label)
    });
    let hover = motion::toggle(
        ui.ctx(),
        response.id.with("hover"),
        response.hovered(),
        motion::MICRO,
    );
    let fill = motion::lerp_color(palette.surface, palette.surface_hover, hover);
    ui.painter().rect_filled(rect, 16.0, fill);
    let mut x = rect.left() + 14.0;
    if let Some(icon) = icon {
        let icon_rect = Rect::from_center_size(pos2(x + 7.0, rect.center().y), Vec2::splat(14.0));
        theme::paint_icon(ui, icon, icon_rect, 14.0, palette.text);
        x += icon_room;
    }
    ui.painter().galley(
        pos2(x, rect.center().y - galley.size().y / 2.0),
        galley,
        palette.text,
    );
    theme::focus_ring(ui, &response);
    response.on_hover_cursor(egui::CursorIcon::PointingHand)
}

/// A round button holding just an icon, named by `label` for tooltips and
/// screen readers.
fn icon_pill(ui: &mut egui::Ui, palette: &Palette, icon: Icon, label: &str) -> egui::Response {
    let (rect, response) = ui.allocate_exact_size(Vec2::splat(32.0), Sense::click());
    response.widget_info(|| {
        egui::WidgetInfo::labeled(egui::WidgetType::Button, ui.is_enabled(), label)
    });
    let hover = motion::toggle(
        ui.ctx(),
        response.id.with("hover"),
        response.hovered(),
        motion::MICRO,
    );
    ui.painter().circle_filled(
        rect.center(),
        16.0,
        motion::lerp_color(palette.surface, palette.surface_hover, hover),
    );
    theme::paint_icon(ui, icon, rect, 14.0, palette.text);
    theme::focus_ring(ui, &response);
    response
        .on_hover_cursor(egui::CursorIcon::PointingHand)
        .on_hover_text(label)
}

/// An arrow and a label in the secondary colour, brightening on hover.
fn back_link(ui: &mut egui::Ui, palette: &Palette, label: &str) -> bool {
    let font = theme::medium(13.0);
    let galley = ui
        .painter()
        .layout_no_wrap(label.to_string(), font, palette.secondary);
    let size = vec2(galley.size().x + 22.0, 32.0);
    let (rect, response) = ui.allocate_exact_size(size, Sense::click());
    response.widget_info(|| {
        egui::WidgetInfo::labeled(egui::WidgetType::Button, ui.is_enabled(), label)
    });
    let hover = motion::toggle(
        ui.ctx(),
        response.id.with("hover"),
        response.hovered(),
        motion::MICRO,
    );
    let color = motion::lerp_color(palette.secondary, palette.text, hover);
    let arrow = Rect::from_center_size(
        pos2(rect.left() + 7.0 - 2.0 * hover, rect.center().y),
        Vec2::splat(14.0),
    );
    theme::paint_icon(ui, Icon::ArrowLeft, arrow, 14.0, color);
    ui.painter().text(
        pos2(rect.left() + 22.0, rect.center().y),
        Align2::LEFT_CENTER,
        label,
        theme::medium(13.0),
        color,
    );
    theme::focus_ring(ui, &response);
    response
        .on_hover_cursor(egui::CursorIcon::PointingHand)
        .clicked()
}

/// Proxy Settings, under the welcome and the Client ID, opening the fields
/// beneath it.
fn proxy_link(app: &mut App, ui: &mut egui::Ui) {
    let palette = app.palette;
    let proxy_id = Id::new("login-proxy-open");
    let mut open = ui
        .data(|data| data.get_temp::<bool>(proxy_id))
        .unwrap_or(false);
    if theme::link(
        ui,
        gettext(app.locale, "Proxy Settings"),
        theme::regular(13.0),
        palette.secondary,
    )
    .clicked()
    {
        open = !open;
    }
    if open {
        ui.add_space(8.0);
        let width = column(ui, 440.0);
        ui.scope(|ui| {
            ui.set_max_width(width);
            proxy_fields(ui, app);
        });
    }
    ui.data_mut(|data| data.insert_temp(proxy_id, open));
}

fn proxy_fields(ui: &mut egui::Ui, app: &mut App) {
    let palette = app.palette;
    let mut changed = false;
    let mut apply = false;
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 6.0;
        let row_width: f32 = ProxyMode::ALL
            .iter()
            .map(|choice| {
                let galley = ui.painter().layout_no_wrap(
                    choice.label(app.locale).into_owned(),
                    theme::medium(13.0),
                    palette.text,
                );
                galley.size().x + 24.0
            })
            .sum::<f32>()
            + 6.0 * (ProxyMode::ALL.len() - 1) as f32;
        ui.add_space((ui.available_width() - row_width).max(0.0) / 2.0);
        for choice in ProxyMode::ALL {
            if theme::soft_button(
                ui,
                &palette,
                None,
                &choice.label(app.locale),
                app.settings.proxy_mode == choice,
            )
            .clicked()
                && app.settings.proxy_mode != choice
            {
                app.settings.proxy_mode = choice;
                changed = true;
                apply = !choice.is_manual();
            }
        }
    });
    if app.settings.proxy_mode.is_manual() {
        ui.add_space(10.0);
        if super::widgets::proxy_manual_form(
            ui,
            &palette,
            app.locale,
            &mut app.settings.proxy_host,
            &mut app.settings.proxy_port,
            &mut app.settings.proxy_username,
            &mut app.settings.proxy_password,
        ) {
            changed = true;
        }
        ui.add_space(6.0);
        super::widgets::proxy_scope_note(ui, &palette, app.locale, app.settings.proxy_mode);
        ui.add_space(8.0);
        ui.horizontal(|ui| {
            if theme::pill_button(ui, &palette, &gettext(app.locale, "Apply settings"), true)
                .clicked()
            {
                apply = true;
            }
        });
    }
    if changed {
        app.actions.push(Action::ProxyEdited);
        app.mark_settings_dirty();
    }
    if apply {
        app.actions.push(Action::ApplyProxy);
    }
}

/// A soft round glow, brightest in the middle, with no visible edge.
fn radial_glow(painter: &egui::Painter, center: Pos2, radius: f32, color: Color32) {
    use egui::epaint::Mesh;
    const RINGS: u32 = 24;
    const SEGMENTS: u32 = 64;
    let mut mesh = Mesh::default();
    mesh.colored_vertex(center, color);
    for ring in 1..=RINGS {
        let k = ring as f32 / RINGS as f32;
        let alpha = (1.0 - k).powf(2.2);
        for segment in 0..SEGMENTS {
            let angle = segment as f32 / SEGMENTS as f32 * std::f32::consts::TAU;
            mesh.colored_vertex(
                center + vec2(angle.cos(), angle.sin()) * radius * k,
                color.gamma_multiply(alpha),
            );
        }
    }
    for segment in 0..SEGMENTS {
        let next = (segment + 1) % SEGMENTS;
        mesh.add_triangle(0, 1 + segment, 1 + next);
    }
    for ring in 0..RINGS - 1 {
        let a = 1 + ring * SEGMENTS;
        let b = a + SEGMENTS;
        for segment in 0..SEGMENTS {
            let next = (segment + 1) % SEGMENTS;
            mesh.add_triangle(a + segment, b + segment, b + next);
            mesh.add_triangle(a + segment, b + next, a + next);
        }
    }
    painter.add(egui::Shape::mesh(mesh));
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{app::AppOptions, backend::Waker, paths::AppDirs, settings::Settings};

    fn test_app(name: &str, settings: Settings) -> App {
        let root = std::env::temp_dir().join(format!("spotsie-{name}-{}", std::process::id()));
        App::new(
            &Waker::default(),
            AppDirs {
                config: root.join("config"),
                state: root.join("state"),
                cache: root.join("cache"),
            },
            settings,
            AppOptions {
                media_controls: false,
                restore_sign_in: false,
                tray: false,
            },
        )
    }

    fn accessible(
        ctx: &egui::Context,
        app: &mut App,
        screen: Rect,
        events: Vec<egui::Event>,
    ) -> egui::accesskit::TreeUpdate {
        let mut output = ctx.run_ui(
            egui::RawInput {
                screen_rect: Some(screen),
                events,
                ..Default::default()
            },
            |ui| show(app, ui, false),
        );
        output.textures_delta.clear();
        output.platform_output.accesskit_update.unwrap()
    }

    fn bounds(tree: &egui::accesskit::TreeUpdate, label: &str) -> Option<egui::accesskit::Rect> {
        tree.nodes
            .iter()
            .find(|(_, node)| {
                node.label() == Some(label) && node.role() == egui::accesskit::Role::Button
            })
            .and_then(|(_, node)| node.bounds())
    }

    /// Someone new is welcomed; someone back with an app goes straight to
    /// signing in; someone whose shared sign-in Spotify retired goes to the
    /// steps that replace it.
    #[test]
    fn onboarding_starts_where_the_listener_left_off() {
        let mut app = test_app("onboarding-first", Settings::default());
        app.auth = AuthStatus::SignedOut;
        assert_eq!(first_step(&app), Step::Welcome);
        app.settings.web_client_id = Some("0123456789abcdef0123456789abcdef".into());
        assert_eq!(first_step(&app), Step::ClientId);
        app.settings.web_client_id = Some("not an id".into());
        assert_eq!(first_step(&app), Step::Welcome);
        app.auth = AuthStatus::Refused(SignInProblem::SharedRetired);
        assert_eq!(first_step(&app), Step::MakeApp);
    }

    /// Get started leads to the steps, and their Next to the Client ID,
    /// whose Sign in only works once what was typed can be a Client ID.
    #[test]
    fn the_steps_lead_to_a_sign_in_that_waits_for_a_client_id() {
        let ctx = egui::Context::default();
        ctx.enable_accesskit();
        theme::install(&ctx);
        let mut app = test_app("onboarding-walk", Settings::default());
        app.auth = AuthStatus::SignedOut;
        let screen = Rect::from_min_size(Pos2::ZERO, vec2(1000.0, 700.0));
        let click = |tree: &egui::accesskit::TreeUpdate, label: &str| {
            let at = bounds(tree, label).unwrap_or_else(|| panic!("missing {label}"));
            let pos = pos2(
                ((at.x0 + at.x1) / 2.0) as f32,
                ((at.y0 + at.y1) / 2.0) as f32,
            );
            vec![
                egui::Event::PointerMoved(pos),
                egui::Event::PointerButton {
                    pos,
                    button: egui::PointerButton::Primary,
                    pressed: true,
                    modifiers: egui::Modifiers::NONE,
                },
                egui::Event::PointerButton {
                    pos,
                    button: egui::PointerButton::Primary,
                    pressed: false,
                    modifiers: egui::Modifiers::NONE,
                },
            ]
        };
        let tree = accessible(&ctx, &mut app, screen, vec![]);
        let tree = accessible(&ctx, &mut app, screen, click(&tree, "Get started"));
        accessible(&ctx, &mut app, screen, vec![]);
        assert_eq!(app.onboarding.step, Some(Step::MakeApp));
        let _ = tree;
        let tree = accessible(&ctx, &mut app, screen, vec![]);
        accessible(&ctx, &mut app, screen, click(&tree, "Continue"));
        assert_eq!(app.onboarding.step, Some(Step::ClientId));
        let tree = accessible(&ctx, &mut app, screen, vec![]);
        accessible(&ctx, &mut app, screen, click(&tree, "Sign in with Spotify"));
        assert!(
            !app.actions
                .iter()
                .any(|action| matches!(action, Action::SignIn))
        );
        app.settings.web_client_id = Some("0123456789abcdef0123456789abcdef".into());
        let tree = accessible(&ctx, &mut app, screen, vec![]);
        accessible(&ctx, &mut app, screen, click(&tree, "Sign in with Spotify"));
        assert!(
            app.actions
                .iter()
                .any(|action| matches!(action, Action::SignIn))
        );
    }

    #[test]
    fn expanded_proxy_settings_remain_reachable_in_a_short_window() {
        let ctx = egui::Context::default();
        ctx.enable_accesskit();
        theme::install(&ctx);
        let mut app = test_app(
            "proxy-short-login",
            Settings {
                proxy_mode: ProxyMode::Http,
                ..Default::default()
            },
        );
        app.auth = AuthStatus::SignedOut;
        app.onboarding = Onboarding::at(Step::ClientId);
        ctx.data_mut(|data| data.insert_temp(Id::new("login-proxy-open"), true));
        let screen = Rect::from_min_size(Pos2::ZERO, vec2(760.0, 520.0));
        let first = accessible(&ctx, &mut app, screen, vec![]);
        assert!(bounds(&first, "Sign in with Spotify").is_some());
        accessible(
            &ctx,
            &mut app,
            screen,
            vec![
                egui::Event::PointerMoved(pos2(380.0, 260.0)),
                egui::Event::MouseWheel {
                    unit: egui::MouseWheelUnit::Point,
                    phase: egui::TouchPhase::Move,
                    delta: vec2(0.0, -2000.0),
                    modifiers: egui::Modifiers::NONE,
                },
            ],
        );
        for _ in 0..30 {
            accessible(&ctx, &mut app, screen, vec![]);
        }
        let apply = bounds(
            &accessible(&ctx, &mut app, screen, vec![]),
            "Apply settings",
        )
        .expect("Apply is laid out");
        assert!(
            apply.y0 >= 0.0 && apply.y1 <= 520.0 - BOTTOM as f64 + 1.0,
            "scrolling must reach Apply above the version line: {apply:?}"
        );
    }
}

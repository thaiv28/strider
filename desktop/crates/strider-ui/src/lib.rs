//! Topo primitives. Color tokens are generated from the web application's stylesheet.
pub mod categories;
pub use categories::category_color;
mod controls;
pub use controls::*;
use gpui::{prelude::*, *};
use std::{borrow::Cow, sync::Arc};
include!(concat!(env!("OUT_DIR"), "/theme.rs"));
impl Global for Theme {}
impl Theme {
    pub fn get(cx: &App) -> Self {
        *cx.global::<Self>()
    }
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Appearance {
    Light,
    Dark,
    #[default]
    System,
}
impl Global for Appearance {}

/// Register the embedded fonts and initialize both theme globals after gpui_component::init.
pub fn initialize(cx: &mut App) {
    cx.text_system()
        .add_fonts(vec![
            Cow::Borrowed(include_bytes!(
                "../../../assets/fonts/InstrumentSans-Regular.ttf"
            )),
            Cow::Borrowed(include_bytes!(
                "../../../assets/fonts/InstrumentSans-Medium.ttf"
            )),
            Cow::Borrowed(include_bytes!(
                "../../../assets/fonts/InstrumentSans-SemiBold.ttf"
            )),
            Cow::Borrowed(include_bytes!(
                "../../../assets/fonts/InstrumentSans-Bold.ttf"
            )),
            Cow::Borrowed(include_bytes!(
                "../../../assets/fonts/BricolageGrotesque-Bold.ttf"
            )),
        ])
        .expect("embedded Strider fonts must register");
    cx.set_global(Theme::light());
    cx.set_global(Appearance::System);
    sync_component_theme(Theme::light(), false, cx);
}
fn sync_component_theme(theme: Theme, dark: bool, cx: &mut App) {
    gpui_component::Theme::change(
        if dark {
            gpui_component::ThemeMode::Dark
        } else {
            gpui_component::ThemeMode::Light
        },
        None,
        cx,
    );
    gpui_component::Theme::update(cx, |component| {
        component.font_family = "Instrument Sans".into();
        component.font_size = px(14.);
        component.radius = px(10.);
        component.colors.caret = theme.ink;
        component.colors.ring = theme.accent.opacity(0.6);
        component.colors.selection = theme.accent.opacity(0.25);
        component.colors.background = theme.canvas;
        component.colors.foreground = theme.ink;
        component.colors.input = theme.field_border;
        component.colors.border = theme.field_border;
        component.colors.primary = theme.accent;
        component.colors.primary_foreground = theme.accent_ink;
        component.colors.muted = theme.panel2;
        component.colors.muted_foreground = theme.muted;
    });
}
pub fn set_appearance(appearance: Appearance, window: &mut Window, cx: &mut App) {
    let dark = match appearance {
        Appearance::Light => false,
        Appearance::Dark => true,
        Appearance::System => matches!(
            window.appearance(),
            WindowAppearance::Dark | WindowAppearance::VibrantDark
        ),
    };
    let theme = if dark { Theme::dark() } else { Theme::light() };
    cx.set_global(appearance);
    cx.set_global(theme);
    sync_component_theme(theme, dark, cx);
    window.refresh();
}
/// Keep this subscription alive for the lifetime of the window.
pub fn observe_appearance(window: &Window) -> Subscription {
    window.observe_window_appearance(|window, cx| {
        if *cx.global::<Appearance>() == Appearance::System {
            set_appearance(Appearance::System, window, cx);
        }
    })
}
/// Typography helpers keep font families out of application code.
pub trait TopoStyle: Styled + Sized {
    fn body(self) -> Self {
        self.font_family("Instrument Sans").text_size(px(14.))
    }
    fn page_title(self) -> Self {
        self.font_family("Bricolage Grotesque")
            .text_size(px(44.))
            .font_weight(FontWeight::BOLD)
    }
    fn sheet_title(self) -> Self {
        self.font_family("Bricolage Grotesque")
            .text_size(px(26.))
            .font_weight(FontWeight::BOLD)
    }
    fn section_heading(self) -> Self {
        self.font_family("Bricolage Grotesque")
            .text_size(px(22.))
            .font_weight(FontWeight::BOLD)
    }
    fn card_heading(self) -> Self {
        self.text_size(px(17.)).font_weight(FontWeight::BOLD)
    }
    fn field_label(self, theme: Theme) -> Self {
        self.text_size(px(13.))
            .font_weight(FontWeight::SEMIBOLD)
            .text_color(theme.muted)
    }
    fn eyebrow(self, theme: Theme) -> Self {
        self.field_label(theme)
    }
    fn meta(self, theme: Theme) -> Self {
        self.text_size(px(12.)).text_color(theme.muted)
    }
    fn readout(self) -> Self {
        self.font_features(FontFeatures(Arc::new(vec![("tnum".into(), 1)])))
    }
}
impl<T: Styled> TopoStyle for T {}
pub fn card(theme: Theme) -> Div {
    div().bg(theme.panel).rounded(px(14.)).overflow_hidden()
}
#[derive(Clone, Copy)]
pub enum ButtonVariant {
    Primary,
    Outline,
    Ghost,
    Danger,
    Header,
    HeaderOutline,
}
pub fn button(
    id: impl Into<ElementId>,
    label: impl Into<SharedString>,
    variant: ButtonVariant,
    theme: Theme,
) -> Stateful<Div> {
    let base = div()
        .id(id)
        .role(Role::Button)
        .focusable()
        .focus_visible(move |s| s.border_2().border_color(theme.accent))
        .flex()
        .items_center()
        .justify_center()
        .min_h(px(44.))
        .px_4()
        .py_1p5()
        .gap_1p5()
        .rounded(px(12.))
        .body()
        .font_weight(FontWeight::SEMIBOLD)
        .cursor_pointer()
        .child(label.into());
    match variant {
        ButtonVariant::Primary => base
            .bg(theme.accent)
            .text_color(theme.accent_ink)
            .hover(|s| s.opacity(0.9)),
        ButtonVariant::Outline => base
            .border_1()
            .border_color(theme.field_border)
            .bg(theme.panel)
            .text_color(theme.ink)
            .hover(move |s| s.bg(theme.panel2)),
        ButtonVariant::Ghost => base
            .text_color(theme.muted)
            .hover(move |s| s.bg(theme.panel2).text_color(theme.ink)),
        ButtonVariant::Danger => base
            .text_color(theme.signal)
            .hover(move |s| s.bg(theme.signal.opacity(0.1))),
        ButtonVariant::Header => base
            .bg(theme.header_ink)
            .text_color(theme.header_action_ink)
            .hover(|s| s.opacity(0.9)),
        ButtonVariant::HeaderOutline => base
            .border_1()
            .border_color(theme.header_border)
            .text_color(theme.header_ink)
            .hover(move |s| s.bg(theme.header_active)),
    }
}
#[derive(Clone, Copy)]
pub enum Icon {
    Grip,
    ChevronDown,
    MoreHorizontal,
    X,
    Search,
}
pub fn icon(kind: Icon, color: Hsla) -> impl IntoElement {
    canvas(
        |_, _, _| {},
        move |bounds, _, window, _| {
            let scale = f32::from(bounds.size.width) / 20.;
            let point_at = |x: f32, y: f32| {
                point(
                    bounds.origin.x + px(x * scale),
                    bounds.origin.y + px(y * scale),
                )
            };
            let mut path = PathBuilder::stroke(px(1.5));
            match kind {
                Icon::ChevronDown => {
                    path.move_to(point_at(5., 8.));
                    path.line_to(point_at(10., 13.));
                    path.line_to(point_at(15., 8.));
                }
                Icon::X => {
                    for (a, b, c, d) in [(5., 5., 15., 15.), (15., 5., 5., 15.)] {
                        path.move_to(point_at(a, b));
                        path.line_to(point_at(c, d));
                    }
                }
                Icon::MoreHorizontal | Icon::Grip => {
                    let xs: &[f32] = if matches!(kind, Icon::Grip) {
                        &[7., 13.]
                    } else {
                        &[4., 10., 16.]
                    };
                    let ys: &[f32] = if matches!(kind, Icon::Grip) {
                        &[5., 10., 15.]
                    } else {
                        &[10.]
                    };
                    for x in xs {
                        for y in ys {
                            path.move_to(point_at(*x - 0.7, *y));
                            path.line_to(point_at(*x + 0.7, *y));
                        }
                    }
                }
                Icon::Search => {
                    for index in 0..=24 {
                        let angle = index as f32 / 24. * std::f32::consts::TAU;
                        let p = point_at(8.5 + 5.5 * angle.cos(), 8.5 + 5.5 * angle.sin());
                        if index == 0 {
                            path.move_to(p);
                        } else {
                            path.line_to(p);
                        }
                    }
                    path.move_to(point_at(12.5, 12.5));
                    path.line_to(point_at(17., 17.));
                }
            }
            if let Ok(path) = path.build() {
                window.paint_path(path, color);
            }
        },
    )
    .size(px(20.))
}
pub fn icon_button(id: impl Into<ElementId>, kind: Icon, theme: Theme) -> Stateful<Div> {
    div()
        .id(id)
        .role(Role::Button)
        .focusable()
        .focus_visible(move |s| s.border_2().border_color(theme.accent))
        .aria_label(match kind {
            Icon::Grip => "Reorder gear",
            Icon::ChevronDown => "Expand kit",
            Icon::MoreHorizontal => "Edit gear",
            Icon::X => "Close",
            Icon::Search => "Search",
        })
        .flex()
        .items_center()
        .justify_center()
        .size(px(44.))
        .flex_shrink_0()
        .rounded(px(12.))
        .cursor_pointer()
        .hover(move |s| s.bg(theme.panel2))
        .child(icon(kind, theme.muted))
}
pub fn chip(
    id: impl Into<ElementId>,
    label: impl Into<SharedString>,
    selected: bool,
    theme: Theme,
) -> Stateful<Div> {
    button(
        id,
        label,
        if selected {
            ButtonVariant::Primary
        } else {
            ButtonVariant::Outline
        },
        theme,
    )
}
pub fn page_tab(
    id: impl Into<ElementId>,
    label: impl Into<SharedString>,
    selected: bool,
    header_tone: bool,
    theme: Theme,
) -> Stateful<Div> {
    button(
        id,
        label,
        match (header_tone, selected) {
            (true, true) => ButtonVariant::Header,
            (true, false) => ButtonVariant::HeaderOutline,
            (false, true) => ButtonVariant::Primary,
            (false, false) => ButtonVariant::Outline,
        },
        theme,
    )
}
pub fn page_tabs(children: impl IntoIterator<Item = AnyElement>) -> Div {
    div().flex().gap_1p5().children(children)
}
pub fn field(label: impl Into<SharedString>, child: impl IntoElement, theme: Theme) -> Div {
    div()
        .flex()
        .flex_col()
        .gap_1p5()
        .child(div().field_label(theme).child(label.into()))
        .child(child)
}
pub fn page_header(
    title: impl Into<SharedString>,
    actions: AnyElement,
    summary: AnyElement,
    theme: Theme,
) -> Div {
    div()
        .relative()
        .flex_shrink_0()
        .bg(theme.header)
        .text_color(theme.header_ink)
        .overflow_hidden()
        .child(contour_band(theme))
        .child(
            div()
                .relative()
                .w_full()
                .max_w(px(1136.))
                .mx_auto()
                .px_6()
                .pb_16()
                .child(
                    div()
                        .flex()
                        .items_center()
                        .justify_between()
                        .gap_3()
                        .child(div().page_title().child(title.into()))
                        .child(actions),
                )
                .child(div().mt_3().child(summary)),
        )
}
pub fn page_body(children: impl IntoIterator<Item = AnyElement>, theme: Theme) -> Div {
    div()
        .relative()
        .w_full()
        .max_w(px(1136.))
        .mx_auto()
        .mt(px(-36.))
        .px_6()
        .pb_8()
        .text_color(theme.ink)
        .children(children)
}
/// The caller wires close and scrim events to the same close action and handles Escape.
pub fn sheet(
    title: impl Into<SharedString>,
    content: AnyElement,
    close: Stateful<Div>,
    theme: Theme,
) -> Div {
    div()
        .bg(theme.panel)
        .text_color(theme.ink)
        .rounded(px(20.))
        .w_full()
        .max_w(px(560.))
        .p_6()
        .pt_5()
        .flex()
        .flex_col()
        .gap_5()
        .child(
            div()
                .flex()
                .items_center()
                .justify_between()
                .child(div().sheet_title().child(title.into()))
                .child(close),
        )
        .child(content)
}
pub fn sheet_scrim(theme: Theme) -> Stateful<Div> {
    div()
        .id("sheet-scrim")
        .absolute()
        .inset_0()
        .flex()
        .items_center()
        .justify_center()
        .p_4()
        .bg(theme.scrim)
}
pub fn contour_band(theme: Theme) -> impl IntoElement {
    canvas(
        |_, _, _| {},
        move |bounds, _, window, _| {
            let width = f32::from(bounds.size.width);
            let height = f32::from(bounds.size.height);
            let scale = (width / 1280.).max(height / 200.);
            let offset_x = (width - 1280. * scale) / 2.;
            let offset_y = (height - 200. * scale) / 2.;
            let p = |x: f32, y: f32| {
                point(
                    bounds.origin.x + px(offset_x + x * scale),
                    bounds.origin.y + px(offset_y + y * scale),
                )
            };
            let mut path = PathBuilder::stroke(px(1.2 * scale));
            for i in 0..4 {
                let step = i as f32;
                let y = step * 50.;
                path.move_to(p(-10., 40. + y));
                path.cubic_bezier_to(
                    p(640. + 10. * step, 36. + y),
                    p(200. + 10. * step, 8. + y),
                    p(400. + 10. * step, 76. + y),
                );
                path.cubic_bezier_to(
                    p(1290., 36. + y),
                    p(880. + 10. * step, -4. + y),
                    p(1040. + 10. * step, 6. + y),
                );
            }
            if let Ok(path) = path.build() {
                window.paint_path(path, theme.header_line);
            }
        },
    )
    .absolute()
    .inset_0()
    .size_full()
}

pub mod route;
pub use route::{ElevationProfile, RouteClick, RouteMap, route_thumbnail};

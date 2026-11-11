//! `ActivityRail` — the row of view buttons above the sidebar of the main
//! window.

use std::rc::Rc;

use gpui::prelude::*;
use gpui::{App, ElementId, Hsla, Role, SharedString, Window, div};
use gpui_component::ActiveTheme;
use gpui_component::tooltip::Tooltip;

use super::shell_bar::{BellUrgency, NotificationBell, count_badge};
use crate::icon::IconSource;
use crate::primitives::{Chamfer, Icon};
use crate::tokens::{ChamferCut, ChromeColors, ShellMetrics};

/// Key context the rail sets on itself, so a keymap predicate can name it.
pub const ACTIVITY_RAIL_KEY_CONTEXT: &str = "ActivityRail";

/// Where an entry sits in the rail: in order from the left (`Top`), or
/// pinned to the right edge (`Bottom`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RailPlacement {
    Top,
    Bottom,
}

/// One button of the rail. The host builds the list from its own state; the
/// rail never knows which view an entry opens.
#[derive(Clone, Debug, PartialEq)]
pub struct RailEntry {
    /// Stable identifier, passed back to the selection handler and used as
    /// the element id (`rail-<id>`).
    pub id: SharedString,
    pub icon: IconSource,
    /// Tooltip and accessible name.
    pub label: SharedString,
    /// The view the entry opens is the one on screen.
    pub active: bool,
    pub placement: RailPlacement,
    /// Count shown in a badge over the button's top-right corner, with the
    /// urgency that colors it. A zero count shows no badge.
    pub count: Option<(usize, BellUrgency)>,
}

impl RailEntry {
    pub fn new(
        id: impl Into<SharedString>,
        icon: impl Into<IconSource>,
        label: impl Into<SharedString>,
    ) -> Self {
        Self {
            id: id.into(),
            icon: icon.into(),
            label: label.into(),
            active: false,
            placement: RailPlacement::Top,
            count: None,
        }
    }

    pub fn active(mut self, active: bool) -> Self {
        self.active = active;
        self
    }

    pub fn bottom(mut self) -> Self {
        self.placement = RailPlacement::Bottom;
        self
    }

    /// Shows `count` in a badge colored by `urgency`, as the notification
    /// bell does.
    pub fn count(mut self, count: usize, urgency: BellUrgency) -> Self {
        self.count = Some((count, urgency));
        self
    }

    /// Element id of the entry's button.
    pub fn element_id(&self) -> SharedString {
        format!("rail-{}", self.id).into()
    }

    /// Element id of the entry's count badge.
    pub fn count_element_id(&self) -> SharedString {
        format!("rail-{}-count", self.id).into()
    }
}

/// Fills and icon color of a rail button: the active entry takes a neutral
/// raised fill and the strong icon color, as VS Code's activity bar does; the
/// others are transparent with a muted icon and the ghost hover and press
/// fills.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RailButtonColors {
    pub rest: Hsla,
    pub hover: Hsla,
    pub pressed: Hsla,
    pub icon: Hsla,
}

pub fn rail_button_colors(theme: &gpui_component::Theme, active: bool) -> RailButtonColors {
    if active {
        return RailButtonColors {
            rest: theme.secondary,
            hover: theme.secondary_hover,
            pressed: theme.secondary_hover,
            icon: ChromeColors::strong(theme),
        };
    }

    RailButtonColors {
        rest: gpui::transparent_black(),
        hover: theme.secondary,
        pressed: theme.secondary_hover,
        icon: theme.muted_foreground,
    }
}

type RailSelectHandler = Rc<dyn Fn(&SharedString, &mut Window, &mut App)>;

/// The activity rail: a row directly on the desk, with no fill or line of
/// its own, the top entries from the left and the bottom entries pinned to
/// the right edge.
#[derive(IntoElement)]
pub struct ActivityRail {
    id: ElementId,
    entries: Vec<RailEntry>,
    on_select: Option<RailSelectHandler>,
}

impl ActivityRail {
    pub fn new(id: impl Into<ElementId>, entries: Vec<RailEntry>) -> Self {
        Self {
            id: id.into(),
            entries,
            on_select: None,
        }
    }

    /// Called with the entry id when a button is clicked.
    pub fn on_select(
        mut self,
        handler: impl Fn(&SharedString, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_select = Some(Rc::new(handler));
        self
    }

    /// Top entries in display order, then bottom entries in display order.
    pub fn partition(entries: &[RailEntry]) -> (Vec<&RailEntry>, Vec<&RailEntry>) {
        entries
            .iter()
            .partition(|entry| entry.placement == RailPlacement::Top)
    }

    fn render_button(
        entry: &RailEntry,
        on_select: Option<RailSelectHandler>,
        cx: &App,
    ) -> impl IntoElement {
        let theme = cx.theme();
        let colors = rail_button_colors(theme, entry.active);
        let element_id = entry.element_id();
        let entry_id = entry.id.clone();
        let label = entry.label.clone();
        let badge = entry.count.and_then(|(count, urgency)| {
            NotificationBell::badge_text(count).zip(NotificationBell::badge_colors(urgency, theme))
        });
        let badge_id = entry.count_element_id();
        let element_id = ElementId::Name(element_id);
        let tooltip = crate::keymap::tooltip_with_shortcut(cx, &element_id, label.clone());

        div()
            .debug_selector(crate::selector::id_selector(&element_id))
            .id(element_id)
            .role(Role::Button)
            .aria_label(label.clone())
            .aria_selected(entry.active)
            .relative()
            .flex()
            .flex_shrink_0()
            .items_center()
            .justify_center()
            .size(ShellMetrics::RAIL_BUTTON)
            .cursor_pointer()
            .child(
                Chamfer::new(ChamferCut::CONTROL)
                    .fill(colors.rest)
                    .fill_hover(colors.hover)
                    .fill_active(colors.pressed)
                    .interactive("rail-button-chamfer"),
            )
            .child(
                Icon::new(entry.icon.clone())
                    .size(ShellMetrics::RAIL_ICON)
                    .color(colors.icon),
            )
            .when_some(badge, |button, (text, colors)| {
                button.child(count_badge(badge_id, text, colors))
            })
            .tooltip(move |window, cx| Tooltip::new(tooltip.clone()).build(window, cx))
            .when_some(on_select, |button, handler| {
                button.on_click(move |_, window, cx| handler(&entry_id, window, cx))
            })
    }
}

impl RenderOnce for ActivityRail {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let (top, bottom) = Self::partition(&self.entries);

        let top_buttons: Vec<_> = top
            .into_iter()
            .map(|entry| Self::render_button(entry, self.on_select.clone(), cx))
            .collect();
        let bottom_buttons: Vec<_> = bottom
            .into_iter()
            .map(|entry| Self::render_button(entry, self.on_select.clone(), cx))
            .collect();

        div()
            .debug_selector(crate::selector::id_selector(&self.id))
            .id(self.id)
            .key_context(ACTIVITY_RAIL_KEY_CONTEXT)
            .flex()
            .flex_row()
            .flex_shrink_0()
            .items_center()
            .gap(ShellMetrics::RAIL_GAP)
            .w_full()
            .min_w_0()
            .overflow_hidden()
            .p(ShellMetrics::RAIL_PADDING_Y)
            .children(top_buttons)
            .child(div().flex_1())
            .children(bottom_buttons)
    }
}

#[cfg(test)]
mod tests;

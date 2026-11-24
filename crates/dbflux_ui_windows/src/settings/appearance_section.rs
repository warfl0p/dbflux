//! The Settings > Appearance page: theme, density, language, fonts and
//! scrollbar visibility.
//!
//! It is a view of the same stored `GeneralSettings` the General page edits, so
//! it reuses [`GeneralSection`]'s rows, controls and save path and only picks
//! the fields it owns.

use super::general_section::{FontFamilyField, GeneralFormRow, GeneralSection};
use super::layout;
use crate::tokens::{FormMetrics, SettingsMetrics};
use dbflux_components::controls::{Button as FluxButton, Input, InputEvent, InputState};
use dbflux_components::icons::AppIcon;
use dbflux_components::primitives::SegmentedItem;
use dbflux_components::primitives::Text;
use dbflux_components::tokens::ui;
use dbflux_core::{AccentColor, GeneralSettings, SyntaxRole, ThemeSetting};
use gpui::prelude::FluentBuilder;
use gpui::*;
use gpui_component::ActiveTheme;

/// Side of the square swatch showing an accent.
const ACCENT_SWATCH: Rems = ui(20.0);

/// One of the two accent fields of the Accent color row: the accent of the
/// Dark palette and the accent of the Light palette.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(super) enum AccentField {
    Dark,
    Light,
}

/// What an accent field holds.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(super) enum AccentEntry {
    /// The palette's own accent: nothing is stored for it.
    Palette,
    Custom(AccentColor),
    /// Text that is not a `#RRGGBB` color. Never saved, never applied.
    Invalid,
}

impl AccentField {
    const ALL: [Self; 2] = [Self::Dark, Self::Light];

    pub(super) fn from_row(row: GeneralFormRow) -> Option<Self> {
        match row {
            GeneralFormRow::AccentDark => Some(Self::Dark),
            GeneralFormRow::AccentLight => Some(Self::Light),
            _ => None,
        }
    }

    fn setting(self) -> ThemeSetting {
        match self {
            Self::Dark => ThemeSetting::Dark,
            Self::Light => ThemeSetting::Light,
        }
    }

    fn palette_accent(self) -> AccentColor {
        dbflux_components::theme::palette_accent(self.setting())
    }

    fn row(self) -> GeneralFormRow {
        match self {
            Self::Dark => GeneralFormRow::AccentDark,
            Self::Light => GeneralFormRow::AccentLight,
        }
    }

    fn caption(self) -> String {
        match self {
            Self::Dark => dbflux_i18n::t!("settings.general.appearance.accent.dark"),
            Self::Light => dbflux_i18n::t!("settings.general.appearance.accent.light"),
        }
    }

    fn reset_id(self) -> &'static str {
        match self {
            Self::Dark => "accent-reset-dark",
            Self::Light => "accent-reset-light",
        }
    }

    fn swatch_id(self) -> &'static str {
        match self {
            Self::Dark => "accent-swatch-dark",
            Self::Light => "accent-swatch-light",
        }
    }
}

/// Classifies the text of an accent field.
pub(super) fn accent_entry_of(text: &str, field: AccentField) -> AccentEntry {
    match AccentColor::parse(text) {
        Some(color) if color == field.palette_accent() => AccentEntry::Palette,
        Some(color) => AccentEntry::Custom(color),
        None => AccentEntry::Invalid,
    }
}

impl GeneralSection {
    /// Rows of the Appearance page, in display order.
    pub(super) fn appearance_form_rows() -> Vec<GeneralFormRow> {
        vec![
            GeneralFormRow::Theme,
            GeneralFormRow::AccentDark,
            GeneralFormRow::AccentLight,
            GeneralFormRow::Style,
            GeneralFormRow::Language,
            GeneralFormRow::UiFontFamily,
            GeneralFormRow::UiFontSize,
            GeneralFormRow::EditorFontFamily,
            GeneralFormRow::EditorFontSize,
            GeneralFormRow::GridFontFamily,
            GeneralFormRow::GridFontSize,
            GeneralFormRow::Scrollbars,
            GeneralFormRow::SyntaxVariant,
        ]
        .into_iter()
        .chain(SyntaxRole::ALL.map(GeneralFormRow::SyntaxColor))
        .chain([GeneralFormRow::SyntaxReset, GeneralFormRow::SaveButton])
        .collect()
    }

    pub(super) fn new_accent_input(
        field: AccentField,
        saved: Option<AccentColor>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Entity<InputState> {
        let palette = field.palette_accent().to_hex();
        let shown = saved.map_or_else(|| palette.clone(), AccentColor::to_hex);

        cx.new(|cx| {
            InputState::new(window, cx)
                .placeholder(palette)
                .default_value(shown)
        })
    }

    fn accent_input(&self, field: AccentField) -> &Entity<InputState> {
        match field {
            AccentField::Dark => &self.input_accent_dark,
            AccentField::Light => &self.input_accent_light,
        }
    }

    fn saved_accent(&self, field: AccentField, cx: &App) -> Option<AccentColor> {
        let saved = self.app_state.read(cx).general_settings();

        match field {
            AccentField::Dark => saved.accent_dark,
            AccentField::Light => saved.accent_light,
        }
    }

    pub(super) fn accent_entry(&self, field: AccentField, cx: &App) -> AccentEntry {
        accent_entry_of(self.accent_input(field).read(cx).value().as_ref(), field)
    }

    /// Number of accent fields that differ from the saved accents. Text that
    /// is not a color counts as a change: it differs from what is saved.
    pub(super) fn accent_change_count(&self, cx: &App) -> usize {
        AccentField::ALL
            .into_iter()
            .filter(|field| {
                let saved = self.saved_accent(*field, cx);

                match self.accent_entry(*field, cx) {
                    AccentEntry::Palette => saved.is_some(),
                    AccentEntry::Custom(color) => saved != Some(color),
                    AccentEntry::Invalid => true,
                }
            })
            .count()
    }

    /// Puts the palette's accent back in `field`. Saving then stores nothing
    /// for it.
    fn reset_accent(&mut self, field: AccentField, window: &mut Window, cx: &mut Context<Self>) {
        let palette = field.palette_accent().to_hex();

        self.accent_input(field)
            .clone()
            .update(cx, |input, cx| input.set_value(palette, window, cx));

        if self
            .gen_field_error
            .as_ref()
            .is_some_and(|(row, _)| *row == field.row())
        {
            self.gen_field_error = None;
        }
    }

    /// `R` on an accent row resets that field. Returns whether the cursor was
    /// on one and the field was not already the palette's accent.
    pub(super) fn reset_current_accent(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> bool {
        let Some(field) = self.gen_current_row().and_then(AccentField::from_row) else {
            return false;
        };

        if self.accent_entry(field, cx) == AccentEntry::Palette {
            return false;
        }

        self.reset_accent(field, window, cx);
        true
    }

    /// Reads both accent fields, or reports the first invalid one under its
    /// field and returns `None`: nothing is saved with an invalid color.
    fn read_accents(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Option<(Option<AccentColor>, Option<AccentColor>)> {
        let mut accents = [None, None];

        for (slot, field) in accents.iter_mut().zip(AccentField::ALL) {
            match self.accent_entry(field, cx) {
                AccentEntry::Palette => {}
                AccentEntry::Custom(color) => *slot = Some(color),
                AccentEntry::Invalid => {
                    self.reject_field(
                        field.row(),
                        dbflux_i18n::t!("settings.general.appearance.accent.error"),
                        window,
                        cx,
                    );
                    return None;
                }
            }
        }

        Some((accents[0], accents[1]))
    }

    /// Validates the three font sizes and saves the Appearance fields. A
    /// rejected size is reported under its field and nothing is saved.
    pub(super) fn save_appearance_settings(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some((accent_dark, accent_light)) = self.read_accents(window, cx) else {
            return;
        };
        if !self.normalize_syntax_colors(window, cx) {
            return;
        }

        let Some(ui_font_size) = self.read_font_size(
            &self.input_ui_font_size.clone(),
            GeneralFormRow::UiFontSize,
            "settings.general.ui_font_size.error",
            window,
            cx,
        ) else {
            return;
        };
        let Some(editor_font_size) = self.read_font_size(
            &self.input_editor_font_size.clone(),
            GeneralFormRow::EditorFontSize,
            "settings.general.editor_font_size.error",
            window,
            cx,
        ) else {
            return;
        };
        let Some(grid_font_size) = self.read_font_size(
            &self.input_grid_font_size.clone(),
            GeneralFormRow::GridFontSize,
            "settings.general.grid_font_size.error",
            window,
            cx,
        ) else {
            return;
        };

        self.gen_settings.accent_dark = accent_dark;
        self.gen_settings.accent_light = accent_light;
        self.gen_settings.ui_font_size = ui_font_size;
        self.gen_settings.editor_font_size = editor_font_size;
        self.gen_settings.grid_font_size = grid_font_size;

        self.commit_general_settings(window, cx);
    }

    pub(super) fn render_appearance_section(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let theme_index = Self::theme_index(self.gen_settings.theme);
        let style_index = Self::style_index(self.gen_settings.style);
        let scrollbars_index =
            Self::scrollbar_visibility_index(self.gen_settings.scrollbar_visibility);

        layout::scrolled_form_section_shell(
            dbflux_components::composites::page_header(
                dbflux_i18n::t!("settings.nav.appearance"),
                dbflux_i18n::t!("settings.general.appearance.subtitle"),
                cx,
            ),
            div()
                .flex()
                .flex_col()
                .child(self.render_gen_segmented(
                    dbflux_i18n::t!("settings.general.theme.label"),
                    Some(dbflux_i18n::t!("settings.general.theme.help")),
                    Self::theme_items(),
                    theme_index,
                    GeneralFormRow::Theme,
                    "theme",
                    |this, index| this.gen_settings.theme = Self::theme_for_index(index),
                    cx,
                ))
                .child(self.render_accent_row(cx))
                .child(self.render_gen_segmented(
                    dbflux_i18n::t!("settings.general.style.label"),
                    Some(dbflux_i18n::t!("settings.general.style.help")),
                    Self::style_items(),
                    style_index,
                    GeneralFormRow::Style,
                    "density",
                    |this, index| this.gen_settings.style = Self::style_for_index(index),
                    cx,
                ))
                .child(self.render_gen_dropdown(
                    dbflux_i18n::t!("settings.general.language.label"),
                    Some(dbflux_i18n::t!("settings.general.language.notice")),
                    self.dropdown_language.clone(),
                    GeneralFormRow::Language,
                    cx,
                ))
                .child(self.render_gen_font_family(
                    FontFamilyField::Ui,
                    &self.select_ui_font_family,
                    GeneralFormRow::UiFontFamily,
                    cx,
                ))
                .child(self.render_gen_font_size(
                    dbflux_i18n::t!("settings.general.ui_font_size.label"),
                    dbflux_i18n::t!("settings.general.ui_font_size.help"),
                    &self.input_ui_font_size,
                    GeneralFormRow::UiFontSize,
                    cx,
                ))
                .child(self.render_gen_font_family(
                    FontFamilyField::Editor,
                    &self.select_editor_font_family,
                    GeneralFormRow::EditorFontFamily,
                    cx,
                ))
                .child(self.render_gen_font_size(
                    dbflux_i18n::t!("settings.general.editor_font_size.label"),
                    dbflux_i18n::t!("settings.general.editor_font_size.help"),
                    &self.input_editor_font_size,
                    GeneralFormRow::EditorFontSize,
                    cx,
                ))
                .child(self.render_gen_font_family(
                    FontFamilyField::Grid,
                    &self.select_grid_font_family,
                    GeneralFormRow::GridFontFamily,
                    cx,
                ))
                .child(self.render_gen_font_size(
                    dbflux_i18n::t!("settings.general.grid_font_size.label"),
                    dbflux_i18n::t!("settings.general.grid_font_size.help"),
                    &self.input_grid_font_size,
                    GeneralFormRow::GridFontSize,
                    cx,
                ))
                .child(self.render_gen_segmented(
                    dbflux_i18n::t!("settings.general.scrollbars.label"),
                    Some(dbflux_i18n::t!("settings.general.scrollbars.help")),
                    Self::scrollbar_visibility_items(),
                    scrollbars_index,
                    GeneralFormRow::Scrollbars,
                    "scrollbars",
                    |this, index| {
                        this.gen_settings.scrollbar_visibility =
                            Self::scrollbar_visibility_for_index(index)
                    },
                    cx,
                ))
                .child(self.render_syntax_colors(cx)),
            &self.form_scroll,
            self.form_viewport.clone(),
        )
    }
}

impl GeneralSection {
    /// The Accent color row: one field per palette, each with a swatch of the
    /// color it applies and a Reset that returns it to the palette's accent.
    fn render_accent_row(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let fields = AccentField::ALL
            .into_iter()
            .map(|field| self.render_accent_field(field, cx).into_any_element());

        layout::form_row(
            dbflux_i18n::t!("settings.general.appearance.accent.label"),
            div()
                .flex()
                .flex_wrap()
                .gap(FormMetrics::ROW_GAP)
                .children(fields),
            Some(dbflux_i18n::t!("settings.general.appearance.accent.help").into()),
        )
        .debug_selector(|| "general-accent-row".to_string())
    }

    fn render_accent_field(&self, field: AccentField, cx: &mut Context<Self>) -> impl IntoElement {
        let row = field.row();
        let entry = self.accent_entry(field, cx);
        let element_id = Self::input_element_id(row);

        // An invalid entry never applies: the swatch keeps showing the accent
        // that does, the saved one or the palette's.
        let shown = match entry {
            AccentEntry::Custom(color) => color,
            AccentEntry::Palette => field.palette_accent(),
            AccentEntry::Invalid => self
                .saved_accent(field, cx)
                .unwrap_or_else(|| field.palette_accent()),
        };
        // Derived from the text, so it shows exactly while the text is invalid
        // and a rejected save's recorded error cannot outlive a correction.
        let error = (entry == AccentEntry::Invalid)
            .then(|| dbflux_i18n::t!("settings.general.appearance.accent.error"));

        let input = Input::new(self.accent_input(field))
            .id(element_id)
            .aria_label(format!(
                "{} {}",
                dbflux_i18n::t!("settings.general.appearance.accent.label"),
                field.caption()
            ));

        let field_bounds = self.field_bounds.clone();
        let form_scroll = self.form_scroll.clone();
        let bounds_recorder = canvas(
            move |bounds, _, _| {
                field_bounds
                    .borrow_mut()
                    .insert(row, (bounds, form_scroll.offset().y));
            },
            |_, _, _, _| {},
        )
        .absolute()
        .top_0()
        .left_0()
        .size_full();

        let swatch = div()
            .id(field.swatch_id())
            .size(ACCENT_SWATCH)
            .flex_shrink_0()
            .border_1()
            .border_color(cx.theme().border)
            .bg(Hsla::from(gpui::rgb(shown.rgb()))); // guardrail-allow: swatch of the user-chosen accent

        let control = layout::cursor_ring(
            self.is_at(row) && !self.gen_editing_field,
            div()
                .relative()
                .w(SettingsMetrics::NUMBER_FIELD_WIDTH)
                .font_family(dbflux_components::fonts::editor_family(cx))
                .child(input)
                .child(bounds_recorder),
            cx,
        )
        .w(SettingsMetrics::NUMBER_FIELD_WIDTH)
        .debug_selector(move || format!("{element_id}-control"))
        .on_mouse_down(
            MouseButton::Left,
            cx.listener(move |this, _, window, cx| {
                this.switching_input = true;
                this.select_row(row);
                this.gen_focus_current_input(window, cx);
                cx.notify();
            }),
        );

        let reset = FluxButton::new(
            field.reset_id(),
            dbflux_i18n::t!("settings.general.appearance.accent.reset"),
        )
        .ghost()
        .inline()
        .icon(AppIcon::RotateCcw)
        .kbd("R")
        .disabled(entry == AccentEntry::Palette)
        .on_click(cx.listener(move |this, _, window, cx| {
            this.select_row(row);
            this.reset_accent(field, window, cx);
            cx.notify();
        }));

        div()
            .flex()
            .flex_col()
            .gap(FormMetrics::HELP_GAP)
            .child(Text::caption(field.caption()))
            .child(
                layout::inline_controls()
                    .child(swatch)
                    .child(control)
                    .child(reset),
            )
            .when_some(error, |column, error| {
                let error_id = SharedString::from(format!("{element_id}-error"));
                column.child(
                    div()
                        .id(error_id.clone())
                        .debug_selector(move || error_id.to_string())
                        .child(layout::help_text(error).danger()),
                )
            })
    }
}

impl GeneralSection {
    /// One `#RRGGBB` field per syntax role for `variant`, and the
    /// subscriptions that store what is typed in them.
    pub(super) fn new_syntax_inputs(
        settings: &GeneralSettings,
        variant: ThemeSetting,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> (Vec<(SyntaxRole, Entity<InputState>)>, Vec<Subscription>) {
        let inputs: Vec<(SyntaxRole, Entity<InputState>)> = SyntaxRole::ALL
            .into_iter()
            .map(|role| {
                let default = Self::default_syntax_hex(variant, role);
                let shown = settings
                    .syntax_colors
                    .for_variant(variant)
                    .get(&role)
                    .cloned()
                    .unwrap_or_else(|| default.clone());
                let input = cx.new(|cx| {
                    InputState::new(window, cx)
                        .placeholder(default)
                        .default_value(shown)
                });
                (role, input)
            })
            .collect();

        let subscriptions = inputs
            .iter()
            .map(|(role, input)| {
                let role = *role;
                cx.subscribe(input, move |this, input, event: &InputEvent, cx| {
                    if matches!(event, InputEvent::Change) {
                        let text = input.read(cx).value().trim().to_string();
                        this.set_syntax_override(role, text);
                        cx.notify();
                    } else {
                        this.return_focus_on_blur(input, event, cx);
                    }
                })
            })
            .collect();

        (inputs, subscriptions)
    }

    /// The variant the syntax rows open on: the one on screen, Dark for
    /// `System` before it resolves.
    pub(super) fn syntax_variant_for(theme: ThemeSetting) -> ThemeSetting {
        match theme {
            ThemeSetting::Light => ThemeSetting::Light,
            ThemeSetting::Dark | ThemeSetting::System => ThemeSetting::Dark,
        }
    }

    /// The palette's own color for `role` in `variant`, as `#RRGGBB`.
    pub(super) fn default_syntax_hex(variant: ThemeSetting, role: SyntaxRole) -> String {
        dbflux_components::tokens::SyntaxColors::hex(dbflux_components::theme::syntax_role(
            dbflux_components::theme::default_syntax_colors(variant),
            role,
        ))
    }

    /// Stores the text of `role`'s field for the edited variant. Empty text,
    /// or the palette's own color, keeps the palette's color, so only real
    /// changes become overrides.
    pub(super) fn set_syntax_override(&mut self, role: SyntaxRole, text: String) {
        let default = Self::default_syntax_hex(self.syntax_variant, role);
        let is_default = dbflux_core::parse_hex_color(&text).is_some()
            && dbflux_core::parse_hex_color(&text) == dbflux_core::parse_hex_color(&default);
        let overrides = self
            .gen_settings
            .syntax_colors
            .for_variant_mut(self.syntax_variant);

        if text.is_empty() || is_default {
            overrides.remove(&role);
        } else {
            overrides.insert(role, text);
        }
    }

    /// Restores the palette's color for `role` in the edited variant.
    pub(super) fn reset_syntax_color(&mut self, role: SyntaxRole) {
        self.set_syntax_override(role, String::new());
        self.pending_syntax_reload = true;
    }

    /// `R` on a syntax row resets that color. Returns whether the cursor was
    /// on one.
    pub(super) fn reset_current_syntax_color(&mut self) -> bool {
        let Some(GeneralFormRow::SyntaxColor(role)) = self.gen_current_row() else {
            return false;
        };

        self.reset_syntax_color(role);
        true
    }

    /// Restores every syntax color of the edited variant.
    pub(super) fn reset_variant_colors(&mut self) {
        self.gen_settings
            .syntax_colors
            .for_variant_mut(self.syntax_variant)
            .clear();
        self.pending_syntax_reload = true;
    }

    pub(super) fn set_syntax_variant(&mut self, variant: ThemeSetting) {
        self.syntax_variant = Self::syntax_variant_for(variant);
        self.pending_syntax_reload = true;
    }

    pub(super) fn syntax_input(&self, role: SyntaxRole) -> Option<&Entity<InputState>> {
        self.syntax_inputs
            .iter()
            .find(|(input_role, _)| *input_role == role)
            .map(|(_, input)| input)
    }

    /// Number of syntax colors, in both variants, that differ from the saved
    /// ones.
    pub(super) fn syntax_change_count(&self, cx: &App) -> usize {
        let saved = &self.app_state.read(cx).general_settings().syntax_colors;
        let current = &self.gen_settings.syntax_colors;

        [ThemeSetting::Dark, ThemeSetting::Light]
            .into_iter()
            .flat_map(|variant| {
                SyntaxRole::ALL.into_iter().map(move |role| {
                    current.for_variant(variant).get(&role) != saved.for_variant(variant).get(&role)
                })
            })
            .filter(|changed| *changed)
            .count()
    }

    /// Shows the edited variant's colors in the syntax fields: each override,
    /// or the palette's color where there is none.
    pub(super) fn reload_syntax_inputs(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if !std::mem::take(&mut self.pending_syntax_reload) {
            return;
        }

        let variant = self.syntax_variant;
        for (role, input) in &self.syntax_inputs {
            let placeholder = Self::default_syntax_hex(variant, *role);
            let value = self
                .gen_settings
                .syntax_colors
                .for_variant(variant)
                .get(role)
                .cloned()
                .unwrap_or_else(|| placeholder.clone());

            input.update(cx, |state, cx| {
                state.set_value(value, window, cx);
                state.set_placeholder(placeholder, window, cx);
            });
        }
    }

    /// Syntax variant segments: Dark, then Light.
    pub(super) fn syntax_variant_items() -> Vec<SegmentedItem> {
        vec![
            SegmentedItem::new(
                "dark",
                dbflux_i18n::t!("settings.general.theme.option.dark"),
            ),
            SegmentedItem::new(
                "light",
                dbflux_i18n::t!("settings.general.theme.option.light"),
            ),
        ]
    }

    pub(super) fn syntax_variant_index(variant: ThemeSetting) -> usize {
        usize::from(variant == ThemeSetting::Light)
    }

    pub(super) fn syntax_variant_for_index(index: usize) -> ThemeSetting {
        if index == 1 {
            ThemeSetting::Light
        } else {
            ThemeSetting::Dark
        }
    }

    pub(super) fn syntax_role_label(role: SyntaxRole) -> String {
        match role {
            SyntaxRole::Keyword => dbflux_i18n::t!("settings.appearance.syntax.role.keyword"),
            SyntaxRole::String => dbflux_i18n::t!("settings.appearance.syntax.role.string"),
            SyntaxRole::Number => dbflux_i18n::t!("settings.appearance.syntax.role.number"),
            SyntaxRole::Comment => dbflux_i18n::t!("settings.appearance.syntax.role.comment"),
            SyntaxRole::Type => dbflux_i18n::t!("settings.appearance.syntax.role.type"),
            SyntaxRole::Function => dbflux_i18n::t!("settings.appearance.syntax.role.function"),
            SyntaxRole::Operator => dbflux_i18n::t!("settings.appearance.syntax.role.operator"),
            SyntaxRole::Identifier => {
                dbflux_i18n::t!("settings.appearance.syntax.role.identifier")
            }
            SyntaxRole::Namespace => dbflux_i18n::t!("settings.appearance.syntax.role.namespace"),
            SyntaxRole::Field => dbflux_i18n::t!("settings.appearance.syntax.role.field"),
        }
    }

    /// Checks every syntax color of both variants and stores each as
    /// `#RRGGBB`. The first one that does not parse is reported on its row,
    /// with its variant shown, and `false` is returned.
    fn normalize_syntax_colors(&mut self, window: &mut Window, cx: &mut Context<Self>) -> bool {
        let variants = [ThemeSetting::Dark, ThemeSetting::Light];

        let invalid = variants.into_iter().find_map(|variant| {
            self.gen_settings
                .syntax_colors
                .for_variant(variant)
                .iter()
                .find(|(_, text)| dbflux_core::parse_hex_color(text).is_none())
                .map(|(role, _)| (variant, GeneralFormRow::SyntaxColor(*role)))
        });

        if let Some((variant, row)) = invalid {
            if self.syntax_variant != variant {
                self.set_syntax_variant(variant);
            }
            self.reject_field(
                row,
                dbflux_i18n::t!("settings.appearance.syntax.error"),
                window,
                cx,
            );
            return false;
        }

        for variant in variants {
            for text in self
                .gen_settings
                .syntax_colors
                .for_variant_mut(variant)
                .values_mut()
            {
                if let Some(value) = dbflux_core::parse_hex_color(text) {
                    *text = format!("#{value:06X}");
                }
            }
        }

        self.pending_syntax_reload = true;
        true
    }

    /// Syntax color group: which variant is edited, then one row per syntax
    /// role, each with its `#RRGGBB` field, a swatch and a reset button, then
    /// Restore defaults.
    fn render_syntax_colors(&self, cx: &mut Context<Self>) -> Div {
        let variant = self.syntax_variant;
        let effective = dbflux_components::theme::syntax_colors_with_overrides(
            variant,
            self.gen_settings.syntax_colors.for_variant(variant),
        );

        let rows: Vec<Div> = self
            .syntax_inputs
            .iter()
            .map(|(role, input)| {
                let role = *role;
                let overridden = self
                    .gen_settings
                    .syntax_colors
                    .for_variant(variant)
                    .contains_key(&role);

                self.render_color_field(
                    Self::syntax_role_label(role),
                    input,
                    GeneralFormRow::SyntaxColor(role),
                    dbflux_components::theme::syntax_role(effective, role),
                    overridden,
                    move |this| this.reset_syntax_color(role),
                    cx,
                )
            })
            .collect();

        let reset_all = layout::cursor_ring(
            self.is_at(GeneralFormRow::SyntaxReset),
            div().flex().child(
                FluxButton::new(
                    "syntax-reset-all",
                    dbflux_i18n::t!("settings.appearance.syntax.reset_all.button"),
                )
                .icon(AppIcon::RotateCcw)
                .disabled(
                    self.gen_settings
                        .syntax_colors
                        .for_variant(variant)
                        .is_empty(),
                )
                .on_click(cx.listener(|this, _, _, cx| {
                    this.select_row(GeneralFormRow::SyntaxReset);
                    this.reset_variant_colors();
                    cx.notify();
                })),
            ),
            cx,
        );

        div()
            .flex()
            .flex_col()
            .child(dbflux_components::composites::section_header(
                dbflux_i18n::t!("settings.appearance.syntax.group"),
                Some(AppIcon::Code.into()),
                cx,
            ))
            .child(self.render_gen_segmented(
                dbflux_i18n::t!("settings.appearance.syntax.variant.label"),
                Some(dbflux_i18n::t!("settings.appearance.syntax.variant.help")),
                Self::syntax_variant_items(),
                Self::syntax_variant_index(variant),
                GeneralFormRow::SyntaxVariant,
                "syntax-variant",
                |this, index| this.set_syntax_variant(Self::syntax_variant_for_index(index)),
                cx,
            ))
            .children(rows)
            .child(layout::form_row(
                dbflux_i18n::t!("settings.appearance.syntax.reset_all.label"),
                reset_all,
                Some(SharedString::from(dbflux_i18n::t!(
                    "settings.appearance.syntax.reset_all.help"
                ))),
            ))
    }

    /// One color row: the `#RRGGBB` field, a swatch of the color in use and
    /// a reset button, enabled while the color differs from the default.
    #[allow(clippy::too_many_arguments)]
    fn render_color_field(
        &self,
        label: String,
        input: &Entity<InputState>,
        row: GeneralFormRow,
        color: Hsla,
        overridden: bool,
        reset: impl Fn(&mut Self) + 'static,
        cx: &mut Context<Self>,
    ) -> Div {
        let swatch = div()
            .flex_shrink_0()
            .size(ui(16.0))
            .border_1()
            .border_color(cx.theme().border)
            .bg(color);

        let reset = FluxButton::new(
            SharedString::from(format!("syntax-reset-{}", Self::input_element_id(row))),
            dbflux_i18n::t!("settings.appearance.syntax.reset"),
        )
        .ghost()
        .icon(AppIcon::RotateCcw)
        .disabled(!overridden)
        .on_click(cx.listener(move |this, _, _, cx| {
            this.select_row(row);
            reset(this);
            cx.notify();
        }));

        let trailing = div()
            .flex()
            .items_center()
            .gap(FormMetrics::HELP_GAP)
            .child(swatch)
            .child(reset)
            .into_any_element();

        self.render_gen_input_field_with(label, input, None, None, Some(trailing), row, cx)
    }
}

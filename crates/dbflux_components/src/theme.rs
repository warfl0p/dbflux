//! DBFlux's product adapter over `bolt`.
//!
//! Bolt owns the theme engine and the shared palettes; this module supplies
//! DBFlux's brand (the byzantine accent), maps the persisted `ThemeSetting`
//! and `AppStyle` settings onto bolt's `PaletteMode` and `Density`, and keeps
//! the public API the rest of the workspace compiles against.

use crate::fonts::FontSettings;
use crate::semantic::ThemeSettingGlobal;
pub use crate::typography::AppFonts;
use crate::typography::load_bundled_fonts;
use bolt::brand::Brand;
use bolt::palette::PaletteMode;
use bolt::theme::rgb_to_hsla;
use bolt::theme::{self as bolt_theme};
use bolt::tokens::{CustomSyntaxColors, SyntaxColors};
use dbflux_core::{AccentColorOverrides, AppStyle, SyntaxColorOverrides, SyntaxRole, ThemeSetting};
use gpui::{App, Global, Hsla, Window, WindowAppearance};
use gpui_component::theme::Theme;

/// Byzantine `#702963`, the solid fill of a primary control in both variants.
const ACCENT: u32 = 0x702963;
/// Byzantine hover `#7F3171`.
const ACCENT_HOVER: u32 = 0x7F3171;
/// Byzantine deep `#4A1B41`, the pressed state.
const ACCENT_PRESSED: u32 = 0x4A1B41;
/// Text-accent tint `#D48CC8` on dark surfaces; on light surfaces the accent
/// itself takes this role.
const ACCENT_TINT: u32 = 0xD48CC8;
const ON_ACCENT: u32 = 0xFFFFFF;
/// The blue accent `#7AA2F7` of DBFlux's dark palette, in place of byzantine.
const DARK_ACCENT: u32 = 0x7AA2F7;

/// Lightness offset between an accent and its hover state.
const HOVER_LIGHTNESS_STEP: f32 = 0.05;
/// Lightness offset between an accent and its pressed state.
const ACTIVE_LIGHTNESS_STEP: f32 = -0.06;
/// Lowest lightness of the text tint on the dark palette, so tinted text
/// stays readable on its near-black surfaces (`#D48CC8` sits at 0.69).
const DARK_TINT_MIN_LIGHTNESS: f32 = 0.68;
/// Highest lightness of the text tint on the light palette, so tinted text
/// stays readable on its near-white surfaces (byzantine sits at 0.30).
const LIGHT_TINT_MAX_LIGHTNESS: f32 = 0.40;
/// Relative luminance above which near-black text contrasts more with a fill
/// than white does: the point where both WCAG contrast ratios are equal.
const DARK_FOREGROUND_LUMINANCE: f32 = 0.179;

/// The DBFlux brand: the byzantine accent of the Bolt Byzantium palette.
pub fn brand() -> Brand {
    Brand {
        name: "DBFlux".into(),
        accent: rgb_to_hsla(ACCENT),
        accent_hover: rgb_to_hsla(ACCENT_HOVER),
        accent_pressed: rgb_to_hsla(ACCENT_PRESSED),
        accent_tint: rgb_to_hsla(ACCENT_TINT),
        on_accent: rgb_to_hsla(ON_ACCENT),
    }
}

/// The brand for `variant`: the user's accent when one is set, else the blue
/// accent on dark and byzantine on light.
fn brand_for(variant: ThemeSetting, cx: &App) -> Brand {
    match (accent_override(variant, cx), variant) {
        (Some(accent), _) => brand_from_accent(accent, variant),
        (None, ThemeSetting::Light) => brand(),
        (None, ThemeSetting::Dark | ThemeSetting::System) => {
            brand_from_accent(rgb_to_hsla(DARK_ACCENT), variant)
        }
    }
}

/// A brand whose hover, pressed, tint and foreground roles derive from `accent`.
fn brand_from_accent(accent: Hsla, variant: ThemeSetting) -> Brand {
    Brand {
        accent,
        accent_hover: shift_lightness(accent, HOVER_LIGHTNESS_STEP),
        accent_pressed: shift_lightness(accent, ACTIVE_LIGHTNESS_STEP),
        accent_tint: tint_for_accent(accent, variant),
        on_accent: foreground_on(accent),
        ..brand()
    }
}

fn shift_lightness(color: Hsla, amount: f32) -> Hsla {
    Hsla {
        l: (color.l + amount).clamp(0.0, 1.0),
        ..color
    }
}

/// The text tint for `accent`: the accent itself, lightened on the dark
/// palette or darkened on the light one when it would be hard to read.
fn tint_for_accent(accent: Hsla, variant: ThemeSetting) -> Hsla {
    let l = match variant {
        ThemeSetting::Light => accent.l.min(LIGHT_TINT_MAX_LIGHTNESS),
        ThemeSetting::Dark | ThemeSetting::System => accent.l.max(DARK_TINT_MIN_LIGHTNESS),
    };
    Hsla { l, ..accent }
}

/// White or near-black, whichever reads better on `fill`.
fn foreground_on(fill: Hsla) -> Hsla {
    let rgba = gpui::Rgba::from(fill);
    let linear = |channel: f32| {
        if channel <= 0.04045 {
            channel / 12.92
        } else {
            ((channel + 0.055) / 1.055).powf(2.4)
        }
    };
    let luminance = 0.2126 * linear(rgba.r) + 0.7152 * linear(rgba.g) + 0.0722 * linear(rgba.b);

    if luminance > DARK_FOREGROUND_LUMINANCE {
        rgb_to_hsla(0x09090B)
    } else {
        rgb_to_hsla(0xFFFFFF)
    }
}

/// The syntax colors the user picked in place of the palette's.
#[derive(Default)]
struct SyntaxOverridesGlobal(SyntaxColorOverrides);

impl Global for SyntaxOverridesGlobal {}

/// Store the user's syntax colors. They take effect in
/// [`SyntaxColors::for_current`] at once and in the editor with the next
/// [`apply_theme`].
pub fn set_syntax_overrides(overrides: SyntaxColorOverrides, cx: &mut App) {
    cx.set_global(SyntaxOverridesGlobal(overrides));
    install_syntax_colors(cx);
}

/// The user's syntax colors, empty until [`set_syntax_overrides`] runs.
pub fn syntax_overrides(cx: &App) -> Option<&SyntaxColorOverrides> {
    cx.try_global::<SyntaxOverridesGlobal>()
        .map(|global| &global.0)
}

/// Publish DBFlux's syntax colors, with the user's overrides, to bolt.
fn install_syntax_colors(cx: &mut App) {
    let colors_for = |variant: ThemeSetting, cx: &App| {
        let colors = default_syntax_colors(variant);
        match syntax_overrides(cx) {
            Some(overrides) => with_syntax_overrides(colors, overrides.for_variant(variant)),
            None => colors,
        }
    };
    let custom = CustomSyntaxColors {
        dark: Some(colors_for(ThemeSetting::Dark, cx)),
        light: Some(colors_for(ThemeSetting::Light, cx)),
    };
    bolt::tokens::set_custom_syntax_colors(custom, cx);
}

/// DBFlux's own syntax colors for `variant`, before the user's overrides:
/// the keyword keeps its byzantine role whatever the accent is.
pub fn default_syntax_colors(variant: ThemeSetting) -> SyntaxColors {
    match variant {
        ThemeSetting::Light => SyntaxColors {
            keyword: rgb_to_hsla(ACCENT),
            string: rgb_to_hsla(0x736F30),
            ..SyntaxColors::light()
        },
        ThemeSetting::Dark | ThemeSetting::System => SyntaxColors {
            keyword: rgb_to_hsla(ACCENT_TINT),
            string: rgb_to_hsla(0xCAC580),
            ..SyntaxColors::dark()
        },
    }
}

/// The color `colors` give `role`.
pub fn syntax_role(mut colors: SyntaxColors, role: SyntaxRole) -> Hsla {
    *syntax_role_slot(&mut colors, role)
}

fn syntax_role_slot(colors: &mut SyntaxColors, role: SyntaxRole) -> &mut Hsla {
    match role {
        SyntaxRole::Keyword => &mut colors.keyword,
        SyntaxRole::String => &mut colors.string,
        SyntaxRole::Number => &mut colors.number,
        SyntaxRole::Comment => &mut colors.comment,
        SyntaxRole::Type => &mut colors.type_name,
        SyntaxRole::Function => &mut colors.function,
        SyntaxRole::Operator => &mut colors.operator,
        SyntaxRole::Identifier => &mut colors.plain,
        SyntaxRole::Namespace => &mut colors.namespace,
        SyntaxRole::Field => &mut colors.field,
    }
}

/// `colors` with every parseable `#RRGGBB` entry of `overrides` in place of
/// its role's color. Entries that do not parse are skipped.
fn with_syntax_overrides(
    mut colors: SyntaxColors,
    overrides: &std::collections::BTreeMap<SyntaxRole, String>,
) -> SyntaxColors {
    for (role, text) in overrides {
        if let Some(value) = dbflux_core::parse_hex_color(text) {
            *syntax_role_slot(&mut colors, *role) = rgb_to_hsla(value);
        }
    }
    colors
}

/// The accent colors the user picked in place of the palette's.
#[derive(Default)]
struct AccentOverridesGlobal(AccentColorOverrides);

impl Global for AccentOverridesGlobal {}

/// Store the user's accent colors. They take effect with the next
/// [`apply_theme`].
pub fn set_accent_overrides(overrides: AccentColorOverrides, cx: &mut App) {
    cx.set_global(AccentOverridesGlobal(overrides));
}

/// The user's accent colors, empty until [`set_accent_overrides`] runs.
pub fn accent_overrides(cx: &App) -> Option<&AccentColorOverrides> {
    cx.try_global::<AccentOverridesGlobal>()
        .map(|global| &global.0)
}

/// The palette's own accent for `variant`, before the user's override.
pub fn default_accent(variant: ThemeSetting) -> Hsla {
    match variant {
        ThemeSetting::Light => rgb_to_hsla(ACCENT),
        ThemeSetting::Dark | ThemeSetting::System => rgb_to_hsla(DARK_ACCENT),
    }
}

/// The accent `text` names, or the palette's accent for `variant` when
/// `text` is absent or does not parse.
pub fn accent_or_default(text: Option<&str>, variant: ThemeSetting) -> Hsla {
    text.and_then(dbflux_core::parse_hex_color)
        .map(rgb_to_hsla)
        .unwrap_or_else(|| default_accent(variant))
}

/// DBFlux's syntax colors for `variant` with `overrides` applied.
pub fn syntax_colors_with_overrides(
    variant: ThemeSetting,
    overrides: &std::collections::BTreeMap<SyntaxRole, String>,
) -> SyntaxColors {
    with_syntax_overrides(default_syntax_colors(variant), overrides)
}

/// The user's accent for `variant`, when one is set and parses.
fn accent_override(variant: ThemeSetting, cx: &App) -> Option<Hsla> {
    accent_overrides(cx)?
        .for_variant(variant)
        .and_then(dbflux_core::parse_hex_color)
        .map(rgb_to_hsla)
}

/// Structural separator between major UI regions; resolves to the palette line.
pub fn ghost_border_color(theme: &Theme) -> Hsla {
    bolt_theme::ghost_border_color(theme)
}

pub fn init(cx: &mut App) {
    gpui_component::init(cx);
    crate::highlighting::register_languages();
    load_bundled_fonts(cx);
    install_translator(cx);
    crate::controls::install_keymap_seams(cx);
    crate::vim::install_vim_seams(cx);
    crate::modals::modal::install_modal_seams(cx);
    crate::composites::install_shell_icons(cx);
    apply_theme(ThemeSetting::Dark, AppStyle::Default, None, cx);
}

/// The product's translator for `bolt::strings`, answered from the
/// `dbflux_i18n` catalogue: the same lookups the `t!` macro performs, as
/// plain functions so the components carry no catalogue dependency. The
/// `%{name}` interpolation matches the macro's `replace` semantics exactly.
fn install_translator(cx: &mut App) {
    bolt::strings::install(
        cx,
        bolt::strings::Strings {
            text: |key| dbflux_i18n::translate(key).into(),
            text_with: |key, args| {
                let mut text = dbflux_i18n::translate(key);
                for (name, value) in args {
                    text = text.replace(&format!("%{{{name}}}"), value);
                }
                text.into()
            },
        },
    );
}

/// Initialize the theme, density and font globals from persisted settings.
///
/// Call this after `init` and after the config has been loaded, before
/// the first window opens. This sets up the correct radius tokens, density
/// global and fonts for the first frame.
pub fn init_with_settings(
    setting: ThemeSetting,
    style: AppStyle,
    fonts: FontSettings,
    cx: &mut App,
) {
    crate::density::init(cx, style);
    crate::fonts::init(cx, fonts);
    apply_theme(setting, style, None, cx);
}

/// Write the active font settings into the global theme without changing
/// its palette, so open windows pick them up on their next render.
///
/// Does nothing before the theme has been initialized.
pub fn apply_fonts(cx: &mut App) {
    bolt_theme::apply_fonts(cx);
}

/// Apply the Bolt Byzantium palette for `setting`.
///
/// `ThemeSetting::System` resolves through the OS appearance (the window's when
/// one is given, which is more reliable on Linux). The resolved Dark or Light
/// variant is published to `bolt` so semantic color accessors follow the
/// palette that is actually on screen, and recorded in `ThemeSettingGlobal`
/// as the product-side setting.
pub fn apply_theme(
    setting: ThemeSetting,
    style: AppStyle,
    window: Option<&mut Window>,
    cx: &mut App,
) {
    let resolved = match setting {
        ThemeSetting::System => {
            let appearance = window
                .as_ref()
                .map(|window| window.appearance())
                .unwrap_or_else(|| cx.window_appearance());
            variant_for_appearance(appearance)
        }
        ThemeSetting::Dark | ThemeSetting::Light => setting,
    };

    bolt::brand::install(brand_for(resolved, cx), cx);
    install_syntax_colors(cx);
    bolt_theme::apply_theme(
        palette_mode(resolved),
        crate::density::bolt_density(style),
        window,
        cx,
    );

    ThemeSettingGlobal::set(cx, resolved);
}

/// Map an OS appearance to the palette variant that matches it.
pub fn variant_for_appearance(appearance: WindowAppearance) -> ThemeSetting {
    match appearance {
        WindowAppearance::Dark | WindowAppearance::VibrantDark => ThemeSetting::Dark,
        WindowAppearance::Light | WindowAppearance::VibrantLight => ThemeSetting::Light,
    }
}

fn palette_mode(setting: ThemeSetting) -> PaletteMode {
    match setting {
        ThemeSetting::System => PaletteMode::System,
        ThemeSetting::Dark => PaletteMode::Dark,
        ThemeSetting::Light => PaletteMode::Light,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bolt::theme::rgb_to_hsla_alpha;
    use gpui::TestAppContext;
    use gpui_component::theme::Theme;

    /// Blue is the primary fill of the dark variant, byzantine of the light.
    fn expected_primary(setting: ThemeSetting) -> Hsla {
        default_accent(setting)
    }

    /// Primary buttons fill from `tokens.button_primary` and checked
    /// checkboxes from `tokens.primary`; both must carry the byzantine fill
    /// of the applied setting, not the dependency's default (white in dark
    /// mode). Exercises the production `apply_theme` path.
    #[gpui::test]
    fn component_tokens_follow_byzantine_primary_across_all_settings(cx: &mut TestAppContext) {
        cx.update(gpui_component::init);

        for setting in [
            ThemeSetting::System,
            ThemeSetting::Dark,
            ThemeSetting::Light,
        ] {
            cx.update(|cx| apply_theme(setting, AppStyle::Default, None, cx));
            cx.update(|cx| {
                let theme = Theme::global(cx);
                // System follows the test platform's appearance.
                let expected = expected_primary(if theme.mode.is_dark() {
                    ThemeSetting::Dark
                } else {
                    ThemeSetting::Light
                });

                assert_eq!(
                    theme.tokens.primary.color, expected,
                    "{setting:?}: checkbox fill"
                );
                assert_eq!(
                    theme.tokens.button_primary.color, expected,
                    "{setting:?}: primary button fill"
                );
                assert_eq!(
                    theme.tokens.button_primary_hover.color, theme.colors.primary_hover,
                    "{setting:?}: primary button hover"
                );
                assert_eq!(
                    theme.tokens.button_primary_active.color, theme.colors.primary_active,
                    "{setting:?}: primary button active"
                );
                assert_eq!(
                    theme.tokens.button_primary_foreground.color, theme.colors.primary_foreground,
                    "{setting:?}: primary button foreground"
                );
                assert_eq!(
                    theme.colors.button_primary, expected,
                    "{setting:?}: legacy button field"
                );
            });
        }
    }

    /// Switching settings re-resolves the tokens every time; no palette may
    /// leak into another. Pins the byzantine accent, its tint and the neutral
    /// backgrounds of both variants.
    #[gpui::test]
    fn theme_switches_keep_component_tokens_in_sync(cx: &mut TestAppContext) {
        cx.update(gpui_component::init);

        cx.update(|cx| apply_theme(ThemeSetting::Dark, AppStyle::Default, None, cx));
        cx.update(|cx| apply_theme(ThemeSetting::Light, AppStyle::Default, None, cx));
        cx.update(|cx| {
            let theme = Theme::global(cx);
            assert_eq!(theme.colors.background, rgb_to_hsla(0xF6F4F7));
            let primary = expected_primary(ThemeSetting::Light);
            assert_eq!(theme.tokens.button_primary.color, primary);
            assert_eq!(theme.tokens.primary.color, primary);
            // On light surfaces the accent doubles as the text accent.
            assert_eq!(theme.colors.ring, primary);
        });

        cx.update(|cx| apply_theme(ThemeSetting::Dark, AppStyle::Default, None, cx));
        cx.update(|cx| {
            let theme = Theme::global(cx);
            assert_eq!(theme.colors.background, rgb_to_hsla(0x191A1B));
            let primary = expected_primary(ThemeSetting::Dark);
            assert_eq!(theme.tokens.button_primary.color, primary);
            assert_eq!(theme.tokens.primary.color, primary);
            assert_eq!(theme.colors.ring, rgb_to_hsla(DARK_ACCENT));
        });
    }

    /// The selection and drop-target washes derive from the blue accent on
    /// dark and byzantine on light, at the design-board shares.
    #[gpui::test]
    fn selection_washes_follow_the_byzantine_accent(cx: &mut TestAppContext) {
        cx.update(gpui_component::init);

        cx.update(|cx| apply_theme(ThemeSetting::Dark, AppStyle::Default, None, cx));
        cx.update(|cx| {
            let theme = Theme::global(cx);
            assert_eq!(theme.colors.selection, rgb_to_hsla_alpha(DARK_ACCENT, 0.25));
            assert_eq!(
                theme.colors.drop_target,
                rgb_to_hsla_alpha(DARK_ACCENT, 0.10)
            );
            assert_eq!(
                theme.colors.table_active,
                rgb_to_hsla_alpha(DARK_ACCENT, 0.07)
            );
            assert_eq!(
                theme.colors.list_active,
                rgb_to_hsla_alpha(DARK_ACCENT, 0.12)
            );
        });

        cx.update(|cx| apply_theme(ThemeSetting::Light, AppStyle::Default, None, cx));
        cx.update(|cx| {
            let theme = Theme::global(cx);
            assert_eq!(theme.colors.selection, rgb_to_hsla_alpha(ACCENT, 0.25));
            assert_eq!(theme.colors.drop_target, rgb_to_hsla_alpha(ACCENT, 0.10));
            assert_eq!(theme.colors.table_active, rgb_to_hsla_alpha(ACCENT, 0.07));
            assert_eq!(theme.colors.list_active, rgb_to_hsla_alpha(ACCENT, 0.14));
        });
    }

    #[test]
    fn os_appearance_maps_to_the_matching_variant() {
        assert_eq!(
            variant_for_appearance(WindowAppearance::Dark),
            ThemeSetting::Dark
        );
        assert_eq!(
            variant_for_appearance(WindowAppearance::VibrantDark),
            ThemeSetting::Dark
        );
        assert_eq!(
            variant_for_appearance(WindowAppearance::Light),
            ThemeSetting::Light
        );
        assert_eq!(
            variant_for_appearance(WindowAppearance::VibrantLight),
            ThemeSetting::Light
        );
    }

    /// The semantic color accessors read `ThemeSettingGlobal`, so every
    /// `apply_theme` call must publish the variant it put on screen. The test
    /// platform reports a light appearance, which `System` must follow.
    #[gpui::test]
    fn apply_theme_publishes_the_resolved_variant(cx: &mut TestAppContext) {
        cx.update(gpui_component::init);

        cx.update(|cx| apply_theme(ThemeSetting::Dark, AppStyle::Default, None, cx));
        cx.update(|cx| assert_eq!(ThemeSettingGlobal::get(cx), ThemeSetting::Dark));

        cx.update(|cx| apply_theme(ThemeSetting::Light, AppStyle::Default, None, cx));
        cx.update(|cx| assert_eq!(ThemeSettingGlobal::get(cx), ThemeSetting::Light));

        cx.update(|cx| apply_theme(ThemeSetting::Dark, AppStyle::Default, None, cx));
        cx.update(|cx| apply_theme(ThemeSetting::System, AppStyle::Default, None, cx));
        cx.update(|cx| {
            let expected = variant_for_appearance(cx.window_appearance());
            assert_eq!(ThemeSettingGlobal::get(cx), expected);
            assert_eq!(
                Theme::global(cx).colors.background == rgb_to_hsla(0xF6F4F7),
                expected == ThemeSetting::Light
            );
        });
    }

    fn hex_of(color: Hsla) -> u32 {
        let rgba = gpui::Rgba::from(color);
        let channel = |value: f32| (value * 255.0).round() as u32;

        (channel(rgba.r) << 16) | (channel(rgba.g) << 8) | channel(rgba.b)
    }

    fn syntax_hex(theme: &Theme, capture: &str) -> Option<u32> {
        theme
            .highlight_theme
            .style
            .syntax
            .style(capture)
            .and_then(|style| style.color)
            .map(hex_of)
    }

    /// The SQL editor's highlight theme and the tint accessor follow DBFlux's
    /// syntax roles in both variants. The keyword keeps byzantine (`#D48CC8`
    /// on dark, `#702963` on light) while the tint follows the accent.
    #[gpui::test]
    fn editor_syntax_and_tint_follow_the_palette(cx: &mut TestAppContext) {
        cx.update(gpui_component::init);

        for (setting, keyword_hex, tint_hex) in [
            (ThemeSetting::Dark, ACCENT_TINT, DARK_ACCENT),
            (ThemeSetting::Light, ACCENT, ACCENT),
        ] {
            let expected = default_syntax_colors(setting);
            assert_eq!(
                hex_of(expected.keyword),
                keyword_hex,
                "{setting:?}: keyword role"
            );

            cx.update(|cx| apply_theme(setting, AppStyle::Default, None, cx));
            cx.update(|cx| {
                let theme = Theme::global(cx);

                for (capture, color) in [
                    ("keyword", expected.keyword),
                    ("string", expected.string),
                    ("number", expected.number),
                    ("comment", expected.comment),
                    ("type", expected.type_name),
                    ("function", expected.function),
                    ("operator", expected.operator),
                    ("punctuation.delimiter", expected.operator),
                    ("variable", expected.plain),
                ] {
                    assert_eq!(
                        syntax_hex(theme, capture),
                        Some(hex_of(color)),
                        "{setting:?}: {capture}"
                    );
                }

                assert_eq!(
                    hex_of(crate::tokens::ChromeColors::tint(theme)),
                    tint_hex,
                    "{setting:?}: tint"
                );
            });

            cx.update(|cx| assert_eq!(SyntaxColors::for_current(cx), expected));
        }
    }

    /// The user's colors replace the palette's in the editor and in
    /// `for_current`, only for their own variant, and clearing them restores
    /// the palette.
    #[gpui::test]
    fn syntax_overrides_recolor_only_their_variant(cx: &mut TestAppContext) {
        cx.update(gpui_component::init);

        let mut overrides = SyntaxColorOverrides::default();
        overrides
            .dark
            .insert(SyntaxRole::Keyword, "#112233".to_string());
        overrides
            .dark
            .insert(SyntaxRole::Comment, "not a color".to_string());
        overrides
            .dark
            .insert(SyntaxRole::Field, "#445566".to_string());

        cx.update(|cx| {
            set_syntax_overrides(overrides, cx);
            apply_theme(ThemeSetting::Dark, AppStyle::Default, None, cx);
        });
        cx.update(|cx| {
            let theme = Theme::global(cx);
            assert_eq!(syntax_hex(theme, "keyword"), Some(0x112233));
            assert_eq!(
                syntax_hex(theme, "field"),
                Some(0x445566),
                "the column role takes its override too"
            );
            assert_eq!(
                syntax_hex(theme, "comment"),
                Some(hex_of(default_syntax_colors(ThemeSetting::Dark).comment)),
                "an unparseable entry keeps the palette color"
            );
            assert_eq!(hex_of(SyntaxColors::for_current(cx).keyword), 0x112233);
        });

        cx.update(|cx| apply_theme(ThemeSetting::Light, AppStyle::Default, None, cx));
        cx.update(|cx| {
            assert_eq!(
                syntax_hex(Theme::global(cx), "keyword"),
                Some(hex_of(default_syntax_colors(ThemeSetting::Light).keyword))
            );
        });

        cx.update(|cx| {
            set_syntax_overrides(SyntaxColorOverrides::default(), cx);
            apply_theme(ThemeSetting::Dark, AppStyle::Default, None, cx);
        });
        cx.update(|cx| {
            assert_eq!(
                syntax_hex(Theme::global(cx), "keyword"),
                Some(hex_of(default_syntax_colors(ThemeSetting::Dark).keyword))
            );
        });
    }

    /// The user's accent fills primary buttons and tints the selection only
    /// in its own variant, keeps tinted text readable on dark, picks a
    /// readable foreground, and clearing it restores the default accent.
    #[gpui::test]
    fn accent_override_recolors_only_its_variant(cx: &mut TestAppContext) {
        cx.update(gpui_component::init);

        let overrides = AccentColorOverrides {
            dark: Some("#1F5FD1".to_string()),
            light: Some("#FFD54F".to_string()),
        };
        cx.update(|cx| {
            set_accent_overrides(overrides, cx);
            apply_theme(ThemeSetting::Dark, AppStyle::Default, None, cx);
        });
        cx.update(|cx| {
            let theme = Theme::global(cx);
            let accent = rgb_to_hsla(0x1F5FD1);
            assert_eq!(theme.tokens.button_primary.color, accent);
            assert_eq!(theme.colors.primary_foreground, rgb_to_hsla(0xFFFFFF));
            assert!(
                theme.colors.ring.l >= DARK_TINT_MIN_LIGHTNESS,
                "the dark tint is lightened to stay readable"
            );
            assert_eq!(theme.colors.ring.h, accent.h);
            assert_eq!(
                bolt::semantic::ChartColors::for_current(cx).checkbox_checked,
                theme.colors.ring
            );
        });

        cx.update(|cx| apply_theme(ThemeSetting::Light, AppStyle::Default, None, cx));
        cx.update(|cx| {
            let theme = Theme::global(cx);
            assert_eq!(theme.colors.primary, rgb_to_hsla(0xFFD54F));
            assert_eq!(
                theme.colors.primary_foreground,
                rgb_to_hsla(0x09090B),
                "a light accent takes dark text"
            );
        });

        cx.update(|cx| {
            set_accent_overrides(AccentColorOverrides::default(), cx);
            apply_theme(ThemeSetting::Dark, AppStyle::Default, None, cx);
        });
        cx.update(|cx| {
            let theme = Theme::global(cx);
            assert_eq!(theme.colors.primary, default_accent(ThemeSetting::Dark));
            assert_eq!(theme.colors.ring, rgb_to_hsla(DARK_ACCENT));
        });
    }

    #[test]
    fn hex_round_trips_the_palette_colors() {
        for variant in [ThemeSetting::Dark, ThemeSetting::Light] {
            let colors = default_syntax_colors(variant);
            for role in SyntaxRole::ALL {
                let hex = SyntaxColors::hex(syntax_role(colors, role));
                assert_eq!(
                    dbflux_core::parse_hex_color(&hex),
                    Some(hex_of(syntax_role(colors, role)))
                );
            }
        }
        assert_eq!(
            SyntaxColors::hex(default_syntax_colors(ThemeSetting::Dark).keyword),
            "#D48CC8"
        );
    }
}

use gpui::prelude::*;
use gpui::{
    AbsoluteLength, Animation, AnimationExt, App, Hsla, Transformation, Window, img, percentage,
    svg,
};
use gpui_component::{ActiveTheme, IconNamed};

use crate::icon::IconSource;
use crate::tokens::{ChromeColors, Heights};

#[derive(Clone, Copy)]
enum IconTone {
    Explicit(Hsla),
    Muted,
    Primary,
    Warning,
    Danger,
}

/// Stateless SVG icon primitive with consistent sizing and color support.
///
/// Defaults to `Heights::ICON_SM` (16px) and `theme.muted_foreground`.
/// Use builder methods to override size or color.
#[derive(IntoElement)]
pub struct Icon {
    source: IconSource,
    size: AbsoluteLength,
    tone: Option<IconTone>,
    transformation: Option<Transformation>,
}

impl Icon {
    /// Create an icon from any source (named or SVG path).
    ///
    /// `source` accepts `IconSource`, `IconName`, or any type that
    /// implements `Into<IconSource>`.
    pub fn new(source: impl Into<IconSource>) -> Self {
        Self {
            source: source.into(),
            size: Heights::ICON_SM.into(),
            tone: None,
            transformation: None,
        }
    }

    /// Override the icon size. Accepts pixels or rems; a `tokens::ui` rem
    /// size follows the interface scale.
    pub fn size(mut self, size: impl Into<AbsoluteLength>) -> Self {
        self.size = size.into();
        self
    }

    /// Override the icon color (default: `theme.muted_foreground`).
    pub fn color(mut self, color: impl Into<Hsla>) -> Self {
        self.tone = Some(IconTone::Explicit(color.into()));
        self
    }

    pub fn muted(mut self) -> Self {
        self.tone = Some(IconTone::Muted);
        self
    }

    /// Accent icon: the palette tint (`ChromeColors::tint`), not the accent fill.
    pub fn primary(mut self) -> Self {
        self.tone = Some(IconTone::Primary);
        self
    }

    pub fn warning(mut self) -> Self {
        self.tone = Some(IconTone::Warning);
        self
    }

    pub fn danger(mut self) -> Self {
        self.tone = Some(IconTone::Danger);
        self
    }

    pub fn with_transformation(mut self, transformation: Transformation) -> Self {
        self.transformation = Some(transformation);
        self
    }

    /// Convenience: small icon (16px).
    pub fn small(self) -> Self {
        self.size(Heights::ICON_SM)
    }

    /// Convenience: medium icon (20px).
    pub fn medium(self) -> Self {
        self.size(Heights::ICON_MD)
    }

    /// Convenience: large icon (24px).
    pub fn large(self) -> Self {
        self.size(Heights::ICON_LG)
    }
}

/// One full turn of a spinning loader.
const LOADER_TURN: std::time::Duration = std::time::Duration::from_millis(900);

/// Whether `path` names the loader glyph, by our path or gpui_component's.
fn is_loader(path: &str) -> bool {
    path.ends_with("/loader.svg")
}

/// Whether `path` names a full-color raster mark rather than a monochrome
/// SVG glyph.
fn is_color_mark(path: &str) -> bool {
    path.ends_with(".png")
}

impl RenderOnce for Icon {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = cx.theme();
        let color = match self.tone.unwrap_or(IconTone::Muted) {
            IconTone::Explicit(color) => color,
            IconTone::Muted => theme.muted_foreground,
            IconTone::Primary => ChromeColors::tint(theme),
            IconTone::Warning => theme.warning,
            IconTone::Danger => theme.danger,
        };

        let path = match self.source {
            IconSource::Named(name) => name.path(),
            IconSource::Svg(path) => path,
        };

        // A full-color mark (a brand logo) is a raster drawn as is: neither
        // tinted nor transformed.
        if is_color_mark(&path) {
            return img(path).size(self.size).flex_shrink_0().into_any_element();
        }

        let is_loader = is_loader(&path);
        let icon = svg().path(path).size(self.size).text_color(color);

        // The loader glyph is an open arc that only reads as progress while
        // it turns, so every loader spins.
        if is_loader {
            return icon
                .with_animation(
                    "loader-spin",
                    Animation::new(LOADER_TURN).repeat(),
                    |icon, delta| {
                        icon.with_transformation(Transformation::rotate(percentage(delta)))
                    },
                )
                .into_any_element();
        }

        match self.transformation {
            Some(transformation) => icon.with_transformation(transformation).into_any_element(),
            None => icon.into_any_element(),
        }
    }
}

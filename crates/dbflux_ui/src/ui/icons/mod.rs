pub use dbflux_components::icons::AppIcon;

#[cfg(test)]
mod tests {
    use super::AppIcon;

    #[test]
    fn semantic_driver_fallback_assets_are_registered() {
        for icon in [
            AppIcon::ChartNoAxesColumn,
            AppIcon::Boxes,
            AppIcon::BrandTurso,
        ] {
            assert!(AppIcon::ALL.contains(&icon));
            assert!(icon.embedded_bytes().starts_with(b"<svg"));
        }
    }

    #[test]
    fn schema_toolbar_icons_are_registered() {
        for icon in [
            AppIcon::ZoomIn,
            AppIcon::ZoomOut,
            AppIcon::Grid3x3,
            AppIcon::Minus,
            AppIcon::Snowflake,
            AppIcon::DatabaseZap,
        ] {
            assert!(AppIcon::ALL.contains(&icon), "{icon:?} is not registered");
        }
    }

    #[test]
    fn every_registered_icon_has_a_unique_path_and_matching_bytes() {
        let mut paths = std::collections::HashSet::new();

        for icon in AppIcon::ALL {
            assert!(paths.insert(icon.path()), "duplicate path for {icon:?}");

            let bytes = icon.embedded_bytes();

            // Full-color brand marks are PNGs; every other icon is an SVG.
            if icon.path().ends_with(".png") {
                assert!(
                    bytes.starts_with(b"\x89PNG"),
                    "{icon:?} does not embed a PNG"
                );
            } else {
                let has_svg_root = bytes.windows(4).any(|window| window == b"<svg");
                assert!(has_svg_root, "{icon:?} does not embed an SVG document");
            }
        }
    }
}

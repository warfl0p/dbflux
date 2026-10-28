use super::*;
use dbflux_components::tokens::ShellMetrics;

/// What the sidebar footer says about external scripts folders on the Scripts
/// tab, `None` when there are none.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct ExternalFoldersSummary {
    pub total: usize,
    pub unavailable: usize,
}

impl ExternalFoldersSummary {
    pub(crate) fn new(availabilities: &[&dbflux_core::ScriptRootAvailability]) -> Option<Self> {
        if availabilities.is_empty() {
            return None;
        }

        let unavailable = availabilities
            .iter()
            .filter(|availability| {
                matches!(
                    availability,
                    dbflux_core::ScriptRootAvailability::Unavailable { .. }
                )
            })
            .count();

        Some(Self {
            total: availabilities.len(),
            unavailable,
        })
    }
}

impl Sidebar {
    /// Footer of the Scripts tab: the workspace path or, once external
    /// folders exist, "3 external folders · 1 unavailable". Other tabs and an
    /// empty Scripts tab have no footer.
    pub(super) fn render_footer(&self, cx: &mut Context<Self>) -> Option<impl IntoElement> {
        let theme = cx.theme();

        let state = self.app_state.read(cx);
        let external_summary = match self.active_tab {
            SidebarTab::Scripts => state.scripts_directory().and_then(|dir| {
                let availabilities: Vec<_> = dir
                    .external_roots()
                    .iter()
                    .map(dbflux_core::MountedScriptRoot::availability)
                    .collect();
                ExternalFoldersSummary::new(&availabilities)
            }),
            _ => None,
        };

        // An open workspace shows where it lives, like VS Code's title.
        let workspace_path = match self.active_tab {
            SidebarTab::Scripts => state
                .scripts_directory()
                .and_then(|dir| dir.workspace())
                .map(|workspace| workspace.path().to_string_lossy().to_string()),
            _ => None,
        };

        let content = match external_summary {
            _ if workspace_path.is_some() => div()
                .min_w_0()
                .truncate()
                .children(workspace_path)
                .into_any_element(),
            Some(external) => div()
                .flex()
                .items_center()
                .gap(ShellMetrics::SIDEBAR_FOOTER_GAP)
                .child(dbflux_i18n::t!(
                    "sidebar.status.external_folders",
                    count = external.total
                ))
                .when(external.unavailable > 0, |el| {
                    el.child("\u{b7}")
                        .child(div().text_color(theme.warning).child(dbflux_i18n::t!(
                            "sidebar.status.unavailable_folders",
                            count = external.unavailable
                        )))
                })
                .into_any_element(),
            None => return None,
        };

        Some(
            div()
                .id("sidebar-footer")
                .w_full()
                .flex()
                .flex_shrink_0()
                .items_center()
                .h(ShellMetrics::SIDEBAR_FOOTER_HEIGHT)
                .px(ShellMetrics::SIDEBAR_FOOTER_PADDING_X)
                .text_size(ShellMetrics::SIDEBAR_FOOTER_FONT)
                .text_color(theme.muted_foreground)
                .child(content),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::ExternalFoldersSummary;

    #[test]
    fn external_folder_summary_counts_unavailable_folders() {
        use dbflux_core::ScriptRootAvailability;

        assert_eq!(ExternalFoldersSummary::new(&[]), None);

        let unavailable = ScriptRootAvailability::Unavailable {
            reason: "gone".to_string(),
        };

        assert_eq!(
            ExternalFoldersSummary::new(&[
                &ScriptRootAvailability::Available,
                &ScriptRootAvailability::Pending,
                &unavailable,
            ]),
            Some(ExternalFoldersSummary {
                total: 3,
                unavailable: 1,
            }),
            "a folder still scanning is not reported unavailable"
        );
    }
}

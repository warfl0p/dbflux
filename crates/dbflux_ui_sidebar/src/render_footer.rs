use super::*;
use dbflux_components::tokens::ShellMetrics;

/// What the sidebar footer says about the open workspace folders on the
/// Scripts tab, `None` when no folder is open.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct WorkspaceFoldersSummary {
    pub total: usize,
    pub unavailable: usize,
}

impl WorkspaceFoldersSummary {
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
    /// Footer of the Scripts tab: with one workspace folder open its path,
    /// with more "3 folders in workspace"; either followed by
    /// "· 1 unavailable" when a folder cannot be read. Other tabs and a
    /// Scripts tab without a workspace have no footer.
    pub(super) fn render_footer(&self, cx: &mut Context<Self>) -> Option<impl IntoElement> {
        let theme = cx.theme();

        let state = self.app_state.read(cx);
        let workspace_summary = match self.active_tab {
            SidebarTab::Scripts => state.scripts_directory().and_then(|dir| {
                let availabilities: Vec<_> = dir
                    .external_roots()
                    .iter()
                    .map(dbflux_core::MountedScriptRoot::availability)
                    .collect();
                WorkspaceFoldersSummary::new(&availabilities)
            }),
            _ => None,
        }?;

        // A one-folder workspace shows where it lives, like VS Code's title.
        let workspace_path = state
            .scripts_directory()
            .and_then(|dir| match dir.external_roots() {
                [workspace] => Some(workspace),
                _ => None,
            })
            .map(|workspace| workspace.path().to_string_lossy().to_string());

        let label = workspace_path.unwrap_or_else(|| {
            dbflux_i18n::t!(
                "sidebar.status.workspace_folders",
                count = workspace_summary.total
            )
        });
        let unavailable = dbflux_i18n::t!(
            "sidebar.status.unavailable_folders",
            count = workspace_summary.unavailable
        );

        let content = div()
            .flex()
            .items_center()
            .min_w_0()
            .gap(ShellMetrics::SIDEBAR_FOOTER_GAP)
            .child(div().min_w_0().truncate().child(label))
            .when(workspace_summary.unavailable > 0, |el| {
                el.child("\u{b7}").child(
                    div()
                        .flex_shrink_0()
                        .text_color(theme.warning)
                        .child(unavailable),
                )
            });

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
    use super::WorkspaceFoldersSummary;

    #[test]
    fn workspace_folder_summary_counts_unavailable_folders() {
        use dbflux_core::ScriptRootAvailability;

        assert_eq!(WorkspaceFoldersSummary::new(&[]), None);

        let unavailable = ScriptRootAvailability::Unavailable {
            reason: "gone".to_string(),
        };

        assert_eq!(
            WorkspaceFoldersSummary::new(&[
                &ScriptRootAvailability::Available,
                &ScriptRootAvailability::Pending,
                &unavailable,
            ]),
            Some(WorkspaceFoldersSummary {
                total: 3,
                unavailable: 1,
            }),
            "a folder still scanning is not reported unavailable"
        );
    }
}

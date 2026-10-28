//! Keyboard coverage registries of the workspace shell (see
//! `dbflux_ui_base::keyboard_coverage`). The shell tests in
//! `keyboard_coverage_tests` check rendered frames against them, and the
//! tooltips read them to show each command's shortcut.

use crate::keymap::{Command, ContextId};
use dbflux_ui_base::keyboard_coverage::{KeyboardPath, SurfaceRegistry};

pub(crate) const SHELL: SurfaceRegistry = SurfaceRegistry {
    name: "workspace shell (rail, title bar, status bar, tab bar, empty state)",
    contexts: &[ContextId::Global],
    entries: &[
        (
            "title-bar-drag-area*",
            KeyboardPath::MouseOnly("window chrome: drags the window"),
        ),
        (
            "title-bar-command-search",
            KeyboardPath::Command(Command::ToggleCommandPalette),
        ),
        (
            "notifications-bell",
            KeyboardPath::Command(Command::ToggleNotifications),
        ),
        (
            "rail-connections",
            KeyboardPath::Command(Command::ShowConnectionsView),
        ),
        (
            "rail-scripts",
            KeyboardPath::Command(Command::ShowScriptsView),
        ),
        (
            "rail-dashboards",
            KeyboardPath::Command(Command::ShowDashboardsView),
        ),
        (
            "rail-approvals",
            KeyboardPath::Command(Command::OpenMcpApprovals),
        ),
        (
            "rail-audit",
            KeyboardPath::Command(Command::OpenAuditViewer),
        ),
        (
            "rail-settings",
            KeyboardPath::Command(Command::OpenSettings),
        ),
        ("tasks-toggle", KeyboardPath::Command(Command::ToggleTasks)),
        ("tasks-failed", KeyboardPath::Command(Command::ToggleTasks)),
        (
            "error-badge",
            KeyboardPath::Command(Command::OpenLastErrorInAudit),
        ),
        (
            "status-bar-approvals",
            KeyboardPath::Command(Command::OpenMcpApprovals),
        ),
        ("new-tab-btn", KeyboardPath::Command(Command::NewQueryTab)),
        (
            "tab-close-*",
            KeyboardPath::Command(Command::CloseCurrentTab),
        ),
        ("tab-*", KeyboardPath::Command(Command::NextTab)),
        (
            "empty-start-new_query_tab",
            KeyboardPath::Command(Command::NewQueryTab),
        ),
        (
            "empty-start-toggle_command_palette",
            KeyboardPath::Command(Command::ToggleCommandPalette),
        ),
        (
            "empty-start-open_script_file",
            KeyboardPath::Command(Command::OpenScriptFile),
        ),
        (
            "empty-start-open_connection_manager",
            KeyboardPath::Command(Command::OpenConnectionManager),
        ),
        (
            "empty-recent-*",
            KeyboardPath::Command(Command::OpenScriptFile),
        ),
    ],
};

pub(crate) const SIDEBAR_HEADER: SurfaceRegistry = SurfaceRegistry {
    name: "sidebar header",
    contexts: &[ContextId::Sidebar],
    entries: &[
        // The add menu offers new folders, connections and scripts; the item
        // menu (`m`) lists the same entries for the selected folder.
        ("sidebar-add", KeyboardPath::Command(Command::OpenItemMenu)),
    ],
};

pub(crate) const TASKS_PANEL: SurfaceRegistry = SurfaceRegistry {
    name: "tasks panel",
    contexts: &[ContextId::BackgroundTasks],
    entries: &[
        (
            "tasks-panel-header",
            KeyboardPath::Command(Command::TogglePanel),
        ),
        (
            "tasks-clear-finished",
            KeyboardPath::Command(Command::ClearFinishedTasks),
        ),
        ("task-row-*", KeyboardPath::Command(Command::ExpandCollapse)),
        (
            "cancel-task-button-*",
            KeyboardPath::Command(Command::CancelTask),
        ),
        (
            "dismiss-task-button-*",
            KeyboardPath::Command(Command::Delete),
        ),
    ],
};

pub(crate) const NOTIFICATIONS: SurfaceRegistry = SurfaceRegistry {
    name: "notifications popover",
    contexts: &[ContextId::Notifications],
    entries: &[
        (
            "notifications-close",
            KeyboardPath::Command(Command::ToggleNotifications),
        ),
        (
            "notifications-mark-all-read",
            KeyboardPath::Command(Command::MarkAllNotificationsRead),
        ),
        (
            "notifications-clear-read",
            KeyboardPath::Command(Command::ClearReadNotifications),
        ),
        (
            "notifications-filter-*",
            KeyboardPath::Command(Command::NextPanelTab),
        ),
        (
            "notification-install-*",
            KeyboardPath::Command(Command::InstallUpdate),
        ),
        (
            "notification-later-*",
            KeyboardPath::Command(Command::Delete),
        ),
        ("notification-*", KeyboardPath::Command(Command::Execute)),
    ],
};

pub(crate) const TOASTS: SurfaceRegistry = SurfaceRegistry {
    name: "toasts",
    contexts: &[ContextId::Global],
    entries: &[
        ("toast-action-*", KeyboardPath::Menu("toast-action-*")),
        ("toast-toggle-*", KeyboardPath::Menu("toast-details")),
        ("toast-close-*", KeyboardPath::Menu("toast-dismiss")),
    ],
};

pub(crate) const PANE_ACTIONS_MENU: SurfaceRegistry = SurfaceRegistry {
    name: "pane actions menu",
    contexts: &[ContextId::ContextMenu],
    entries: &[("pane-action-*", KeyboardPath::Command(Command::MenuSelect))],
};

/// Every shell surface's registry, for the tooltip shortcuts.
pub(crate) fn registries() -> Vec<SurfaceRegistry> {
    vec![
        SHELL,
        SIDEBAR_HEADER,
        TASKS_PANEL,
        NOTIFICATIONS,
        TOASTS,
        PANE_ACTIONS_MENU,
    ]
}

/// Makes every tooltip of a button that runs a command name the command's
/// shortcut. Call once at startup, after the keymap is installed.
pub fn install_shortcut_tooltips(cx: &mut gpui::App) {
    let mut registries = registries();
    registries.extend(dbflux_ui_document::keyboard_coverage::registries());
    registries.extend(dbflux_ui_windows::keyboard_coverage::registries());
    registries.push(dbflux_ui_base::keyboard_coverage::MODAL_CHROME);

    dbflux_ui_base::keyboard_coverage::install_element_shortcuts(&registries, cx);
}

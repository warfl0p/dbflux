use super::*;

impl Workspace {
    pub(super) fn dispatch_scripts(
        &mut self,
        cmd: Command,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Option<bool> {
        match cmd {
            Command::OpenScriptFile => {
                self.open_script_file(window, cx);
                Some(true)
            }
            Command::OpenRecentWorkspace => {
                self.open_recent_workspaces(window, cx);
                Some(true)
            }
            Command::RemoveFolderFromWorkspace => {
                self.open_remove_workspace_folder(window, cx);
                Some(true)
            }
            Command::AddExternalScriptsFolder
            | Command::OpenWorkspaceFromFile
            | Command::AddFolderToWorkspace
            | Command::SaveWorkspaceAs
            | Command::CloseWorkspace => {
                self.reveal_sidebar_view(SidebarTab::Scripts, cx);
                self.sidebar.update(cx, |sidebar, cx| match cmd {
                    Command::OpenWorkspaceFromFile => sidebar.open_workspace_from_file(cx),
                    Command::AddFolderToWorkspace => sidebar.add_folder_to_workspace(cx),
                    Command::SaveWorkspaceAs => sidebar.save_workspace_as(cx),
                    Command::CloseWorkspace => sidebar.close_workspace(cx),
                    _ => sidebar.open_workspace_folder(cx),
                });
                Some(true)
            }
            _ => None,
        }
    }
}

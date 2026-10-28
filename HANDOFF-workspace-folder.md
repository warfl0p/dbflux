# Handoff: feat/workspace-folder

Notes for whoever picks this branch up next. Delete this file before a PR.

## Goal

Make the Scripts sidebar workspace work like VS Code:

- Open folder
- Open Recent (Ctrl+R)
- Add folder to workspace
- Remove folder from workspace
- Close workspace
- A multi-root tree

The user does not want to save a workspace to a file. In VS Code they never have to, so the workspace-file commands were removed.

## Behaviour

- **Open folder…** (palette id `add_external_scripts_folder`): the picked folder replaces the open workspace.
- **Add folder to workspace…**: adds a folder next to the open ones. The Scripts + menu offers it too.
- **Remove folder from workspace…**: opens a picker of the open folders.
- **Close workspace**: unregisters every external folder. No files are touched.
- **Open recent…** (Ctrl+R, global): lists earlier folder sets, without the one that is open.
  - Stored as JSON in `st_ui_state` under the key `recent_workspaces`.
  - Opening a workspace records the one it replaces. Add and Remove record the new set.
  - The Scripts + menu "Recent folders" list reads the same store.
- **Restart**: the open folders come back without any file, because they live in the `script_roots` table.
- **Tree**:
  - No external folder: the managed scripts root.
  - One folder: its contents, flat.
  - Two or more folders: one node per folder.
- **Go to file**: with several folders open, each file is prefixed with its folder name.

## Last session (2026-10-28)

1. Removed "Open workspace from file…" and "Save workspace as…":
   - `WorkspaceFile` in dbflux_core.
   - `RecentWorkspace.file`.
   - Both `Command` variants, their dispatch, palette entries, labels, and the i18n keys in all 5 locales.
   - Old recents JSON that has a `file` field still parses, because serde ignores unknown fields.
2. Fixed the palette search. Typing `workspace` after Ctrl+Shift+P (no `>` prefix) is a mixed query. A mixed query kept only 3 commands (`MIXED_QUERY_COMMAND_LIMIT`), even when no connection or table matched. Now the cap applies only when the query also matches a non-command item. See `arrange_results` in `crates/dbflux_ui/src/ui/overlays/command_palette.rs`, which has a test.

Checks run after this session's changes:

- `cargo nextest run` for dbflux_core, dbflux_app, dbflux_ui_sidebar, dbflux_i18n and dbflux_ui_base: 2667 passed.
- `cargo test -p dbflux_ui --lib command_palette`: 23 passed.
- `cargo clippy -p dbflux_ui`: no new warnings.

This session did not check the changes in the running app.

## Still to do

- [ ] Check in the app, with isolated XDG dirs as AGENTS.md describes:
  - The palette `workspace` search shows all 6 workspace commands.
  - Open folder makes the picked folder the workspace.
  - The folder survives a restart.
- [ ] Merge this branch into the fork's main. Fork main (008b8a41) still has the older version with the workspace-file commands. The memory notes cover the rules: merge commits only, never reset or force-push main. Then rebuild the improved app with the `update-improved` skill.
- [ ] `feat/compact-chrome` still holds commit `0d793bc5`. That commit adds a second Ctrl+R command (`OpenRecentFolder`). Drop the commit before that branch becomes a PR.
- [ ] Optional: in VS Code, Ctrl+Shift+P prefills `>`, so it shows only commands. Ask the user before changing this.
- [ ] Optional: add a Ctrl+K F shortcut for Close workspace.
- [ ] Optional: the multi-root footer says "N external folders". Consider a better label.
- [ ] The new strings in es, ko, pt_BR and zh_Hans are still English.
- [ ] Before a PR, follow the label and issue rules in AGENTS.md and CONTRIBUTING.md. Features need an assigned issue first.

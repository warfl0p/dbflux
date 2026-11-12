//! Follows the file behind a file-backed document when another program
//! changes or removes it.
//!
//! The document compares the file with the bytes it last loaded or wrote (the
//! write queue's baseline), so its own saves never count as outside changes.
//! A clean buffer reloads silently; a buffer with unsaved edits keeps them and
//! shows a banner offering to reload or to save over the file.

use super::file_persistence::{FileBaseline, read_disk_state};
use super::*;
use dbflux_components::controls::Button;
use dbflux_components::primitives::Icon;
use dbflux_components::tokens::{EditorMetrics, Fields};
use dbflux_ui_base::file_watcher::{FileWatcher, FilesChanged, FolderWatch};
use dbflux_ui_base::user_error::{ErrorKind, UserFacingError, report_error};

/// How the file on disk relates to the buffer.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(super) enum DiskState {
    #[default]
    InSync,
    /// Another program wrote the file while the buffer had unsaved edits.
    Changed,
    /// The file is gone.
    Deleted,
}

/// What one look at the file means for the document.
#[derive(Debug, PartialEq, Eq)]
enum DiskOutcome {
    /// The file holds exactly the buffer: it counts as saved.
    MatchesBuffer,
    /// The file still holds what the document last loaded or wrote.
    Unchanged,
    /// Changed outside, and the buffer has nothing to lose.
    Reload,
    /// Changed outside while the buffer has unsaved edits.
    Conflict,
    Deleted,
}

fn classify(disk: Option<&str>, buffer: &str, baseline: Option<&str>, dirty: bool) -> DiskOutcome {
    match disk {
        None => DiskOutcome::Deleted,
        Some(bytes) if bytes == buffer => DiskOutcome::MatchesBuffer,
        Some(bytes) if baseline == Some(bytes) => DiskOutcome::Unchanged,
        Some(_) if !dirty => DiskOutcome::Reload,
        Some(_) => DiskOutcome::Conflict,
    }
}

pub(super) struct DiskSync {
    /// `None` without an app file watcher (tests); the document then never
    /// checks the disk on its own.
    watch: Option<FolderWatch>,
    watched_folder: Option<PathBuf>,
    _subscription: Option<Subscription>,
    _check: Option<Task<()>>,
    pub(super) state: DiskState,
    /// File bytes waiting for a `Window` to replace the buffer with.
    reload: Option<String>,
}

impl DiskSync {
    pub(super) fn new(cx: &mut Context<CodeDocument>) -> Self {
        let watcher = FileWatcher::global(cx);
        let watch = watcher
            .as_ref()
            .map(|watcher| watcher.read(cx).folder_watch());
        let subscription = watcher.map(|watcher| {
            cx.subscribe(&watcher, |document, _, event: &FilesChanged, cx| {
                document.on_files_changed(event, cx);
            })
        });

        Self {
            watch,
            watched_folder: None,
            _subscription: subscription,
            _check: None,
            state: DiskState::InSync,
            reload: None,
        }
    }
}

impl DiskSync {
    /// Whether outside changes reach this document, so the banner can report
    /// them.
    pub(super) fn is_watching(&self) -> bool {
        self.watch.is_some()
    }
}

impl CodeDocument {
    /// Watches the folder holding this document's file, following renames and
    /// Save As. Checks the file once whenever the folder changes, to catch a
    /// change made between loading it and watching it.
    pub(super) fn sync_disk_watch(&mut self, cx: &mut Context<Self>) {
        let folder = self
            .editor
            .path
            .as_deref()
            .and_then(Path::parent)
            .map(Path::to_path_buf);
        if folder == self.disk_sync.watched_folder {
            return;
        }
        let Some(watch) = self.disk_sync.watch.as_mut() else {
            return;
        };

        watch.set_folders(folder.iter().cloned().collect());
        self.disk_sync.watched_folder = folder;
        self.check_disk_change(cx);
    }

    fn on_files_changed(&mut self, event: &FilesChanged, cx: &mut Context<Self>) {
        // `starts_with` also matches the removal of a folder holding the file.
        let touched = self.editor.path.as_deref().is_some_and(|path| {
            event
                .changed
                .iter()
                .any(|changed| path.starts_with(changed))
        });
        if touched {
            self.check_disk_change(cx);
        }
    }

    /// Reads the file in the background and reacts to what it holds. Skipped
    /// while a write of ours is queued; the queue checks again once it drains.
    pub(super) fn check_disk_change(&mut self, cx: &mut Context<Self>) {
        if self.read_only || self.disk_sync.watch.is_none() || self.physical_writes.has_pending() {
            return;
        }
        let Some(path) = self.editor.path.clone() else {
            return;
        };

        let read = cx.background_executor().spawn({
            let path = path.clone();
            async move { read_disk_state(&path) }
        });

        self.disk_sync._check = Some(cx.spawn(async move |this, cx| {
            let disk = read.await;
            if let Err(error) = this.update(cx, |document, cx| {
                document.apply_disk_state(&path, disk, cx)
            }) {
                log::debug!("disk check for {} dropped: {error}", path.display());
            }
        }));
    }

    fn apply_disk_state(
        &mut self,
        path: &Path,
        disk: std::io::Result<Option<String>>,
        cx: &mut Context<Self>,
    ) {
        // The document moved or started a write while the read ran.
        if self.editor.path.as_deref() != Some(path) || self.physical_writes.has_pending() {
            return;
        }

        let disk = match disk {
            Ok(disk) => disk,
            Err(error) => {
                log::warn!(
                    "could not check {} for outside changes: {error}",
                    path.display()
                );
                return;
            }
        };

        let buffer = self.editor.input_state.read(cx).value();
        let baseline = self
            .physical_writes
            .baseline()
            .filter(|baseline| baseline.path == path)
            .map(|baseline| baseline.bytes.as_str());

        let state = match classify(disk.as_deref(), &buffer, baseline, self.editor.is_dirty) {
            DiskOutcome::MatchesBuffer => {
                if let Some(bytes) = disk {
                    self.physical_writes
                        .adopt_baseline(Some(FileBaseline::new(path.to_path_buf(), bytes)));
                }
                self.mark_clean(cx);
                DiskState::InSync
            }
            DiskOutcome::Unchanged => DiskState::InSync,
            DiskOutcome::Reload => {
                self.disk_sync.reload = disk;
                DiskState::InSync
            }
            DiskOutcome::Conflict => DiskState::Changed,
            DiskOutcome::Deleted => DiskState::Deleted,
        };

        self.disk_sync.state = state;
        cx.notify();
    }

    /// Replaces the buffer with the file's current contents, discarding
    /// unsaved edits. The "Reload from Disk" pane action and banner button.
    pub(super) fn reload_from_disk(&mut self, cx: &mut Context<Self>) {
        let Some(path) = self.editor.path.clone() else {
            return;
        };

        let read = cx.background_executor().spawn({
            let path = path.clone();
            async move { read_disk_state(&path) }
        });

        self.disk_sync._check = Some(cx.spawn(async move |this, cx| {
            let disk = read.await;
            let applied = this.update(cx, |document, cx| match disk {
                Ok(Some(bytes)) => {
                    document.disk_sync.reload = Some(bytes);
                    cx.notify();
                }
                Ok(None) => report_error(
                    UserFacingError::new(
                        ErrorKind::User,
                        dbflux_i18n::t!(
                            "document.code.disk.missing",
                            path = path.display().to_string()
                        ),
                    ),
                    cx,
                ),
                Err(error) => report_error(
                    UserFacingError::new(
                        ErrorKind::Storage,
                        dbflux_i18n::t!(
                            "document.code.disk.reload_failed",
                            path = path.display().to_string()
                        ),
                    )
                    .with_cause(error.to_string()),
                    cx,
                ),
            });
            if let Err(error) = applied {
                log::debug!("reload dropped: {error}");
            }
        }));
    }

    /// Swaps in file contents read by a check or a reload, keeping the cursor
    /// and scroll position where the new text allows.
    pub(super) fn apply_pending_disk_reload(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(bytes) = self.disk_sync.reload.take() else {
            return;
        };
        let Some(path) = self.editor.path.clone() else {
            return;
        };

        let input = self.editor.input_state.clone();
        let (cursor, scroll, focused) = {
            let state = input.read(cx);
            (
                state.cursor_position(),
                state.scroll_offset(),
                state.focus_handle(cx).is_focused(window),
            )
        };

        // Drops the debounce and session shadow of the edits being discarded.
        self.mark_clean(cx);
        self.set_content(&bytes, window, cx);
        self.physical_writes
            .adopt_baseline(Some(FileBaseline::new(path, bytes)));

        input.update(cx, |state, cx| {
            // Moving the cursor focuses the editor, so only restore it where it
            // already had focus.
            if focused {
                state.set_cursor_position(cursor, window, cx);
            }
            state.set_scroll_offset(scroll, cx);
        });

        self.disk_sync.state = DiskState::InSync;
        cx.emit(DocumentEvent::MetaChanged);
        cx.notify();
    }

    /// The bar above the editor while the file and the buffer disagree.
    pub(super) fn render_disk_banner(&self, cx: &mut Context<Self>) -> Option<AnyElement> {
        let (message, save_label) = match self.disk_sync.state {
            DiskState::InSync => return None,
            DiskState::Changed => (
                dbflux_i18n::t!("document.code.disk.changed"),
                dbflux_i18n::t!("document.code.disk.keep_mine"),
            ),
            DiskState::Deleted => (
                dbflux_i18n::t!("document.code.disk.deleted"),
                dbflux_i18n::t!("document.code.disk.save_again"),
            ),
        };
        let theme = cx.theme();

        Some(
            div()
                .id("disk-banner")
                .flex()
                .flex_shrink_0()
                .items_center()
                .gap(EditorMetrics::BANNER_GAP)
                .h(EditorMetrics::BANNER_HEIGHT)
                .px(EditorMetrics::BANNER_PADDING_X)
                .bg(theme.warning.opacity(EditorMetrics::BANNER_FILL_ALPHA))
                .border_b_1()
                .border_color(theme.warning.opacity(EditorMetrics::BANNER_LINE_ALPHA))
                .text_size(Fields::TEXT)
                .child(
                    Icon::new(AppIcon::TriangleAlert)
                        .size(EditorMetrics::BANNER_ICON)
                        .color(theme.warning),
                )
                .child(
                    div()
                        .flex_1()
                        .min_w_0()
                        .truncate()
                        .text_color(theme.foreground)
                        .child(message),
                )
                .when(self.disk_sync.state == DiskState::Changed, |banner| {
                    banner.child(
                        Button::new(
                            "disk-banner-reload",
                            dbflux_i18n::t!("document.code.disk.reload"),
                        )
                        .inline()
                        .ghost()
                        .on_click(cx.listener(|this, _, _, cx| this.reload_from_disk(cx))),
                    )
                })
                .child(
                    Button::new("disk-banner-save", save_label)
                        .inline()
                        .ghost()
                        .on_click(cx.listener(|this, _, window, cx| this.save_file(window, cx))),
                )
                .into_any_element(),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::{DiskOutcome, classify};

    #[test]
    fn classify_spares_unsaved_edits_and_ignores_own_writes() {
        assert_eq!(classify(None, "a", Some("a"), false), DiskOutcome::Deleted);
        assert_eq!(
            classify(Some("b"), "b", Some("a"), true),
            DiskOutcome::MatchesBuffer
        );
        assert_eq!(
            classify(Some("a"), "a2", Some("a"), true),
            DiskOutcome::Unchanged
        );
        assert_eq!(
            classify(Some("b"), "a", Some("a"), false),
            DiskOutcome::Reload
        );
        assert_eq!(
            classify(Some("b"), "a2", Some("a"), true),
            DiskOutcome::Conflict
        );
    }
}

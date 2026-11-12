//! Notices files changed by other programs.
//!
//! One OS watcher serves the whole app. Each owner (an open script, the
//! scripts sidebar) holds a [`FolderWatch`] naming the folders it cares about;
//! the watcher follows the union of them. Every folder is watched on its own,
//! never recursively, so a folder the scripts scan skips (`.git`, hidden
//! folders) costs no watch.
//!
//! Events are batched for [`DEBOUNCE`] and emitted as one [`FilesChanged`] on
//! the [`FileWatcher`] entity, which owners subscribe to.

use futures::StreamExt as _;
use futures::channel::mpsc;
use gpui::{App, AppContext as _, Context, Entity, EventEmitter, Global, Task};
use notify::event::ModifyKind;
use notify::{EventKind, RecursiveMode, Watcher as _};
use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::rc::{Rc, Weak};
use std::time::Duration;

/// How long a burst of events is collected before it is reported. Editors
/// that save through a temporary file and a rename produce several events for
/// one save.
const DEBOUNCE: Duration = Duration::from_millis(200);

/// Paths touched outside DBFlux since the previous batch.
#[derive(Debug, Default, Clone)]
pub struct FilesChanged {
    /// Every path an event named: written, created, removed or renamed.
    pub changed: HashSet<PathBuf>,
    /// The paths that appeared, disappeared or were renamed, which change what
    /// a folder lists.
    pub added_or_removed: HashSet<PathBuf>,
}

impl FilesChanged {
    fn add(&mut self, event: notify::Event) {
        let lists_change = matches!(
            event.kind,
            EventKind::Any
                | EventKind::Other
                | EventKind::Create(_)
                | EventKind::Remove(_)
                | EventKind::Modify(ModifyKind::Name(_) | ModifyKind::Any)
        );

        for path in event.paths {
            if lists_change {
                self.added_or_removed.insert(path.clone());
            }
            self.changed.insert(path);
        }
    }
}

/// The folders being watched and who asked for them.
struct Watches {
    /// `None` when the platform watcher could not be created; every watch is
    /// then a no-op and DBFlux behaves as it did before watching.
    watcher: Option<notify::RecommendedWatcher>,
    owners: HashMap<u64, HashSet<PathBuf>>,
    active: HashSet<PathBuf>,
    next_owner: u64,
}

impl Watches {
    /// Brings the OS watches in line with the union of the owners' folders.
    /// Removals run first, so a folder that leaves one owner and joins another
    /// is never briefly unwatched by a stale removal.
    fn reconcile(&mut self) {
        let desired: HashSet<PathBuf> = self.owners.values().flatten().cloned().collect();

        let Some(watcher) = self.watcher.as_mut() else {
            return;
        };

        for gone in self.active.difference(&desired) {
            // A deleted folder has already lost its watch, so this can fail
            // harmlessly.
            if let Err(error) = watcher.unwatch(gone) {
                log::debug!("file watcher: unwatch {}: {error}", gone.display());
            }
        }
        self.active.retain(|folder| desired.contains(folder));

        for folder in desired {
            if self.active.contains(&folder) {
                continue;
            }
            match watcher.watch(&folder, RecursiveMode::NonRecursive) {
                Ok(()) => {
                    self.active.insert(folder);
                }
                Err(error) => log::warn!("file watcher: watch {}: {error}", folder.display()),
            }
        }
    }
}

/// The app's single file watcher. Subscribe to it for [`FilesChanged`].
pub struct FileWatcher {
    watches: Rc<RefCell<Watches>>,
    _events: Task<()>,
}

struct FileWatcherGlobal(Entity<FileWatcher>);

impl Global for FileWatcherGlobal {}

impl EventEmitter<FilesChanged> for FileWatcher {}

impl FileWatcher {
    /// Starts the watcher once per app. Later calls do nothing.
    pub fn init(cx: &mut App) {
        if cx.has_global::<FileWatcherGlobal>() {
            return;
        }
        let watcher = cx.new(Self::new);
        cx.set_global(FileWatcherGlobal(watcher));
    }

    /// The app's watcher, or `None` before [`FileWatcher::init`] (tests).
    pub fn global(cx: &App) -> Option<Entity<FileWatcher>> {
        cx.try_global::<FileWatcherGlobal>()
            .map(|global| global.0.clone())
    }

    fn new(cx: &mut Context<Self>) -> Self {
        let (sender, mut receiver) = mpsc::unbounded::<notify::Event>();

        let watcher = notify::recommended_watcher(move |result: notify::Result<notify::Event>| {
            match result {
                // Reading a file (ours included) is not a change.
                Ok(event) if matches!(event.kind, EventKind::Access(_)) => {}
                Ok(event) => {
                    if let Err(error) = sender.unbounded_send(event) {
                        log::debug!("file watcher: receiver gone: {error}");
                    }
                }
                Err(error) => log::warn!("file watcher: {error}"),
            }
        });

        let watcher = match watcher {
            Ok(watcher) => Some(watcher),
            Err(error) => {
                log::warn!("file watcher unavailable, outside changes go unnoticed: {error}");
                None
            }
        };

        let events = cx.spawn(async move |this, cx| {
            while let Some(first) = receiver.next().await {
                cx.background_executor().timer(DEBOUNCE).await;

                let mut batch = FilesChanged::default();
                batch.add(first);
                while let Ok(event) = receiver.try_recv() {
                    batch.add(event);
                }

                if this.update(cx, |_, cx| cx.emit(batch)).is_err() {
                    break;
                }
            }
        });

        Self {
            watches: Rc::new(RefCell::new(Watches {
                watcher,
                owners: HashMap::new(),
                active: HashSet::new(),
                next_owner: 0,
            })),
            _events: events,
        }
    }

    /// A new, empty set of folders to watch. Dropping it stops its watches.
    pub fn folder_watch(&self) -> FolderWatch {
        let mut watches = self.watches.borrow_mut();
        let owner = watches.next_owner;
        watches.next_owner += 1;
        watches.owners.insert(owner, HashSet::new());

        FolderWatch {
            watches: Rc::downgrade(&self.watches),
            owner,
            folders: HashSet::new(),
        }
    }
}

/// One owner's folders. Watching stops when it is dropped.
pub struct FolderWatch {
    watches: Weak<RefCell<Watches>>,
    owner: u64,
    folders: HashSet<PathBuf>,
}

impl FolderWatch {
    /// Watches exactly `folders` for this owner. Unchanged sets cost nothing,
    /// so callers can pass their current set on every refresh.
    pub fn set_folders(&mut self, folders: HashSet<PathBuf>) {
        if folders == self.folders {
            return;
        }
        self.folders = folders.clone();

        if let Some(watches) = self.watches.upgrade() {
            let mut watches = watches.borrow_mut();
            watches.owners.insert(self.owner, folders);
            watches.reconcile();
        }
    }
}

impl Drop for FolderWatch {
    fn drop(&mut self) {
        if let Some(watches) = self.watches.upgrade() {
            let mut watches = watches.borrow_mut();
            watches.owners.remove(&self.owner);
            watches.reconcile();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use notify::event::{CreateKind, DataChange, RemoveKind};

    fn event(kind: EventKind, path: &str) -> notify::Event {
        notify::Event::new(kind).add_path(PathBuf::from(path))
    }

    #[test]
    fn content_writes_do_not_count_as_listing_changes() {
        let mut batch = FilesChanged::default();
        batch.add(event(
            EventKind::Modify(ModifyKind::Data(DataChange::Content)),
            "/s/a.sql",
        ));
        batch.add(event(EventKind::Create(CreateKind::File), "/s/b.sql"));
        batch.add(event(EventKind::Remove(RemoveKind::Folder), "/s/old"));

        assert_eq!(batch.changed.len(), 3);
        assert!(!batch.added_or_removed.contains(&PathBuf::from("/s/a.sql")));
        assert!(batch.added_or_removed.contains(&PathBuf::from("/s/b.sql")));
        assert!(batch.added_or_removed.contains(&PathBuf::from("/s/old")));
    }

    #[test]
    fn a_folder_stays_watched_while_any_owner_wants_it() {
        let dir = tempfile::tempdir().expect("tempdir");
        let folder = dir.path().to_path_buf();
        let watches = Rc::new(RefCell::new(Watches {
            watcher: notify::recommended_watcher(|_: notify::Result<notify::Event>| {}).ok(),
            owners: HashMap::new(),
            active: HashSet::new(),
            next_owner: 0,
        }));
        let watch = |owner| FolderWatch {
            watches: Rc::downgrade(&watches),
            owner,
            folders: HashSet::new(),
        };

        let mut first = watch(0);
        let mut second = watch(1);
        first.set_folders(HashSet::from([folder.clone()]));
        second.set_folders(HashSet::from([folder.clone()]));
        assert!(watches.borrow().active.contains(&folder));

        drop(first);
        assert!(watches.borrow().active.contains(&folder));

        drop(second);
        assert!(watches.borrow().active.is_empty());
    }
}

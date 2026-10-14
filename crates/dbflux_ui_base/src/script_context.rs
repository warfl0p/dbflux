//! The connection, database and schema a script file runs against, kept in
//! `dbflux.db` under the file's path so the file itself holds only the script.

use std::path::Path;

use dbflux_core::{ExecutionContext, LogErr};
use dbflux_storage::StorageRuntime;

fn key(path: &Path) -> String {
    format!("script-context:{}", path.display())
}

/// The context stored for the script at `path`, if one was saved.
pub fn load(storage: &StorageRuntime, path: &Path) -> Option<ExecutionContext> {
    let json = storage.ui_state().get(&key(path)).log_err()??;

    serde_json::from_str(&json)
        .map_err(|error| {
            log::warn!(
                "Ignoring unreadable script context for {}: {error}",
                path.display()
            );
        })
        .ok()
}

/// Stores `context` as the one the script at `path` runs against.
pub fn save(storage: &StorageRuntime, path: &Path, context: &ExecutionContext) {
    let Some(json) = serde_json::to_string(context).log_err() else {
        return;
    };

    storage.ui_state().set(&key(path), &json).log_err();
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    #[test]
    fn a_saved_context_is_loaded_back_for_the_same_path_only() {
        let storage = StorageRuntime::in_memory().expect("in-memory storage");
        let context = ExecutionContext {
            connection_id: Some(Uuid::new_v4()),
            database: Some("mobilityplus".into()),
            ..ExecutionContext::default()
        };

        save(&storage, Path::new("/scripts/a.sql"), &context);

        assert_eq!(load(&storage, Path::new("/scripts/a.sql")), Some(context));
        assert_eq!(load(&storage, Path::new("/scripts/b.sql")), None);
    }
}

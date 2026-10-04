//! Writable client files and non-destructive import of the previous user profile.

use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};

const IMPORT_MARKER: &str = ".user-data-imported";
/// SJK's client folder in `GameData`. JKR names its own `jkr`.
const CLIENT_FOLDER: &str = "SJK";
/// JKR's client folder in `GameData`, imported from once like the per-user one.
const JKR_FOLDER: &str = "jkr";

/// The configuration path shared by the console, menus and storage consumers.
pub(super) struct Selection {
    pub(super) config: PathBuf,
    pub(super) fallback_reason: Option<io::Error>,
}

/// Prefer the installation's own client folder; probe actual file creation
/// rather than permission bits, which do not describe Windows ACLs reliably.
pub(super) fn select(game_data: &Path, legacy_config: Option<&Path>) -> io::Result<Selection> {
    let portable = game_data.join(CLIENT_FOLDER);
    match writable_directory(&portable) {
        Ok(()) => {
            // JKR's GameData folder first, then the older per-user folder; a file
            // imported from the first is not replaced by the second.
            let jkr = game_data.join(JKR_FOLDER);
            let sources = [Some(jkr.as_path()), legacy_config.and_then(Path::parent)];
            import_profile(sources.into_iter().flatten(), &portable)?;
            Ok(Selection {
                config: portable.join("config.cfg"),
                fallback_reason: None,
            })
        }
        Err(reason) => {
            let config = legacy_config.ok_or_else(|| {
                io::Error::other(format!(
                    "cannot write {}: {reason}; no user storage location available",
                    portable.display()
                ))
            })?;
            let directory = config
                .parent()
                .ok_or_else(|| io::Error::other("config has no parent"))?;
            writable_directory(directory)?;
            Ok(Selection {
                config: config.to_owned(),
                fallback_reason: Some(reason),
            })
        }
    }
}

fn writable_directory(directory: &Path) -> io::Result<()> {
    fs::create_dir_all(directory)?;
    let mut probe = tempfile::NamedTempFile::new_in(directory)?;
    probe.write_all(b"JKR")?;
    probe.close()
}

// Import only client-owned file categories. Retail packs, downloads and caches
// are not profile data. Never follow links or remove the old files.
fn import_profile<'a>(
    sources: impl IntoIterator<Item = &'a Path>,
    destination: &Path,
) -> io::Result<()> {
    let marker = destination.join(IMPORT_MARKER);
    if marker.try_exists()? {
        return Ok(());
    }
    for source in sources {
        if source.try_exists()? && fs::canonicalize(source)? != fs::canonicalize(destination)? {
            import_files(source, destination)?;
        }
    }
    // Publish only after a complete import. A failed import can be retried;
    // successful imports never resurrect files the player subsequently deletes.
    let mut pending = tempfile::NamedTempFile::new_in(destination)?;
    pending.write_all(b"Imported previous JKR user files. Existing files were retained.\n")?;
    publish(pending, &marker)
}

fn import_files(source: &Path, destination: &Path) -> io::Result<()> {
    for entry in fs::read_dir(source)? {
        let entry = entry?;
        let kind = entry.file_type()?;
        let name = entry.file_name();
        let path = entry.path();
        if kind.is_file()
            && (path
                .extension()
                .is_some_and(|extension| extension.eq_ignore_ascii_case("cfg"))
                || matches!(
                    name.to_str(),
                    Some(
                        "marks.txt"
                            | "jakey"
                            | "favorites.json"
                            | "chat-friends.txt"
                            | "hud.json"
                            | "qconsole.log"
                            | "debug_panel_tested.txt"
                    )
                ))
        {
            copy_missing(&path, &destination.join(name))?;
        } else if kind.is_dir()
            && matches!(
                name.to_str(),
                Some("screenshots" | "demos" | "chatlogs" | "configs")
            )
        {
            copy_directory(&path, &destination.join(name))?;
        }
    }
    Ok(())
}

fn copy_directory(source: &Path, destination: &Path) -> io::Result<()> {
    if destination
        .symlink_metadata()
        .is_ok_and(|metadata| metadata.file_type().is_symlink())
    {
        return Err(io::Error::other(format!(
            "refusing to import into linked directory {}",
            destination.display()
        )));
    }
    fs::create_dir_all(destination)?;
    for entry in fs::read_dir(source)? {
        let entry = entry?;
        let kind = entry.file_type()?;
        let path = entry.path();
        let target = destination.join(entry.file_name());
        if kind.is_dir() {
            copy_directory(&path, &target)?;
        } else if kind.is_file()
            && !path
                .extension()
                .is_some_and(|extension| extension.eq_ignore_ascii_case("pk3"))
        {
            copy_missing(&path, &target)?;
        }
    }
    Ok(())
}

fn copy_missing(source: &Path, destination: &Path) -> io::Result<()> {
    match destination.symlink_metadata() {
        Ok(_) => return Ok(()),
        Err(error) if error.kind() == io::ErrorKind::NotFound => {}
        Err(error) => return Err(error),
    }
    let mut input = fs::File::open(source)?;
    let mut pending =
        tempfile::NamedTempFile::new_in(destination.parent().expect("profile child"))?;
    io::copy(&mut input, &mut pending)?;
    publish(pending, destination)
}

fn publish(pending: tempfile::NamedTempFile, destination: &Path) -> io::Result<()> {
    pending.as_file().sync_all()?;
    match pending.persist_noclobber(destination) {
        Ok(_) => Ok(()),
        Err(error) if error.error.kind() == io::ErrorKind::AlreadyExists => Ok(()),
        Err(error) => Err(error.error),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sjk_folder_imports_jkr_then_per_user_files_once() {
        let root = tempfile::tempdir().unwrap();
        let game_data = root.path().join("GameData");
        let jkr = game_data.join(JKR_FOLDER);
        let per_user = root.path().join("per-user");
        fs::create_dir_all(&jkr).unwrap();
        fs::create_dir_all(&per_user).unwrap();
        fs::write(jkr.join("config.cfg"), "from jkr").unwrap();
        fs::write(per_user.join("config.cfg"), "from per-user").unwrap();
        fs::write(per_user.join("marks.txt"), "marks").unwrap();
        fs::write(per_user.join("debug_panel_tested.txt"), "ticks").unwrap();

        let selection = select(&game_data, Some(&per_user.join("config.cfg"))).unwrap();
        let sjk = game_data.join(CLIENT_FOLDER);
        assert_eq!(selection.config, sjk.join("config.cfg"));
        assert!(selection.fallback_reason.is_none());
        // JKR's GameData folder wins; the per-user folder fills in the rest.
        assert_eq!(
            fs::read_to_string(sjk.join("config.cfg")).unwrap(),
            "from jkr"
        );
        assert_eq!(fs::read_to_string(sjk.join("marks.txt")).unwrap(), "marks");
        assert_eq!(
            fs::read_to_string(sjk.join("debug_panel_tested.txt")).unwrap(),
            "ticks"
        );
        // The originals stay, and a file deleted after the import is not brought back.
        assert!(jkr.join("config.cfg").is_file());
        fs::remove_file(sjk.join("marks.txt")).unwrap();
        select(&game_data, Some(&per_user.join("config.cfg"))).unwrap();
        assert!(!sjk.join("marks.txt").exists());
    }
}

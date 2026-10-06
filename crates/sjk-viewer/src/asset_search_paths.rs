//! MP startup search-path policy. Generic mount mechanics remain in sjk-vfs.
use crate::console::ViewerConsole;
use sjk_shell::{CvarDefinition, CvarFlags, CvarRegistry};
use sjk_vfs::VirtualFileSystem;
use std::error::Error;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Mutex, OnceLock};

static STARTUP: OnceLock<Options> = OnceLock::new();
/// The cosmetics packs were named in the log (every world mounts them).
static COSMETICS_LOGGED: AtomicBool = AtomicBool::new(false);
/// What the EternalJK character set probe found for one installation, so that
/// each world load does not open every EternalJK PK3 again.
static CHARSET_PROBE: Mutex<Option<(PathBuf, Option<ConsoleCharset>)>> = Mutex::new(None);

/// A character set image found in an EternalJK PK3: `(archive, path, bytes)`.
type ConsoleCharset = (PathBuf, String, Vec<u8>);

/// The folder JoF EJK and EternalJK keep their own content in, where JoF's
/// launcher installs hat and cape packs.
const COSMETICS_GAME: &str = "EternalJK";
/// Where a pack's hats and capes are; a PK3 with models there is a
/// cosmetics pack.
const COSMETIC_FOLDERS: [&str; 2] = ["models/cosmetics/hats", "models/cosmetics/capes"];
/// JoF EJK's client pictures (`jofclient-assets.pk3`): the Force wheel's Repulse,
/// Dash and flamethrower icons. The pack also holds the sounds and effects JoF
/// servers play, such as `sound/jof/repulse.mp3`.
const JOF_CLIENT_FOLDER: &str = "gfx/jof";

/// Immutable startup settings shared by world-loading workers.
pub(crate) struct Options {
    game: String,
    basegame: String,
    home: Option<PathBuf>,
    portable: bool,
    directory_first: bool,
    debug: bool,
    /// Prefer base-game shader definitions; immutable after startup.
    pub(crate) protect_shaders: bool,
}

impl Default for Options {
    fn default() -> Self {
        Self {
            game: String::new(),
            basegame: String::new(),
            home: None,
            portable: true,
            directory_first: false,
            debug: false,
            protect_shaders: true,
        }
    }
}

/// Register startup preferences; edits apply on next process launch, not a map reload.
pub(crate) fn register(cvars: &mut CvarRegistry) -> Result<(), sjk_shell::CvarError> {
    for definition in [
        CvarDefinition::new(
            "fs_game",
            "",
            CvarFlags::ARCHIVE,
            "Mod directory; restart the client after changing",
        ),
        CvarDefinition::new(
            "fs_basegame",
            "",
            CvarFlags::ARCHIVE,
            "Intermediate game directory; client restart required",
        ),
        CvarDefinition::new(
            "fs_homepath",
            "",
            CvarFlags::ARCHIVE,
            "Optional explicit content home; empty disables it; restart required",
        ),
    ] {
        cvars.register(definition)?;
    }
    for (name, value, help) in [
        (
            "fs_portable",
            true,
            "Ignore optional content home; client restart required",
        ),
        (
            "fs_dirbeforepak",
            false,
            "Prefer loose files within each directory; restart required",
        ),
        (
            "fs_debug",
            false,
            "Log mounted sources and asset reads; client restart required",
        ),
        (
            "fs_protectShaders",
            true,
            "Prefer base-game shader definitions; client restart required",
        ),
    ] {
        cvars.register(CvarDefinition::new(name, value, CvarFlags::ARCHIVE, help))?;
    }
    Ok(())
}

/// Freeze settings once before the first native world load; evidence defaults stay isolated.
pub(crate) fn initialize(console: &ViewerConsole) -> Result<(), Box<dyn Error>> {
    STARTUP
        .set(Options::from_console(console)?)
        .map_err(|_| "asset startup settings already initialized")?;
    Ok(())
}

/// Snapshot shared by all subsequent mounts without locks or config reads.
pub(crate) fn startup() -> &'static Options {
    STARTUP.get_or_init(Options::default)
}

impl Options {
    fn from_console(console: &ViewerConsole) -> Result<Self, Box<dyn Error>> {
        let game = console.text_value("fs_game").unwrap_or("").to_owned();
        let basegame = console.text_value("fs_basegame").unwrap_or("").to_owned();
        validate_directory(&game)?;
        validate_directory(&basegame)?;
        let home = console
            .text_value("fs_homepath")
            .filter(|value| !value.is_empty())
            .map(PathBuf::from);
        Ok(Self {
            game,
            basegame,
            home,
            portable: console.bool_cvar("fs_portable").unwrap_or(true),
            directory_first: console.bool_cvar("fs_dirbeforepak").unwrap_or(false),
            debug: console.bool_cvar("fs_debug").unwrap_or(false),
            protect_shaders: console.bool_cvar("fs_protectShaders").unwrap_or(true),
        })
    }

    /// Low-to-high priority directories, matching FS_Startup's nested ordering.
    pub(crate) fn directories(&self, install: &Path) -> Result<Vec<PathBuf>, Box<dyn Error>> {
        let home = if self.portable {
            None
        } else {
            // Downloads are session-selected, never an implicit global content home.
            self.home.as_deref()
        };
        let mut paths = Vec::new();
        for game in ["base", self.basegame.as_str(), self.game.as_str()] {
            if game.is_empty() {
                continue;
            }
            for root in std::iter::once(install).chain(home) {
                let path = root.join(game);
                if !paths.contains(&path) {
                    paths.push(path);
                }
            }
        }
        Ok(paths)
    }

    /// The PK3s in `install/EternalJK` that carry hats or capes
    /// (`models/cosmetics/`) or JoF's client pictures (`gfx/jof/`), lowest
    /// priority first: JoF EJK's cosmetics and client assets, found without
    /// mounting the rest of that folder (its menus, HUD and strings). None
    /// when the folder is a game directory already.
    pub(crate) fn cosmetic_packs(&self, install: &Path) -> Vec<PathBuf> {
        let mounted = [self.basegame.as_str(), self.game.as_str()]
            .iter()
            .any(|game| game.eq_ignore_ascii_case(COSMETICS_GAME));
        let directory = install.join(COSMETICS_GAME);
        if mounted || !directory.is_dir() {
            return Vec::new();
        }
        sjk_vfs::pk3_search_order(&directory)
            .unwrap_or_default()
            .into_iter()
            .filter(|archive| carries_jof_content(archive))
            .collect()
    }

    /// EternalJK's console character set: `gfx/2d/charsgrid_med` from the
    /// highest-priority PK3 in `install/EternalJK` that has one (jaPRO's
    /// `japro-assets.pk3`), as `(archive, path, image bytes)`. EternalJK draws its
    /// console with it, and unlike the retail set it has `¬`, `¥`, `²`, `½` and
    /// the rest of Latin-1. None when the folder is a game directory already.
    /// The PK3s are probed once per installation path for the life of the process.
    pub(crate) fn eternaljk_console_charset(&self, install: &Path) -> Option<ConsoleCharset> {
        let mounted = [self.basegame.as_str(), self.game.as_str()]
            .iter()
            .any(|game| game.eq_ignore_ascii_case(COSMETICS_GAME));
        let directory = install.join(COSMETICS_GAME);
        if mounted || !directory.is_dir() {
            return None;
        }
        let mut cache = CHARSET_PROBE
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        match cache.as_ref() {
            Some((cached, found)) if cached == install => found.clone(),
            _ => {
                let found = probe_console_charset(&directory);
                *cache = Some((install.to_path_buf(), found.clone()));
                found
            }
        }
    }

    /// Mount existing directories; unreadable archives warn without discarding other packs.
    pub(crate) fn mount(&self, install: &Path) -> Result<VirtualFileSystem, Box<dyn Error>> {
        // Large offline imports may opt into a higher per-asset ceiling. Keep the
        // ordinary viewer default and generic VFS policy unchanged.
        let limit = std::env::var("JKR_MAX_ASSET_MIB")
            .ok()
            .map(|value| value.parse::<u64>())
            .transpose()?
            .unwrap_or(1024)
            .checked_mul(1024 * 1024)
            .filter(|&n| n > 0)
            .ok_or("JKR_MAX_ASSET_MIB must be positive and fit in u64")?;
        let mut vfs = VirtualFileSystem::with_max_asset_bytes(limit);
        vfs.set_read_diagnostics(self.debug);
        // JoF EJK's hats and capes, below everything else so they never
        // replace other content.
        let log = !COSMETICS_LOGGED.swap(true, Ordering::Relaxed);
        for pack in self.cosmetic_packs(install) {
            match vfs.mount_pk3(&pack) {
                Ok(_) if log => crate::log::progress(format_args!(
                    "JoF EJK content (hats, capes, client assets) from {}",
                    pack.display(),
                )),
                Ok(_) => {}
                Err(error) => crate::log::progress(format_args!(
                    "warning: skipping cosmetics PK3 {}: {error}",
                    pack.display(),
                )),
            }
        }
        // EternalJK mounts its folder above `base` and below `fs_basegame` and the
        // `fs_game` mod, so its character set goes in after the `base` directories:
        // a mod's own `charsgrid_med` still wins. Only that image, nothing else from
        // the pack.
        let mut charset = self.eternaljk_console_charset(install);
        for directory in self.directories(install)? {
            if !is_base_directory(&directory) {
                mount_console_charset(&mut vfs, charset.take(), log)?;
            }
            if !directory.is_dir() {
                continue;
            }
            if !self.directory_first {
                vfs.mount_directory(&directory)?;
            }
            vfs.mount_pk3_directory_with_warnings(&directory, |path, error| {
                crate::log::progress(format_args!(
                    "warning: skipping PK3 {}: {error}",
                    path.display(),
                ));
            })?;
            if self.directory_first {
                vfs.mount_directory(&directory)?;
            }
        }
        mount_console_charset(&mut vfs, charset.take(), log)?;
        // `JKR_CONTENT=<dir>[:<dir>...]`: further content directories (loose files and
        // PK3s), above the installation: locally made content that has no place in it.
        for directory in std::env::var_os("JKR_CONTENT")
            .iter()
            .flat_map(std::env::split_paths)
            .filter(|directory| directory.is_dir())
        {
            vfs.mount_pk3_directory_with_warnings(&directory, |path, error| {
                crate::log::progress(format_args!(
                    "warning: skipping PK3 {}: {error}",
                    path.display()
                ));
            })?;
            vfs.mount_directory(&directory)?;
        }
        if self.debug {
            for mount in vfs.mounts() {
                crate::log::progress(format_args!(
                    "fs mount: {} ({} entries)",
                    mount.name, mount.entries,
                ));
            }
        }
        Ok(vfs)
    }
}

/// Whether `directory` is a `base` game directory (the install's or the content home's).
fn is_base_directory(directory: &Path) -> bool {
    directory
        .file_name()
        .is_some_and(|name| name.eq_ignore_ascii_case("base"))
}

/// Mount EternalJK's console character set as a one-image source, naming it in
/// the log when `log` is set.
fn mount_console_charset(
    vfs: &mut VirtualFileSystem,
    charset: Option<ConsoleCharset>,
    log: bool,
) -> Result<(), Box<dyn Error>> {
    let Some((archive, path, bytes)) = charset else {
        return Ok(());
    };
    if log {
        crate::log::progress(format_args!(
            "console character set {path} from {}",
            archive.display(),
        ));
    }
    vfs.mount_memory("EternalJK console character set", [(path, bytes)])?;
    Ok(())
}

/// The first PK3 in `directory`, highest priority first, that has the console
/// character set.
fn probe_console_charset(directory: &Path) -> Option<ConsoleCharset> {
    sjk_vfs::pk3_search_order(directory)
        .unwrap_or_default()
        .into_iter()
        .rev()
        .find_map(|archive| {
            let mut probe = VirtualFileSystem::new();
            probe.mount_pk3(&archive).ok()?;
            ["tga", "png", "jpg"].iter().find_map(|extension| {
                let path = format!("{}.{extension}", crate::text::charset::PATH);
                let asset = probe.read(&path).ok()??;
                Some((archive.clone(), path, asset.bytes))
            })
        })
}

/// Whether `archive` holds hat or cape models.
fn carries_jof_content(archive: &Path) -> bool {
    let mut probe = VirtualFileSystem::new();
    probe.mount_pk3(archive).is_ok()
        && (COSMETIC_FOLDERS
            .iter()
            .any(|folder| !probe.list_files(folder, ".md3").is_empty())
            || !probe.list_files(JOF_CLIENT_FOLDER, "").is_empty())
}

fn validate_directory(name: &str) -> Result<(), Box<dyn Error>> {
    if name == "."
        || name.contains("..")
        || name.contains(['/', '\\', ':'])
        || name.chars().any(char::is_control)
        || name.trim() != name
    {
        return Err(
            format!("invalid game directory {name:?}: expected a single directory name",).into(),
        );
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write as _;

    fn pk3(path: &Path, entries: &[&str]) {
        let file = std::fs::File::create(path).unwrap();
        let mut zip = zip::ZipWriter::new(file);
        for entry in entries {
            zip.start_file(*entry, zip::write::SimpleFileOptions::default())
                .unwrap();
            zip.write_all(b"x").unwrap();
        }
        zip.finish().unwrap();
    }

    #[test]
    fn only_eternaljk_packs_with_hats_capes_or_jof_pictures_are_found() {
        let install = tempfile::tempdir().unwrap();
        let folder = install.path().join("EternalJK");
        std::fs::create_dir_all(&folder).unwrap();
        pk3(
            &folder.join("zzz_jof_cosmetics.pk3"),
            &[
                "models/cosmetics/hats/santahat.md3",
                "shaders/japro_hats.shader",
            ],
        );
        pk3(
            &folder.join("capes_only.pk3"),
            &["models/cosmetics/capes/royalcape.md3"],
        );
        pk3(&folder.join("menus.pk3"), &["ui/jamp/main.menu"]);
        pk3(
            &folder.join("jofclient-assets.pk3"),
            &["gfx/jof/force_dash.tga", "sound/jof/repulse.mp3"],
        );
        let options = Options::default();
        let names: Vec<String> = options
            .cosmetic_packs(install.path())
            .iter()
            .map(|path| path.file_name().unwrap().to_string_lossy().into_owned())
            .collect();
        assert_eq!(names.len(), 3, "{names:?}");
        assert!(names.iter().all(|name| name != "menus.pk3"));
        // The hats reach the mounted file system, the menus do not.
        let vfs = options.mount(install.path()).unwrap();
        assert!(vfs.contains("models/cosmetics/hats/santahat.md3").unwrap());
        assert!(vfs.contains("gfx/jof/force_dash.tga").unwrap());
        assert!(!vfs.contains("ui/jamp/main.menu").unwrap());
        // With the folder as a game directory, it is mounted whole instead.
        let whole = Options {
            basegame: "EternalJK".to_owned(),
            ..Options::default()
        };
        assert!(whole.cosmetic_packs(install.path()).is_empty());
    }

    #[test]
    fn eternaljk_console_charset_overrides_the_base_one_alone() {
        let install = tempfile::tempdir().unwrap();
        let base = install.path().join("base");
        let folder = install.path().join("EternalJK");
        std::fs::create_dir_all(&base).unwrap();
        std::fs::create_dir_all(&folder).unwrap();
        pk3(
            &base.join("hd_fonts.pk3"),
            &["gfx/2d/charsgrid_med.tga", "ui/jamp/main.menu"],
        );
        pk3(
            &folder.join("japro-assets.pk3"),
            &["gfx/2d/charsgrid_med.tga", "ui/jamp/ingame.menu"],
        );
        let options = Options::default();
        let (archive, path, _) = options.eternaljk_console_charset(install.path()).unwrap();
        assert!(archive.ends_with("japro-assets.pk3"));
        assert_eq!(path, "gfx/2d/charsgrid_med.tga");
        let vfs = options.mount(install.path()).unwrap();
        let charset = vfs.read("gfx/2d/charsgrid_med.tga").unwrap().unwrap();
        assert!(vfs.mounts().any(|mount| mount.id == charset.source.mount_id
            && &*mount.name == "EternalJK console character set"));
        // Nothing else from that pack is mounted.
        assert!(!vfs.contains("ui/jamp/ingame.menu").unwrap());
        assert!(vfs.contains("ui/jamp/main.menu").unwrap());
    }

    #[test]
    fn eternaljk_console_charset_sits_above_base_and_below_the_mod() {
        let install = tempfile::tempdir().unwrap();
        let base = install.path().join("base");
        let folder = install.path().join("EternalJK");
        let modification = install.path().join("mymod");
        for directory in [&base, &folder, &modification] {
            std::fs::create_dir_all(directory).unwrap();
        }
        pk3(&base.join("assets0.pk3"), &["gfx/2d/charsgrid_med.tga"]);
        pk3(
            &folder.join("japro-assets.pk3"),
            &["gfx/2d/charsgrid_med.tga"],
        );
        let source = |vfs: &VirtualFileSystem| {
            let asset = vfs.read("gfx/2d/charsgrid_med.tga").unwrap().unwrap();
            vfs.mounts()
                .find(|mount| mount.id == asset.source.mount_id)
                .unwrap()
                .name
                .to_string()
        };
        // Without a mod the EternalJK image is the one used, above `base`.
        let options = Options::default();
        assert_eq!(
            source(&options.mount(install.path()).unwrap()),
            "EternalJK console character set"
        );
        // A mod directory with its own set wins over EternalJK's, as `fs_game`
        // is mounted above EternalJK's folder.
        pk3(&modification.join("mod.pk3"), &["gfx/2d/charsgrid_med.tga"]);
        let with_mod = Options {
            game: "mymod".to_owned(),
            ..Options::default()
        };
        let vfs = with_mod.mount(install.path()).unwrap();
        assert_ne!(source(&vfs), "EternalJK console character set");
        let names: Vec<_> = vfs.mounts().map(|mount| mount.name.to_string()).collect();
        let charset = names
            .iter()
            .position(|name| name == "EternalJK console character set")
            .expect("EternalJK set is mounted");
        let mod_pack = names
            .iter()
            .position(|name| name.ends_with("mod.pk3"))
            .expect("mod pack is mounted");
        let base_pack = names
            .iter()
            .position(|name| name.ends_with("assets0.pk3"))
            .expect("base pack is mounted");
        assert!(base_pack < charset && charset < mod_pack, "{names:?}");
    }
}

//! Viewer filesystem policy for portable shell scripts.

use sjk_shell::CommandFileResolver;
use std::fs;
use std::path::{Component, Path};

pub(super) struct Resolver<'a> {
    config_directory: &'a Path,
    vfs: Option<&'a sjk_vfs::VirtualFileSystem>,
}

impl<'a> Resolver<'a> {
    pub(super) fn new(
        config_directory: &'a Path,
        vfs: Option<&'a sjk_vfs::VirtualFileSystem>,
    ) -> Self {
        Self {
            config_directory,
            vfs,
        }
    }

    /// Names of the files ending in `.extension` at the top of the roots `exec`
    /// reads from, in the order it searches them: the config folder, then the VFS.
    /// For tab completion of a file argument (`Cmd_CompleteCfgName`).
    pub(super) fn root_files(&self, extension: &str) -> Vec<String> {
        let suffix = format!(".{}", extension.to_ascii_lowercase());
        let mut names = Vec::new();
        if let Ok(entries) = fs::read_dir(self.config_directory) {
            for entry in entries.flatten() {
                if entry.file_type().is_ok_and(|kind| kind.is_file())
                    && let Some(name) = entry.file_name().to_str()
                    && name.to_ascii_lowercase().ends_with(&suffix)
                {
                    names.push(name.to_owned());
                }
            }
        }
        if let Some(vfs) = self.vfs {
            for name in vfs.list_files("", &suffix) {
                if !name.contains('/') {
                    names.push(vfs.original_name(&name).unwrap_or(name));
                }
            }
        }
        names
    }
}

impl CommandFileResolver for Resolver<'_> {
    fn file_command(
        &mut self,
        name: &str,
        args: &[String],
        dump: &str,
    ) -> Result<Vec<String>, String> {
        self.command(name, args, dump)
    }
    fn read_command_file(&mut self, path: &str) -> Result<Option<String>, String> {
        validate_relative_path(path)?;
        let host_path = self.config_directory.join(path);
        match fs::read(&host_path) {
            Ok(bytes) => return Ok(Some(sjk_shell::decode_config_text(bytes))),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => return Err(format!("could not read {}: {error}", host_path.display())),
        }
        let Some(vfs) = self.vfs else {
            return Ok(None);
        };
        let asset = vfs.read(path).map_err(|error| error.to_string())?;
        Ok(asset.map(|asset| sjk_shell::decode_config_text(asset.bytes)))
    }
}

fn validate_relative_path(path: &str) -> Result<(), String> {
    let path = Path::new(path);
    if path.is_absolute()
        || path
            .components()
            .any(|part| !matches!(part, Component::Normal(_) | Component::CurDir))
    {
        return Err("exec path must stay inside the config or VFS root".to_owned());
    }
    Ok(())
}

#[path = "console_files.rs"]
mod files;

#[cfg(test)]
mod tests {
    use super::Resolver;

    #[test]
    fn root_files_list_the_config_folder_then_the_vfs_root() {
        let directory = tempfile::tempdir().unwrap();
        std::fs::write(directory.path().join("Moj.cfg"), "").unwrap();
        std::fs::write(directory.path().join("notes.txt"), "").unwrap();
        std::fs::create_dir(directory.path().join("sub")).unwrap();
        std::fs::write(directory.path().join("sub/deep.cfg"), "").unwrap();
        let mut vfs = sjk_vfs::VirtualFileSystem::new();
        vfs.mount_memory(
            "base",
            [
                ("AutoExec.CFG", b"".to_vec()),
                ("cfg/nested.cfg", b"".to_vec()),
            ],
        )
        .unwrap();
        let resolver = Resolver::new(directory.path(), Some(&vfs));
        assert_eq!(resolver.root_files("cfg"), ["Moj.cfg", "AutoExec.CFG"]);
        assert_eq!(
            Resolver::new(directory.path(), None).root_files("cfg"),
            ["Moj.cfg"]
        );
    }
}

//! Strict tooling and fault-isolated client shader catalogue loading.

use sjk_vfs::{VirtualFileSystem, VirtualPath};

use crate::{ShaderCatalog, ShaderDefinition, ShaderError, normalize_name, parse::parse_script};

mod authority;
pub use authority::DefinitionSource;

impl ShaderCatalog {
    /// Loads `.shader` and higher-priority `.oshader` scripts, returning the first error.
    /// Useful for validation tools which must detect broken content.
    pub fn load(vfs: &VirtualFileSystem) -> Result<Self, ShaderError> {
        let mut catalog = Self::default();
        for path in shader_paths(vfs) {
            catalog.load_script(vfs, &path)?;
        }
        Ok(catalog)
    }

    /// Loads usable definitions, reporting each malformed shader independently.
    /// Unreadable files are reported once. Valid override scripts retain `.oshader` priority.
    pub fn load_with_warnings(
        vfs: &VirtualFileSystem,
        mut warning: impl FnMut(&VirtualPath, &ShaderError),
    ) -> Self {
        let mut catalog = Self::default();
        for path in shader_paths(vfs) {
            match vfs.read(path.as_str()) {
                Ok(Some(asset)) => {
                    let definitions =
                        crate::recovery::parse_recovering(&asset.bytes, &path, |error| {
                            warning(&path, &error)
                        });
                    for definition in definition_order(definitions, &path) {
                        catalog
                            .definitions
                            .insert(normalize_name(&definition.name), definition);
                    }
                }
                Ok(None) => warning(&path, &ShaderError::DisappearedAsset(path.clone())),
                Err(error) => warning(&path, &ShaderError::Vfs(error)),
            }
        }
        catalog
    }

    fn load_script(
        &mut self,
        vfs: &VirtualFileSystem,
        path: &VirtualPath,
    ) -> Result<(), ShaderError> {
        let asset = vfs
            .read(path.as_str())?
            .ok_or_else(|| ShaderError::DisappearedAsset(path.clone()))?;
        for definition in definition_order(parse_script(&asset.bytes, path)?, path) {
            self.definitions
                .insert(normalize_name(&definition.name), definition);
        }
        Ok(())
    }
}

/// Normal scripts retain SJK's last-wins order. JoF EJK #138 places override
/// files before normal scripts in its first-wins lookup; reverse the override
/// paths/definitions here to produce the same precedence in our last-wins map.
fn shader_paths(vfs: &VirtualFileSystem) -> impl Iterator<Item = VirtualPath> {
    let paths = vfs.paths();
    let normal = paths
        .iter()
        .filter(|path| script_kind(path) == Some(false))
        .cloned();
    let overrides = paths
        .iter()
        .rev()
        .filter(|path| script_kind(path) == Some(true))
        .cloned();
    normal.chain(overrides).collect::<Vec<_>>().into_iter()
}

fn script_kind(path: &VirtualPath) -> Option<bool> {
    let name = path.as_str();
    if !name.starts_with("shaders/") {
        None
    } else if name.ends_with(".oshader") {
        Some(true)
    } else if name.ends_with(".shader") {
        Some(false)
    } else {
        None
    }
}

fn definition_order(
    mut definitions: Vec<ShaderDefinition>,
    path: &VirtualPath,
) -> impl Iterator<Item = ShaderDefinition> {
    if script_kind(path) == Some(true) {
        definitions.reverse();
    }
    definitions.into_iter()
}

#[cfg(test)]
#[path = "catalog_overrides_tests.rs"]
mod override_tests;

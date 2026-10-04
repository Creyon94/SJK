//! Shader remaps of the loaded map (`R_RemapShader`): where they come from, the
//! `cg_remaps` gate, the console commands, and applying them to the renderer.
//!
//! The table and its rules are [`sjk_shader::ShaderRemaps`]. Sources, as in JoF
//! EternalJK: the map's worldspawn `remapshader` keys (`tr_bsp.cpp`), the game
//! module's `CS_SHADERSTATE` configstring and `remapShader` server command
//! (`cg_servercmds.c`, gated by `cg_remaps`), and the `remapShader` console command
//! (`cg_consolecmds.c`). Like rd-vanilla, a remap naming a shader that has neither a
//! definition nor an image is dropped with a warning. A new map starts with no remaps
//! (rd-vanilla's shaders are recreated with the renderer); a reload of the same map
//! (`map_restart`) keeps the console's, as rd-vanilla keeps them over a map restart.
//!
//! Requests are queued as they arrive and applied once per frame by
//! [`GpuState::sync_shader_remaps`], which recompiles affected materials only when the
//! effective table changed ([`crate::world_materials::Runtime::apply_remaps`]).

use crate::GpuState;
use sjk_shader::{RemapLevel, RemapSource, ShaderCatalog, ShaderRemaps};
use sjk_vfs::VirtualFileSystem;

/// EternalJK's cvar name and default (JoF EJK `cg_xcvar.h`: "2", archived).
pub(crate) const CVAR: &str = "cg_remaps";
pub(crate) const DEFAULT_LEVEL: i64 = 2;
pub(crate) const CVAR_HELP: &str =
    "Server shader remaps: 0 off, 1 all but player models, 2 all (EternalJK); live";

/// Console commands this module answers, with their browser help.
pub(crate) const COMMANDS: &[(&str, &str)] = &[
    (
        "remapShader",
        "Draw one shader as another: remapShader <old> <new> (same name restores)",
    ),
    ("listRemaps", "List shader remaps and where they came from"),
    (
        "clearRemaps",
        "Remove every shader remap until the server sends new ones",
    ),
];

/// One remap waiting for the game data check.
struct Request {
    source: RemapSource,
    old: String,
    new: String,
    time_offset: Option<f32>,
}

/// The map's remap table plus the requests not yet checked against the game data.
#[derive(Default)]
pub(crate) struct State {
    table: ShaderRemaps,
    pending: Vec<Request>,
    /// Table generation the renderer was last brought in line with.
    applied: u64,
}

impl State {
    pub(crate) fn new(level: RemapLevel) -> Self {
        Self {
            table: ShaderRemaps::new(level),
            pending: Vec::new(),
            applied: 0,
        }
    }

    pub(crate) fn table(&self) -> &ShaderRemaps {
        &self.table
    }

    /// The map's worldspawn `remapshader` keys, applied first as rd-vanilla applies
    /// them while loading the world.
    pub(crate) fn queue_worldspawn(&mut self, bsp: &sjk_bsp::Bsp) {
        let Ok(entities) = sjk_entity::parse_entity_lump(bsp.entities()) else {
            return;
        };
        // `R_LoadEntities` reads the first entity only: the worldspawn.
        let Some(world) = entities.first() else {
            return;
        };
        let fields = world
            .fields()
            .iter()
            .map(|(key, value)| (key.as_str(), value.as_str()));
        for (old, new) in sjk_shader::worldspawn_remaps(fields) {
            self.queue(RemapSource::Map, old, new, None);
        }
    }

    /// Every entry of a `CS_SHADERSTATE` value, applied again in order as
    /// `CG_ShaderStateChanged` does on each change.
    pub(crate) fn queue_shader_state(&mut self, value: &[u8]) {
        let value = String::from_utf8_lossy(value);
        for entry in sjk_shader::parse_shader_state(&value) {
            self.queue(
                RemapSource::Server,
                &entry.old,
                &entry.new,
                Some(entry.time_offset),
            );
        }
    }

    /// The `remapShader <old> <new> <timeOffset>` server command.
    pub(crate) fn queue_server_command(&mut self, old: &str, new: &str, time_offset: &str) {
        self.queue(
            RemapSource::Server,
            old,
            new,
            Some(sjk_shader::atof(time_offset)),
        );
    }

    fn queue(&mut self, source: RemapSource, old: &str, new: &str, time_offset: Option<f32>) {
        self.pending.push(Request {
            source,
            old: old.to_owned(),
            new: new.to_owned(),
            time_offset,
        });
    }

    /// Check the queued requests against the game data and record those whose
    /// shaders exist; returns rd-vanilla's warning for each dropped one.
    fn admit_pending(&mut self, vfs: &VirtualFileSystem, shaders: &ShaderCatalog) -> Vec<String> {
        let mut warnings = Vec::new();
        for request in std::mem::take(&mut self.pending) {
            match check(vfs, shaders, &request.old, &request.new) {
                Ok(()) => {
                    self.table.remap(
                        request.source,
                        &request.old,
                        &request.new,
                        request.time_offset,
                    );
                }
                Err(warning) => warnings.push(warning),
            }
        }
        warnings
    }

    /// Keep the console's remaps of the same map across a world reload.
    pub(crate) fn inherit_console(&mut self, previous: &State) {
        for entry in previous.table.entries() {
            if entry.source == RemapSource::Console {
                self.queue(
                    RemapSource::Console,
                    entry.old,
                    entry.new,
                    entry.time_offset,
                );
            }
        }
    }

    /// The `listRemaps` report.
    fn listing(&self, slots: &[(&str, &str)]) -> Vec<String> {
        let entries = self.table.entries();
        if entries.is_empty() {
            return vec!["Remaps: none".into()];
        }
        let mut lines = vec![format!(
            "Remaps (cg_remaps {}):",
            match self.table.level() {
                RemapLevel::Off => 0,
                RemapLevel::MapOnly => 1,
                RemapLevel::All => 2,
            }
        )];
        for entry in entries {
            let offset = entry
                .time_offset
                .map(|offset| format!(", time {offset:.2}"))
                .unwrap_or_default();
            let state = if entry.active {
                let surfaces = slots
                    .iter()
                    .filter(|(old, new)| {
                        sjk_shader::remap_key(old) == entry.old && *new == entry.new
                    })
                    .count();
                if surfaces > 0 {
                    String::new()
                } else {
                    " ^3(not drawn on this map)".into()
                }
            } else if entry.old == entry.new {
                " ^3(restores the shader)".into()
            } else if !self.table.level().allows(entry.source, entry.old) {
                " ^3(off: cg_remaps)".into()
            } else {
                " ^3(replaced by a later remap)".into()
            };
            lines.push(format!(
                "  {} -> {}  ^7[{}{offset}]{state}",
                entry.old,
                entry.new,
                entry.source.label()
            ));
        }
        lines
    }
}

/// rd-vanilla's `R_RemapShader` checks: both shaders must register, by definition or
/// by image.
fn check(
    vfs: &VirtualFileSystem,
    shaders: &ShaderCatalog,
    old: &str,
    new: &str,
) -> Result<(), String> {
    if !known(vfs, shaders, old) {
        return Err(format!("^3WARNING: R_RemapShader: shader {old} not found"));
    }
    if !known(vfs, shaders, new) {
        return Err(format!(
            "^3WARNING: R_RemapShader: new shader {new} not found"
        ));
    }
    Ok(())
}

fn known(vfs: &VirtualFileSystem, shaders: &ShaderCatalog, name: &str) -> bool {
    !name.is_empty()
        && (shaders.get(name).is_some()
            || shaders
                .resolve_image(vfs, name)
                .is_ok_and(|image| image.is_some()))
}

/// The `cg_remaps` level a cvar value selects; unset is EternalJK's default.
pub(crate) fn level(value: Option<i64>) -> RemapLevel {
    RemapLevel::from_cvar(value.unwrap_or(DEFAULT_LEVEL))
}

impl GpuState {
    /// Follow `cg_remaps`, admit queued remaps and, when the effective table changed,
    /// bring the materials and the effect atlas in line. Called once per frame; does
    /// nothing (and allocates nothing) while no remap changes.
    pub(crate) fn sync_shader_remaps(&mut self) {
        if let Some(console) = &self.console {
            let level = level(console.integer_cvar(CVAR));
            self.shader_remaps.table.set_level(level);
        }
        if !self.shader_remaps.pending.is_empty()
            && let Some(vfs) = self.vfs.clone()
        {
            for warning in self.shader_remaps.admit_pending(&vfs, &self.shaders) {
                crate::log::progress(format_args!("{warning}"));
            }
        }
        self.apply_shader_remaps();
    }

    /// Recompile what the remap table changed; `None` when it had not changed.
    fn apply_shader_remaps(&mut self) -> Option<crate::world_materials::AppliedRemaps> {
        let generation = self.shader_remaps.table.generation();
        if generation == self.shader_remaps.applied {
            return None;
        }
        self.shader_remaps.applied = generation;
        let vfs = self.vfs.clone()?;
        let started = std::time::Instant::now();
        let applied = self.world_materials.apply_remaps(
            &self.device,
            &self.queue,
            &vfs,
            &self.shaders,
            &self.shader_remaps.table,
        );
        let effects = self.apply_effect_remaps();
        crate::log::progress(format_args!(
            "shader remaps: {} material slots remapped ({} changed, {} failed), \
             {effects} effect shaders, in {:.1} ms",
            applied.remapped,
            applied.changed,
            applied.failed,
            started.elapsed().as_secs_f64() * 1e3
        ));
        Some(applied)
    }

    /// Point remapped effect shaders at their targets, adding missing targets to
    /// the effect atlas first. Returns the number of remapped effect shaders.
    pub(crate) fn apply_effect_remaps(&mut self) -> usize {
        let missing = self
            .particle_atlas
            .missing_remap_targets(&self.shader_remaps.table);
        if !missing.is_empty()
            && let Some(vfs) = self.vfs.clone()
        {
            let mut required = self.particle_atlas.shader_names();
            required.extend(missing);
            match crate::particle_atlas::create(
                &self.device,
                &self.queue,
                &crate::particle_atlas::layout(&self.device),
                &vfs,
                &self.shaders,
                &required,
            ) {
                Ok(atlas) => self.particle_atlas = atlas,
                Err(error) => crate::log::progress(format_args!(
                    "warning: effect atlas for shader remaps: {error}"
                )),
            }
        }
        self.particle_atlas.apply_remaps(&self.shader_remaps.table)
    }

    /// `remapShader`, `listRemaps` and `clearRemaps`; `None` for other commands.
    pub(crate) fn shader_remap_command(
        &mut self,
        name: &str,
        args: &[String],
    ) -> Option<Result<Vec<String>, String>> {
        Some(match name {
            "remapshader" => self.remap_shader_command(args),
            "listremaps" => {
                let slots = self.world_materials.remapped_slots();
                Ok(self.shader_remaps.listing(&slots))
            }
            "clearremaps" => {
                self.shader_remaps.pending.clear();
                let count = self.shader_remaps.table.entries().len();
                self.shader_remaps.table.clear();
                self.apply_shader_remaps();
                Ok(vec![format!("Cleared {count} shader remaps")])
            }
            _ => return None,
        })
    }

    fn remap_shader_command(&mut self, args: &[String]) -> Result<Vec<String>, String> {
        let [old, new] = args else {
            return Err("Usage: /remapShader <old> <new>".into());
        };
        let vfs = self.vfs.clone().ok_or("No game data loaded")?;
        // The flush also admits anything queued earlier this frame first.
        self.sync_shader_remaps();
        if let Err(warning) = check(&vfs, &self.shaders, old, new) {
            return Ok(vec![warning]);
        }
        self.shader_remaps
            .table
            .remap(RemapSource::Console, old, new, None);
        self.apply_shader_remaps();
        let key = sjk_shader::remap_key(old);
        if sjk_shader::remap_key(new) == key {
            return Ok(vec![format!("{old} restored")]);
        }
        let drawn = self
            .world_materials
            .remapped_slots()
            .iter()
            .any(|(origin, _)| sjk_shader::remap_key(origin) == key);
        Ok(vec![if drawn {
            format!("{old} -> {new}")
        } else {
            format!("{old} -> {new} (no world or model surface uses {old} yet)")
        }])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cg_remaps_defaults_to_eternaljk_level_two() {
        assert_eq!(level(None), RemapLevel::All);
        assert_eq!(level(Some(0)), RemapLevel::Off);
        assert_eq!(level(Some(1)), RemapLevel::MapOnly);
    }

    #[test]
    fn console_remaps_carry_over_and_others_do_not() {
        let mut previous = State::new(RemapLevel::All);
        previous
            .table
            .remap(RemapSource::Console, "textures/a", "textures/b", None);
        previous
            .table
            .remap(RemapSource::Server, "textures/c", "textures/d", Some(1.0));
        previous
            .table
            .remap(RemapSource::Map, "textures/e", "textures/f", None);
        let mut next = State::new(RemapLevel::All);
        next.inherit_console(&previous);
        assert_eq!(next.pending.len(), 1);
        assert_eq!(next.pending[0].old, "textures/a");
        assert_eq!(next.pending[0].source, RemapSource::Console);
    }

    #[test]
    fn shader_state_and_server_commands_queue_server_remaps() {
        let mut state = State::new(RemapLevel::All);
        state.queue_shader_state(b"a=b: 1.50@c=d:2.00@");
        state.queue_server_command("e", "f", " 3.25");
        let queued: Vec<_> = state
            .pending
            .iter()
            .map(|r| (r.source, r.old.as_str(), r.new.as_str(), r.time_offset))
            .collect();
        assert_eq!(
            queued,
            vec![
                (RemapSource::Server, "a", "b", Some(1.5)),
                (RemapSource::Server, "c", "d", Some(2.0)),
                (RemapSource::Server, "e", "f", Some(3.25)),
            ]
        );
    }

    #[test]
    fn listing_marks_gated_and_replaced_remaps() {
        let mut state = State::new(RemapLevel::MapOnly);
        state.table.remap(
            RemapSource::Server,
            "models/players/kyle/body",
            "models/x",
            Some(4.0),
        );
        state
            .table
            .remap(RemapSource::Server, "textures/a", "textures/b", None);
        state
            .table
            .remap(RemapSource::Console, "textures/a", "textures/c", None);
        let lines = state.listing(&[("textures/A.tga", "textures/c")]);
        assert_eq!(lines[0], "Remaps (cg_remaps 1):");
        assert!(lines[1].contains("models/players/kyle/body -> models/x"));
        assert!(lines[1].contains("server, time 4.00"));
        assert!(lines[1].ends_with("(off: cg_remaps)"));
        assert!(lines[2].ends_with("(replaced by a later remap)"));
        assert!(lines[3].starts_with("  textures/a -> textures/c  ^7[console]"));
        assert!(!lines[3].contains("^3"));
        assert_eq!(State::default().listing(&[]), vec!["Remaps: none"]);
    }
}

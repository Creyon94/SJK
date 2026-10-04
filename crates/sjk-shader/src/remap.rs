//! Quake 3 / Jedi Academy shader remapping (`R_RemapShader`).
//!
//! A remap makes everything drawn with one shader use another shader's definition
//! instead: world surfaces, models and effects alike. rd-vanilla stores it as
//! `shader_t::remappedShader` and swaps it in when a surface begins drawing
//! (`tr_shade.cpp` `RB_BeginSurface`), so a remap is one level deep: with `a -> b`
//! and `b -> c`, `a` draws `b`'s own stages, not `c`'s. Remapping a shader to
//! itself clears its remap (`tr_shader.cpp` `R_RemapShader`, `sh != sh2`). Names
//! compare without case, with `\` as `/` and without an extension
//! (`COM_StripExtension`), as rd-vanilla's shader hash does.
//!
//! Remaps come from three places, kept apart here so that EternalJK's `cg_remaps`
//! can be changed live instead of only after a restart (`CVAR_LATCH` in JoF EJK):
//!
//! - [`RemapSource::Map`]: the map's worldspawn `remapshader` keys, applied when the
//!   renderer loads the world (`tr_bsp.cpp` `R_LoadEntities`);
//! - [`RemapSource::Server`]: the game module's `CS_SHADERSTATE` configstring
//!   (`g_utils.c` `BuildShaderStateConfig`, parsed by [`parse_shader_state`]) and the
//!   `remapShader <old> <new> <timeOffset>` server command mods send
//!   (`cg_servercmds.c` `CG_RemapShader_f`). Only these are gated by
//!   [`RemapLevel`];
//! - [`RemapSource::Console`]: the player's `remapShader <old> <new>` console command
//!   (`cg_consolecmds.c` `CG_RemapShader_f`).
//!
//! As in rd-vanilla, where every source writes the same `remappedShader` field, the
//! latest remap of a shader wins; disabling a source here reveals the remap it hid.
//! Checking that both shaders exist (rd-vanilla drops a remap naming a missing
//! shader with a warning) is the caller's job, since it needs the game data.

use std::collections::HashMap;

/// `MAX_SHADER_REMAPS` in the game module (`g_utils.c`); the server never sends more.
pub const MAX_SERVER_REMAPS: usize = 128;

/// The configstring index of the game module's shader remaps (`bg_public.h`).
pub const CS_SHADERSTATE: usize = 24;

/// Where a remap came from; see the [module documentation](self).
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum RemapSource {
    /// The map's worldspawn `remapshader` keys.
    Map,
    /// `CS_SHADERSTATE` or the server's `remapShader` command.
    Server,
    /// The local `remapShader` console command.
    Console,
}

impl RemapSource {
    /// Short lowercase label for listings.
    pub fn label(self) -> &'static str {
        match self {
            Self::Map => "map",
            Self::Server => "server",
            Self::Console => "console",
        }
    }
}

/// EternalJK's `cg_remaps` (JoF EJK `cg_servercmds.c`: "0 off, 1 map only (block
/// player model remaps), 2 map + model"). It gates server-sent remaps only.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum RemapLevel {
    /// `0`: server-sent remaps are ignored.
    Off,
    /// `1`: server-sent remaps apply except to shaders under `models/players/`.
    MapOnly,
    /// `2` (EternalJK's default): every server-sent remap applies.
    #[default]
    All,
}

impl RemapLevel {
    /// The level a `cg_remaps` integer selects. As in EternalJK, which tests
    /// `cg_remaps.integer == 1` and then any other non-zero value, every value but
    /// 0 and 1 means [`RemapLevel::All`].
    pub fn from_cvar(value: i64) -> Self {
        match value {
            0 => Self::Off,
            1 => Self::MapOnly,
            _ => Self::All,
        }
    }

    /// Whether a remap of `old` from `source` applies at this level. `old` is
    /// compared without case, as `Q_stricmpn` does.
    pub fn allows(self, source: RemapSource, old: &str) -> bool {
        if source != RemapSource::Server {
            return true;
        }
        match self {
            Self::Off => false,
            Self::MapOnly => !is_player_model_shader(old),
            Self::All => true,
        }
    }
}

/// `Q_stricmpn(name, "models/players/", 15)`: a player model's own shader.
fn is_player_model_shader(name: &str) -> bool {
    const PREFIX: &str = "models/players/";
    name.len() >= PREFIX.len()
        && name.as_bytes()[..PREFIX.len()]
            .iter()
            .zip(PREFIX.as_bytes())
            .all(|(a, b)| (if *a == b'\\' { b'/' } else { *a }).eq_ignore_ascii_case(b))
}

/// A shader name as rd-vanilla's shader table compares it: lower case, `/`
/// separators and no extension (`COM_StripExtension` drops a dot after the last
/// slash and everything following it).
pub fn remap_key(name: &str) -> String {
    let mut key = name.replace('\\', "/").to_ascii_lowercase();
    if let Some(dot) = key.rfind('.')
        && key.rfind('/').is_none_or(|slash| slash < dot)
    {
        key.truncate(dot);
    }
    key
}

/// One remap from `CS_SHADERSTATE`.
#[derive(Clone, Debug, PartialEq)]
pub struct ShaderStateEntry {
    pub old: String,
    pub new: String,
    /// Seconds of level time at which the remap happened; the new shader's
    /// animation clock starts there (`shader_t::timeOffset`).
    pub time_offset: f32,
}

/// Parse a `CS_SHADERSTATE` value, `old=new:offset@old=new:offset@...`, exactly as
/// `CG_ShaderStateChanged` (`cg_servercmds.c`) walks it: parsing stops at the first
/// entry missing its `=`, `:` or closing `@`, and that entry is dropped.
pub fn parse_shader_state(value: &str) -> Vec<ShaderStateEntry> {
    let mut entries = Vec::new();
    let mut rest = value;
    while !rest.is_empty() {
        let Some(equals) = rest.find('=') else { break };
        let old = &rest[..equals];
        let after_old = &rest[equals + 1..];
        let Some(colon) = after_old.find(':') else {
            break;
        };
        let new = &after_old[..colon];
        let after_new = &after_old[colon + 1..];
        let Some(at) = after_new.find('@') else { break };
        entries.push(ShaderStateEntry {
            old: old.to_owned(),
            new: new.to_owned(),
            time_offset: atof(&after_new[..at]),
        });
        rest = &after_new[at + 1..];
    }
    entries
}

/// The C library's `atof`: leading whitespace, then the longest decimal prefix;
/// 0 when there is none. The game module writes offsets with `%5.2f`, so they
/// may start with spaces.
pub fn atof(text: &str) -> f32 {
    let text = text.trim_start_matches([' ', '\t', '\n', '\r', '\x0b', '\x0c']);
    let bytes = text.as_bytes();
    let mut end = 0;
    if matches!(bytes.first(), Some(b'+' | b'-')) {
        end = 1;
    }
    let digits_start = end;
    while bytes.get(end).is_some_and(u8::is_ascii_digit) {
        end += 1;
    }
    let mut mantissa_digits = end - digits_start;
    if bytes.get(end) == Some(&b'.') {
        let fraction_start = end + 1;
        let mut fraction_end = fraction_start;
        while bytes.get(fraction_end).is_some_and(u8::is_ascii_digit) {
            fraction_end += 1;
        }
        mantissa_digits += fraction_end - fraction_start;
        if mantissa_digits > 0 {
            end = fraction_end;
        }
    }
    if mantissa_digits == 0 {
        return 0.0;
    }
    if matches!(bytes.get(end), Some(b'e' | b'E')) {
        let mut exponent_end = end + 1;
        if matches!(bytes.get(exponent_end), Some(b'+' | b'-')) {
            exponent_end += 1;
        }
        let exponent_digits = exponent_end;
        while bytes.get(exponent_end).is_some_and(u8::is_ascii_digit) {
            exponent_end += 1;
        }
        if exponent_end > exponent_digits {
            end = exponent_end;
        }
    }
    text[..end].parse().unwrap_or(0.0)
}

/// The worldspawn `remapshader` keys rd-vanilla applies while loading a map
/// (`tr_bsp.cpp` `R_LoadEntities`): every key starting with `remapshader`
/// (case-sensitive, so `remapshader2` counts too) whose value is `old;new`. A value
/// without `;` stops the scan, as rd-vanilla's `break` does. `vertexremapshader`
/// keys only apply under `r_vertexLight`, which SJK does not have.
pub fn worldspawn_remaps<'a>(
    fields: impl IntoIterator<Item = (&'a str, &'a str)>,
) -> Vec<(&'a str, &'a str)> {
    let mut remaps = Vec::new();
    for (key, value) in fields {
        if !key.starts_with("remapshader") {
            continue;
        }
        let Some((old, new)) = value.split_once(';') else {
            break;
        };
        remaps.push((old, new));
    }
    remaps
}

#[derive(Clone, Debug, PartialEq)]
struct RemapRecord {
    old: String,
    new: String,
    source: RemapSource,
    time_offset: Option<f32>,
    sequence: u64,
}

/// One applied remap, as [`ShaderRemaps::entries`] lists it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RemapEntry<'a> {
    pub old: &'a str,
    pub new: &'a str,
    pub source: RemapSource,
    pub time_offset: Option<f32>,
    /// Whether the entry is in effect: allowed by the [`RemapLevel`], not hidden
    /// by a later remap of the same shader, and not a remap to itself.
    pub active: bool,
}

/// The remap table. Each source keeps one remap per shader; the effective table
/// is rebuilt only when something changes, and [`ShaderRemaps::generation`]
/// tells users when to re-resolve their cached shaders.
#[derive(Clone, Debug, Default)]
pub struct ShaderRemaps {
    records: Vec<RemapRecord>,
    level: RemapLevel,
    next_sequence: u64,
    active: HashMap<String, String>,
    time_offsets: HashMap<String, f32>,
    generation: u64,
}

impl ShaderRemaps {
    pub fn new(level: RemapLevel) -> Self {
        Self {
            level,
            ..Self::default()
        }
    }

    /// Counter bumped whenever the effective table may have changed.
    pub fn generation(&self) -> u64 {
        self.generation
    }

    pub fn level(&self) -> RemapLevel {
        self.level
    }

    /// Change the `cg_remaps` gate; returns whether the effective table changed.
    pub fn set_level(&mut self, level: RemapLevel) -> bool {
        if self.level == level {
            return false;
        }
        self.level = level;
        self.rebuild()
    }

    /// Record `old -> new` from `source`, replacing that source's earlier remap of
    /// `old`. A `None` offset keeps the target's current offset, as rd-vanilla does
    /// for the console command's NULL `timeOffset`. Returns whether the effective
    /// table changed.
    pub fn remap(
        &mut self,
        source: RemapSource,
        old: &str,
        new: &str,
        time_offset: Option<f32>,
    ) -> bool {
        let old = remap_key(old);
        let new = remap_key(new);
        self.next_sequence += 1;
        let sequence = self.next_sequence;
        if let Some(record) = self
            .records
            .iter_mut()
            .find(|record| record.source == source && record.old == old)
        {
            record.new = new;
            record.time_offset = time_offset;
            record.sequence = sequence;
        } else {
            self.records.push(RemapRecord {
                old,
                new,
                source,
                time_offset,
                sequence,
            });
        }
        self.rebuild()
    }

    /// Drop every remap from `source`; returns whether the effective table changed.
    pub fn clear_source(&mut self, source: RemapSource) -> bool {
        let before = self.records.len();
        self.records.retain(|record| record.source != source);
        before != self.records.len() && self.rebuild()
    }

    /// Drop every remap (EternalJK's `clearRemaps`, and a new map).
    pub fn clear(&mut self) -> bool {
        if self.records.is_empty() {
            return false;
        }
        self.records.clear();
        self.rebuild()
    }

    pub fn is_empty(&self) -> bool {
        self.active.is_empty()
    }

    /// The shader to draw in place of `name`, if it is remapped. One level only.
    pub fn target(&self, name: &str) -> Option<&str> {
        if self.active.is_empty() {
            return None;
        }
        self.active.get(&remap_key(name)).map(String::as_str)
    }

    /// `name`, or the shader it is remapped to.
    pub fn resolve<'a>(&'a self, name: &'a str) -> &'a str {
        self.target(name).unwrap_or(name)
    }

    /// The animation time offset, in seconds, of a remap target (`shaderTime =
    /// time - timeOffset`); `None` when no applied remap set one.
    pub fn time_offset(&self, target: &str) -> Option<f32> {
        if self.time_offsets.is_empty() {
            return None;
        }
        self.time_offsets.get(&remap_key(target)).copied()
    }

    /// Every recorded remap in the order it was last set, marked active or not.
    pub fn entries(&self) -> Vec<RemapEntry<'_>> {
        let mut records: Vec<&RemapRecord> = self.records.iter().collect();
        records.sort_by_key(|record| record.sequence);
        records
            .into_iter()
            .map(|record| RemapEntry {
                old: &record.old,
                new: &record.new,
                source: record.source,
                time_offset: record.time_offset,
                active: record.old != record.new
                    && self
                        .winner(&record.old)
                        .is_some_and(|winner| std::ptr::eq(winner, record)),
            })
            .collect()
    }

    /// The latest allowed record for `old`, identity remaps included.
    fn winner(&self, old: &str) -> Option<&RemapRecord> {
        self.records
            .iter()
            .filter(|record| record.old == old && self.level.allows(record.source, &record.old))
            .max_by_key(|record| record.sequence)
    }

    fn rebuild(&mut self) -> bool {
        let mut records: Vec<&RemapRecord> = self
            .records
            .iter()
            .filter(|record| self.level.allows(record.source, &record.old))
            .collect();
        records.sort_by_key(|record| record.sequence);
        let mut active = HashMap::with_capacity(records.len());
        let mut time_offsets = HashMap::new();
        for record in records {
            if record.old == record.new {
                active.remove(&record.old);
            } else {
                active.insert(record.old.clone(), record.new.clone());
            }
            if let Some(offset) = record.time_offset {
                time_offsets.insert(record.new.clone(), offset);
            }
        }
        let changed = active != self.active || time_offsets != self.time_offsets;
        self.active = active;
        self.time_offsets = time_offsets;
        if changed {
            self.generation += 1;
        }
        changed
    }
}

#[cfg(test)]
mod tests;

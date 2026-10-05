//! JoF EJK's `cosmetics` console command (`CG_Cosmetics_f`): list the
//! installed hats or capes, wear or take one off by number or name, take
//! both off, set whose cosmetics are drawn, and list or choose jaPRO's
//! race-unlock hats (`CG_Cosmetics_Unlocks_f`, `cp_cosmetics`).

use super::{Catalog, VISIBILITY_CVAR, Visibility};
use crate::console::ViewerConsole;
use sjk_client::{CompatProfile, CosmeticSlot, CosmeticUnlockTable};

/// The userinfo cvar holding the jaPRO cosmetic worn.
const UNLOCKS_CVAR: &str = "cp_cosmetics";

/// What the server tells about jaPRO's unlocks: its profile and the
/// requirements its `cosmetics` command sent.
pub(crate) type Server<'a> = Option<(&'a CompatProfile, &'a CosmeticUnlockTable)>;

/// Completion and help entry.
pub(crate) const COMMANDS: &[(&str, &str)] = &[(
    "cosmetics",
    "List, wear or take off hats and capes: cosmetics <hats|capes|clear|visibility> [value]",
)];

const USAGE: [&str; 6] = [
    "Usage: ^3cosmetics <hats|capes|clear|visibility|unlocks> [value]^7",
    "  ^3hats^7 [num|name]    list hats, or wear one (again takes it off)",
    "  ^3capes^7 [num|name]   list capes, or wear one (again takes it off)",
    "  ^3clear^7              take off the hat and the cape",
    "  ^3visibility^7 [off|on|onlyme]   whose cosmetics are drawn",
    "  ^3unlocks^7 [num]      jaPRO server-granted cosmetics",
];

/// Run `cosmetics` with `args` against the installed `catalog`.
pub(crate) fn run(
    console: &mut ViewerConsole,
    catalog: &Catalog,
    server: Server<'_>,
    args: &[String],
) -> Result<Vec<String>, String> {
    let Some(category) = args.first() else {
        return Ok(USAGE.map(str::to_owned).to_vec());
    };
    match category.to_ascii_lowercase().as_str() {
        "hats" => wear(console, catalog, CosmeticSlot::Hat, args.get(1)),
        "capes" => wear(console, catalog, CosmeticSlot::Cape, args.get(1)),
        "clear" => {
            for slot in CosmeticSlot::ALL {
                set(console, slot, None);
            }
            Ok(vec!["Hat and cape removed.".to_owned()])
        }
        "visibility" => visibility(console, args.get(1)),
        "unlocks" => unlocks(console, server, args.get(1)),
        _ => Err(format!(
            "Unknown category '{category}'. Run ^3cosmetics^7 for usage."
        )),
    }
}

fn wear(
    console: &mut ViewerConsole,
    catalog: &Catalog,
    slot: CosmeticSlot,
    argument: Option<&String>,
) -> Result<Vec<String>, String> {
    let category = match slot {
        CosmeticSlot::Hat => "hats",
        CosmeticSlot::Cape => "capes",
    };
    let pieces = catalog.pieces(slot);
    if pieces.is_empty() {
        return Ok(vec![format!(
            "No {category} installed. Add JoF EJK's cosmetics (models/cosmetics/{category}/) \
             or set fs_basegame EternalJK and restart."
        )]);
    }
    let worn = worn(console, slot);
    let Some(argument) = argument else {
        let mut lines = vec![format!("^5Available {category}:")];
        for (index, piece) in pieces.iter().enumerate() {
            let mark = if worn
                .as_deref()
                .is_some_and(|worn| worn.eq_ignore_ascii_case(&piece.name))
            {
                "^2[X]^7"
            } else {
                "[ ]"
            };
            lines.push(format!("{index:2} {mark} {}", piece.name));
        }
        lines.push(format!(
            "Wear one with ^3cosmetics {category} <num>^7, take it off with the same command."
        ));
        if catalog.has_missing(slot) {
            lines.push(format!(
                "^3Get more {category} from JoF Launcher or Cloud.^7"
            ));
        }
        return Ok(lines);
    };
    // A name as well as a number (`CG_CosmeticForName`).
    let index = match catalog.position(slot, argument) {
        Some(index) => index,
        None => match argument.parse::<usize>() {
            Ok(index) if index < pieces.len() => index,
            Ok(index) => {
                return Err(format!(
                    "cosmetics {category}: Invalid range: {index} [0, {}]",
                    pieces.len() - 1
                ));
            }
            Err(_) => {
                return Err(format!(
                    "No such {category}: '{argument}'. Run ^3cosmetics {category}^7 to see the list."
                ));
            }
        },
    };
    let name = &pieces[index].name;
    if worn
        .as_deref()
        .is_some_and(|worn| worn.eq_ignore_ascii_case(name))
    {
        set(console, slot, None);
        Ok(vec![format!("^3'{name}' ^1removed")])
    } else {
        set(console, slot, Some(name));
        Ok(vec![format!("^3'{name}' ^2equipped")])
    }
}

fn visibility(
    console: &mut ViewerConsole,
    argument: Option<&String>,
) -> Result<Vec<String>, String> {
    let current = Visibility::from_cvar(console.integer_cvar(VISIBILITY_CVAR).unwrap_or(1));
    let Some(argument) = argument else {
        return Ok(vec![
            format!("Cosmetics visibility: ^3{}^7", current.label()),
            "Set it with ^3cosmetics visibility <off|on|onlyme>^7.".to_owned(),
        ]);
    };
    let next = match argument.to_ascii_lowercase().as_str() {
        "off" | "0" => Visibility::Off,
        "on" | "1" => Visibility::On,
        "onlyme" | "me" | "2" => Visibility::OnlyMe,
        _ => return Err("Use ^3cosmetics visibility <off|on|onlyme>^7.".to_owned()),
    };
    console.set_cvar(VISIBILITY_CVAR, next.cvar_value());
    Ok(vec![format!("Cosmetics visibility: ^3{}^7", next.label())])
}

/// `cosmetics unlocks [num]`: jaPRO's hats, the one worn marked and each
/// one's requirement when the server sent it; a number wears that one alone
/// or, worn already, takes it off.
fn unlocks(
    console: &mut ViewerConsole,
    server: Server<'_>,
    argument: Option<&String>,
) -> Result<Vec<String>, String> {
    let Some((_, table)) = server.filter(|(profile, _)| **profile == CompatProfile::TaystJk) else {
        return Ok(vec!["This server has no cosmetic unlocks.".to_owned()]);
    };
    let bits = console
        .integer_cvar(UNLOCKS_CVAR)
        .and_then(|bits| u32::try_from(bits).ok())
        .unwrap_or(0);
    let hats = sjk_client::JAPRO_HATS;
    let Some(argument) = argument else {
        return Ok(hats
            .iter()
            .enumerate()
            .map(|(index, (name, _))| {
                let mark = if bits & (1 << index) != 0 { "X" } else { " " };
                let requirement = table
                    .active()
                    .find(|row| usize::from(row.bitvalue) == index)
                    .map(|row| {
                        let style = sjk_client::race_style_name(row.style);
                        match row.duration {
                            0 => format!(" ^3(requires {} {style})^7", row.map_name()),
                            millis => format!(
                                " ^3(requires {} {style} in under {:.3} seconds)^7",
                                row.map_name(),
                                f64::from(millis) * 0.001
                            ),
                        }
                    })
                    .unwrap_or_default();
                format!("{index:2} [{mark}] {name}{requirement}")
            })
            .collect());
    };
    let index = argument
        .parse::<usize>()
        .ok()
        .filter(|index| *index < hats.len())
        .ok_or_else(|| {
            format!(
                "cosmetics unlocks: Invalid range: {argument} [0, {}]",
                hats.len() - 1
            )
        })?;
    // One at a time, as JoF's radio buttons.
    let bit = 1_u32 << index;
    let worn = bits & bit == 0;
    let value = if worn { bit } else { 0 };
    console.set_cvar(UNLOCKS_CVAR, &value.to_string());
    Ok(vec![format!(
        "{} {}^7",
        hats[index].0,
        if worn { "^2Enabled" } else { "^1Disabled" }
    )])
}

fn worn(console: &ViewerConsole, slot: CosmeticSlot) -> Option<String> {
    let value = console.text_value(slot.cvar())?;
    sjk_client::split_color_value(value).1.map(str::to_owned)
}

fn set(console: &mut ViewerConsole, slot: CosmeticSlot, name: Option<&str>) {
    let colour = console
        .text_value(slot.cvar())
        .map_or(4, |value| sjk_client::split_color_value(value).0);
    console.set_cvar(slot.cvar(), &sjk_client::join_color_value(colour, name));
}

impl crate::GpuState {
    /// `cosmetics ...`, listing what the mounted game data holds.
    pub(crate) fn cosmetics_command(&mut self, args: &[String]) -> Result<Vec<String>, String> {
        let vfs = self.vfs.clone().ok_or("No game data mounted")?;
        let catalog = Catalog::scan(&vfs);
        let server = self
            .live_session
            .as_ref()
            .map(|session| (session.compat_profile(), session.cosmetic_unlocks()));
        let console = self.console.as_mut().ok_or("Console unavailable")?;
        run(console, &catalog, server, args)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sjk_vfs::VirtualFileSystem;

    fn catalog() -> Catalog {
        let mut vfs = VirtualFileSystem::new();
        vfs.mount_memory(
            "cosmetics",
            [
                ("models/cosmetics/hats/santahat.md3", b"x".to_vec()),
                ("models/cosmetics/hats/tophat.md3", b"x".to_vec()),
                ("models/cosmetics/capes/royalcape.md3", b"x".to_vec()),
            ],
        )
        .unwrap();
        Catalog::scan(&vfs)
    }

    #[test]
    fn wearing_by_number_or_name_keeps_the_saber_colour() {
        let directory = tempfile::tempdir().unwrap();
        let mut console = ViewerConsole::new(directory.path().join("config.cfg")).unwrap();
        let catalog = catalog();
        console.set_cvar("color1", "3");
        let args = |list: &[&str]| list.iter().map(|arg| (*arg).to_owned()).collect::<Vec<_>>();
        run(&mut console, &catalog, None, &args(&["hats", "1"])).unwrap();
        assert_eq!(console.text_value("color1"), Some("3tophat"));
        run(&mut console, &catalog, None, &args(&["hats", "SantaHat"])).unwrap();
        assert_eq!(console.text_value("color1"), Some("3santahat"));
        // The same piece again takes it off.
        run(&mut console, &catalog, None, &args(&["hats", "santahat"])).unwrap();
        assert_eq!(console.text_value("color1"), Some("3"));
        run(&mut console, &catalog, None, &args(&["capes", "0"])).unwrap();
        assert_eq!(console.text_value("color2"), Some("4royalcape"));
        let listed = run(&mut console, &catalog, None, &args(&["capes"])).unwrap();
        assert!(listed.iter().any(|line| line.contains("^2[X]^7 royalcape")));
        run(&mut console, &catalog, None, &args(&["clear"])).unwrap();
        assert_eq!(console.text_value("color2"), Some("4"));
        assert!(run(&mut console, &catalog, None, &args(&["hats", "9"])).is_err());
        assert!(run(&mut console, &catalog, None, &args(&["hats", "crown"])).is_err());
        run(
            &mut console,
            &catalog,
            None,
            &args(&["visibility", "onlyme"]),
        )
        .unwrap();
        assert_eq!(console.integer_cvar(VISIBILITY_CVAR), Some(2));
    }

    #[test]
    fn unlocks_list_and_choose_one_on_japro_only() {
        let directory = tempfile::tempdir().unwrap();
        let mut console = ViewerConsole::new(directory.path().join("config.cfg")).unwrap();
        let catalog = catalog();
        let mut table = CosmeticUnlockTable::default();
        table.apply_payload(b"6:mp/ffa3:1:12500");
        let japro = CompatProfile::TaystJk;
        let server = Some((&japro, &table));
        let args = |list: &[&str]| list.iter().map(|arg| (*arg).to_owned()).collect::<Vec<_>>();
        let listed = run(&mut console, &catalog, server, &args(&["unlocks"])).unwrap();
        assert_eq!(listed.len(), 7);
        assert_eq!(listed[0], " 0 [ ] Santa hat");
        assert_eq!(
            listed[6],
            " 6 [ ] Top hat ^3(requires mp/ffa3 jka in under 12.500 seconds)^7"
        );
        run(&mut console, &catalog, server, &args(&["unlocks", "6"])).unwrap();
        assert_eq!(console.integer_cvar(UNLOCKS_CVAR), Some(64));
        let listed = run(&mut console, &catalog, server, &args(&["unlocks"])).unwrap();
        assert!(listed[6].starts_with(" 6 [X] Top hat"));
        // Another one replaces it; the same one again takes it off.
        run(&mut console, &catalog, server, &args(&["unlocks", "1"])).unwrap();
        assert_eq!(console.integer_cvar(UNLOCKS_CVAR), Some(2));
        run(&mut console, &catalog, server, &args(&["unlocks", "1"])).unwrap();
        assert_eq!(console.integer_cvar(UNLOCKS_CVAR), Some(0));
        assert!(run(&mut console, &catalog, server, &args(&["unlocks", "7"])).is_err());
        let ja_plus = CompatProfile::JaPlus { version: None };
        let other = run(
            &mut console,
            &catalog,
            Some((&ja_plus, &table)),
            &args(&["unlocks"]),
        );
        assert_eq!(other.unwrap(), ["This server has no cosmetic unlocks."]);
    }

    #[test]
    fn the_catalogue_lists_installed_pieces_and_knows_what_is_missing() {
        let catalog = catalog();
        let hats: Vec<&str> = catalog
            .pieces(CosmeticSlot::Hat)
            .iter()
            .map(|piece| piece.name.as_str())
            .collect();
        assert_eq!(hats, ["santahat", "tophat"]);
        assert!(catalog.has_missing(CosmeticSlot::Hat));
        assert_eq!(catalog.position(CosmeticSlot::Cape, "RoyalCape"), Some(0));
        // Older packs' folder counts when the new one is empty.
        let mut vfs = VirtualFileSystem::new();
        vfs.mount_memory(
            "legacy",
            [("models/players/hats/fedora.md3", b"x".to_vec())],
        )
        .unwrap();
        let legacy = Catalog::scan(&vfs);
        assert_eq!(legacy.pieces(CosmeticSlot::Hat)[0].name, "fedora");
        assert_eq!(
            super::super::model_path(&vfs, CosmeticSlot::Hat, "fedora").as_deref(),
            Some("models/players/hats/fedora.md3")
        );
    }
}

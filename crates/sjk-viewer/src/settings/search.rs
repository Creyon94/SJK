//! Classic+ search over every setting of Settings' OPTIONS tab: the rows of
//! each OPTIONS group in the group list's order, each with its group's name,
//! so a search shows its results under their groups' headings and the detail
//! box can say where a setting lives.

use super::catalog::*;
use super::{Group, help, settings};
use std::sync::OnceLock;

/// Every searchable setting, in the OPTIONS groups' order, and its group.
struct Searchable {
    rows: Vec<Setting>,
    groups: Vec<&'static str>,
}

fn searchable() -> &'static Searchable {
    static SEARCHABLE: OnceLock<Searchable> = OnceLock::new();
    SEARCHABLE.get_or_init(|| {
        let tab =
            |caption: &str| settings(TABS.iter().position(|tab| *tab == caption).unwrap_or(0));
        let sources: [(&'static str, &'static [Setting]); 11] = [
            ("Video", tab("VIDEO")),
            ("Sound", tab("AUDIO")),
            ("Mouse", tab("CONTROLS")),
            ("Game options", Group::GameOptions.rows()),
            ("Interface", Group::Interface.rows()),
            ("HUD", Group::Hud.rows()),
            ("Scoreboard", Group::Scoreboard.rows()),
            ("Network", tab("NETWORK")),
            ("Renderer: image", RENDER_IMAGE),
            ("Renderer: lighting", RENDER_LIGHTING),
            ("Renderer: shadows", RENDER_SHADOWS),
        ];
        let mut found = Searchable {
            rows: Vec::new(),
            groups: Vec::new(),
        };
        for (group, rows) in sources {
            for setting in rows {
                if !found.rows.iter().any(|seen| seen.cvar == setting.cvar) {
                    found.rows.push(*setting);
                    found.groups.push(group);
                }
            }
        }
        found
    })
}

/// The rows a search section shows (all of them; the panel filters).
pub(super) fn rows() -> &'static [Setting] {
    &searchable().rows
}

/// The OPTIONS group of searchable row `row`.
pub(super) fn group(row: usize) -> Option<&'static str> {
    searchable().groups.get(row).copied()
}

/// Whether row `setting` of group `group` matches `query` (lower case): its
/// label, console name, description or group.
fn matches(setting: &Setting, group: &str, query: &str) -> bool {
    let has = |text: &str| text.to_ascii_lowercase().contains(query);
    has(setting.label)
        || has(setting.cvar)
        || has(group)
        || help::help(setting.cvar).is_some_and(has)
}

/// The rows matching `query`, each group's results after its heading.
pub(super) fn lines(query: &str) -> Vec<super::Line> {
    let query = query.trim().to_ascii_lowercase();
    let found = searchable();
    let mut lines = Vec::new();
    let mut last_group = None;
    for (row, (setting, group)) in found.rows.iter().zip(&found.groups).enumerate() {
        if !matches(setting, group, &query) {
            continue;
        }
        if last_group != Some(*group) {
            lines.push(super::Line::Heading(group));
            last_group = Some(*group);
        }
        lines.push(super::Line::Row(row));
    }
    lines
}

/// How many settings match `query` (the number of rows [`lines`] would show).
pub(super) fn count(query: &str) -> usize {
    lines(query)
        .iter()
        .filter(|line| matches!(line, super::Line::Row(_)))
        .count()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_option_is_searchable_once() {
        let rows = rows();
        for (index, setting) in rows.iter().enumerate() {
            assert!(
                rows[..index].iter().all(|seen| seen.cvar != setting.cvar),
                "{} twice",
                setting.cvar
            );
        }
        for caption in TABS {
            let tab = TABS.iter().position(|tab| *tab == caption).unwrap();
            for setting in settings(tab) {
                assert!(
                    rows.iter().any(|row| row.cvar == setting.cvar),
                    "{} ({caption}) cannot be found",
                    setting.cvar
                );
            }
        }
    }

    #[test]
    fn results_sit_under_their_groups() {
        let lines = lines("fov");
        assert_eq!(lines[0], super::super::Line::Heading("Video"));
        assert!(lines.iter().any(|line| matches!(
            line,
            super::super::Line::Row(row) if rows()[*row].cvar == "cg_fov"
        )));
        // A description finds its setting: "brightness" is in r_gamma's.
        let bright = super::lines("brightness");
        assert!(
            bright
                .iter()
                .any(|line| matches!(line, super::super::Line::Row(_)))
        );
        assert!(super::lines("no such setting at all").is_empty());
        assert_eq!(count("no such setting at all"), 0);
        assert!(count("fov") >= 1);
        // A group name lists the group.
        let hud = super::lines("scoreboard");
        assert!(hud.contains(&super::super::Line::Heading("Scoreboard")));
    }
}

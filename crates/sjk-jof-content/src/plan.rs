//! Which groups the player wants, and what a sync does about it.

use crate::store::{AssetState, Snapshot};
use std::collections::BTreeSet;

/// The groups chosen (`jof.contentGroups`): the repository's defaults until the
/// player changes one, then the ids chosen.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Selection {
    chosen: Option<BTreeSet<u32>>,
}

/// The setting's value before any choice.
pub const DEFAULT: &str = "default";

impl Selection {
    /// `default`, `none`, or group ids separated by spaces or commas.
    pub fn parse(value: &str) -> Self {
        let value = value.trim();
        if value.is_empty() || value.eq_ignore_ascii_case(DEFAULT) {
            return Self::default();
        }
        Self {
            chosen: Some(
                value
                    .split([' ', ','])
                    .filter_map(|id| id.trim().parse().ok())
                    .collect(),
            ),
        }
    }

    /// The setting's value.
    pub fn saved(&self) -> String {
        match &self.chosen {
            None => DEFAULT.to_owned(),
            Some(ids) if ids.is_empty() => "none".to_owned(),
            Some(ids) => ids.iter().map(u32::to_string).collect::<Vec<_>>().join(" "),
        }
    }

    pub fn chosen(&self, group: &crate::Group) -> bool {
        match &self.chosen {
            None => group.kind.default_chosen(),
            Some(ids) => ids.contains(&group.id),
        }
    }

    /// Choose or leave out one group; the others keep what they show now.
    pub fn toggle(&mut self, manifest: &crate::Manifest, group: u32) {
        let mut ids: BTreeSet<u32> = manifest
            .groups
            .iter()
            .filter(|known| self.chosen(known))
            .map(|known| known.id)
            .collect();
        if !ids.remove(&group) {
            ids.insert(group);
        }
        self.chosen = Some(ids);
    }
}

/// What a sync will do.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Plan {
    /// `(group, asset)` indices of the files to fetch, smallest first so the
    /// most files are ready soonest.
    pub downloads: Vec<(usize, usize)>,
    pub download_bytes: u64,
    /// File names to delete from SJK's JoF folder.
    pub removals: Vec<String>,
    pub removal_bytes: u64,
}

impl Plan {
    /// Fetch what the chosen groups miss; delete the files SJK downloaded that
    /// no chosen group lists, and its strays. Files in the game's folder, and
    /// files put in SJK's folder by hand, are never touched.
    pub fn new(snapshot: &Snapshot, selection: &Selection) -> Self {
        let mut plan = Plan::default();
        let kept: BTreeSet<String> = snapshot
            .manifest
            .groups
            .iter()
            .filter(|group| selection.chosen(group))
            .flat_map(|group| &group.assets)
            .map(|asset| asset.name.to_ascii_lowercase())
            .collect();
        let mut fetched = BTreeSet::new();
        for (group_index, group) in snapshot.manifest.groups.iter().enumerate() {
            let chosen = selection.chosen(group);
            for (asset_index, asset) in group.assets.iter().enumerate() {
                let name = asset.name.to_ascii_lowercase();
                match snapshot.state(group_index, asset_index) {
                    AssetState::Missing | AssetState::Outdated
                        if chosen && fetched.insert(name.clone()) =>
                    {
                        plan.downloads.push((group_index, asset_index));
                        plan.download_bytes += asset.size;
                    }
                    AssetState::Installed | AssetState::Outdated
                        if !kept.contains(&name)
                            && snapshot.downloaded.contains(&name)
                            && !plan
                                .removals
                                .iter()
                                .any(|known| known.eq_ignore_ascii_case(&name)) =>
                    {
                        plan.removals.push(asset.name.clone());
                        plan.removal_bytes += asset.size;
                    }
                    _ => {}
                }
            }
        }
        for (name, size) in &snapshot.strays {
            plan.removals.push(name.clone());
            plan.removal_bytes += size;
        }
        let manifest = &snapshot.manifest;
        plan.downloads
            .sort_by_key(|&(group, asset)| manifest.groups[group].assets[asset].size);
        plan
    }

    pub fn is_empty(&self) -> bool {
        self.downloads.is_empty() && self.removals.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::store::tests::manifest;

    fn snapshot(states: Vec<Vec<AssetState>>) -> Snapshot {
        Snapshot {
            manifest: manifest(),
            states,
            strays: vec![("gone.pk3".into(), 9)],
            downloaded: ["c.pk3".to_owned()].into(),
        }
    }

    #[test]
    fn the_defaults_choose_all_but_optional_groups_and_save_once_changed() {
        let manifest = manifest();
        let mut selection = Selection::parse("default");
        assert!(selection.chosen(&manifest.groups[0]));
        assert!(!selection.chosen(&manifest.groups[1]));
        assert_eq!(selection.saved(), "default");
        selection.toggle(&manifest, 2);
        assert_eq!(selection.saved(), "1 2");
        selection.toggle(&manifest, 1);
        selection.toggle(&manifest, 2);
        assert_eq!(selection.saved(), "none");
        assert_eq!(Selection::parse("none"), selection);
        assert_eq!(Selection::parse("2,1"), Selection::parse("1 2"));
    }

    #[test]
    fn a_sync_fetches_chosen_files_and_removes_only_its_own() {
        use AssetState::*;
        let snapshot = snapshot(vec![vec![Missing, InGameFolder], vec![Installed]]);
        let plan = Plan::new(&snapshot, &Selection::parse("1"));
        assert_eq!(plan.downloads, [(0, 0)]);
        assert_eq!(plan.download_bytes, 3);
        assert_eq!(plan.removals, ["c.pk3", "gone.pk3"]);
        assert_eq!(plan.removal_bytes, 2 + 9);

        let by_hand = Snapshot {
            downloaded: BTreeSet::new(),
            ..snapshot.clone()
        };
        assert_eq!(
            Plan::new(&by_hand, &Selection::parse("1")).removals,
            ["gone.pk3"],
            "c.pk3 was put there by hand"
        );

        let plan = Plan::new(&snapshot, &Selection::parse("none"));
        assert!(plan.downloads.is_empty());
        assert_eq!(
            plan.removals,
            ["c.pk3", "gone.pk3"],
            "the game folder's b.pk3 is left alone"
        );

        let snapshot = snapshot_without_strays(vec![vec![Outdated, Missing], vec![Missing]]);
        let plan = Plan::new(&snapshot, &Selection::parse("1 2"));
        assert_eq!(plan.downloads, [(1, 0), (0, 0), (0, 1)], "smallest first");
        assert!(plan.removals.is_empty());
    }

    fn snapshot_without_strays(states: Vec<Vec<AssetState>>) -> Snapshot {
        Snapshot {
            strays: Vec::new(),
            ..snapshot(states)
        }
    }
}

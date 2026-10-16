//! The JoF content card (`jof.downloads`, Settings > Mods > JoF content): an
//! SJK UI pop-up in every menu style. The switch that loads the folder, the
//! repository's groups with a tick each and what is here of them, what a
//! download will do, its progress, and Check again and Download.
//!
//! The job runs on its own thread ([`sjk_jof_content::job`]); a download goes
//! on while the card is closed, and the card shows it again when reopened.

use crate::menu::sjk::{Frame, TextTarget, color, kit, text};
use crate::menu_widgets::{BACK_TOKEN, MenuCanvas, TextFamily};
use sjk_jof_content::job::{Job, Outcome, Phase, Settings};
use sjk_jof_content::{AssetState, Plan, Selection, Snapshot, Target, human_size};
use sjk_ui::{Color, DrawCommand, FontWeight, InputEvent, TextAlign, UiEventKind};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use winit::event::{ElementState, KeyEvent};
use winit::keyboard::{KeyCode, PhysicalKey};

/// Group rows shown at once.
const ROWS: usize = 7;
const ROW_HEIGHT: f32 = 62.0;
const SWITCH_TOKEN: u16 = 800;
const CHECK_TOKEN: u16 = 801;
const MAIN_TOKEN: u16 = 802;
/// The card, in frame pixels.
const CARD: [f32; 4] = [360.0, 96.0, 1200.0, 888.0];
const LEFT: f32 = 404.0;
const RIGHT: f32 = 1516.0;
const GROUPS_TOP: f32 = 372.0;

/// What the console does after an input.
pub(crate) enum Action {
    None,
    Close,
    /// Store `jof.content`.
    Switch(bool),
    /// Store `jof.contentGroups`.
    Groups(String),
    /// Start a refresh with the settings.
    Check,
    /// Start a sync with the settings.
    Download,
}

/// One group's line, from the latest snapshot.
#[derive(Clone, Copy, Default)]
struct Summary {
    /// Files SJK would fetch, and how many of them are here.
    files: usize,
    here: usize,
    size: u64,
    /// Bytes a sync would fetch while chosen.
    missing: u64,
    /// Bytes of SJK's own copies a sync would delete while not chosen.
    ours: u64,
    /// Files only for JoF's own client.
    elsewhere: usize,
}

pub(crate) struct Card {
    pub(crate) open: bool,
    pub(crate) owns_console: bool,
    game_data: PathBuf,
    job: Job,
    on: bool,
    selection: Selection,
    folder: String,
    /// 0 the switch, then the groups, then Check again and the main button.
    focus: usize,
    first: usize,
    /// The snapshot and selection the plan and summaries were made from.
    made_from: Option<(usize, Selection)>,
    snapshot: Option<Arc<Snapshot>>,
    plan: Plan,
    summaries: Vec<Summary>,
    phase: Phase,
    ui: MenuCanvas,
}

impl Default for Card {
    fn default() -> Self {
        Self {
            open: false,
            owns_console: false,
            game_data: PathBuf::new(),
            job: Job::default(),
            on: false,
            selection: Selection::default(),
            folder: String::new(),
            focus: 0,
            first: 0,
            made_from: None,
            snapshot: None,
            plan: Plan::default(),
            summaries: Vec::new(),
            phase: Phase::Idle,
            ui: MenuCanvas::with_text_capacity(96),
        }
    }
}

impl Card {
    /// Show the card with the settings' values. The returned flag asks for a
    /// refresh (none is running and nothing was looked at yet).
    pub(crate) fn open(
        &mut self,
        game_data: &Path,
        on: bool,
        selection: Selection,
        folder: &Path,
    ) -> bool {
        self.game_data = game_data.to_path_buf();
        self.on = on;
        self.selection = selection;
        self.folder = folder.display().to_string();
        self.open = true;
        self.focus = 0;
        self.first = 0;
        self.update();
        !self.job.busy() && self.snapshot.is_none()
    }

    pub(crate) fn game_data(&self) -> &Path {
        &self.game_data
    }

    pub(crate) fn refresh(&mut self, settings: Settings) {
        self.job.refresh(settings);
    }

    pub(crate) fn download(&mut self, settings: Settings) {
        self.job.sync(settings, self.selection.clone());
    }

    /// Whether a refresh or download is still at work.
    #[cfg(test)]
    pub(crate) fn busy(&self) -> bool {
        self.job.busy()
    }

    pub(crate) fn draw_list(&self) -> &sjk_ui::DrawList {
        self.ui.draw_list()
    }

    fn groups(&self) -> usize {
        self.snapshot
            .as_ref()
            .map_or(0, |snapshot| snapshot.manifest.groups.len())
    }

    fn check_focus(&self) -> usize {
        self.groups() + 1
    }

    fn main_focus(&self) -> usize {
        self.groups() + 2
    }

    /// Take the job's latest state; the plan is made again only when the
    /// snapshot or the choice changed.
    fn update(&mut self) {
        let progress = self.job.progress();
        self.phase = progress.phase;
        if let Some(snapshot) = progress.snapshot {
            self.snapshot = Some(snapshot);
        }
        let Some(snapshot) = self.snapshot.clone() else {
            return;
        };
        let key = (Arc::as_ptr(&snapshot) as usize, self.selection.clone());
        if self.made_from.as_ref() == Some(&key) {
            return;
        }
        self.plan = Plan::new(&snapshot, &self.selection);
        self.summaries = summaries(&snapshot);
        self.made_from = Some(key);
        self.focus = self.focus.min(self.main_focus());
    }

    fn set_focus(&mut self, focus: usize) {
        self.focus = focus.min(self.main_focus());
        if (1..=self.groups()).contains(&self.focus) {
            let row = self.focus - 1;
            if row < self.first {
                self.first = row;
            }
            if row >= self.first + ROWS {
                self.first = row + 1 - ROWS;
            }
        }
    }

    fn running(&self) -> bool {
        self.phase.running() || self.job.busy()
    }

    /// What the main button does now, its label and whether it can.
    fn main_button(&self) -> (String, bool) {
        if self.running() {
            return ("Stop".into(), true);
        }
        if self.snapshot.is_none() {
            return ("Download".into(), false);
        }
        if !self.on {
            return ("Download".into(), false);
        }
        if !self.plan.downloads.is_empty() {
            return (
                format!("Download {}", human_size(self.plan.download_bytes)),
                true,
            );
        }
        if !self.plan.removals.is_empty() {
            return (
                format!("Remove {}", human_size(self.plan.removal_bytes)),
                true,
            );
        }
        ("Up to date".into(), false)
    }

    fn activate(&mut self) -> Action {
        let groups = self.groups();
        match self.focus {
            0 => {
                self.on = !self.on;
                Action::Switch(self.on)
            }
            focus if focus <= groups => self.toggle_group(focus - 1),
            focus if focus == groups + 1 => self.check(),
            _ => self.main(),
        }
    }

    fn toggle_group(&mut self, index: usize) -> Action {
        let Some(snapshot) = self.snapshot.clone() else {
            return Action::None;
        };
        let Some(group) = snapshot.manifest.groups.get(index) else {
            return Action::None;
        };
        if group.size() == 0 {
            // Only files for JoF's own client: nothing to choose.
            return Action::None;
        }
        self.selection.toggle(&snapshot.manifest, group.id);
        self.update();
        Action::Groups(self.selection.saved())
    }

    fn check(&mut self) -> Action {
        if self.running() {
            Action::None
        } else {
            Action::Check
        }
    }

    fn main(&mut self) -> Action {
        if self.running() {
            self.job.cancel();
            return Action::None;
        }
        if self.main_button().1 {
            Action::Download
        } else {
            Action::None
        }
    }

    pub(crate) fn key(&mut self, event: &KeyEvent) -> Action {
        if event.state != ElementState::Pressed || event.repeat {
            return Action::None;
        }
        let PhysicalKey::Code(key) = event.physical_key else {
            return Action::None;
        };
        self.update();
        let groups = self.groups();
        match key {
            KeyCode::Escape => return Action::Close,
            KeyCode::Enter | KeyCode::NumpadEnter | KeyCode::Space => return self.activate(),
            KeyCode::F5 => return self.check(),
            KeyCode::ArrowUp => self.set_focus(self.focus.saturating_sub(1)),
            KeyCode::ArrowDown => self.set_focus(self.focus + 1),
            KeyCode::ArrowLeft if self.focus == self.main_focus() => {
                self.set_focus(self.check_focus())
            }
            KeyCode::ArrowRight if self.focus == self.check_focus() => {
                self.set_focus(self.main_focus())
            }
            KeyCode::PageUp => self.set_focus(self.focus.saturating_sub(ROWS).max(1).min(groups)),
            KeyCode::PageDown => self.set_focus((self.focus + ROWS).min(groups.max(1))),
            _ => {}
        }
        Action::None
    }

    pub(crate) fn pointer(&mut self, event: InputEvent) -> Action {
        let Some(event) = self.ui.pointer(event) else {
            return Action::None;
        };
        self.update();
        if event.kind == UiEventKind::Wheel {
            let groups = self.groups();
            let direction = event.delta.map_or(0, |delta| -delta.y.signum() as isize);
            self.first = self
                .first
                .saturating_add_signed(direction)
                .min(groups.saturating_sub(ROWS));
            return Action::None;
        }
        let Some(token) = event.token else {
            return Action::None;
        };
        let focus = match token {
            SWITCH_TOKEN => 0,
            CHECK_TOKEN => self.check_focus(),
            MAIN_TOKEN => self.main_focus(),
            BACK_TOKEN => usize::MAX,
            row if usize::from(row) < ROWS => self.first + usize::from(row) + 1,
            _ => return Action::None,
        };
        match event.kind {
            UiEventKind::HoverEnter if focus != usize::MAX => {
                self.focus = focus.min(self.main_focus());
                Action::None
            }
            UiEventKind::Activate if focus == usize::MAX => Action::Close,
            UiEventKind::Activate => {
                self.focus = focus.min(self.main_focus());
                self.activate()
            }
            _ => Action::None,
        }
    }

    pub(crate) fn append(&mut self, target: TextTarget<'_>, viewport: [f32; 2]) {
        self.update();
        let frame = Frame::new(viewport);
        let s = frame.s;
        let (status, status_colour, ratio) = self.status();
        let (label, enabled) = self.main_button();
        let running = self.running();
        let check_focus = self.check_focus();
        let main_focus = self.main_focus();
        self.ui.begin_transparent(viewport);
        kit::scrim(&mut self.ui, viewport);
        kit::card(&mut self.ui, &frame, CARD);
        let ui = &mut self.ui;
        text(
            ui,
            TextFamily::Display,
            format_args!("JoF content"),
            frame.rect(LEFT, 126.0, 700.0, 50.0),
            36.0 * s,
            color::GOLD_BRIGHT,
            FontWeight::Semibold,
            TextAlign::Start,
        );
        text(
            ui,
            TextFamily::Body,
            format_args!(
                "JoF's maps, skins, hilts and packs, from JoF's repository as JoF's launcher installs them."
            ),
            frame.rect(LEFT, 180.0, 1100.0, 26.0),
            18.0 * s,
            color::MUTED,
            FontWeight::Regular,
            TextAlign::Start,
        );
        ui.hit_region(BACK_TOKEN, frame.rect(1420.0, 124.0, 100.0, 44.0));
        text(
            ui,
            TextFamily::Body,
            format_args!("Close"),
            frame.rect(1420.0, 132.0, 96.0, 28.0),
            17.0 * s,
            if ui.token_hovered(BACK_TOKEN) {
                color::TEXT
            } else {
                color::MUTED
            },
            FontWeight::Regular,
            TextAlign::End,
        );

        // The switch: the opt-in, said plainly.
        let top = 222.0;
        if self.focus == 0 {
            kit::band(ui, &frame, [LEFT - 16.0, top, RIGHT - LEFT + 32.0, 116.0]);
        }
        text(
            ui,
            TextFamily::Body,
            format_args!("Use JoF content"),
            frame.rect(LEFT, top + 12.0, 700.0, 30.0),
            21.0 * s,
            color::TEXT,
            FontWeight::Semibold,
            TextAlign::Start,
        );
        kit::switch(ui, &frame, RIGHT, top + 27.0, self.on, self.focus == 0);
        let explained = if self.on {
            "On: SJK loads these files from its JoF folder, from the next map. Off leaves them on disk unused."
        } else {
            "Off by default. Turn it on to download and load JoF's files; nothing is fetched until you press Download."
        };
        text(
            ui,
            TextFamily::Body,
            format_args!("{explained}"),
            frame.rect(LEFT, top + 50.0, 1100.0, 24.0),
            16.0 * s,
            color::MUTED,
            FontWeight::Regular,
            TextAlign::Start,
        );
        text(
            ui,
            TextFamily::Body,
            format_args!(
                "Folder: {}   ·   files JoF's launcher put in your game folder are used as they are",
                shorten(&self.folder, 70)
            ),
            frame.rect(LEFT, top + 78.0, 1100.0, 24.0),
            15.0 * s,
            color::QUIET,
            FontWeight::Regular,
            TextAlign::Start,
        );
        ui.hit_region(
            SWITCH_TOKEN,
            frame.rect(LEFT - 16.0, top, RIGHT - LEFT + 32.0, 116.0),
        );

        kit::heading(ui, &frame, LEFT, 346.0, RIGHT - LEFT, "Groups");
        match &self.snapshot {
            None => {
                let waiting = match &self.phase {
                    Phase::Failed(_) => "The list could not be fetched. Check again to retry.",
                    _ => "Fetching the list from JoF's repository...",
                };
                text(
                    ui,
                    TextFamily::Body,
                    format_args!("{waiting}"),
                    frame.rect(LEFT, GROUPS_TOP + 10.0, 1100.0, 28.0),
                    18.0 * s,
                    color::MUTED,
                    FontWeight::Regular,
                    TextAlign::Start,
                );
            }
            Some(snapshot) => {
                let groups = &snapshot.manifest.groups;
                for (slot, (index, group)) in groups
                    .iter()
                    .enumerate()
                    .skip(self.first)
                    .take(ROWS)
                    .enumerate()
                {
                    let top = GROUPS_TOP + slot as f32 * ROW_HEIGHT;
                    let focused = self.focus == index + 1;
                    let chosen = self.selection.chosen(group);
                    let summary = self.summaries.get(index).copied().unwrap_or_default();
                    let fetchable = summary.files > 0;
                    if focused {
                        kit::band(
                            ui,
                            &frame,
                            [LEFT - 16.0, top, RIGHT - LEFT + 32.0, ROW_HEIGHT - 4.0],
                        );
                    }
                    if fetchable {
                        kit::tick(ui, &frame, LEFT, top + 29.0, chosen, focused);
                    }
                    text(
                        ui,
                        TextFamily::Body,
                        format_args!("{}", group.name),
                        frame.rect(LEFT + 42.0, top + 6.0, 560.0, 28.0),
                        20.0 * s,
                        if fetchable { color::TEXT } else { color::QUIET },
                        FontWeight::Semibold,
                        TextAlign::Start,
                    );
                    text(
                        ui,
                        TextFamily::Body,
                        format_args!("{}  ·  {}", group.kind.label(), cut(&group.description, 80)),
                        frame.rect(LEFT + 42.0, top + 33.0, 600.0, 22.0),
                        15.0 * s,
                        color::MUTED,
                        FontWeight::Regular,
                        TextAlign::Start,
                    );
                    let (state, colour) = group_state(&summary, chosen);
                    text(
                        ui,
                        TextFamily::Body,
                        format_args!("{state}"),
                        frame.rect(RIGHT - 470.0, top + 6.0, 470.0, 28.0),
                        18.0 * s,
                        colour,
                        FontWeight::Regular,
                        TextAlign::End,
                    );
                    let detail = if fetchable {
                        format!("{} files  ·  {}", summary.files, human_size(summary.size))
                    } else {
                        format!(
                            "{} for JoF's own client, not used by SJK",
                            summary.elsewhere
                        )
                    };
                    text(
                        ui,
                        TextFamily::Body,
                        format_args!("{detail}"),
                        frame.rect(RIGHT - 470.0, top + 33.0, 470.0, 22.0),
                        15.0 * s,
                        color::QUIET,
                        FontWeight::Regular,
                        TextAlign::End,
                    );
                    ui.hit_region(
                        slot as u16,
                        frame.rect(LEFT - 16.0, top, RIGHT - LEFT + 32.0, ROW_HEIGHT - 4.0),
                    );
                }
                if groups.len() > ROWS {
                    text(
                        ui,
                        TextFamily::Body,
                        format_args!(
                            "{}-{} of {} groups",
                            self.first + 1,
                            (self.first + ROWS).min(groups.len()),
                            groups.len()
                        ),
                        frame.rect(RIGHT - 300.0, 338.0, 300.0, 22.0),
                        14.0 * s,
                        color::QUIET,
                        FontWeight::Regular,
                        TextAlign::End,
                    );
                }
            }
        }

        // What happens, or is happening.
        let status_top = GROUPS_TOP + ROWS as f32 * ROW_HEIGHT + 14.0;
        text(
            ui,
            TextFamily::Body,
            format_args!("{status}"),
            frame.rect(LEFT, status_top, 1110.0, 26.0),
            17.0 * s,
            status_colour,
            FontWeight::Regular,
            TextAlign::Start,
        );
        if let Some(ratio) = ratio {
            bar(
                ui,
                &frame,
                [LEFT, status_top + 36.0, RIGHT - LEFT, 8.0],
                ratio,
            );
        }
        let buttons = status_top + 62.0;
        kit::button(
            ui,
            &frame,
            [LEFT, buttons, 200.0, 46.0],
            "Check again",
            false,
            !running,
            self.focus == check_focus,
            CHECK_TOKEN,
        );
        kit::button(
            ui,
            &frame,
            [RIGHT - 280.0, buttons, 280.0, 46.0],
            &label,
            !running,
            enabled,
            self.focus == main_focus,
            MAIN_TOKEN,
        );
        text(
            ui,
            TextFamily::Body,
            format_args!("Up Down choose  ·  Space tick  ·  Enter act  ·  F5 check  ·  Esc close"),
            frame.rect(LEFT + 230.0, buttons + 12.0, 640.0, 24.0),
            15.0 * s,
            color::QUIET,
            FontWeight::Regular,
            TextAlign::Center,
        );
        let focused_token = match self.focus {
            0 => SWITCH_TOKEN,
            focus if focus == check_focus => CHECK_TOKEN,
            focus if focus == main_focus => MAIN_TOKEN,
            focus => (focus - 1).saturating_sub(self.first) as u16,
        };
        self.ui.finish(focused_token);
        target.append(&self.ui, viewport);
    }

    /// The status line, its colour and the progress bar's fill.
    fn status(&self) -> (String, Color, Option<f32>) {
        let ratio = |done: u64, total: u64| {
            Some(if total == 0 {
                0.0
            } else {
                done as f32 / total as f32
            })
        };
        match &self.phase {
            Phase::Manifest => (
                "Asking JoF's repository for its list...".into(),
                color::MUTED,
                None,
            ),
            Phase::Checking { done, total } => (
                format!(
                    "Checking the files already here ({} of {})...",
                    human_size(*done),
                    human_size(*total)
                ),
                color::MUTED,
                ratio(*done, *total),
            ),
            Phase::Removing { done, total } => (
                format!("Removing files no longer chosen ({done} of {total})..."),
                color::MUTED,
                ratio(*done as u64, *total as u64),
            ),
            Phase::Downloading {
                file,
                files,
                name,
                done,
                total,
                ..
            } => (
                format!(
                    "Downloading {file} of {files}: {}  ·  {} of {}  ·  goes on while you play",
                    shorten(name, 40),
                    human_size(*done),
                    human_size(*total)
                ),
                color::GOLD_BRIGHT,
                ratio(*done, *total),
            ),
            Phase::Finished(outcome) if outcome.synced => self.finished(outcome),
            Phase::Failed(reason) => (
                format!(
                    "JoF's repository could not be reached: {}",
                    shorten(reason, 80)
                ),
                color::EMBER,
                None,
            ),
            Phase::Cancelled => (
                "Stopped. Download again to carry on where it stopped.".into(),
                color::MUTED,
                None,
            ),
            Phase::Idle | Phase::Finished(_) => (self.ready(), color::MUTED, None),
        }
    }

    fn finished(&self, outcome: &Outcome) -> (String, Color, Option<f32>) {
        if let Some(first) = outcome.failures.first() {
            return (
                format!(
                    "{} downloaded, {} failed: {}",
                    outcome.downloaded,
                    outcome.failures.len(),
                    shorten(first, 70)
                ),
                color::EMBER,
                None,
            );
        }
        let mut done = format!(
            "Done: {} files downloaded ({}), {} removed.",
            outcome.downloaded,
            human_size(outcome.downloaded_bytes),
            outcome.removed
        );
        done.push_str(if self.on {
            " They load from the next map."
        } else {
            " Turn on Use JoF content to load them."
        });
        (done, color::GOLD_BRIGHT, None)
    }

    fn ready(&self) -> String {
        if self.snapshot.is_none() {
            return "Not checked yet.".into();
        }
        let plan = &self.plan;
        let what = match (plan.downloads.is_empty(), plan.removals.is_empty()) {
            (true, true) => return "Everything chosen is here.".into(),
            (false, true) => format!(
                "{} files to download ({}).",
                plan.downloads.len(),
                human_size(plan.download_bytes)
            ),
            (true, false) => format!(
                "{} files of groups no longer chosen to remove ({}).",
                plan.removals.len(),
                human_size(plan.removal_bytes)
            ),
            (false, false) => format!(
                "{} files to download ({}), {} to remove ({}).",
                plan.downloads.len(),
                human_size(plan.download_bytes),
                plan.removals.len(),
                human_size(plan.removal_bytes)
            ),
        };
        if self.on {
            what
        } else {
            format!("{what} Turn on Use JoF content first.")
        }
    }
}

/// Each group's counts, in the manifest's order.
fn summaries(snapshot: &Snapshot) -> Vec<Summary> {
    snapshot
        .manifest
        .groups
        .iter()
        .enumerate()
        .map(|(group_index, group)| {
            let mut summary = Summary::default();
            for (asset_index, asset) in group.assets.iter().enumerate() {
                if asset.target != Target::Base {
                    summary.elsewhere += 1;
                    continue;
                }
                summary.files += 1;
                summary.size += asset.size;
                let state = snapshot.state(group_index, asset_index);
                if state.present() {
                    summary.here += 1;
                } else {
                    summary.missing += asset.size;
                }
                if matches!(state, AssetState::Installed | AssetState::Outdated) {
                    summary.ours += asset.size;
                }
            }
            summary
        })
        .collect()
}

/// The group's state on the right of its row.
fn group_state(summary: &Summary, chosen: bool) -> (String, Color) {
    if summary.files == 0 {
        return ("Not for SJK".into(), color::QUIET);
    }
    if !chosen {
        return if summary.ours > 0 {
            (
                format!("Not chosen  ·  {} to remove", human_size(summary.ours)),
                color::MUTED,
            )
        } else if summary.here > 0 {
            (
                format!(
                    "Not chosen  ·  {} of {} in game folder",
                    summary.here, summary.files
                ),
                color::QUIET,
            )
        } else {
            ("Not chosen".into(), color::QUIET)
        };
    }
    if summary.here == summary.files {
        (format!("All {} here", summary.files), color::GOLD_BRIGHT)
    } else {
        (
            format!(
                "{} of {} here  ·  {} to download",
                summary.here,
                summary.files,
                human_size(summary.missing)
            ),
            color::TEXT,
        )
    }
}

/// A thin track with its gold fill.
fn bar(canvas: &mut MenuCanvas, frame: &Frame, rect: [f32; 4], ratio: f32) {
    let [x, y, width, height] = rect;
    let radius = height * 0.5 * frame.s;
    let _ = canvas.draw_list_mut().push(DrawCommand::RoundedRect {
        rect: frame.rect(x, y, width, height),
        radius,
        color: color::alpha(color::HOLO, 0.12),
    });
    let fill = width * ratio.clamp(0.0, 1.0);
    if fill > 0.0 {
        let _ = canvas.draw_list_mut().push(DrawCommand::RoundedRect {
            rect: frame.rect(x, y, fill.max(height), height),
            radius,
            color: color::GOLD,
        });
    }
}

/// `text` cut to `most` characters with an ellipsis at the end.
fn cut(text: &str, most: usize) -> String {
    if text.chars().count() <= most {
        return text.to_owned();
    }
    let kept: String = text.chars().take(most - 1).collect();
    format!("{}…", kept.trim_end())
}

/// `text` cut to `most` characters with an ellipsis in the middle.
fn shorten(text: &str, most: usize) -> String {
    let count = text.chars().count();
    if count <= most {
        return text.to_owned();
    }
    let head = most / 2;
    let tail = most - head - 1;
    let start: String = text.chars().take(head).collect();
    let end: String = text.chars().skip(count - tail).collect();
    format!("{start}…{end}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn long_paths_keep_both_ends() {
        assert_eq!(shorten("short", 10), "short");
        let cut = shorten(
            "C:/Program Files (x86)/Steam/steamapps/common/Jedi Academy/GameData/SJK/jof",
            30,
        );
        assert_eq!(cut.chars().count(), 30);
        assert!(cut.starts_with("C:/Program"));
        assert!(cut.ends_with("SJK/jof"));
    }

    #[test]
    fn a_group_says_what_is_here_and_what_a_sync_does() {
        let summary = Summary {
            files: 35,
            here: 12,
            size: 4_600_000_000,
            missing: 1_200_000_000,
            ours: 0,
            elsewhere: 0,
        };
        assert_eq!(
            group_state(&summary, true).0,
            "12 of 35 here  ·  1.2 GB to download"
        );
        assert_eq!(
            group_state(&summary, false).0,
            "Not chosen  ·  12 of 35 in game folder"
        );
        let all = Summary {
            here: 35,
            ..summary
        };
        assert_eq!(group_state(&all, true).0, "All 35 here");
        let ours = Summary {
            ours: 200_000_000,
            ..summary
        };
        assert_eq!(
            group_state(&ours, false).0,
            "Not chosen  ·  200 MB to remove"
        );
    }

    #[test]
    #[ignore = "needs a GPU, JKA_GAME_DATA and the network"]
    fn the_jof_content_card_renders_on_a_gpu() {
        crate::world_shot::on_big_stack(|| {
            let (mut gpu, _profile) = crate::world_shot::open(
                "maps/mp/duel6.bsp",
                [1920, 1080],
                None,
                &[("r_hdr", "0"), ("cg_materialMaps", "0")],
            )
            .expect("a GPU adapter");
            let game_data = gpu.game_data.clone();
            let console = gpu.console.as_mut().unwrap();
            if std::env::var_os("SJK_JOF_CONTENT_ON").is_some() {
                console.set_cvar(crate::jof_content::SWITCH, "1");
            }
            console.open_jof_content(&game_data);
            let start = std::time::Instant::now();
            while console.jof_content.busy() && start.elapsed().as_secs() < 300 {
                std::thread::sleep(std::time::Duration::from_millis(100));
            }
            assert!(matches!(
                gpu.render(&mut None),
                crate::gpu_context::FrameStatus::Rendered
            ));
            let console = gpu.console.as_ref().unwrap();
            assert!(console.covers_frame());
            assert!(console.sjk_page_open());
            if let Some(path) = std::env::var_os("SJK_JOF_CONTENT_SHOT") {
                crate::world_shot::frame(&mut gpu, 2).save(path).unwrap();
            }
        });
    }

    #[test]
    fn download_waits_for_the_switch_and_a_look_at_the_disk() {
        let mut card = Card::default();
        assert_eq!(card.main_button(), ("Download".into(), false));
        card.snapshot = Some(Arc::new(Snapshot::default()));
        card.update();
        assert_eq!(card.main_button(), ("Download".into(), false), "switch off");
        card.on = true;
        assert_eq!(card.main_button(), ("Up to date".into(), false));
        assert!(
            matches!(card.activate(), Action::Switch(false)),
            "focus starts on the switch"
        );
    }
}

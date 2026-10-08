//! The Profile page: the player's SJK profile as others see it on the hub (name,
//! verified flag, names worn, medals and bio), their own record from the achievement
//! counts, and the achievements board (`docs/identity.md`, "Profile" and
//! "Achievements"). The bio is written here, under the hub's rules
//! (`sjk_identity::bio`): what cannot be in a bio cannot be typed, and what the hub
//! sends back is shown only through those rules.
//!
//! Opened by the SJK menus' Profile entry or the `profile` and `achievements`
//! commands; its Identity settings button opens the Identity page (the key, the
//! switch that shares it, the hub). Like the Identity page it lives in the console and is drawn
//! in place of it, always in the SJK UI's look ([`view`]). Tab moves between the tabs,
//! the bio and its buttons; Left and Right switch tabs from the tabs; Enter saves the
//! bio from the field (Shift+Enter starts a new line); Escape goes back.

use crate::achievements::Standing;
use crate::menu_widgets::{BACK_TOKEN, MenuCanvas};
use sjk_identity::bio;
use sjk_identity::{Snapshot, Status};
use sjk_ui::{InputEvent, UiEventKind};
use std::time::Instant;
use winit::event::{ElementState, KeyEvent};
use winit::keyboard::{KeyCode, PhysicalKey};

#[path = "profile_panel_view.rs"]
mod view;

const TAB_TOKEN: u16 = 1_000;
const BIO_TOKEN: u16 = 1_010;
const SAVE_TOKEN: u16 = 1_011;
const REVERT_TOKEN: u16 = 1_012;
const BOARD_TOKEN: u16 = 1_013;
const IDENTITY_TOKEN: u16 = 1_014;

/// The page's two tabs.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum Tab {
    Profile,
    Achievements,
}

impl Tab {
    const ALL: [Self; 2] = [Self::Profile, Self::Achievements];

    fn index(self) -> usize {
        self as usize
    }
}

/// The control the keyboard is on.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Focus {
    Tabs,
    Identity,
    Bio,
    Save,
    Revert,
    Board,
}

/// What the console does after the page handled an event.
#[derive(Debug, Eq, PartialEq)]
pub(crate) enum PanelAction {
    None,
    Close,
    /// Send the bio to the hub.
    Save {
        bio: String,
    },
    /// Open the Identity page.
    Identity,
}

/// A save on its way: the bio sent and the service's notice before it, to tell the
/// hub's answer from an older one.
#[derive(Clone, Debug, Eq, PartialEq)]
struct Saving {
    bio: String,
    notice_before: Option<String>,
}

/// What the page knows about the identity, the hub and the counts.
pub(crate) struct Inputs<'a> {
    /// `cl_identity`.
    pub(crate) enabled: bool,
    pub(crate) snapshot: Option<&'a Snapshot>,
    /// Every achievement as the board shows it.
    pub(crate) standings: &'a [Standing],
    /// The player's record: label and value, in order.
    pub(crate) record: &'a [(&'static str, String)],
}

pub(crate) struct Panel {
    open: bool,
    owns_console: bool,
    ui: MenuCanvas,
    tab: Tab,
    focus: Focus,
    /// The bio being written.
    bio: String,
    /// Something was typed since the hub's copy was taken.
    edited: bool,
    /// The hub's copy the draft was last taken from.
    taken: Option<String>,
    saving: Option<Saving>,
    /// A problem found before anything was sent, or the hub's answer.
    message: String,
    /// The bio can be written: the identity is on and the hub answered.
    writable: bool,
    epoch: Instant,
    /// What a world shot shows in place of the live identity and counts.
    #[cfg(test)]
    pub(crate) preview: Option<Preview>,
}

/// A made-up identity and counts for a world shot.
#[cfg(test)]
pub(crate) struct Preview {
    pub(crate) snapshot: Snapshot,
    pub(crate) standings: Vec<Standing>,
    pub(crate) record: Vec<(&'static str, String)>,
}

impl Default for Panel {
    fn default() -> Self {
        Self::new()
    }
}

/// Add `text` to the bio `field` as far as the rules allow: characters a bio cannot
/// hold are left out, tabs become spaces, a line break only while lines are left, and
/// nothing past [`bio::BIO_MAX`] characters. Whether the field changed.
fn type_into(field: &mut String, text: &str) -> bool {
    let mut changed = false;
    let text = text.replace("\r\n", "\n").replace('\r', "\n");
    for c in text.chars() {
        if field.chars().count() >= bio::BIO_MAX {
            break;
        }
        let c = if c == '\t' { ' ' } else { c };
        let fits = match c {
            '\n' => field.split('\n').count() < bio::LINES_MAX,
            '^' => true,
            c => bio::allowed(c),
        };
        if fits {
            field.push(c);
            changed = true;
        }
    }
    changed
}

impl Panel {
    pub(crate) fn new() -> Self {
        Self {
            open: false,
            owns_console: false,
            ui: MenuCanvas::with_capacities(240, 192, 1_000),
            tab: Tab::Profile,
            focus: Focus::Tabs,
            bio: String::new(),
            edited: false,
            taken: None,
            saving: None,
            message: String::new(),
            writable: false,
            epoch: Instant::now(),
            #[cfg(test)]
            preview: None,
        }
    }

    pub(crate) fn is_open(&self) -> bool {
        self.open
    }

    /// Show the page on `tab`; `owns_console` when the console was closed before it.
    pub(crate) fn open(&mut self, tab: Tab, owns_console: bool) {
        self.open = true;
        self.owns_console = owns_console;
        self.tab = tab;
        self.focus = Focus::Tabs;
        self.message.clear();
    }

    /// Hide the page; returns whether it had opened the console.
    pub(crate) fn close(&mut self) -> bool {
        let owned = self.open && self.owns_console;
        self.open = false;
        self.owns_console = false;
        owned
    }

    pub(crate) fn tab(&self) -> Tab {
        self.tab
    }

    pub(crate) fn draw_list(&self) -> &sjk_ui::DrawList {
        self.ui.draw_list()
    }

    /// Take in what the service knows: the hub's bio replaces an untouched draft,
    /// and a save's answer becomes the message.
    fn sync(&mut self, inputs: &Inputs<'_>) {
        let me = inputs
            .snapshot
            .filter(|snapshot| snapshot.status == Status::Online)
            .and_then(|snapshot| snapshot.me.as_ref());
        self.writable = inputs.enabled && me.is_some();
        if !self.writable && matches!(self.focus, Focus::Bio | Focus::Save | Focus::Revert) {
            self.focus = Focus::Tabs;
        }
        let Some(me) = me else {
            return;
        };
        let hub = bio::for_display(&me.bio);
        if self.taken.as_ref() != Some(&hub) {
            if !self.edited {
                self.bio.clone_from(&hub);
            }
            self.taken = Some(hub.clone());
        }
        if let Some(saving) = &self.saving {
            let notice = inputs.snapshot.and_then(|snapshot| snapshot.notice.clone());
            if hub == saving.bio {
                self.message = "Saved".to_owned();
                self.saving = None;
                self.edited = false;
            } else if notice != saving.notice_before
                && let Some(notice) = notice.filter(|notice| notice != "saved")
            {
                self.message = notice;
                self.saving = None;
            }
        }
    }

    /// Send the bio as written, once it passes the rules.
    fn save(&mut self, notice_before: Option<String>) -> PanelAction {
        if !self.writable {
            return PanelAction::None;
        }
        match bio::check(&self.bio) {
            Ok(tidied) => {
                self.message = "Saving...".to_owned();
                self.saving = Some(Saving {
                    bio: tidied.clone(),
                    notice_before,
                });
                PanelAction::Save { bio: tidied }
            }
            Err(error) => {
                self.message = error.to_string();
                PanelAction::None
            }
        }
    }

    /// Put back the hub's copy of the bio.
    fn revert(&mut self) {
        if let Some(hub) = &self.taken {
            self.bio.clone_from(hub);
        }
        self.edited = false;
        self.message.clear();
    }

    /// The controls Tab visits on this tab, in order.
    fn order(&self) -> &'static [Focus] {
        match (self.tab, self.writable) {
            (Tab::Achievements, _) => &[Focus::Tabs],
            (Tab::Profile, false) => &[Focus::Tabs, Focus::Identity, Focus::Board],
            (Tab::Profile, true) => &[
                Focus::Tabs,
                Focus::Identity,
                Focus::Bio,
                Focus::Save,
                Focus::Revert,
                Focus::Board,
            ],
        }
    }

    fn step(&mut self, forward: bool) {
        let order = self.order();
        let at = order.iter().position(|f| *f == self.focus).unwrap_or(0);
        let next = if forward {
            (at + 1) % order.len()
        } else {
            (at + order.len() - 1) % order.len()
        };
        self.focus = order[next];
    }

    fn show(&mut self, tab: Tab) {
        self.tab = tab;
        self.focus = Focus::Tabs;
    }

    /// What Enter (or a click) does on the focused control.
    fn activate(&mut self, notice: Option<String>) -> PanelAction {
        match self.focus {
            Focus::Tabs => {
                let next = Tab::ALL[(self.tab.index() + 1) % Tab::ALL.len()];
                self.show(next);
                PanelAction::None
            }
            Focus::Identity => PanelAction::Identity,
            Focus::Bio | Focus::Save => self.save(notice),
            Focus::Revert => {
                self.revert();
                PanelAction::None
            }
            Focus::Board => {
                self.show(Tab::Achievements);
                PanelAction::None
            }
        }
    }

    /// A key; `notice` is the service's last notice, for a save.
    pub(crate) fn handle_key(
        &mut self,
        event: &KeyEvent,
        shift: bool,
        notice: Option<String>,
    ) -> PanelAction {
        if event.state != ElementState::Pressed {
            return PanelAction::None;
        }
        let PhysicalKey::Code(key) = event.physical_key else {
            return PanelAction::None;
        };
        let typing = self.focus == Focus::Bio && self.writable;
        match key {
            KeyCode::Escape => return PanelAction::Close,
            KeyCode::Tab => self.step(!shift),
            KeyCode::ArrowDown if !typing => self.step(true),
            KeyCode::ArrowUp if !typing => self.step(false),
            KeyCode::ArrowLeft | KeyCode::ArrowRight if self.focus == Focus::Tabs => {
                self.show(if key == KeyCode::ArrowLeft {
                    Tab::Profile
                } else {
                    Tab::Achievements
                });
            }
            KeyCode::Enter | KeyCode::NumpadEnter if typing && shift => {
                if type_into(&mut self.bio, "\n") {
                    self.edited = true;
                }
            }
            KeyCode::Enter | KeyCode::NumpadEnter => return self.activate(notice),
            KeyCode::Space if !typing => return self.activate(notice),
            KeyCode::Backspace if typing => {
                if self.bio.pop().is_some() {
                    self.edited = true;
                }
            }
            _ if typing && !event.repeat => {
                let pasted;
                let text = match event.text.as_deref() {
                    Some("\u{16}") => {
                        pasted = crate::console::clipboard::paste().unwrap_or_default();
                        pasted.as_str()
                    }
                    Some(text) => text,
                    None => return PanelAction::None,
                };
                if type_into(&mut self.bio, text) {
                    self.edited = true;
                }
            }
            _ => {}
        }
        PanelAction::None
    }

    /// A pointer event; `notice` as for [`Self::handle_key`].
    pub(crate) fn handle_pointer(
        &mut self,
        event: InputEvent,
        notice: Option<String>,
    ) -> PanelAction {
        let Some(event) = self.ui.pointer(event) else {
            return PanelAction::None;
        };
        if event.kind != UiEventKind::Activate {
            return PanelAction::None;
        }
        match event.token {
            Some(BACK_TOKEN) => PanelAction::Close,
            Some(token) if (TAB_TOKEN..TAB_TOKEN + 2).contains(&token) => {
                self.show(Tab::ALL[usize::from(token - TAB_TOKEN)]);
                PanelAction::None
            }
            Some(BIO_TOKEN) if self.writable && self.tab == Tab::Profile => {
                self.focus = Focus::Bio;
                PanelAction::None
            }
            Some(SAVE_TOKEN) if self.writable && self.tab == Tab::Profile => {
                self.focus = Focus::Save;
                self.save(notice)
            }
            Some(REVERT_TOKEN) if self.writable && self.tab == Tab::Profile => {
                self.focus = Focus::Revert;
                self.revert();
                PanelAction::None
            }
            Some(IDENTITY_TOKEN) if self.tab == Tab::Profile => {
                self.focus = Focus::Identity;
                PanelAction::Identity
            }
            Some(BOARD_TOKEN) if self.tab == Tab::Profile => {
                self.show(Tab::Achievements);
                PanelAction::None
            }
            _ => PanelAction::None,
        }
    }

    /// Put the keyboard on the bio and type `text` there, for a world shot.
    #[cfg(test)]
    pub(crate) fn type_for_shot(&mut self, text: &str) {
        self.writable = true;
        self.focus = Focus::Bio;
        self.bio.clear();
        type_into(&mut self.bio, text);
        self.edited = true;
    }

    fn focus_token(&self) -> u16 {
        match self.focus {
            Focus::Tabs => TAB_TOKEN + self.tab.index() as u16,
            Focus::Identity => IDENTITY_TOKEN,
            Focus::Bio => BIO_TOKEN,
            Focus::Save => SAVE_TOKEN,
            Focus::Revert => REVERT_TOKEN,
            Focus::Board => BOARD_TOKEN,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sjk_identity::Profile;
    use std::collections::HashMap;

    fn me(bio: &str) -> Profile {
        Profile {
            key_id: "0123456789abcdef".to_owned(),
            key: String::new(),
            name: "^1Sol".to_owned(),
            bio: bio.to_owned(),
            verified: true,
            created: 1_759_708_800,
            names: Vec::new(),
            medals: Vec::new(),
            achievements: Vec::new(),
        }
    }

    pub(super) fn snapshot(me: Option<Profile>, notice: Option<&str>) -> Snapshot {
        Snapshot {
            status: Status::Online,
            key_id: "0123456789abcdef".to_owned(),
            me,
            server: None,
            players: Vec::new(),
            profiles: HashMap::new(),
            notice: notice.map(str::to_owned),
            revision: 0,
            report: None,
            note: None,
            player_report: None,
        }
    }

    fn inputs<'a>(snapshot: Option<&'a Snapshot>) -> Inputs<'a> {
        Inputs {
            enabled: true,
            snapshot,
            standings: &[],
            record: &[],
        }
    }

    fn opened(snapshot: &Snapshot) -> Panel {
        let mut panel = Panel::new();
        panel.open(Tab::Profile, true);
        panel.sync(&inputs(Some(snapshot)));
        panel
    }

    #[test]
    fn typing_keeps_to_what_a_bio_may_hold() {
        let mut field = String::new();
        assert!(type_into(&mut field, "Hi \u{1F600}\u{200B}there\u{202E}!"));
        assert_eq!(field, "Hi there!");
        assert!(!type_into(&mut field, "\u{1F600}"));
        type_into(&mut field, "\tok\r\nnext");
        assert_eq!(field, "Hi there! ok\nnext");
        // Six lines at most.
        let mut lines = String::new();
        type_into(&mut lines, "1\n2\n3\n4\n5\n6\n7");
        assert_eq!(lines, "1\n2\n3\n4\n5\n67");
        let mut long = String::new();
        type_into(&mut long, &"ab".repeat(400));
        assert_eq!(long.chars().count(), bio::BIO_MAX);
    }

    #[test]
    fn the_hubs_bio_fills_an_untouched_draft_shown_through_the_rules() {
        let shot = snapshot(Some(me("hello\u{202E} there")), None);
        let mut panel = opened(&shot);
        assert_eq!(panel.bio, "hello there");
        assert!(panel.writable);
        panel.bio.push('!');
        panel.edited = true;
        let changed = snapshot(Some(me("another")), None);
        panel.sync(&inputs(Some(&changed)));
        assert_eq!(panel.bio, "hello there!", "a draft being written stays");
        panel.revert();
        assert_eq!(panel.bio, "another");
    }

    #[test]
    fn saving_sends_the_tidied_bio_and_shows_the_answer() {
        let shot = snapshot(Some(me("")), Some("saved"));
        let mut panel = opened(&shot);
        panel.bio = "  two   spaces  ".to_owned();
        panel.edited = true;
        assert_eq!(
            panel.save(Some("saved".to_owned())),
            PanelAction::Save {
                bio: "two spaces".to_owned()
            }
        );
        assert_eq!(panel.message, "Saving...");
        // The same old notice is not the answer; the hub's copy is.
        panel.sync(&inputs(Some(&shot)));
        assert_eq!(panel.message, "Saving...");
        let saved = snapshot(Some(me("two spaces")), Some("saved"));
        panel.sync(&inputs(Some(&saved)));
        assert_eq!(panel.message, "Saved");
        assert!(!panel.edited);
        // A refusal shows the hub's words.
        panel.bio = "x".to_owned();
        let _ = panel.save(Some("saved".to_owned()));
        let refused = snapshot(Some(me("two spaces")), Some("the hub is busy"));
        panel.sync(&inputs(Some(&refused)));
        assert_eq!(panel.message, "the hub is busy");
        // A bio breaking a rule is not sent.
        panel.bio = "aaaaaaaaaaaa".to_owned();
        assert_eq!(panel.save(None), PanelAction::None);
        assert_eq!(panel.message, bio::BioError::Noise.message());
    }

    #[test]
    fn without_the_hub_nothing_can_be_written() {
        let mut panel = Panel::new();
        panel.open(Tab::Profile, false);
        panel.sync(&Inputs {
            enabled: false,
            ..inputs(None)
        });
        assert!(!panel.writable);
        assert_eq!(panel.order(), [Focus::Tabs, Focus::Identity, Focus::Board]);
        panel.bio = "hello there".to_owned();
        assert_eq!(panel.save(None), PanelAction::None);
    }

    #[test]
    fn tab_walks_the_controls_and_enter_switches_tabs() {
        let shot = snapshot(Some(me("")), None);
        let mut panel = opened(&shot);
        let steps: Vec<Focus> = (0..6)
            .map(|_| {
                panel.step(true);
                panel.focus
            })
            .collect();
        assert_eq!(
            steps,
            [
                Focus::Identity,
                Focus::Bio,
                Focus::Save,
                Focus::Revert,
                Focus::Board,
                Focus::Tabs
            ]
        );
        assert_eq!(panel.activate(None), PanelAction::None);
        assert_eq!(panel.tab(), Tab::Achievements);
        assert_eq!(panel.order(), [Focus::Tabs]);
        panel.focus = Focus::Tabs;
        let _ = panel.activate(None);
        assert_eq!(panel.tab(), Tab::Profile);
        panel.focus = Focus::Board;
        let _ = panel.activate(None);
        assert_eq!(panel.tab(), Tab::Achievements);
    }
}

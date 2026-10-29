//! The channels the chat composer offers (Creo's idea, 29/10/2026): All always, then
//! Team, SJK and Admin, each behind its own switch, so a player who never uses one can
//! leave it out. Admin is JA+'s and jaPRO's `amsay`, a message only the server's admins
//! read (JoF EJK `Cmd_Amsay_f`, `SAY_ADMIN`); it is offered only on a server whose mod
//! has the command. A hidden channel is only left off the row and out of Tab: the team
//! chat key still opens Team, and a whisper still opens from a name.
use super::*;
use crate::console::ViewerConsole;

/// Team chat in the composer's row.
pub(crate) const TEAM_CVAR: &str = "cg_chatTabTeam";
/// SJK chat in the composer's row.
pub(crate) const SJK_CVAR: &str = "cg_chatTabSjk";
/// Admin chat (`amsay`) in the composer's row, on servers that have it.
pub(crate) const ADMIN_CVAR: &str = "cg_chatTabAdmin";

/// Which of the switchable channels the player keeps in the row.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) struct Tabs {
    pub(super) team: bool,
    pub(super) sjk: bool,
    pub(super) admin: bool,
}

impl Tabs {
    pub(super) fn read(console: Option<&ViewerConsole>) -> Self {
        let on = |name| {
            console
                .and_then(|console| console.integer_cvar(name))
                .is_none_or(|value| value != 0)
        };
        Self {
            team: on(TEAM_CVAR),
            sjk: on(SJK_CVAR),
            admin: on(ADMIN_CVAR),
        }
    }
}

/// Whether a server of this kind takes `amsay`: JA+ and jaPRO (and its TaystJK lineage,
/// JoF's servers among them).
pub(crate) fn server_has_admin_chat(kind: sjk_mod::ServerKind) -> bool {
    matches!(
        kind,
        sjk_mod::ServerKind::JaPlus | sjk_mod::ServerKind::JaPro
    )
}

impl ChatOverlay {
    /// The channels the composer's row offers now, in order.
    pub(super) fn offered(&self) -> impl Iterator<Item = Channel> + '_ {
        [Channel::Global, Channel::Team, Channel::Sjk, Channel::Admin]
            .into_iter()
            .filter(|channel| self.offers(*channel))
    }

    /// Whether the row offers `channel`.
    pub(super) fn offers(&self, channel: Channel) -> bool {
        let tabs = self.options.tabs;
        match channel {
            Channel::Global => true,
            Channel::Team => tabs.team,
            Channel::Sjk => tabs.sjk,
            Channel::Admin => tabs.admin && self.admin_server,
            Channel::Whisper => false,
        }
    }

    /// Say whether the server being played takes `amsay`. Leaving one that does moves a
    /// composer on Admin to All.
    pub(crate) fn set_admin_server(&mut self, admin: bool) {
        self.admin_server = admin;
        if !self.offers(Channel::Admin)
            && let Some(input) = self.input.as_mut()
            && input.channel == Channel::Admin
        {
            input.channel = Channel::Global;
        }
    }

    /// The channel Tab moves to from `channel`: the next one in the row, then the last
    /// whisperer (`chat/reply.rs`), then All again.
    pub(super) fn next_channel(&self, channel: Channel) -> Option<Channel> {
        if channel != Channel::Whisper {
            let mut row = self.offered().skip_while(|offered| *offered != channel);
            row.next();
            if let Some(next) = row.next() {
                return Some(next);
            }
            if self.reply_target().is_ok() {
                return None;
            }
        }
        Some(Channel::Global)
    }

    /// Left edge of the whisper's "To ..." and the SJK chat's red note: past the row's
    /// last channel, in 1080p pixels at the chat's scale.
    pub(super) fn row_end(&self) -> f32 {
        self.offered().count() as f32 * 58.0 + 4.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn chat(team: bool, sjk: bool, admin: bool, server: bool) -> ChatOverlay {
        let mut chat = ChatOverlay::new();
        chat.options.tabs = Tabs { team, sjk, admin };
        chat.admin_server = server;
        chat
    }

    #[test]
    fn the_row_keeps_all_and_the_channels_switched_on() {
        let every = chat(true, true, true, true);
        assert_eq!(
            every.offered().collect::<Vec<_>>(),
            [Channel::Global, Channel::Team, Channel::Sjk, Channel::Admin]
        );
        // Dave's row: no SJK or Team chat most of the time.
        let lean = chat(false, false, true, true);
        assert_eq!(
            lean.offered().collect::<Vec<_>>(),
            [Channel::Global, Channel::Admin]
        );
        assert_eq!(lean.row_end(), 120.0);
        // Admin only where the server takes amsay.
        let base = chat(true, true, true, false);
        assert!(!base.offers(Channel::Admin));
        assert_eq!(base.row_end(), 178.0);
    }

    #[test]
    fn tab_walks_only_the_offered_channels() {
        let lean = chat(false, true, true, true);
        assert_eq!(lean.next_channel(Channel::Global), Some(Channel::Sjk));
        assert_eq!(lean.next_channel(Channel::Sjk), Some(Channel::Admin));
        assert_eq!(lean.next_channel(Channel::Admin), Some(Channel::Global));
        // The team key opened Team though it is hidden: Tab goes on to the next one.
        assert_eq!(lean.next_channel(Channel::Team), Some(Channel::Global));
        assert_eq!(lean.next_channel(Channel::Whisper), Some(Channel::Global));
    }

    #[test]
    fn the_switches_default_on_and_read_zero_as_off() {
        assert_eq!(
            Tabs::read(None),
            Tabs {
                team: true,
                sjk: true,
                admin: true
            }
        );
    }

    #[test]
    fn admin_chat_is_offered_on_ja_plus_and_japro_servers() {
        use sjk_mod::ServerKind;
        assert!(server_has_admin_chat(ServerKind::JaPlus));
        assert!(server_has_admin_chat(ServerKind::JaPro));
        assert!(!server_has_admin_chat(ServerKind::BaseJka));
        assert!(!server_has_admin_chat(ServerKind::Other));
    }

    #[test]
    fn leaving_an_admin_server_moves_the_composer_to_all() {
        let mut chat = chat(true, true, true, true);
        chat.open(false);
        chat.input.as_mut().unwrap().channel = Channel::Admin;
        chat.set_admin_server(false);
        assert_eq!(chat.input.as_ref().unwrap().channel, Channel::Global);
    }

    #[test]
    fn admin_chat_goes_out_as_amsay() {
        assert_eq!(
            sjk_client::chat_command(sjk_client::ChatDestination::Admin, "need help \"here\"")
                .as_deref(),
            Some("amsay \"need help ''here''\"")
        );
        let mut chat = chat(true, true, true, true);
        chat.open(false);
        chat.choose(Channel::Admin);
        chat.input.as_mut().unwrap().text = "hi admins".to_owned();
        assert!(matches!(
            chat.submit(),
            ChatInputResult::Submit(command) if command == "amsay \"hi admins\""
        ));
    }
}

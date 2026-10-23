//! Trades between players in game (`docs/sjk-ui.md`, "Trading"): the frame's side of
//! [`crate::trade_window`]. It reads the player's trades from the identity service each
//! frame, shows the toast over play while one runs and the window over the game menu,
//! and sends what the player chose.
//!
//! A trade starts from a player's card (the game chat's sender card, while the composer
//! frees the pointer, or the game menu's Players page) or the `trade` command. In game
//! the pointer is free only in the menus, so the window shows over the game menu: Esc
//! opens it while a trade runs, and its own Esc closes it and goes back to play; the
//! trade goes on and the toast says so. `trade` opens it again from anywhere.

use crate::player_identity;
use crate::trade_window::{Choice, Item, Said, View};
use std::time::{Duration, Instant};

/// The console command: `trade <player>` asks them, `trade` opens the window, `trade
/// accept|decline|cancel` answers without it.
pub(crate) const COMMAND: &str = "trade";
/// The trade window's size: 0 small, 1 normal (the default), 2 large.
pub(crate) const SIZE_CVAR: &str = "cl_tradeWindowSize";
pub(crate) const HELP: &str = "Trade holocrons and duplicate shaders with a player on this server: trade <player>; trade alone opens the trade window; trade accept, decline or cancel answers";
/// How long the toast tells of an ended trade, or of a refusal with no trade on.
const TOLD_FOR: Duration = Duration::from_secs(6);

/// The trade window, its toast, and what the player did with them.
#[derive(Default)]
pub(crate) struct State {
    pub(crate) window: crate::trade_window::Window,
    pub(crate) toast: crate::trade_window::Toast,
    /// The trade whose window the player closed: only the toast shows for it.
    hidden: Option<i64>,
    /// The last trade seen running, to tell when it ends.
    live_seen: Option<i64>,
    /// A trade that ended while seen, and when this PC saw it end.
    ended: Option<(i64, Instant)>,
    /// The ended trade the player closed the window on.
    ended_closed: Option<i64>,
    /// The last refusal shown and when it came, by the service's step count.
    problem: Option<(u64, String, Instant)>,
    /// The toast's words this frame, when it shows.
    said: Option<Said>,
    /// The toast was drawn last frame (its targets are live).
    toast_shown: bool,
    /// The window opened the game menu (from a command or a card): closing it closes the
    /// menu.
    opened_menu: bool,
    /// A world shot's trade, shown in the window (true) or the toast, with no hub.
    #[cfg(test)]
    pub(crate) shot: Option<(View, bool)>,
}

/// The player's own items that may go in a trade, from their profile: unopened
/// holocrons found by play and not traded within the hub's lock, and one copy of each
/// shader of the trade held twice or more. Most worth first; those already offered left
/// out.
fn own_items(
    me: &sjk_identity::Profile,
    list: &sjk_identity::TradesAnswer,
    offered: &[Item],
    hub_now: i64,
) -> Vec<Item> {
    let rules = &list.rules;
    let lock = rules.resale_lock;
    let mut items: Vec<Item> = me
        .holocrons
        .iter()
        // Holocrons only once the hub trades them (duplicates first).
        .filter(|holocron| rules.holocrons && holocron.is_sealed() && holocron.source == "play")
        .map(|holocron| {
            let value = rules
                .holocron_value
                .get(&holocron.tier)
                .copied()
                .unwrap_or(0);
            let mut item = Item::holocron(holocron.id, &holocron.tier, value);
            // Received in a trade within the hub's lock: shown, waiting its week.
            item.locked_for = holocron
                .traded
                .map(|at| at + lock - hub_now)
                .filter(|left| *left > 0);
            item
        })
        .collect();
    items.extend(
        me.unlocks
            .iter()
            .filter(|unlock| unlock.count >= 2 && unlock.medal.is_none())
            .filter_map(|unlock| {
                let skin = crate::unlockables::find(&unlock.id)?;
                crate::economy::in_trade(skin).then(|| {
                    let value = rules
                        .shader_value
                        .get(skin.tier as usize)
                        .copied()
                        .unwrap_or(0);
                    Item::shader(&unlock.id, None, value)
                })
            }),
    );
    items.retain(|item| !offered.iter().any(|offered| offered.what == item.what));
    // Those that may go in first, most worth first; then those that wait.
    items.sort_by(|a, b| {
        a.locked_for
            .is_some()
            .cmp(&b.locked_for.is_some())
            .then(b.value.cmp(&a.value))
            .then(a.name.cmp(&b.name))
    });
    items
}

/// The day's limits as the window lists them: holocrons, Mythical holocrons and
/// duplicates, each received and given. A limit the hub does not send is left out.
fn limits(list: &sjk_identity::TradesAnswer) -> Vec<crate::trade_window::Limit> {
    let (today, rules) = (&list.today, &list.rules);
    let mut rows = Vec::new();
    // The look Creyon chose (23/10/2026): one Holocrons row, one Duplicates row. The
    // hub's one-of-each-tier rule shows only when it holds an item back.
    if rules.holocrons {
        rows.push(crate::trade_window::Limit {
            label: "Holocrons",
            received: (today.holocrons_received, rules.holocrons_received_day),
            given: (today.holocrons_given, rules.holocrons_given_day),
            colour: None,
        });
    }
    rows.push(crate::trade_window::Limit {
        label: "Duplicates",
        received: (today.duplicates_received, rules.duplicates_received_day),
        given: (today.duplicates_given, rules.duplicates_given_day),
        colour: None,
    });
    rows.retain(|row| row.received.1 > 0 || row.given.1 > 0);
    rows
}

/// How many more the player may give today past `offered`: the hub's given limits less
/// what they gave and what they offer now. Unlimited where the hub sends no limit.
fn room(list: &sjk_identity::TradesAnswer, offered: &[Item]) -> crate::trade_window::Room {
    let (today, rules) = (&list.today, &list.rules);
    let holocrons = offered
        .iter()
        .filter(|item| item.holocron_slot.is_some())
        .count() as i64;
    let shaders = offered.len() as i64 - holocrons;
    let left = |most: i64, done: i64, now: i64| {
        if most > 0 {
            most - done - now
        } else {
            i64::MAX
        }
    };
    // One of each tier: what is given today and offered now, by tier; Mythical's own
    // older limit counts too, for a hub without tiers.
    let mut tiers = [i64::MAX; 4];
    for tier in crate::holocrons::TIERS {
        let offered = offered
            .iter()
            .filter(|item| item.holocron_slot == Some(tier.slot))
            .count() as i64;
        let done = today.tiers_given.get(tier.id).copied().unwrap_or(0);
        let mut most = left(rules.holocron_tier_day, done, offered);
        if tier.slot == 3 {
            most = most.min(left(
                rules.mythical_given_day,
                today.mythical_given,
                offered,
            ));
        }
        if let Some(slot) = tiers.get_mut(tier.slot) {
            *slot = most;
        }
    }
    crate::trade_window::Room {
        holocrons: left(rules.holocrons_given_day, today.holocrons_given, holocrons),
        tiers,
        duplicates: left(rules.duplicates_given_day, today.duplicates_given, shaders),
    }
}

/// The other player's name as they wear it, colour codes kept (white after it): the
/// hub's, else the name the game shows for their key here (`players`, the server's
/// presence); else "an SJK player". Reads only what it is given: it runs under the
/// identity snapshot's lock, which must never be taken again here (it froze the game,
/// Creyon 23/10/2026).
fn partner_shown(with: &sjk_identity::TradePartner, players: &[sjk_identity::Presence]) -> String {
    let named = |name: &str| !crate::medals::plain_note(name).trim().is_empty();
    let name = Some(with.name.clone())
        .filter(|name| named(name))
        .or_else(|| {
            players
                .iter()
                .find(|player| player.key_id == with.key_id)
                .map(|player| player.claimed_name.clone())
                .filter(|name| named(name))
        })
        .unwrap_or_else(|| "an SJK player".to_owned());
    format!("{name}^7")
}

/// Their picture's version: the hub's with the trade, else the server's word for their
/// key here (`players`); empty for none. As [`partner_shown`], it takes no lock.
fn partner_avatar(with: &sjk_identity::TradePartner, players: &[sjk_identity::Presence]) -> String {
    if !with.avatar.is_empty() {
        return with.avatar.clone();
    }
    players
        .iter()
        .find(|player| player.key_id == with.key_id)
        .map(|player| player.avatar.clone())
        .unwrap_or_default()
}

/// Ask the player whose SJK key is `key_id` to trade, from a page that is not the game's
/// (the SJK chat page): the window opens once that page closes. False when the
/// identity is off or it is the player's own key.
pub(crate) fn ask_by_key(key_id: &str) -> bool {
    if player_identity::own_key_id().as_deref() == Some(key_id) {
        return false;
    }
    let asked = player_identity::with_service(|service| service.trade_ask(key_id)).is_some();
    if asked {
        OPEN_REQUEST.store(true, std::sync::atomic::Ordering::Relaxed);
    }
    asked
}

/// Tests: the hub lets this player trade ([`trading_open`]).
#[cfg(test)]
pub(crate) static TRADING_FOR_TEST: std::sync::atomic::AtomicBool =
    std::sync::atomic::AtomicBool::new(false);

/// A page outside the game asked to open the trade window ([`ask_by_key`]).
static OPEN_REQUEST: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

/// The window's view of `trade` from `state` and the player's profile, at `now`.
fn view_of(
    trade: &sjk_identity::PlayerTrade,
    state: &sjk_identity::service::TradesState,
    me: Option<&sjk_identity::Profile>,
    players: &[sjk_identity::Presence],
    now: Instant,
) -> View {
    let mine: Vec<Item> = trade.mine.items.iter().map(Item::from_wire).collect();
    let mine_items = mine.clone();
    let theirs = trade.theirs.items.iter().map(Item::from_wire).collect();
    let own = me.map_or_else(Vec::new, |me| {
        own_items(me, &state.list, &mine, state.hub_now(now))
    });
    let both_ready = trade.mine.ready && trade.theirs.ready;
    View {
        partner: crate::medals::plain_note(&partner_shown(&trade.with, players)),
        partner_shown: partner_shown(&trade.with, players),
        partner_key: trade.with.key_id.clone(),
        partner_avatar: partner_avatar(&trade.with, players),
        mine,
        theirs,
        own,
        items_max: state.list.rules.items_max.max(1),
        seconds_left: trade.expires.map_or(0, |at| state.seconds_until(at, now)),
        left_exact: trade
            .expires
            .map_or(0.0, |at| state.seconds_until_exact(at, now)),
        ask_secs: u64::try_from(state.list.rules.ask_secs).unwrap_or(30),
        confirm_in: trade
            .confirm_from
            .filter(|_| both_ready)
            .map(|at| state.seconds_until(at, now)),
        limits: limits(&state.list),
        holocrons: state.list.rules.holocrons,
        resets_in: state
            .list
            .today
            .resets_at
            .map(|at| state.seconds_until(at, now)),
        room: room(&state.list, &mine_items),
        confirming_gift: false,
        problem: String::new(),
        trade: trade.clone(),
    }
}

impl crate::GpuState {
    /// Once a frame: read the player's trades, choose what the window and the toast show.
    pub(crate) fn update_player_trades(&mut self) {
        let now = Instant::now();
        #[cfg(test)]
        if let Some((view, window)) = self.trades.shot.clone() {
            self.trades.said = (!window).then(|| Said::of(&view));
            self.trades.window.set_view(window.then_some(view));
            return;
        }
        if OPEN_REQUEST.swap(false, std::sync::atomic::Ordering::Relaxed) {
            self.open_trade_window();
        }
        // Out of the game menu in a game, Esc opens the trade window again. In the main
        // menu, it stays closed until the toast's Open trade.
        if !self.game_menu && self.live_session.is_some() {
            self.trades.hidden = None;
        }
        if let Some(slot) = self.chat.take_trade_request()
            && let Err(why) = self.trade_with_slot(slot)
            && let Some(console) = self.console.as_mut()
        {
            console.push_log(format!("^3Trade: {why}."));
        }
        let read = player_identity::with_snapshot(|snapshot| {
            let state = snapshot.trades.as_ref()?;
            let live = state
                .live()
                .map(|trade| view_of(trade, state, snapshot.me.as_ref(), &snapshot.players, now));
            let ended = state
                .list
                .trades
                .iter()
                .find(|trade| !trade.is_live() && Some(trade.id) == self.trades.live_seen)
                .or_else(|| {
                    let (id, _) = self.trades.ended?;
                    state.list.trades.iter().find(|trade| trade.id == id)
                })
                .map(|trade| view_of(trade, state, snapshot.me.as_ref(), &snapshot.players, now));
            Some((live, ended, state.steps, state.problem.clone()))
        })
        .flatten();
        let trades = &mut self.trades;
        let Some((live, ended, steps, problem)) = read else {
            trades.window.set_view(None);
            trades.said = None;
            return;
        };
        // A new refusal is shown for a moment.
        if !problem.is_empty()
            && trades
                .problem
                .as_ref()
                .is_none_or(|(seen, _, _)| *seen != steps)
        {
            if let Some(console) = self.console.as_mut() {
                console.push_log(format!("^3Trade: {problem}"));
            }
            trades.problem = Some((steps, problem, now));
        }
        let fresh_problem = trades
            .problem
            .as_ref()
            .filter(|(_, _, at)| now.duration_since(*at) < TOLD_FOR)
            .map(|(_, words, _)| words.clone());
        match &live {
            Some(view) => {
                if trades.live_seen != Some(view.trade.id) {
                    // A new trade: its window shows again, at Esc.
                    trades.hidden = None;
                }
                trades.live_seen = Some(view.trade.id);
            }
            None => {
                if let (Some(seen), Some(view)) = (trades.live_seen, &ended)
                    && view.trade.id == seen
                {
                    trades.ended = Some((seen, now));
                    trades.live_seen = None;
                }
            }
        }
        let ended_recent = trades
            .ended
            .filter(|(_, at)| now.duration_since(*at) < TOLD_FOR);
        let window_view = match (live, ended) {
            (Some(view), _) => Some(view),
            // An ended trade stays in the window while it is open, until Close.
            (None, Some(view))
                if trades.window.is_open() && trades.ended_closed != Some(view.trade.id) =>
            {
                Some(view)
            }
            _ => None,
        }
        .map(|mut view| {
            view.problem = fresh_problem.clone().unwrap_or_default();
            view
        });
        trades.said = match &window_view {
            Some(view) if view.trade.is_live() => Some(Said::of(view)),
            _ => None,
        }
        .or_else(|| {
            let (id, _) = ended_recent?;
            let view = player_identity::with_snapshot(|snapshot| {
                let state = snapshot.trades.as_ref()?;
                let trade = state.list.trades.iter().find(|trade| trade.id == id)?;
                Some(view_of(trade, state, None, &snapshot.players, now))
            })
            .flatten()?;
            Some(Said::of(&view))
        })
        .or_else(|| {
            fresh_problem.map(|line| Said {
                headline: "Trade".to_owned(),
                line,
                clock: None,
                share: None,
                key: ("", ""),
                initial: '!',
                who: (String::new(), String::new()),
                clickable: false,
            })
        });
        trades.window.set_view(window_view);
        let size = self
            .console
            .as_ref()
            .and_then(|console| console.integer_cvar(SIZE_CVAR))
            .unwrap_or(1);
        self.trades
            .window
            .set_size(crate::trade_window::Size::from_value(size));
    }

    /// Whether the window shows this frame: a trade to show, the player in the game menu
    /// (the pointer free) with nothing else over it, and not closed for this trade.
    pub(crate) fn prepare_trade_window(&mut self, covered: bool) -> bool {
        let console_open = self
            .console
            .as_ref()
            .is_some_and(crate::console::ViewerConsole::is_open);
        let hidden = self.trades.hidden.is_some()
            && self.trades.hidden == self.trades.live_seen
            && self.trades.live_seen.is_some();
        #[cfg(test)]
        let shot = self.trades.shot.is_some();
        #[cfg(not(test))]
        let shot = false;
        let open = self.trades.window.has_view()
            && !hidden
            && ((self.game_menu && self.live_session.is_some()) || self.main_menu_up() || shot)
            && !covered
            && !console_open
            && !self.text_dialog.is_open()
            && self.world_load_task.is_none()
            && !self.pending_map_reload;
        self.trades.window.set_open(open);
        open
    }

    /// Draw the window. It shades everything, so every font batch appended before it is
    /// dropped (text draws above all shapes).
    pub(crate) fn append_trade_window(&mut self, viewport: [f32; 2]) {
        self.text_vertices.clear();
        self.classic_text_vertices.clear();
        self.game_fonts.clear_text();
        let target = crate::ingame_menu::sjk_view::text_target(
            &mut self.game_fonts,
            &mut self.text_vertices,
            &self.ui_font,
        );
        self.trades.window.append(target, viewport);
    }

    /// Draw the toast over play when it has something to say and the window is closed;
    /// false when it does not show. `covered` is a card or the console over the frame.
    pub(crate) fn append_trade_toast(&mut self, viewport: [f32; 2], covered: bool) -> bool {
        self.trades.toast_shown = false;
        let Some(said) = self.trades.said.clone() else {
            return false;
        };
        #[cfg(test)]
        let shot = self.trades.shot.is_some();
        #[cfg(not(test))]
        let shot = false;
        if covered
            || self.trades.window.is_open()
            || (self.live_session.is_none() && !self.main_menu_up() && !shot)
        {
            return false;
        }
        let mut said = said;
        said.clickable = self.trade_toast_clickable();
        let target = crate::ingame_menu::sjk_view::text_target(
            &mut self.game_fonts,
            &mut self.text_vertices,
            &self.ui_font,
        );
        self.trades.toast.append(&said, target, viewport);
        self.trades.toast_shown = true;
        true
    }

    /// Whether the toast answers the pointer: it shows a running trade and the pointer is
    /// free (the game menu, or the chat's composer, is open).
    fn trade_toast_clickable(&self) -> bool {
        self.trades.live_seen.is_some()
            && (self.game_menu || self.chat.is_typing() || self.main_menu_up())
    }

    /// The main menu shows (no game): trades go on there too, from the SJK chat page.
    fn main_menu_up(&self) -> bool {
        self.live_session.is_none()
            && self
                .client_menu
                .as_ref()
                .is_some_and(crate::menu::ClientMenu::is_visible)
    }

    /// A pointer event while the toast shows: a click on it opens the trade window. True
    /// when the toast took it.
    pub(crate) fn trade_toast_pointer(&mut self, event: sjk_ui::InputEvent) -> bool {
        if !self.trades.toast_shown || !self.trade_toast_clickable() {
            return false;
        }
        if self.trades.toast.handle_pointer(event) {
            self.open_trade_window();
            return true;
        }
        // Over the toast, nothing under it takes the event.
        self.trades.toast.hovered()
            && matches!(
                event,
                sjk_ui::InputEvent::PointerPress { .. } | sjk_ui::InputEvent::PointerRelease { .. }
            )
    }

    /// Act on what the player chose in the window.
    pub(crate) fn trade_window_choice(&mut self, choice: Option<Choice>) {
        match choice {
            None => {}
            Some(Choice::Step(id, step)) => {
                player_identity::with_service(|service| service.trade_step(id, step));
            }
            Some(Choice::Close) => {
                match self.trades.live_seen {
                    Some(id) => self.trades.hidden = Some(id),
                    None => self.trades.ended_closed = self.trades.ended.map(|(id, _)| id),
                }
                self.trades.window.set_view(None);
                // The window opened the menu (a card or a command): back to play. Else the
                // menu the player opened with Esc shows; its Esc goes back to play, and the
                // toast's Open trade (or Esc in play) brings the window back.
                if std::mem::take(&mut self.trades.opened_menu) && self.game_menu {
                    self.game_menu = false;
                    self.game_menu_page = crate::ingame_menu::Page::Main;
                    self.game_menu_row = 0;
                    self.in_game_menu.stop_chat_typing();
                }
                self.sync_cursor_policy();
            }
        }
    }

    /// Show the window over the game menu, opening the menu for the pointer.
    fn open_trade_window(&mut self) {
        self.trades.hidden = None;
        if self.chat.is_typing() {
            self.chat.close_composer();
        }
        if self.live_session.is_some() && !self.game_menu {
            self.release_pointer();
            self.game_menu = true;
            self.game_menu_page = crate::ingame_menu::Page::Main;
            self.game_menu_row = 0;
            self.trades.opened_menu = true;
        }
        self.sync_cursor_policy();
    }

    /// Ask the player in `slot` to trade; why not, in the player's words.
    pub(crate) fn trade_with_slot(&mut self, slot: u16) -> Result<(), String> {
        let session = self
            .live_session
            .as_ref()
            .filter(|session| !session.is_local())
            .ok_or_else(|| "only on an online server".to_owned())?;
        let game = session.game_state();
        if u16::try_from(game.client_num).ok() == Some(slot) {
            return Err("that is you".to_owned());
        }
        let shown = u8::try_from(slot)
            .ok()
            .and_then(|slot| {
                let name = player_identity::shown_name(game, usize::from(slot))?;
                Some((slot, name))
            })
            .ok_or_else(|| "no such player".to_owned())?;
        let mark = player_identity::hub_mark(shown.0, &shown.1)
            .ok_or_else(|| "they are not an SJK player the hub knows here".to_owned())?;
        player_identity::with_service(|service| service.trade_ask(&mark.key_id))
            .ok_or_else(|| "the SJK identity is off (cl_identity 1)".to_owned())?;
        self.open_trade_window();
        Ok(())
    }

    /// The `trade` command.
    pub(crate) fn trade_command(&mut self, args: &[String]) -> Result<Vec<String>, String> {
        let wanted = args.join(" ");
        let wanted = wanted.trim();
        let live = player_identity::with_snapshot(|snapshot| {
            let state = snapshot.trades.as_ref()?;
            state
                .live()
                .map(|trade| (trade.id, trade.asker, trade.is_asked()))
        })
        .flatten();
        let step = match (wanted, live) {
            ("", Some(_)) => {
                if let Some(console) = self.console.as_mut() {
                    console.set_open(false);
                }
                self.open_trade_window();
                return Ok(Vec::new());
            }
            ("", None) => return Err(format!("{COMMAND}: no trade on; usage: {COMMAND} <player>")),
            ("accept", Some((id, false, true))) => Some((id, sjk_identity::TradeStep::Accept)),
            ("decline", Some((id, false, true))) => Some((id, sjk_identity::TradeStep::Decline)),
            ("cancel", Some((id, ..))) => Some((id, sjk_identity::TradeStep::Cancel)),
            ("accept" | "decline" | "cancel", _) => {
                return Err(format!("{COMMAND}: no trade request to answer"));
            }
            _ => None,
        };
        if let Some((id, step)) = step {
            let accepted = step == sjk_identity::TradeStep::Accept;
            player_identity::with_service(|service| service.trade_step(id, step));
            if accepted {
                if let Some(console) = self.console.as_mut() {
                    console.set_open(false);
                }
                self.open_trade_window();
            }
            return Ok(Vec::new());
        }
        let session = self
            .live_session
            .as_ref()
            .ok_or_else(|| format!("{COMMAND}: only on a server"))?;
        let mut roster = sjk_client::ChatRoster::default();
        roster.update(session.game_state());
        let slot = match wanted.parse::<u16>() {
            Ok(slot) => slot,
            Err(_) => match roster.lookup(wanted) {
                sjk_client::PlayerLookup::Found(slot) => slot,
                sjk_client::PlayerLookup::NotFound => {
                    return Err(format!("{COMMAND}: no player matches \"{wanted}\""));
                }
                sjk_client::PlayerLookup::Ambiguous(_) => {
                    return Err(format!(
                        "{COMMAND}: \"{wanted}\" matches several players; use a number"
                    ));
                }
            },
        };
        if let Some(console) = self.console.as_mut() {
            console.set_open(false);
        }
        self.trade_with_slot(slot)
            .map(|()| Vec::new())
            .map_err(|why| format!("{COMMAND}: {why}"))
    }
}

/// Whether players may trade on the SJK hub now (for this player), so a card offers
/// Trade only when it can work.
pub(crate) fn trading_open() -> bool {
    #[cfg(test)]
    if TRADING_FOR_TEST.load(std::sync::atomic::Ordering::Relaxed) {
        return true;
    }
    player_identity::with_snapshot(|snapshot| {
        let staff = snapshot.me.as_ref().is_some_and(|me| me.staff);
        snapshot
            .trades
            .as_ref()
            .is_some_and(|state| state.open(staff))
    })
    .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The trade window (a request, an open trade with both sides, both ready) and the
    /// toast over play, over duel6 at 1080p: `JKA_GAME_DATA=<GameData> cargo test -p
    /// sjk-viewer trade_shots -- --ignored --nocapture`.
    #[test]
    #[ignore = "renders with the GPU and the installed game data named by JKA_GAME_DATA"]
    fn trade_shots() {
        crate::world_shot::on_big_stack(|| {
            let menu = crate::menu::ClientMenu::new(true, String::new());
            let cvars = [
                ("ui_menuStyle", "sjk"),
                (crate::settings::quick::HIDE_CVAR, "1"),
            ];
            let Some((mut gpu, _profile)) =
                crate::world_shot::open("maps/mp/duel6.bsp", [1920, 1080], Some(menu), &cvars)
            else {
                return;
            };
            let _ = crate::world_shot::frame(&mut gpu, 10);
            let open = crate::trade_window::tests::open_view();
            let mut asked = open.clone();
            asked.trade.state = "asked".to_owned();
            asked.trade.asker = false;
            asked.seconds_left = 24;
            let mut ready = open.clone();
            ready.trade.mine.ready = true;
            ready.trade.theirs.ready = true;
            ready.confirm_in = Some(0);
            ready.trade.uneven = true;
            for (view, window, name) in [
                (asked.clone(), false, "trade-toast"),
                (asked, true, "trade-request"),
                (open, true, "trade-window"),
                (ready, true, "trade-both-ready"),
            ] {
                gpu.trades.shot = Some((view, window));
                gpu.update_player_trades();
                let path = crate::world_shot::shoot(&mut gpu, 6, name);
                println!("{}", path.display());
            }
        });
    }

    fn profile() -> sjk_identity::Profile {
        serde_json::from_value(serde_json::json!({
            "key_id": "aa", "key": "", "name": "Sol", "bio": "", "verified": true, "created": 0,
            "holocrons": [
                {"id": 1, "tier": "rare", "source": "play"},
                {"id": 2, "tier": "mythical", "source": "play"},
                {"id": 3, "tier": "rare", "source": "staff"},
                {"id": 4, "tier": "rare", "source": "play", "opened": 5},
                {"id": 5, "tier": "legendary", "source": "play", "traded": 990},
                {"id": 6, "tier": "uncommon", "source": "play", "traded": 100}
            ],
            "unlocks": [
                {"id": "saber_tidal", "granted": 1, "count": 2},
                {"id": "saber_circuit", "granted": 1, "count": 1}
            ]
        }))
        .unwrap()
    }

    fn list() -> sjk_identity::TradesAnswer {
        serde_json::from_value(serde_json::json!({
            "trading": "everyone",
            "today": {"trades": 2, "holocrons_received": 1},
            "rules": {"items_max": 6, "trades_day": 10, "holocrons_received_day": 4,
                      "mythical_received_day": 1, "resale_lock": 604800,
                      "holocron_value": {"uncommon": 4, "rare": 8, "mythical": 51},
                      "shader_value": [5, 10, 25, 60, 150]}
        }))
        .unwrap()
    }

    #[test]
    fn only_unopened_found_holocrons_and_spare_copies_are_offered() {
        let items = own_items(&profile(), &list(), &[], 604_905);
        let whats: Vec<_> = items.iter().map(|item| item.what.clone()).collect();
        // The Mythical first (most worth); the staff gift, the opened one and the one
        // traded within the week stay out; a single copy is no duplicate.
        assert_eq!(whats[0], crate::trade_window::What::Holocron(2));
        assert!(whats.contains(&crate::trade_window::What::Holocron(1)));
        assert!(whats.contains(&crate::trade_window::What::Holocron(6)));
        for kept in [3, 4] {
            assert!(
                !whats.contains(&crate::trade_window::What::Holocron(kept)),
                "{kept}"
            );
        }
        // The one received in a trade within the week is shown, waiting, after the rest.
        let waiting = items
            .iter()
            .find(|item| item.what == crate::trade_window::What::Holocron(5))
            .expect("shown");
        assert_eq!(waiting.locked_for, Some(990 + 604_800 - 604_905));
        assert_eq!(items.last().map(|item| &item.what), Some(&waiting.what));
        assert!(!whats.contains(&crate::trade_window::What::Shader("saber_circuit".into())));
        let tidal = crate::unlockables::find("saber_tidal").unwrap();
        assert_eq!(
            whats.contains(&crate::trade_window::What::Shader("saber_tidal".into())),
            crate::economy::in_trade(tidal)
        );
        // What is already offered is not offered twice.
        let offered = [Item::holocron(2, "mythical", 51)];
        let items = own_items(&profile(), &list(), &offered, 604_905);
        assert!(items.iter().all(|item| item.what != offered[0].what));
    }

    #[test]
    fn the_limits_read_received_and_given_and_room_follows_the_offer() {
        let lines = limits(&list());
        assert_eq!(lines[0].label, "Holocrons");
        assert_eq!(lines[0].received, (1, 4));
        assert!(
            lines.iter().all(|line| line.label != "Mythical"),
            "within the 4"
        );
        let mut list = list();
        list.rules.holocrons_given_day = 4;
        list.rules.mythical_given_day = 1;
        list.rules.duplicates_given_day = 10;
        list.today.holocrons_given = 2;
        let offered = [
            Item::holocron(1, "rare", 8),
            Item::holocron(2, "mythical", 51),
        ];
        let room = room(&list, &offered);
        assert_eq!((room.holocrons, room.tiers[3], room.duplicates), (0, 0, 10));
        assert!(!room.takes(&Item::holocron(9, "uncommon", 4)));
        // No limit sent: nothing held back.
        let open = super::room(&super::tests::list(), &[]);
        assert!(open.takes(&Item::holocron(9, "mythical", 51)));
    }
}

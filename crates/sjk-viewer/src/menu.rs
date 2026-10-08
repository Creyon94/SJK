//! Native main-menu and server-browser input/presentation on the shared UI path.

pub(crate) mod address_view;
pub(crate) mod art;
mod controller;
mod pointer;

mod browser_details;
pub(crate) mod browser_filters;
pub(crate) mod browser_table;
pub(crate) mod browser_view;
pub(crate) mod classic;
pub(crate) mod create_game;
pub(crate) mod create_game_catalog;
mod create_game_pointer;
mod create_game_view;
mod destination;
pub(crate) mod emblem;
pub(crate) mod first_setup;
mod hosting;
pub(crate) mod levelshot;
pub(crate) mod main_view;
mod map_picker;
mod map_picker_view;
pub(crate) mod network_view;
pub(crate) mod sjk;
pub(crate) mod style;

use super::{TextVertex, UiFont};
use crate::client_state::{ClientPhase, ClientState};
use crate::console::ViewerConsole;
use crate::keybind_editor::{EditorResult, KeybindEditor};
use crate::menu_backdrop::{Sample, Shot, Stage};
use crate::menu_widgets::MenuCanvas;
use crate::player_menu::{PlayerMenu, PlayerMenuResult, ReturnTarget};
use crate::server_browser::{RefreshPoll, ServerBrowser, SortColumn};
use crate::settings::{SettingsMenu, SettingsResult};
use destination::MainDestination;
use sjk_ui::{AbstractAction, DrawList, InputEvent};
use style::MenuStyle;
use winit::event::{ElementState, KeyEvent};
use winit::keyboard::{KeyCode, PhysicalKey};

/// One top-level entry: the label and the one-line hint shown while selected.
pub(crate) struct MainItem {
    pub(crate) label: &'static str,
    pub(crate) hint: &'static str,
}

const MAIN_ITEMS: [MainItem; 8] = [
    MainItem {
        label: "Play",
        hint: "Browse and join servers",
    },
    MainItem {
        label: "Create game",
        hint: "Host a match with bots on this machine",
    },
    MainItem {
        label: "Player",
        hint: "Name, model, saber and Force",
    },
    MainItem {
        label: "Settings",
        hint: "Video, audio, controls and keys",
    },
    MainItem {
        label: "Changelog",
        hint: "What changed in each SJK release, and who made it",
    },
    MainItem {
        label: "Credits",
        hint: "The people who make SJK",
    },
    MainItem {
        label: "Update",
        hint: "Check for a newer SJK release and install it",
    },
    MainItem {
        label: "Quit",
        hint: "Exit to desktop",
    },
];

/// Hand the loaded map and mounted assets to the menu layer.
pub(crate) fn attach_world(
    menu: &mut Option<ClientMenu>,
    vfs: std::sync::Arc<sjk_vfs::VirtualFileSystem>,
    bsp: &sjk_bsp::Bsp,
    console: Option<&ViewerConsole>,
) {
    if let Some(menu) = menu {
        menu.attach_catalogue(vfs);
        menu.backdrop = crate::menu_backdrop::Backdrop::from_bsp(bsp);
        if let Some(console) = console {
            menu.player.prime(console);
        }
    }
}

/// Feed decoded menu images to the UI renderer while their screen is up: the
/// player screen's model icons, the key bindings' pictures, Create game's map
/// preview, the HUD picker's preview (drawn from `vfs` and `shaders`, the
/// files the HUD reads).
pub(crate) fn upload_menu_images(
    menu: &mut Option<ClientMenu>,
    renderer: &mut crate::ui_renderer::ShapeRenderer,
    device: &wgpu::Device,
    queue: &crate::frame_queue::FrameQueue,
    vfs: Option<&std::sync::Arc<sjk_vfs::VirtualFileSystem>>,
    shaders: &sjk_shader::ShaderCatalog,
) {
    let Some(menu) = menu else { return };
    match menu.state.phase() {
        ClientPhase::Player => menu.player.upload_icons(renderer, queue),
        ClientPhase::Keybinds => menu.keybinds.upload_icons(renderer, queue),
        ClientPhase::Settings => menu.settings.service_hud_picker(vfs, shaders, |image| {
            renderer.upload_hud_preview(device, queue, image);
        }),
        ClientPhase::CreateGame => {
            menu.create_game
                .service_levelshots(|image| renderer.upload_levelshot(device, queue, image));
        }
        // The SJK UI's browser shows the chosen server's map.
        ClientPhase::Browser if menu.menu_style == MenuStyle::Sjk => {
            let map = menu
                .browser
                .visible_entry(menu.browser.selected())
                .map(|entry| classic::loading::levelshot_key(&entry.map))
                .unwrap_or_default();
            if !map.is_empty() {
                menu.create_game.service_levelshot_for(&map, |image| {
                    renderer.upload_levelshot(device, queue, image);
                });
            }
        }
        ClientPhase::Connecting(_) | ClientPhase::ConnectionError if menu.is_classic() => {
            let map = menu.loading.map().to_owned();
            if !map.is_empty() {
                menu.create_game.service_levelshot_for(&map, |image| {
                    renderer.upload_levelshot(device, queue, image);
                });
            }
        }
        _ => {}
    }
}

/// Side effects requested by menu input and executed by the application.
#[derive(Debug, Eq, PartialEq)]
pub(crate) enum MenuAction {
    None,
    Connect(String),
    CancelJoin,
    /// Start a local server with these settings and join it.
    HostGame(crate::connection::local_server::HostSettings),
    Quit,
    ReturnToGameMenu,
}

/// Persistent menu selection, browser data, and high-level client phase.
pub(crate) struct ClientMenu {
    state: ClientState,
    browser: ServerBrowser,
    browser_focus: u16,
    /// Last clicked browser row and when, for double-click joining.
    browser_last_click: Option<(u16, std::time::Instant)>,
    main_selection: usize,
    /// Layout of the main menu (`ui_menuStyle`).
    menu_style: MenuStyle,
    /// Page and entry of the classic main menu.
    classic: classic::ClassicMain,
    /// The SJK UI's main page: its page of the ring, chosen entry and motion.
    home: sjk::home::Home,
    /// The servers joined last, for the SJK UI's main page.
    recent: sjk::recent::RecentServers,
    /// The SJK UI's Settings: whether it is open and on which category.
    sjk_settings: sjk::settings::SettingsPage,
    /// The SJK UI's server browser: which item of its left column has the
    /// keyboard, if any.
    sjk_browser: sjk::browser::BrowserPage,
    /// The SJK UI's loading screen: the levelshot's fade and the furthest
    /// step the join has shown.
    sjk_loading: sjk::loading::LoadingPage,
    /// The classic option panel on show in the settings or key-binding
    /// phase, if any.
    classic_panel: Option<classic::ClassicPanel>,
    /// The classic panel the renderer settings were opened from, reopened when
    /// they close.
    renderer_panel: Option<classic::ClassicPanel>,
    /// Retail menu artwork the classic style can draw this frame.
    art: art::ArtSet,
    /// The classic connect and loading screens' state.
    loading: classic::loading::ClassicLoading,
    /// The world is not drawn under the menu this frame (classic style).
    world_hidden: bool,
    /// The retail background drawn under the modern screens the classic
    /// pages open, while the world is hidden.
    classic_backdrop: MenuCanvas,
    /// The dark of the backdrop tour's fades ([`Self::world_fade`]).
    world_fade: MenuCanvas,
    /// The key-binding editor was opened straight from a classic Controls
    /// entry, so closing it leaves the settings screen out.
    keybinds_direct: bool,
    settings: SettingsMenu,
    /// Where the settings screen (and the key-bindings editor it hosts)
    /// returns when closed: the main menu, or the game menu that opened it.
    settings_return: ReturnTarget,
    /// Where the server browser returns when closed without joining.
    browser_return: ReturnTarget,
    keybinds: KeybindEditor,
    /// The search typed on the settings tab on show and how many entries of the
    /// other tab match it (see `sync_cross_search`).
    cross_search: (String, usize),
    player: PlayerMenu,
    /// The Create game screen and the server it runs.
    create_game: create_game::CreateGameMenu,
    ui: MenuCanvas,
    filter_editing: bool,
    /// The classic browser's SERVER INFO pop-up is open.
    classic_info: bool,
    password_target: Option<String>,
    password: String,
    address_editing: bool,
    address_input: String,
    address_error: String,
    backdrop: Option<crate::menu_backdrop::Backdrop>,
    /// Map of the server being joined, as the browser row advertised it.
    destination_map: Option<String>,
    /// The preview of that map can be shown: the gate may open.
    destination_ready: bool,
    /// Last gate request logged, so the log only records changes.
    gate_logged: bool,
    /// First setup was offered in this run (`offer_first_setup`).
    first_setup_offered: bool,
}

/// Backdrop shot each client phase is presented over. A connect stays on
/// the browser shot: the gate there is what opens onto the server.
fn shot_for(phase: &ClientPhase, player: &PlayerMenu) -> Shot {
    match phase {
        ClientPhase::Browser
        | ClientPhase::Connecting(_)
        | ClientPhase::ConnectionError
        | ClientPhase::CreateGame => Shot::Browser,
        ClientPhase::Settings | ClientPhase::Keybinds => Shot::Settings,
        ClientPhase::Player => player.shot(),
        _ => Shot::Main,
    }
}

/// Whether `phase` is a connect in progress, during which the backdrop's
/// gate opens.
fn connecting(phase: &ClientPhase) -> bool {
    matches!(phase, ClientPhase::Connecting(_))
}

impl ClientMenu {
    pub(crate) fn new(open_main_menu: bool, master: String) -> Self {
        Self {
            state: ClientState::new(open_main_menu),
            browser: ServerBrowser::new(master),
            browser_focus: 1_000,
            browser_last_click: None,
            main_selection: 0,
            menu_style: MenuStyle::default(),
            classic: classic::ClassicMain::new(),
            home: sjk::home::Home::default(),
            // Tests never read or write the player's own list.
            recent: if cfg!(test) {
                sjk::recent::RecentServers::default()
            } else {
                sjk::recent::RecentServers::load()
            },
            sjk_settings: sjk::settings::SettingsPage::default(),
            sjk_browser: sjk::browser::BrowserPage::default(),
            sjk_loading: sjk::loading::LoadingPage::default(),
            classic_panel: None,
            renderer_panel: None,
            art: art::ArtSet::default(),
            loading: classic::loading::ClassicLoading::default(),
            world_hidden: false,
            classic_backdrop: MenuCanvas::new(),
            world_fade: MenuCanvas::new(),
            keybinds_direct: false,
            settings: SettingsMenu::new(),
            settings_return: ReturnTarget::MainMenu,
            browser_return: ReturnTarget::MainMenu,
            keybinds: KeybindEditor::new(),
            cross_search: (String::new(), 0),
            player: PlayerMenu::new(),
            create_game: create_game::CreateGameMenu::new(),
            ui: MenuCanvas::new(),
            filter_editing: false,
            classic_info: false,
            password_target: None,
            destination_map: None,
            destination_ready: false,
            gate_logged: false,
            first_setup_offered: false,
            password: String::with_capacity(64),
            address_editing: false,
            address_input: String::with_capacity(256),
            address_error: String::with_capacity(96),
            backdrop: None,
        }
    }

    /// Advance the live-map backdrop toward the current screen's shot and
    /// return the camera for this frame, once a map is loaded.
    pub(crate) fn drive_backdrop(&mut self, millis: u64) -> Option<Sample> {
        let shot = shot_for(self.state.phase(), &self.player);
        // The classic style shows retail's loading screen instead, and the
        // SJK UI its own: the gate stays shut and the joined world appears
        // when it is live.
        let gate = connecting(self.state.phase())
            && self.destination_ready
            && !self.menu_style.classic_screens();
        if gate != self.gate_logged {
            self.gate_logged = gate;
            crate::log::progress(format_args!(
                "gate wanted={gate} at {millis} ms (phase {:?}, destination ready={})",
                self.state.phase(),
                self.destination_ready
            ));
        }
        self.backdrop.as_mut().map(|backdrop| {
            backdrop.set_gate(gate);
            backdrop.drive(shot, millis)
        })
    }

    /// Whether the joined server's world is built and waiting behind the
    /// gate; the glide through the gate holds until it is.
    pub(crate) fn set_world_ready(&mut self, ready: bool) {
        if let Some(backdrop) = &mut self.backdrop {
            backdrop.set_world_ready(ready);
        }
    }

    /// How far the backdrop's gate prop is open: only during a connect, so
    /// the gate of a map joined for play stands where the server has it.
    pub(crate) fn gate_open(&self, millis: u64) -> f32 {
        match &self.backdrop {
            Some(backdrop) if connecting(self.state.phase()) && !self.is_classic() => {
                backdrop.gate_open(millis)
            }
            _ => 0.0,
        }
    }

    /// Whether a finished join may cut to the server world: the camera has
    /// flown through the gate, or there is no gate flight on this map.
    pub(crate) fn gate_crossed(&self, millis: u64) -> bool {
        match &self.backdrop {
            Some(backdrop) if connecting(self.state.phase()) && !self.is_classic() => {
                backdrop.gate_crossed(millis)
            }
            _ => true,
        }
    }

    /// Progress of the glide through the gate during a connect (diagnostics).
    pub(crate) fn passage_progress(&self, millis: u64) -> f32 {
        match &self.backdrop {
            Some(backdrop) if connecting(self.state.phase()) => backdrop.passage_progress(millis),
            _ => 0.0,
        }
    }

    /// A cue the backdrop's gate passed while opening, with where its dust
    /// falls from.
    pub(crate) fn take_gate_cue(&mut self) -> Option<(crate::world_props::GateCue, [[f32; 3]; 2])> {
        self.backdrop.as_mut()?.take_gate_cue()
    }

    /// The menu world is back after a game: its gate is shut and the glide
    /// through it over, whatever state the join left them in.
    pub(crate) fn reset_gate(&mut self) {
        self.destination_ready = false;
        if let Some(backdrop) = &mut self.backdrop {
            backdrop.reset_gate();
        }
    }

    /// Whether the joined server's map is still loading behind this menu.
    /// Whether a join is in progress (connecting or loading the map).
    pub(crate) fn is_connecting(&self) -> bool {
        connecting(self.state.phase())
    }

    /// Whether the menus' screens are the classic ones: the classic style, or
    /// the SJK UI on a screen it has no version of yet.
    pub(crate) fn is_classic(&self) -> bool {
        self.menu_style.classic_screens()
    }

    /// Whether a screen of the SJK UI's own is on show (its main page, its
    /// Settings, Character, Servers or its loading screen), which sits over
    /// the live map where the classic pages cover it and draws in the UI's
    /// families. The loading screen covers the map itself when it has to
    /// ([`Self::sjk_loading_hides_world`]).
    pub(crate) fn sjk_screen(&self) -> bool {
        self.menu_style == MenuStyle::Sjk
            && match self.state.phase() {
                ClientPhase::MainMenu => true,
                ClientPhase::Settings => {
                    self.sjk_settings_on_show() && !self.settings.picker_open()
                }
                ClientPhase::Keybinds => self.sjk_settings_on_show(),
                ClientPhase::Player => self.player.is_sjk(),
                ClientPhase::Browser
                | ClientPhase::Connecting(_)
                | ClientPhase::ConnectionError => true,
                _ => false,
            }
    }

    /// Whether the connect or loading screen covers the screen: a connect
    /// in progress or its failure.
    pub(crate) fn is_loading_screen(&self) -> bool {
        matches!(
            self.state.phase(),
            ClientPhase::Connecting(_) | ClientPhase::ConnectionError
        )
    }

    /// Tell the menu whether the world is drawn under it this frame.
    pub(crate) fn set_world_hidden(&mut self, hidden: bool) {
        self.world_hidden = hidden;
    }

    /// The map the classic loading screen shows (`mp/ffa3`), if known.
    pub(crate) fn loading_map(&self) -> &str {
        self.loading.map()
    }

    /// The classic loading screen's state (the SJK UI's loading screen reads
    /// it too), for the join to report into.
    pub(crate) fn loading_mut(&mut self) -> &mut classic::loading::ClassicLoading {
        &mut self.loading
    }

    /// Build the dark the backdrop's camera tour fades through between its
    /// shots and cuts through to a screen's shot, over the world `viewport`
    /// shows ([`Self::world_fade`]); false while the world is clear.
    pub(crate) fn prepare_world_fade(&mut self, viewport: [f32; 2]) -> bool {
        let darkness = self
            .backdrop
            .as_ref()
            .map_or(0.0, crate::menu_backdrop::Backdrop::darkness);
        if darkness <= 0.0 {
            return false;
        }
        self.world_fade.begin_transparent(viewport);
        // The SJK UI's deep navy rather than black, as its fades are.
        let _ = self
            .world_fade
            .draw_list_mut()
            .push(sjk_ui::DrawCommand::SolidRect {
                rect: sjk_ui::Rect::new(0.0, 0.0, viewport[0], viewport[1]),
                color: sjk::color::alpha(sjk::color::SPACE, darkness),
            });
        true
    }

    /// The fade [`Self::prepare_world_fade`] built this frame.
    pub(crate) fn world_fade(&self) -> &DrawList {
        self.world_fade.draw_list()
    }

    /// The retail background drawn under a modern screen while the world is
    /// hidden; built in [`Self::append_overlay`].
    pub(crate) fn backdrop_draw_list(&self) -> Option<&DrawList> {
        self.backdrop_draw_list_wanted()
            .then(|| self.classic_backdrop.draw_list())
    }

    fn backdrop_draw_list_wanted(&self) -> bool {
        let modern_screen = matches!(
            self.state.phase(),
            ClientPhase::Browser
                | ClientPhase::Settings
                | ClientPhase::Keybinds
                | ClientPhase::Player
                | ClientPhase::CreateGame
        );
        self.world_hidden && modern_screen
    }

    /// The map the browser row of the server being joined advertised
    /// (`mp/duel1`), if the join came from a row.
    pub(crate) fn destination_map(&self) -> Option<&str> {
        self.destination_map
            .as_deref()
            .filter(|map| !map.is_empty())
    }

    /// Whether the world beyond the gate is ready to be shown.
    pub(crate) fn set_destination_ready(&mut self, ready: bool) {
        self.destination_ready = ready;
    }

    /// The player model that stands on the backdrop's stage: only while a
    /// route with a stage is active (flying to, parked on, or flying back
    /// from the Player screen).
    pub(crate) fn stage_model(&self) -> Option<(Stage, &str)> {
        // The classic profile pages draw the model's portrait over retail
        // art; nothing stands on the stage behind them. Opened from a game,
        // the screen shows the model in its own preview: the menu map's stage
        // is not the match's.
        if self.player.is_classic() || self.player.return_target() == ReturnTarget::InGame {
            return None;
        }
        let stage = self.backdrop.as_ref()?.stage()?;
        Some((stage, self.player.stage_model()))
    }

    /// The model the classic profile's preview shows, with the stage it
    /// may stand on: only while the profile is on screen and its page shows
    /// one.
    pub(crate) fn preview_model(
        &self,
    ) -> Option<(Option<Stage>, &str, crate::player_menu::ModelPreview)> {
        if !self.is_visible() || !matches!(self.state.phase(), ClientPhase::Player) {
            return None;
        }
        let preview = self.player.model_preview()?;
        let stage = self.backdrop.as_ref().and_then(|backdrop| backdrop.stage());
        Some((stage, self.player.stage_model(), preview))
    }

    /// Whether the renderer has a preview frame for the profile to show.
    pub(crate) fn set_preview_ready(&mut self, ready: bool) {
        self.player.set_preview_ready(ready);
    }

    /// The sabers in the stage model's hands.
    pub(crate) fn stage_sabers(&self) -> crate::player_menu::StageSabers<'_> {
        let open = matches!(self.state.phase(), ClientPhase::Player);
        self.player.stage_sabers(open)
    }

    /// Where the thrown saber floats on this map's saber shot.
    pub(crate) fn saber_focus(&self) -> Option<crate::menu_backdrop::Focus> {
        self.backdrop.as_ref()?.focus(Shot::Saber)
    }

    /// Opacity of the current screen: 1 while the backdrop camera is parked
    /// on its shot, fading in as the flight arrives, 0 while it is away.
    fn screen_reveal(&self) -> f32 {
        if self.opened_from_game() {
            // Over a live match the backdrop camera is not flying anywhere.
            return 1.0;
        }
        let shot = shot_for(self.state.phase(), &self.player);
        self.backdrop
            .as_ref()
            .map_or(1.0, |backdrop| backdrop.reveal(shot))
    }

    /// Apply the player's accent colour to every screen this menu draws.
    pub(crate) fn set_accent(&mut self, accent: sjk_ui::Color) {
        self.ui.set_accent(accent);
    }

    /// Whether the settings screen wants the window's monitor facts.
    pub(crate) fn wants_monitor_modes(&self) -> bool {
        self.settings.wants_monitor_modes()
    }

    /// Hand the settings screen the window's monitor facts.
    pub(crate) fn set_monitor_modes(
        &mut self,
        modes: crate::settings::MonitorModes,
        console: &ViewerConsole,
    ) {
        self.settings.set_monitor_modes(modes, console);
    }

    /// Start the master-server fetch now, ahead of the browser being
    /// opened, so its rows are waiting when "Play" is pressed.
    pub(crate) fn prefetch_servers(&mut self) {
        self.browser.refresh();
    }

    /// Evidence hook: open the server browser the way "Play" would, minus
    /// the master-server refresh (evidence runs stay off the network).
    pub(crate) fn open_browser(&mut self) {
        self.browser_return = ReturnTarget::MainMenu;
        self.state.open_browser();
        self.reset_sjk_browser();
    }

    /// Open the server browser over a live match, returning to the game
    /// menu when closed; a join from it leaves the current server. The
    /// caller fetches rows with [`Self::refresh_servers_if_stale`].
    pub(crate) fn open_browser_from_game(&mut self, console: &ViewerConsole) {
        if let Ok(master) = console.master_server() {
            self.browser.set_master(master);
        }
        self.browser_return = ReturnTarget::InGame;
        self.state.open_browser();
        self.filter_editing = false;
        self.browser_focus = 1_000 + self.browser.selected() as u16;
        self.reset_sjk_browser();
    }

    /// Start a master-server fetch unless fresh rows are already on show.
    pub(crate) fn refresh_servers_if_stale(&mut self) {
        if self.browser.is_stale() {
            self.refresh();
        }
    }

    /// Close the browser toward wherever it was opened from.
    pub(super) fn close_browser(&mut self) -> MenuAction {
        self.classic_info = false;
        match self.browser_return {
            ReturnTarget::MainMenu => {
                self.state.main_menu();
                MenuAction::None
            }
            ReturnTarget::InGame => {
                self.state.entered_game();
                MenuAction::ReturnToGameMenu
            }
        }
    }

    /// Open the settings screen on `tab`, returning to `target` when closed.
    pub(crate) fn open_settings_from(
        &mut self,
        console: &ViewerConsole,
        target: ReturnTarget,
        tab: usize,
    ) {
        self.leave_sjk_settings();
        self.settings.open_tab(console, tab);
        self.renderer_panel = None;
        self.settings_return = target;
        self.state.open_settings();
    }

    /// Act on what the settings screen asked for.
    pub(super) fn settings_result(
        &mut self,
        result: SettingsResult,
        console: &mut ViewerConsole,
    ) -> MenuAction {
        if let Some(action) = self.sjk_settings_result(&result, console) {
            return action;
        }
        match result {
            SettingsResult::Back => {
                if let Some(panel) = self.renderer_panel.take() {
                    self.reopen_classic_panel(console, panel);
                    return MenuAction::None;
                }
                // Setup's sub-pages back out to Setup, as their Back does.
                if let Some(panel) = self
                    .classic_panel
                    .filter(|panel| panel.page.is_setup_child())
                {
                    let setup = panel.page.escape();
                    if let Some(entry) = setup.opening_panel() {
                        let target = self.settings_return;
                        self.open_classic_panel(console, setup, entry, panel.frame, target);
                        return MenuAction::None;
                    }
                }
                self.close_settings()
            }
            SettingsResult::OpenKeybinds => {
                self.keybinds.open(console);
                self.keybinds_direct = false;
                self.state.open_keybinds();
                MenuAction::None
            }
            // The classic+ and modern settings show the quick wheel's editor on
            // its own (the SJK UI's has its category, `sjk_settings_result`).
            SettingsResult::OpenWheelPages => {
                self.settings
                    .open_wheel_editor(console, crate::settings::WheelMode::Overlay);
                MenuAction::None
            }
            SettingsResult::Classic(index) => self.classic_panel_button(index, console),
            SettingsResult::ClassicCycle(direction) => {
                self.classic_panel_cycle(direction, console);
                MenuAction::None
            }
            // Only the SJK UI's First setup pop-up offers it.
            SettingsResult::AllSettings | SettingsResult::None => MenuAction::None,
        }
    }

    /// Act on what the key-binding editor asked for.
    pub(super) fn keybinds_result(
        &mut self,
        result: EditorResult,
        console: &mut ViewerConsole,
    ) -> MenuAction {
        if let Some(action) = self.sjk_keys_result(&result, console) {
            return action;
        }
        match result {
            EditorResult::Back => self.close_keybinds(console),
            EditorResult::Classic(index) => self.classic_panel_button(index, console),
            EditorResult::ClassicCycle(direction) => {
                self.classic_panel_cycle(direction, console);
                MenuAction::None
            }
            EditorResult::None => MenuAction::None,
        }
    }

    /// Close the settings screen toward wherever it was opened from.
    pub(super) fn close_settings(&mut self) -> MenuAction {
        self.close_sjk_settings();
        let classic_panel = self.leave_classic_panel();
        match self.settings_return {
            ReturnTarget::MainMenu => {
                self.state.main_menu();
                // Escape on a retail Setup or Controls page closes it to the
                // main page.
                if classic_panel {
                    self.classic.show(classic::layout::Page::Main);
                }
                MenuAction::None
            }
            ReturnTarget::InGame => {
                self.state.entered_game();
                MenuAction::ReturnToGameMenu
            }
        }
    }

    /// Whether the screen on show was opened from the game menu, over a
    /// live match rather than the menu map.
    fn opened_from_game(&self) -> bool {
        match self.state.phase() {
            ClientPhase::Settings | ClientPhase::Keybinds => {
                self.settings_return == ReturnTarget::InGame
            }
            ClientPhase::Player => self.player.return_target() == ReturnTarget::InGame,
            ClientPhase::Browser => self.browser_return == ReturnTarget::InGame,
            _ => false,
        }
    }

    pub(crate) fn is_visible(&self) -> bool {
        self.state.is_overlay_visible()
    }

    /// The main menu is up: not a settings, browser or connection screen.
    pub(crate) fn on_main_menu(&self) -> bool {
        matches!(self.state.phase(), ClientPhase::MainMenu)
    }

    /// The Renderer settings page is on show; eye adaptation holds still under it.
    pub(crate) fn renderer_settings_open(&self) -> bool {
        self.is_visible()
            && matches!(self.state.phase(), ClientPhase::Settings)
            && self.settings.renderer_open()
    }

    pub(crate) fn poll(&mut self) {
        if matches!(self.state.phase(), ClientPhase::Player) || !self.player.is_resolved() {
            self.player.poll();
        }
        if matches!(self.state.phase(), ClientPhase::Keybinds) {
            self.keybinds.poll_icons();
        }
        self.poll_local_server();
        let browser_visible = matches!(self.state.phase(), ClientPhase::Browser);
        if browser_visible {
            self.browser.tick_details();
        }
        match self.browser.poll() {
            RefreshPoll::Idle | RefreshPoll::Pending => {}
            RefreshPoll::Complete if browser_visible && self.browser.entries().is_empty() => {
                self.state
                    .set_status("No responding protocol-26 servers found.");
            }
            RefreshPoll::Complete if browser_visible => self.state.set_status(format!(
                "{} responding servers. Enter joins; R refreshes.",
                self.browser.entries().len()
            )),
            RefreshPoll::Failed(error) if browser_visible => {
                self.state.set_status(format!("Refresh failed: {error}"));
            }
            RefreshPoll::Complete | RefreshPoll::Failed(_) => {}
        }
    }

    /// A key on the server browser, in every style (the SJK UI's handles some
    /// first, [`ClientMenu::sjk_browser_key`]).
    fn browser_key(&mut self, key: KeyCode) -> MenuAction {
        match key {
            KeyCode::ArrowUp | KeyCode::KeyW => {
                self.browser.move_selection(-1);
                self.browser_focus = 1_000 + self.browser.selected() as u16;
                MenuAction::None
            }
            KeyCode::ArrowDown | KeyCode::KeyS => {
                self.browser.move_selection(1);
                self.browser_focus = 1_000 + self.browser.selected() as u16;
                MenuAction::None
            }
            KeyCode::PageUp | KeyCode::PageDown => {
                self.browser
                    .page_selection(if key == KeyCode::PageUp { -1 } else { 1 });
                self.browser_focus = 1_000 + self.browser.selected() as u16;
                MenuAction::None
            }
            KeyCode::Home | KeyCode::End => {
                self.browser.select_end(key == KeyCode::End);
                self.browser_focus = 1_000 + self.browser.selected() as u16;
                MenuAction::None
            }
            KeyCode::Tab => {
                self.browser
                    .set_favorites_only(!self.browser.favorites_only());
                self.browser_focus = 1_000 + self.browser.selected() as u16;
                MenuAction::None
            }
            KeyCode::KeyC => {
                self.open_address_entry();
                MenuAction::None
            }
            KeyCode::KeyR => {
                self.refresh();
                MenuAction::None
            }
            KeyCode::KeyF => {
                self.browser.toggle_selected_favorite();
                MenuAction::None
            }
            KeyCode::Slash => {
                self.filter_editing = true;
                MenuAction::None
            }
            KeyCode::Backspace if self.filter_editing => {
                self.browser.pop_filter();
                MenuAction::None
            }
            KeyCode::Digit1 => {
                self.browser.sort_by(SortColumn::Name);
                MenuAction::None
            }
            KeyCode::Digit2 => {
                self.browser.sort_by(SortColumn::Map);
                MenuAction::None
            }
            KeyCode::Digit3 => {
                self.browser.sort_by(SortColumn::Players);
                MenuAction::None
            }
            KeyCode::Digit4 => {
                self.browser.sort_by(SortColumn::Ping);
                MenuAction::None
            }
            KeyCode::Digit5 => {
                self.browser.sort_by(SortColumn::Gametype);
                MenuAction::None
            }
            KeyCode::Enter | KeyCode::NumpadEnter | KeyCode::Space => self.activate_browser_focus(),
            KeyCode::Escape => {
                if self.classic_info {
                    self.classic_info = false;
                    MenuAction::None
                } else if self.filter_editing {
                    self.filter_editing = false;
                    MenuAction::None
                } else {
                    self.close_browser()
                }
            }
            _ => MenuAction::None,
        }
    }

    pub(crate) fn handle_key(
        &mut self,
        event: &KeyEvent,
        console: &mut ViewerConsole,
    ) -> MenuAction {
        if !self.is_visible() || event.state != ElementState::Pressed || event.repeat {
            return MenuAction::None;
        }
        if self.address_editing {
            let PhysicalKey::Code(key) = event.physical_key else {
                return MenuAction::None;
            };
            match key {
                KeyCode::Escape => {
                    self.address_editing = false;
                    self.address_error.clear();
                }
                KeyCode::Backspace => {
                    self.address_input.pop();
                    self.address_error.clear();
                }
                KeyCode::Enter | KeyCode::NumpadEnter => return self.submit_address(),
                _ => {
                    if let Some(text) = event
                        .text
                        .as_deref()
                        .filter(|text| text.chars().all(|character| !character.is_control()))
                    {
                        let remaining = 255_usize.saturating_sub(self.address_input.len());
                        self.address_input.extend(text.chars().take(remaining));
                        self.address_error.clear();
                    }
                }
            }
            return MenuAction::None;
        }
        if let Some(address) = self.password_target.clone() {
            let PhysicalKey::Code(key) = event.physical_key else {
                return MenuAction::None;
            };
            match key {
                KeyCode::Escape => {
                    self.password_target = None;
                    self.password.clear();
                }
                KeyCode::Backspace => {
                    self.password.pop();
                }
                KeyCode::Enter | KeyCode::NumpadEnter if !self.password.is_empty() => {
                    console.set_cvar("password", &self.password);
                    self.password_target = None;
                    self.state.connecting(address.clone());
                    return MenuAction::Connect(address);
                }
                _ => {
                    if let Some(text) = event
                        .text
                        .as_deref()
                        .filter(|text| text.chars().all(|character| !character.is_control()))
                    {
                        self.password.push_str(text);
                    }
                }
            }
            return MenuAction::None;
        }
        if matches!(self.state.phase(), ClientPhase::Browser) && self.filter_editing {
            if let Some(text) = event
                .text
                .as_deref()
                .filter(|text| text.chars().all(|character| !character.is_control()))
            {
                self.browser.push_filter(text);
                return MenuAction::None;
            }
        }
        let PhysicalKey::Code(key) = event.physical_key else {
            return MenuAction::None;
        };
        match self.state.phase() {
            ClientPhase::MainMenu if self.menu_style == MenuStyle::Sjk => {
                self.sjk_home_key(key, console)
            }
            ClientPhase::MainMenu => match key {
                KeyCode::ArrowUp | KeyCode::KeyW => {
                    self.navigate_main(AbstractAction::Previous);
                    MenuAction::None
                }
                KeyCode::ArrowDown | KeyCode::KeyS | KeyCode::Tab => {
                    self.navigate_main(AbstractAction::Next);
                    MenuAction::None
                }
                KeyCode::Enter | KeyCode::NumpadEnter | KeyCode::Space => {
                    self.activate_main(console)
                }
                KeyCode::Escape if self.menu_style == MenuStyle::Classic => {
                    self.cancel_classic();
                    MenuAction::None
                }
                _ => MenuAction::None,
            },
            ClientPhase::Browser if self.menu_style == MenuStyle::Sjk => {
                match self.sjk_browser_key(key, console) {
                    Some(action) => action,
                    None => self.browser_key(key),
                }
            }
            ClientPhase::Browser => self.browser_key(key),
            ClientPhase::Settings => {
                let result = self.settings.handle_key(event, console);
                self.settings_result(result, console)
            }
            ClientPhase::Keybinds => {
                let result = self.keybinds.handle_key(event, console);
                self.keybinds_result(result, console)
            }
            ClientPhase::Player => {
                let result = self.player.handle_key(event, console);
                self.player_result(result, console)
            }
            ClientPhase::CreateGame => {
                let result = self.create_game.key(key, event.text.as_deref(), console);
                self.create_game_result(result)
            }
            ClientPhase::Connecting(_) if key == KeyCode::Escape => self.cancel_join(),
            ClientPhase::ConnectionError
                if matches!(
                    key,
                    KeyCode::Escape | KeyCode::Enter | KeyCode::NumpadEnter | KeyCode::Space
                ) =>
            {
                self.state.open_browser();
                MenuAction::None
            }
            ClientPhase::Connecting(_) | ClientPhase::ConnectionError | ClientPhase::InGame => {
                MenuAction::None
            }
        }
    }

    pub(crate) fn draw_list(&self) -> Option<&DrawList> {
        match self.state.phase() {
            ClientPhase::MainMenu
            | ClientPhase::Browser
            | ClientPhase::Connecting(_)
            | ClientPhase::ConnectionError => Some(self.ui.draw_list()),
            ClientPhase::Settings => Some(self.settings.draw_list()),
            ClientPhase::Keybinds => Some(self.keybinds.draw_list()),
            ClientPhase::Player => Some(self.player.draw_list()),
            ClientPhase::CreateGame => Some(self.create_game.draw_list()),
            ClientPhase::InGame => None,
        }
    }

    /// Selection metadata consumed by the shared backdrop/highlight shader.
    pub(crate) fn visual_selection(&self) -> (f32, f32) {
        match self.state.phase() {
            ClientPhase::MainMenu => (1.0, self.main_selection as f32),
            ClientPhase::Browser => (2.0, self.browser.selected() as f32),
            ClientPhase::Settings => {
                let (selected, root) = self.settings.visual_selection();
                (if root { 3.0 } else { 4.0 }, selected as f32)
            }
            ClientPhase::Keybinds => (5.0, self.keybinds.visual_selection() as f32),
            ClientPhase::Player => (7.0, self.player.visual_selection() as f32),
            ClientPhase::CreateGame => (8.0, self.create_game.visual_selection() as f32),
            ClientPhase::Connecting(_) => (6.0, 0.0),
            ClientPhase::ConnectionError => (6.0, 0.0),
            ClientPhase::InGame => (0.0, 0.0),
        }
    }

    pub(crate) fn handle_mouse_binding(
        &mut self,
        button: winit::event::MouseButton,
        console: &mut ViewerConsole,
    ) -> bool {
        matches!(self.state.phase(), ClientPhase::Keybinds)
            && self.keybinds.capture_mouse(button, console)
    }

    /// A wheel notch while the key-binding form awaits a key binds it.
    pub(crate) fn handle_wheel_binding(&mut self, up: bool, console: &mut ViewerConsole) -> bool {
        matches!(self.state.phase(), ClientPhase::Keybinds)
            && self.keybinds.capture_wheel(up, console)
    }

    pub(crate) fn joined(&mut self) {
        self.remember_joined_server();
        self.state.entered_game();
    }

    pub(crate) fn state_connecting(&mut self, address: String) {
        // A join leaves the current server; a cancelled or failed one lands
        // in the browser, which then returns to the main menu.
        self.browser_return = ReturnTarget::MainMenu;
        self.state.connecting(address);
    }

    /// A join of `address` starts: the classic loading screen starts over,
    /// with the map the browser row advertised if there was one.
    pub(crate) fn begin_join(&mut self, address: &str) {
        let local = self.hosting_local();
        self.loading.begin(address, local);
        if let Some(map) = &self.destination_map {
            self.loading.set_map(map);
        }
    }

    /// Whether the client hosts the server it joins (Create game), which
    /// leaves the server lines off the loading screen as `sv_running` does.
    pub(crate) fn hosting_local(&self) -> bool {
        self.create_game.hosting()
    }

    /// Show map preparation using the existing cancellable connection notice.
    pub(crate) fn state_loading(&mut self, map: &str) {
        self.state_connecting(map.to_owned());
        self.loading.begin_map_load(map);
        self.state.set_status(format!("Loading {map}..."));
    }

    pub(crate) fn join_failed(&mut self, error: impl Into<String>) {
        self.state.connection_failed(error);
    }

    pub(crate) fn return_to_main_menu(&mut self) {
        self.state.main_menu();
        self.main_selection = 0;
        self.classic.reset();
        self.home.reset();
    }

    pub(crate) fn attach_catalogue(&mut self, vfs: std::sync::Arc<sjk_vfs::VirtualFileSystem>) {
        self.create_game.attach_vfs(std::sync::Arc::clone(&vfs));
        self.keybinds.attach_vfs(std::sync::Arc::clone(&vfs));
        self.player.attach_catalogue(vfs);
    }

    /// Leave or stay on the player screen after one of its events.
    pub(super) fn player_result(
        &mut self,
        result: PlayerMenuResult,
        console: &ViewerConsole,
    ) -> MenuAction {
        match result {
            PlayerMenuResult::None => MenuAction::None,
            PlayerMenuResult::Back(ReturnTarget::MainMenu) => {
                self.state.main_menu();
                MenuAction::None
            }
            PlayerMenuResult::Back(ReturnTarget::InGame) => {
                self.state.entered_game();
                MenuAction::ReturnToGameMenu
            }
            PlayerMenuResult::ClassicPage(page) => {
                self.state.main_menu();
                self.classic.show(page);
                // Settings opens on its first group, as from the main page.
                if let Some(entry) = page.opening_panel() {
                    self.open_classic_panel(
                        console,
                        page,
                        entry,
                        classic::panel::Frame::Main,
                        ReturnTarget::MainMenu,
                    );
                }
                MenuAction::None
            }
        }
    }

    pub(crate) fn open_player(&mut self, console: &ViewerConsole, target: ReturnTarget) {
        self.player.open(console, target);
        self.state.open_player();
    }

    /// Show the player screen's page `index` with row `row` selected (world
    /// shots).
    #[cfg(test)]
    pub(crate) fn player_page_for_shot(&mut self, index: usize, row: usize) {
        self.player.show_page_for_shot(index, row);
    }

    #[cfg(test)]
    pub(crate) fn player_hover_level_for_shot(&mut self, power: usize, level: u8) {
        self.player.hover_level_for_shot(power, level);
    }

    #[cfg(test)]
    pub(crate) fn player_dual_for_shot(&mut self) {
        self.player.dual_for_shot();
    }
}

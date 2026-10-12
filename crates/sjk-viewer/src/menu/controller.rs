//! Menu rendering dispatch and activation helpers.

use super::*;

/// What the address prompt is for: joining the server, or keeping it as a favourite.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) enum AddressPurpose {
    #[default]
    Join,
    Favourite,
}

impl ClientMenu {
    /// Share the browser's favorites persistence with addFavorite.
    pub(crate) fn add_favorite(&mut self, address: std::net::SocketAddr) -> Result<(), String> {
        self.browser.add_favorite(address)
    }
    pub(crate) fn append_overlay(
        &mut self,
        vertices: &mut Vec<TextVertex>,
        font: &UiFont,
        viewport: [f32; 2],
    ) {
        if self.backdrop_draw_list_wanted() {
            classic::view::opaque_backdrop(&mut self.classic_backdrop, viewport, self.art);
        }
        match self.state.phase() {
            ClientPhase::MainMenu => self.append_main(vertices, font, viewport),
            // Drawn in the UI's families by `append_sjk_screen`; here, from a
            // caller without them, in Inter.
            ClientPhase::Connecting(_) | ClientPhase::ConnectionError
                if self.menu_style == MenuStyle::Sjk =>
            {
                let target = sjk::TextTarget::Inter(vertices, font);
                self.append_sjk_loading(target, viewport);
            }
            ClientPhase::Connecting(_) | ClientPhase::ConnectionError => {
                let failed = matches!(self.state.phase(), ClientPhase::ConnectionError);
                let error = failed.then(|| self.state.status().to_owned());
                let preview = self.create_game.levelshot_preview(self.loading.map());
                classic::loading::build(
                    &mut self.ui,
                    viewport,
                    &self.loading,
                    self.art,
                    preview,
                    error.as_deref(),
                );
                self.ui.append_text(vertices, font, viewport);
            }
            // Drawn in the UI's families by `append_sjk_screen`; here, from a
            // caller without them, in Inter.
            ClientPhase::Browser if self.menu_style == MenuStyle::Sjk => {
                let target = sjk::TextTarget::Inter(vertices, font);
                self.append_sjk_browser(target, viewport);
            }
            ClientPhase::Browser => {
                let reveal = self.screen_reveal();
                self.append_browser(vertices, font, viewport, reveal);
            }
            ClientPhase::Settings => match self.classic_panel_frame() {
                Some(frame) => {
                    self.sync_cross_search();
                    let reveal = self.screen_reveal();
                    self.settings
                        .append_classic(vertices, font, viewport, reveal, &frame);
                }
                None if self.settings.hud_picker_open() => {
                    let reveal = self.screen_reveal();
                    self.settings
                        .append_hud_overlay(vertices, font, viewport, reveal);
                }
                // The SJK UI's Settings (with the resolution list's card),
                // drawn in the UI's families by `append_sjk_screen`; here, from
                // a caller without them or under the classic style, in Inter.
                None => {
                    let target = sjk::TextTarget::Inter(vertices, font);
                    self.append_sjk_settings(target, viewport);
                }
            },
            ClientPhase::Keybinds => match self.classic_panel_frame() {
                Some(frame) => {
                    let reveal = self.screen_reveal();
                    self.sync_cross_search();
                    self.keybinds
                        .append_classic(vertices, font, viewport, reveal, &frame)
                }
                // The SJK UI's Key bindings, drawn in the UI's families by
                // `append_sjk_screen`; here, from a caller without them or
                // under the classic style, in Inter.
                None => {
                    let target = sjk::TextTarget::Inter(vertices, font);
                    self.append_sjk_keys(target, viewport);
                }
            },
            // Drawn in the UI's families by `append_sjk_screen`; here, from a
            // caller without them, in Inter.
            ClientPhase::Player if self.player.is_sjk() => {
                let reveal = self.screen_reveal();
                let target = sjk::TextTarget::Inter(vertices, font);
                self.player.append_sjk(target, viewport, reveal);
            }
            ClientPhase::Player => {
                let reveal = self.screen_reveal();
                self.player.append(vertices, font, viewport, reveal);
            }
            // The SJK UI's in every menu style (it has no classic+ version),
            // drawn in the UI's families by `append_sjk_screen`; here, from a
            // caller without them, in Inter.
            ClientPhase::CreateGame => {
                let reveal = self.screen_reveal();
                let target = sjk::TextTarget::Inter(vertices, font);
                self.create_game.append(target, viewport, reveal);
            }
            ClientPhase::InGame => {}
        }
    }

    pub(super) fn refresh(&mut self) {
        self.browser.refresh();
        self.state.set_status("Refreshing master server...");
    }

    pub(super) fn join_selected(&mut self) -> MenuAction {
        let Some(entry) = self.browser.visible_entry(self.browser.selected()) else {
            return MenuAction::None;
        };
        let server = entry.address;
        let map = entry.map.clone();
        if entry.password {
            self.password.clear();
            self.password_target = Some(server.to_string());
            self.destination_map = Some(map);
            return MenuAction::None;
        }
        let address = server.to_string();
        self.destination_map = Some(map);
        self.state.connecting(address.clone());
        MenuAction::Connect(address)
    }

    pub(super) fn activate_browser_focus(&mut self) -> MenuAction {
        use super::browser_view::{ADDRESS_TOKEN, BACK_TOKEN, FAVOURITE_TOKEN, REFRESH_TOKEN};
        match self.browser_focus {
            REFRESH_TOKEN => {
                self.refresh();
                MenuAction::None
            }
            FAVOURITE_TOKEN => {
                self.browser.toggle_selected_favorite();
                MenuAction::None
            }
            BACK_TOKEN => self.close_browser(),
            ADDRESS_TOKEN => {
                self.open_address_entry();
                MenuAction::None
            }
            _ => self.join_selected(),
        }
    }

    pub(super) fn open_address_entry(&mut self) {
        self.address_editing = true;
        self.address_purpose = AddressPurpose::Join;
        self.address_error.clear();
    }

    /// The address prompt, to add a favourite by its address (Sol's request,
    /// 11/10/2026): a server no master lists can be kept and joined.
    pub(super) fn open_favourite_entry(&mut self) {
        self.open_address_entry();
        self.address_purpose = AddressPurpose::Favourite;
    }

    pub(super) fn submit_address(&mut self) -> MenuAction {
        if self.address_purpose == AddressPurpose::Favourite {
            self.submit_favourite();
            return MenuAction::None;
        }
        match sjk_client::LegacyServerAddress::parse(&self.address_input) {
            Ok(address) => {
                let address = address.into_string();
                self.address_editing = false;
                self.address_error.clear();
                self.destination_map = None;
                self.state.connecting(address.clone());
                MenuAction::Connect(address)
            }
            Err(error) => {
                self.address_error.clear();
                use std::fmt::Write as _;
                let _ = write!(self.address_error, "Bad server address: {error}");
                MenuAction::None
            }
        }
    }

    /// Keep the typed address as a favourite, show the favourites and ask the
    /// servers again, so it answers in the list; or say what was wrong.
    fn submit_favourite(&mut self) {
        use std::fmt::Write as _;
        use std::net::ToSocketAddrs as _;
        let resolved = sjk_client::LegacyServerAddress::parse(&self.address_input)
            .map_err(|error| format!("Bad server address: {error}"))
            .and_then(|address| {
                address
                    .into_string()
                    .to_socket_addrs()
                    .ok()
                    .and_then(|mut found| found.next())
                    .ok_or_else(|| "That address could not be found".to_owned())
            });
        let added = resolved.and_then(|address| {
            if self.browser.is_favorite(address) {
                Err("That server is already a favourite".to_owned())
            } else {
                self.browser.add_favorite(address)
            }
        });
        self.address_error.clear();
        match added {
            Ok(()) => {
                self.address_editing = false;
                self.address_input.clear();
                self.browser.set_favorites_only(true);
                self.refresh();
            }
            Err(error) => {
                let _ = write!(self.address_error, "{error}");
            }
        }
    }

    pub(crate) fn set_last_address(&mut self, address: &str) {
        self.address_input.clear();
        self.address_input.push_str(address);
    }
}

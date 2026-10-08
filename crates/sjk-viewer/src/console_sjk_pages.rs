//! SJK's pages (What's new, Update, Identity, Credits) in the SJK UI's look:
//! with `ui_menuStyle sjk` they are drawn by their SJK views, in the UI's
//! families ([`crate::menu::sjk::TextTarget`]), over the map. Opening, closing
//! and their keys and pointer stay the console's, as for the other looks.

use super::*;
use crate::menu::sjk::TextTarget;

impl ViewerConsole {
    /// Draw the pages in the SJK UI's look (`sjk`), or in the others.
    pub(crate) fn set_sjk_pages(&mut self, sjk: bool) {
        self.changelog.set_sjk(sjk);
        self.update_panel.set_sjk(sjk);
        self.identity_panel.set_sjk(sjk);
        self.credits.set_sjk(sjk);
    }

    /// Whether the page drawn in place of the console is one of them in the
    /// SJK UI's look (the import page comes first when open, then credits), or
    /// the command browser in it.
    pub(crate) fn sjk_page_open(&self) -> bool {
        if !self.open || self.config_import.is_open() {
            return false;
        }
        if self.credits.is_open() {
            return self.credits.is_sjk();
        }
        if !self.changelog.is_open()
            && !self.update_panel.is_open()
            && !self.identity_panel.is_open()
        {
            // The test list draws over the browser when both are open.
            return !self.debug_panel.is_open() && self.browser.is_open() && self.browser.is_sjk();
        }
        if self.changelog.is_open() {
            return self.changelog.is_sjk();
        }
        if self.update_panel.is_open() {
            return self.update_panel.is_sjk();
        }
        self.identity_panel.is_open() && self.identity_panel.is_sjk()
    }

    /// Draw the open page ([`Self::sjk_page_open`]) with its text to `target`.
    pub(crate) fn append_sjk_page(&mut self, target: TextTarget<'_>, viewport: [f32; 2]) {
        if self.credits.is_open() {
            self.credits.append_sjk(target, viewport);
        } else if self.changelog.is_open() {
            self.changelog.append_sjk(target, viewport);
        } else if self.update_panel.is_open() {
            self.update_panel.append_sjk(target, viewport);
        } else if self.identity_panel.is_open() {
            // Copied out so the page can borrow itself mutably while it draws.
            let snapshot = crate::player_identity::snapshot();
            let key_error = crate::player_identity::key_error();
            let hub_url = self.text_cvar("cl_hubUrl").unwrap_or_default().to_owned();
            let key_file = self
                .config_directory()
                .join("identity.key")
                .display()
                .to_string();
            let inputs = identity_panel::Inputs {
                enabled: self.bool_cvar("cl_identity") == Some(true),
                hub_url: &hub_url,
                key_error: key_error.as_deref(),
                snapshot: snapshot.as_ref(),
                key_file: &key_file,
            };
            self.identity_panel.append_sjk(&inputs, target, viewport);
        } else if self.browser.is_open() {
            self.browser.append_sjk(target, viewport);
        }
    }
}

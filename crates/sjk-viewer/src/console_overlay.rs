//! Console overlay ordering, including the full-frame command browser.
use super::*;

impl GpuState {
    pub(super) fn console_covers_frame(&self) -> bool {
        self.console
            .as_ref()
            .is_some_and(|console| console.covers_frame())
    }

    pub(super) fn append_console_overlay(&mut self, viewport: [f32; 2], text_scale: f32) {
        let covers_frame = self.console_covers_frame();
        if covers_frame {
            // Every font batch must be cleared: text is drawn above all UI shapes.
            self.text_vertices.clear();
            self.classic_text_vertices.clear();
            self.game_fonts.clear_text();
        }
        self.append_classic_console(viewport, covers_frame);
        // Sol JK's pages in the SJK UI draw in its families once they are loaded.
        if covers_frame
            && let Some(console) = self
                .console
                .as_mut()
                .filter(|console| console.sjk_page_open())
        {
            let target = match self.game_fonts.sjk() {
                Some(fonts) => menu::sjk::TextTarget::Families(fonts, self.ui_font.style()),
                None => menu::sjk::TextTarget::Inter(&mut self.text_vertices, &self.ui_font),
            };
            console.append_sjk_page(target, viewport);
            return;
        }
        // The console and its notify lines draw with the retail console character
        // set when `ui_gameFont` has it; the full-frame browser keeps Inter, or
        // the menus' retail font when it is the classic+ one.
        let classic_browser = self
            .console
            .as_ref()
            .is_some_and(crate::console::ViewerConsole::classic_browser_open);
        let (vertices, font) = if covers_frame && classic_browser {
            self.game_fonts.menu(&mut self.text_vertices, &self.ui_font)
        } else if covers_frame {
            (&mut self.text_vertices, &self.ui_font)
        } else {
            self.game_fonts.target(
                game_font::RetailFont::Console,
                &mut self.text_vertices,
                &self.ui_font,
            )
        };
        if let Some(console) = &mut self.console {
            console.append_overlay(vertices, font, viewport, text_scale);
        }
        if !covers_frame && hud::family::fps(self.console.as_ref()) {
            // CG_DrawFPS draws console characters (CG_DrawBigString). The line is
            // 0.8 times Inter's 38.7-pixel line at 1080 lines, in any font.
            let scale = ui_scale::glyph_scale(font, 31.0, text_scale);
            append_text(
                vertices,
                font,
                self.frame_pacer.label(),
                [(viewport[0] - 780.0).max(8.0), 18.0],
                scale,
                viewport,
            );
        }
    }

    /// Lay out and upload the classic console's own layer
    /// ([`console_backdrop`]), drawn after all other 2D text; empty for the
    /// modern console or under the full-frame browser.
    fn append_classic_console(&mut self, viewport: [f32; 2], covers_frame: bool) {
        let classic = !covers_frame && game_font::grid_console(self.console.as_ref());
        let Some(console) = self.console.as_mut().filter(|_| classic) else {
            self.console_layer.clear();
            return;
        };
        if self.console_layer.needs_loading()
            && let Some(vfs) = &self.vfs
        {
            self.console_layer
                .ensure_loaded(&self.device, &self.queue, vfs, &self.shaders);
        }
        let (font, atlas) = match self.game_fonts.console_font() {
            Some(font) => (font, console_backdrop::TextAtlas::Console),
            None => (&self.ui_font, console_backdrop::TextAtlas::Inter),
        };
        // The SJK UI's console labels in its families once they are loaded.
        let labels = match self.game_fonts.sjk_metrics() {
            Some((display, body)) => console::classic::sjk::Labels {
                display,
                body,
                families: true,
            },
            None => console::classic::sjk::Labels::single(font),
        };
        let in_game =
            self.live_session.is_some() || self.demo_session.is_some() || self.resident.exploring();
        let menu_visible = self
            .client_menu
            .as_ref()
            .is_some_and(|menu| menu.is_visible());
        let env = console::classic::ClassicEnv {
            in_game,
            menu_focus: menu_visible || self.game_menu,
            // Only the game client, whose menu is merely closed, not a map viewer
            // without one.
            full_screen: !in_game && !self.game_menu && self.client_menu.is_some() && !menu_visible,
        };
        let frame = self.console_layer.begin_frame();
        console.append_classic(frame, font, atlas, labels, viewport, env);
        self.console_layer.upload(&self.queue, viewport);
    }
}

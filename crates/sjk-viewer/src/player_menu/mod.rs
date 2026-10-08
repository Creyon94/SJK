//! Modern player screen: a hero form over the live map, with the `model`
//! cvar's character standing on the backdrop's stage. Character and saber
//! changes write their cvars the moment they are made, so the stage model
//! swaps instantly and there is nothing to apply or revert. The Force page
//! is the exception: it edits a draft that only its Apply action writes
//! (see `force`).

mod classic;
mod controller;
mod cosmetics;
mod force;
pub(crate) mod force_icons;
mod force_templates;
mod force_view;
mod grid;
pub(crate) mod icons;
mod model_icons;
mod numeric;
mod part_icons;
mod pointer;
mod rows;
mod saber;
mod saber_view;
mod sjk_view;
mod team_filter;
mod view;

use crate::console::ViewerConsole;
use crate::menu_widgets::MenuCanvas;
pub(crate) use saber::StageSabers;
use sjk_client::{LegacyAssetCatalog, LegacyAssetCatalogLoader};
use sjk_ui::DrawList;
use sjk_vfs::VirtualFileSystem;
use std::sync::Arc;
use team_filter::TeamSkin;

/// Tab labels, in page order.
const PAGE_TABS: [&str; 3] = ["CHARACTER", "SABER", "FORCE"];

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
enum ProfilePage {
    #[default]
    Character,
    Saber,
    Force,
}

impl ProfilePage {
    const ALL: [Self; 3] = [Self::Character, Self::Saber, Self::Force];

    fn index(self) -> usize {
        Self::ALL.iter().position(|page| *page == self).unwrap_or(0)
    }
}

/// Screen to restore when the player selector closes.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum ReturnTarget {
    MainMenu,
    InGame,
}

/// Result of one keyboard or pointer event.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum PlayerMenuResult {
    None,
    Back(ReturnTarget),
    /// Leave for a page of the classic main menu (its navigation row and
    /// Exit lead there).
    ClassicPage(crate::menu::classic::layout::Page),
}

/// Where the profile shows the live model ([`crate::menu_stage::preview`])
/// instead of on the menu map's stage: the classic pages, and the SJK UI's
/// pages opened from a game (where there is no stage). Its area and the
/// animation it plays.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct ModelPreview {
    pub(crate) area: PreviewArea,
    pub(crate) stance: &'static str,
    /// The model holds the saber draft's sabers, lit, in their style's
    /// stance (`stance` is then unused).
    pub(crate) sabers: bool,
    /// Only the sabers are drawn, laid on their side and turning as
    /// retail's lightsaber creation spun its hilt.
    pub(crate) showcase: bool,
    /// How much room the camera leaves round the body: 1 frames it as retail
    /// did, more keeps a raised blade in the picture.
    pub(crate) room: f32,
    /// The camera's angle round the model in degrees, held still; `None`
    /// turns round it as retail's preview did.
    pub(crate) angle: Option<f32>,
}

/// A model preview's area: a rectangle of retail's 640x480 canvas (the
/// classic pages) or of the SJK UI's 16:9 frame of 1080-line pixels.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum PreviewArea {
    Classic([f32; 4]),
    Sjk([f32; 4]),
}

impl PreviewArea {
    /// The area in window pixels for a window of `viewport`.
    pub(crate) fn window_rect(self, viewport: [f32; 2]) -> sjk_ui::Rect {
        match self {
            Self::Classic(rect) => {
                crate::menu::classic::layout::Placement::new(viewport).rect(rect)
            }
            Self::Sjk([x, y, width, height]) => {
                crate::menu::sjk::Frame::new(viewport).rect(x, y, width, height)
            }
        }
    }
}

/// Which catalogue entry the `model` cvar currently names.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Choice {
    Character(usize),
    Species(usize),
}

/// The userinfo values this screen owns (sabers and Force live in their
/// own drafts).
#[derive(Clone, Debug, Eq, PartialEq)]
struct Draft {
    name: String,
    model: String,
    rgb: [u8; 3],
}

impl Default for Draft {
    fn default() -> Self {
        Self {
            name: String::with_capacity(32),
            model: "kyle/default".to_owned(),
            rgb: [255; 3],
        }
    }
}

/// Fixed-storage controller and retained draw list for the player screen.
pub(crate) struct PlayerMenu {
    canvas: MenuCanvas,
    loader: Option<LegacyAssetCatalogLoader>,
    /// Where the model icons come from once the catalogue is known.
    icon_vfs: Option<Arc<VirtualFileSystem>>,
    /// The character grid's icons, loaded as tiles are drawn.
    icons: model_icons::ModelIcons,
    /// The Force page's power icons and side emblems.
    force_icons: icons::IconLoader,
    /// Character creation's part icons and tint base, for the species edited.
    part_icons: part_icons::PartIcons,
    /// Skin set the grid lists (retail's Team Color chooser).
    team: TeamSkin,
    /// Catalogue indices (characters first, then species) the grid shows,
    /// rebuilt when the team, the search or the catalogue changes.
    tiles: Vec<usize>,
    /// Words a listed model's name must contain (SJK's model search).
    search: String,
    /// The search is being typed.
    search_editing: bool,
    /// Catalogue entry of the grid tile under the pointer, for the classic
    /// description line.
    hovered_entry: Option<usize>,
    /// First visible tile row of the model grid.
    grid_scroll: usize,
    /// Largest `grid_scroll` the last frame's layout allowed.
    grid_max_scroll: usize,
    /// Scroll the grid to the current model on the next frame (set when the
    /// choice changes without the pointer, so wheel browsing is not undone).
    grid_follow: bool,
    draft: Draft,
    choice: Option<Choice>,
    /// Selected head, torso, legs and skin colour of the current species.
    variants: [usize; 4],
    /// Keyboard/pointer selection: a row index on the current page.
    selected: usize,
    name_editing: bool,
    numeric: Option<crate::menu_widgets::numeric::NumericEdit>,
    name_before_edit: String,
    return_target: ReturnTarget,
    resolved_catalogue: bool,
    page: ProfilePage,
    saber: saber::SaberMenu,
    force: force::ForceMenu,
    /// The classic Force page's templates (retail's `forcecfg`).
    force_templates: force_templates::TemplateState,
    /// JoF EJK's hats and capes (the classic cosmetics window).
    cosmetics: cosmetics::CosmeticsMenu,
    /// `ui_menuStyle classic`: the retail profile pages instead of the
    /// hero form.
    classic_style: bool,
    /// `ui_menuStyle sjk`: the SJK UI's view of the modern screen (`sjk_view`).
    sjk: bool,
    /// The SJK UI's hilt lists: the first saber's and the second's.
    sjk_hilts: [sjk_view::HiltList; 2],
    /// The Force level last bought and when (menu clock seconds), for the
    /// ring the SJK UI sends out from it.
    sjk_burst: Option<(usize, u8, f64)>,
    classic: classic::ClassicState,
    /// The character draft changed on entering a classic page and is not
    /// written yet.
    classic_dirty: bool,
    /// The model preview has a frame to show (set by the renderer).
    preview_ready: bool,
}

impl PlayerMenu {
    pub(crate) fn new() -> Self {
        Self {
            canvas: MenuCanvas::new(),
            loader: None,
            icon_vfs: None,
            icons: model_icons::ModelIcons::new(),
            force_icons: icons::IconLoader::new(),
            part_icons: part_icons::PartIcons::new(),
            team: TeamSkin::default(),
            tiles: Vec::with_capacity(icons::MAX_ICONS),
            search: String::with_capacity(32),
            search_editing: false,
            hovered_entry: None,
            grid_scroll: 0,
            grid_max_scroll: 0,
            grid_follow: true,
            draft: Draft::default(),
            choice: None,
            variants: [0; 4],
            selected: 0,
            name_editing: false,
            numeric: None,
            name_before_edit: String::with_capacity(32),
            return_target: ReturnTarget::MainMenu,
            resolved_catalogue: false,
            page: ProfilePage::Character,
            saber: saber::SaberMenu::new(),
            force: force::ForceMenu::new(),
            force_templates: force_templates::TemplateState::default(),
            cosmetics: cosmetics::CosmeticsMenu::new(),
            classic_style: false,
            sjk: false,
            sjk_hilts: Default::default(),
            sjk_burst: None,
            classic: classic::ClassicState::default(),
            classic_dirty: false,
            preview_ready: false,
        }
    }

    pub(crate) fn draw_list(&self) -> &DrawList {
        self.canvas.draw_list()
    }

    /// The `model` cvar value currently shown, for the live stage model.
    pub(crate) fn stage_model(&self) -> &str {
        &self.draft.model
    }

    /// The sabers the stage model holds: the saber draft as it stands,
    /// thrown out to the backdrop's saber shot while the screen is `open`
    /// on its Saber tab (the tab stays selected after the screen closes;
    /// the saber must not).
    pub(crate) fn stage_sabers(&self, open: bool) -> StageSabers<'_> {
        self.saber
            .stage_sabers(open && self.page == ProfilePage::Saber)
    }

    /// Where the screen returns when closed.
    pub(crate) fn return_target(&self) -> ReturnTarget {
        self.return_target
    }

    /// Backdrop shot behind the current tab. The classic pages cover the
    /// screen with retail art, so the backdrop camera stays where it is.
    pub(crate) fn shot(&self) -> crate::menu_backdrop::Shot {
        if self.classic_style {
            return crate::menu_backdrop::Shot::Main;
        }
        match self.page {
            ProfilePage::Saber => crate::menu_backdrop::Shot::Saber,
            _ => crate::menu_backdrop::Shot::Player,
        }
    }

    /// Selected row, for the shared highlight shader.
    pub(crate) fn visual_selection(&self) -> usize {
        self.selected
    }

    /// Move a few decoded model and Force icons into the UI atlas, and ask
    /// for the model icons this frame's tiles lacked.
    pub(crate) fn upload_icons(
        &mut self,
        renderer: &crate::ui_renderer::ShapeRenderer,
        queue: &crate::frame_queue::FrameQueue,
    ) {
        self.icons.end_frame(renderer, queue, 32);
        self.force_icons.upload_batch(renderer, queue, 32);
        self.part_icons.upload(renderer, queue);
    }

    /// Start decoding the Force icons once the VFS is known, and serve the
    /// model icons once the catalogue is too.
    fn request_icons_if_ready(&mut self) {
        if let Some(vfs) = self
            .icon_vfs
            .as_ref()
            .filter(|_| self.force_icons.is_idle())
        {
            self.force_icons
                .request_paths(Arc::clone(vfs), force_icons::requests());
        }
        let catalog = self
            .loader
            .as_ref()
            .and_then(LegacyAssetCatalogLoader::catalog);
        if let (Some(vfs), Some(catalog)) = (&self.icon_vfs, catalog) {
            self.icons.attach(vfs, catalog);
        }
    }

    fn catalog(&self) -> Option<&LegacyAssetCatalog> {
        catalog_of(&self.loader)
    }
}

/// Borrow the loaded catalogue independently of the mutable menu canvas.
fn catalog_of(loader: &Option<LegacyAssetCatalogLoader>) -> Option<&LegacyAssetCatalog> {
    loader
        .as_ref()
        .and_then(LegacyAssetCatalogLoader::catalog)
        .map(Arc::as_ref)
}

#[cfg(test)]
impl PlayerMenu {
    /// Show page `index` (Character, Saber, Force) with row `row` selected
    /// (world shots).
    pub(crate) fn show_page_for_shot(&mut self, index: usize, row: usize) {
        self.set_page(ProfilePage::ALL[index.min(2)]);
        self.selected = row;
    }

    /// Move the pointer onto level `level` of power `power` as the last frame
    /// laid it out, for the world shots.
    pub(crate) fn hover_level_for_shot(&mut self, power: usize, level: u8) {
        if let Some(rect) = self.canvas.rect_for(sjk_view::level_token(power, level)) {
            let _ = self
                .canvas
                .pointer(sjk_ui::InputEvent::PointerMove(sjk_ui::Vec2::new(
                    rect.x + rect.width * 0.5,
                    rect.y + rect.height * 0.5,
                )));
        }
    }

    /// Make the draft Dual (not written to the profile), for the world shots.
    pub(crate) fn dual_for_shot(&mut self) {
        self.saber
            .set_style(saber::SaberStyle::Dual, catalog_of(&self.loader));
    }

    /// Open the classic lightsaber creation page on `vfs`'s catalogue as
    /// `console` sets it up, for the menu snapshots: the catalogue is waited
    /// for (a minute at most).
    pub(crate) fn open_classic_saber_for_snapshot(
        &mut self,
        console: &ViewerConsole,
        vfs: Arc<sjk_vfs::VirtualFileSystem>,
        art: crate::menu::art::ArtSet,
        in_game: bool,
    ) {
        self.attach_catalogue(vfs);
        self.set_style(true, art);
        let target = if in_game {
            ReturnTarget::InGame
        } else {
            ReturnTarget::MainMenu
        };
        self.open(console, target);
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(60);
        while !self.is_resolved() && std::time::Instant::now() < deadline {
            self.poll();
            std::thread::sleep(std::time::Duration::from_millis(20));
        }
        self.show_classic(classic::ClassicPage::Saber);
    }
}

impl crate::GpuState {
    pub(crate) fn open_player_menu_from_game(&mut self) {
        if let (Some(menu), Some(console)) = (&mut self.client_menu, &self.console) {
            menu.open_player(console, ReturnTarget::InGame);
            self.game_menu = false;
        }
    }
}

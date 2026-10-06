//! Modern data-driven in-game HUD over authoritative snapshot values.

mod data_source;
use data_source::*;
pub(crate) mod enemy_info;
pub(crate) mod family;
mod force_estimate;
pub(crate) mod force_wheel;
pub(crate) mod icons;
pub(crate) mod identification;
mod info;
pub(crate) mod movement;
pub(crate) mod nameplate;
mod nameplate_math;
mod npc_class;
pub(crate) mod options;
pub(crate) mod player_card;
pub(crate) mod portrait;
mod radial;
mod selection;
pub(crate) mod targeting;
mod text_values;
pub(crate) mod tints;
mod update;
mod vote;
mod widgets;

use super::{Localization, TextVertex, UiFont};
use crate::game_font::RetailFont;
use crate::{console::ViewerConsole, ui_renderer};
use sjk_client::pmove::MovementState;
use sjk_client::{
    ClientSession, HudDataSource as ClientHudData, TeamInfo, legacy_hud_data,
    legacy_predicted_hud_data, legacy_team_location,
};
use sjk_protocol::{GameState, PlayerState};
use sjk_ui::{
    DrawList, Easing, HudDataSource, HudLayoutDocument, Insets, LayoutContext, LayoutEngine,
    LayoutKind, LayoutScratch, TextId, Theme, Tween, Vec2, Widget, WidgetId, WidgetTree,
};
use std::path::Path;

const WIDGET_LIMIT: usize = 64;
const DRAW_LIMIT: usize = 320;
const DEFAULT_LAYOUT: &str = include_str!("../assets/hud/default.json");
/// The weapon name stays fully visible this long after a switch, then fades.
const WEAPON_HOLD_MS: u64 = 2_000;
const WEAPON_FADE_MS: u64 = 600;
/// The newest obituary stays this long at the top left, then fades.
const KILL_HOLD_MS: u64 = 2_500;
const KILL_FADE_MS: u64 = 700;

/// Opacity of a transient that holds for `hold` ms and fades over `fade` ms.
fn transient_alpha(age_ms: u64, hold: u64, fade: u64) -> f32 {
    if age_ms <= hold {
        1.0
    } else {
        1.0 - (age_ms - hold).min(fade) as f32 / fade as f32
    }
}
const CLASSIC_LAYOUT: &str = include_str!("../assets/hud/classic.json");
const RADIAL_LAYOUT: &str = include_str!("../assets/hud/radial.json");

/// Independent visibility switches used by widget predicates and the HUD shader.
#[derive(Clone, Copy)]
pub(crate) struct HudVisibility {
    pub(crate) hud: bool,
    pub(crate) status: bool,
    pub(crate) weapon: bool,
    pub(crate) crosshair: bool,
    pub(crate) crosshair_names: bool,
    pub(crate) timer: bool,
    pub(crate) lagometer: bool,
    /// Team-status visibility, independent of the other HUD widgets.
    pub(crate) team_overlay: bool,
    /// The third-person ground HUD stands in for health, shield, Force and
    /// stance this frame (`crate::ground_hud`), so those widgets hide.
    pub(crate) ground_hud: bool,
    /// The game-data menu HUD (`crate::menu_hud`) draws health, armor,
    /// Force and ammo this frame, so JKR's status and weapon widgets hide.
    pub(crate) menu_hud: bool,
}

impl HudVisibility {
    pub(crate) fn from_console(console: Option<&ViewerConsole>) -> Self {
        let enabled = |name, fallback| {
            console
                .and_then(|console| console.bool_cvar(name))
                .unwrap_or(fallback)
        };
        let hud = enabled("cg_drawHud", true) && enabled("cg_draw2D", true);
        Self {
            hud,
            status: hud && enabled("cg_drawStatus", true),
            weapon: hud && enabled("cg_drawWeapon", true),
            crosshair: hud && enabled("cg_crosshair", true),
            crosshair_names: hud && enabled("cg_drawCrosshairNames", true),
            timer: hud && enabled("cg_drawTimer", false),
            lagometer: hud && enabled("cg_lagometer", false),
            team_overlay: hud
                && console
                    .and_then(|c| c.integer_cvar("cg_drawteamoverlay"))
                    .unwrap_or(0)
                    > 0,
            ground_hud: false,
            menu_hud: false,
        }
    }

    /// Intermission hides every HUD layer.
    pub(crate) const HIDDEN: Self = Self {
        hud: false,
        status: false,
        weapon: false,
        crosshair: false,
        crosshair_names: false,
        timer: false,
        lagometer: false,
        team_overlay: false,
        ground_hud: false,
        menu_hud: false,
    };
}

/// Allocation-free-per-frame widget state for the local player's HUD.
pub(crate) struct HudOverlay {
    /// Map-lifetime textures and sampled icon presentation policy.
    pub(crate) icons: icons::Icons,
    pub(crate) tints: tints::State,
    /// World-projected labels, sharing the chat roster's retained names.
    pub(crate) identification: identification::State,
    /// The card shown beside a player looked at for a moment.
    pub(crate) card: player_card::State,
    /// MMO-style nameplates, drawn in the classic font; they replace the labels above.
    pub(crate) nameplate: nameplate::State,
    guides: movement::Guides,
    family: family::Policy,
    pub(crate) targeting: targeting::State,
    pub(crate) enemy_info: enemy_info::State,
    score_text: String,
    snapshot_text: String,

    inventory_bits: u32,
    selector: Option<sjk_client::selection::SelectionView>,
    /// The Force selector is the retail icon bar ([`force_wheel`]), not the list.
    force_wheel_bar: bool,
    /// The bar's map-lifetime pictures.
    force_wheel_icons: [Option<sjk_ui::TextureId>; force_wheel::ICONS],
    /// JA+ merc mode's flamethrower, latched across snapshots.
    flamethrower: sjk_client::force_wheel::FlamethrowerOverride,
    /// The selector shows Force Lightning as the flamethrower.
    flamethrower_shown: bool,

    values: Option<ClientHudData>,
    health: String,
    armor: String,
    force: String,
    weapon: String,
    ammo: String,
    health_value: String,
    armor_value: String,
    force_value: String,
    weapon_value: String,
    ammo_value: String,
    style_value: String,
    /// HUD time of the last weapon change; drives the weapon-name fade.
    weapon_shown_ms: Option<u64>,
    /// Retail's weapon selection row (`crate::weapon_select`) and its names.
    pub(crate) weapon_select: crate::weapon_select::State,
    weapon_alpha: f32,
    team_revision: u64,
    team_side: u8,
    team_rows: [String; 8],
    team_names: [String; 8],
    team_locations: [String; 8],
    team_stats: [String; 8],
    team_gear: [String; 8],
    team_len: usize,
    vote_active: bool,
    team_vote_active: bool,
    vote_heading: String,
    vote_text: String,
    team_vote_heading: String,
    team_vote_text: String,
    vote_keys: String,
    yes_keys: String,
    no_keys: String,
    kill_rows: [String; 8],
    kill_len: usize,
    kill_alpha: f32,
    crosshair_name: String,
    speed: options::Speed,
    crosshair_alpha: f32,
    crosshair_teammate: bool,
    match_timer: String,
    warmup_text: String,
    interrupted: bool,
    lagometer: sjk_client::LagometerSamples,
    default_document: HudLayoutDocument,
    classic_document: HudLayoutDocument,
    override_document: Option<HudLayoutDocument>,
    tree: WidgetTree,
    scratch: LayoutScratch,
    draw_list: DrawList,
    theme: Theme,
    ratios: [Tween; 3],
    displayed_ratios: [f32; 3],
    ammo_ratio: Tween,
    displayed_ammo: f32,
    radial_document: HudLayoutDocument,
}

/// Which bundled layout document the status HUD is drawn from.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum HudLook {
    /// SJK's modern layout (the classic one without the modern font).
    Modern,
    /// SJK's classic layout.
    Classic,
    /// SJK's radial layout: arcs around the crosshair.
    Radial,
}

/// Pixel-space geometry consumed by the existing fullscreen HUD renderer.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(crate) struct HudLayout {
    pub(crate) health_bar: [f32; 4],
    pub(crate) armor_bar: [f32; 4],
    pub(crate) force_bar: [f32; 4],
}

impl HudOverlay {
    /// The Force power pictures, by `forcePowers_t` index, which the nameplates reuse.
    pub(crate) fn power_icons(&self) -> [Option<sjk_ui::TextureId>; force_wheel::ICONS] {
        self.force_wheel_icons
    }

    pub(crate) fn new() -> Self {
        let override_document = crate::platform::user_config_file()
            .ok()
            .and_then(|path| path.parent().map(|parent| parent.join("hud.json")))
            .and_then(|path| load_override(&path));
        Self {
            icons: icons::Icons::default(),
            weapon_select: crate::weapon_select::State::new(),
            tints: tints::State::default(),
            guides: movement::Guides::default(),
            family: family::Policy::default(),
            targeting: targeting::State::default(),
            enemy_info: enemy_info::State::default(),
            identification: identification::State::default(),
            card: player_card::State::default(),
            nameplate: nameplate::State::default(),
            score_text: String::with_capacity(80),
            snapshot_text: String::with_capacity(96),

            inventory_bits: 0,
            values: None,
            health: String::with_capacity(24),
            armor: String::with_capacity(24),
            force: String::with_capacity(24),
            weapon: String::with_capacity(40),
            ammo: String::with_capacity(24),
            health_value: String::with_capacity(12),
            armor_value: String::with_capacity(12),
            force_value: String::with_capacity(12),
            weapon_value: String::with_capacity(32),
            ammo_value: String::with_capacity(12),
            style_value: String::with_capacity(12),
            weapon_shown_ms: None,
            weapon_alpha: 0.0,
            selector: None,
            force_wheel_bar: false,
            force_wheel_icons: [None; force_wheel::ICONS],
            flamethrower: Default::default(),
            flamethrower_shown: false,

            team_revision: 0,
            team_side: 0,
            team_rows: std::array::from_fn(|_| String::with_capacity(96)),
            team_names: std::array::from_fn(|_| String::with_capacity(32)),
            team_locations: std::array::from_fn(|_| String::with_capacity(32)),
            team_stats: std::array::from_fn(|_| String::with_capacity(16)),
            team_gear: std::array::from_fn(|_| String::with_capacity(256)),
            team_len: 0,
            vote_active: false,
            team_vote_active: false,
            vote_heading: String::with_capacity(64),
            vote_text: String::with_capacity(160),
            team_vote_heading: String::with_capacity(64),
            team_vote_text: String::with_capacity(160),
            vote_keys: String::with_capacity(96),
            yes_keys: String::with_capacity(48),
            no_keys: String::with_capacity(48),
            kill_rows: std::array::from_fn(|_| String::with_capacity(128)),
            kill_len: 0,
            kill_alpha: 0.0,
            crosshair_name: String::with_capacity(64),
            speed: options::Speed::default(),
            crosshair_alpha: 0.0,
            crosshair_teammate: false,
            match_timer: String::with_capacity(16),
            warmup_text: String::with_capacity(48),
            interrupted: false,
            lagometer: sjk_client::LagometerSamples::new(),
            default_document: HudLayoutDocument::from_json(DEFAULT_LAYOUT)
                .expect("bundled modern HUD document is valid"),
            classic_document: HudLayoutDocument::from_json(CLASSIC_LAYOUT)
                .expect("bundled classic HUD document is valid"),
            override_document,
            tree: WidgetTree::new(WIDGET_LIMIT),
            scratch: LayoutScratch::new(WIDGET_LIMIT),
            draw_list: DrawList::new(DRAW_LIMIT),
            theme: Theme::default(),
            ratios: [
                Tween::settled(1.0),
                Tween::settled(0.0),
                Tween::settled(1.0),
            ],
            displayed_ratios: [1.0, 0.0, 1.0],
            ammo_ratio: Tween::settled(0.0),
            displayed_ammo: 0.0,
            radial_document: HudLayoutDocument::from_json(RADIAL_LAYOUT)
                .expect("bundled radial HUD document is valid"),
        }
    }

    /// Build, lay out and retain the HUD widget draw list. `user_scale` is
    /// the player's `cg_hudScale` on top of the resolution-derived scale.
    pub(crate) fn layout(
        &mut self,
        font: &UiFont,
        look: HudLook,
        viewport: [f32; 2],
        user_scale: f32,
        visibility: HudVisibility,
        time_ms: u64,
    ) -> HudLayout {
        self.displayed_ratios = self.ratios.map(|tween| tween.sample(time_ms));
        self.displayed_ammo = self.ammo_ratio.sample(time_ms);
        self.weapon_alpha = self.weapon_shown_ms.map_or(0.0, |shown| {
            transient_alpha(
                time_ms.saturating_sub(shown),
                WEAPON_HOLD_MS,
                WEAPON_FADE_MS,
            )
        });
        let document = match look {
            HudLook::Radial => &self.radial_document,
            HudLook::Classic => &self.classic_document,
            HudLook::Modern if font.is_modern() => self
                .override_document
                .as_ref()
                .unwrap_or(&self.default_document),
            HudLook::Modern => &self.classic_document,
        };
        let data = WidgetData {
            visibility,
            ratios: self.displayed_ratios,
            ammo_ratio: self.displayed_ammo,
            health: &self.health,
            armor: &self.armor,
            force: &self.force,
            weapon: &self.weapon,
            ammo: &self.ammo,
            health_value: &self.health_value,
            armor_value: &self.armor_value,
            force_value: &self.force_value,
            weapon_value: &self.weapon_value,
            ammo_value: &self.ammo_value,
            style: !self.style_value.is_empty(),
            weapon_alpha: self.weapon_alpha,
            team_len: self.team_len,
            vote_active: self.vote_active,
            team_vote_active: self.team_vote_active,
            kill_len: self.kill_len,
            crosshair_name: !self.crosshair_name.is_empty(),
            timer: !self.match_timer.is_empty(),
            warmup: !self.warmup_text.is_empty(),
            interrupted: self.interrupted,
        };
        let mut visible = [false; WIDGET_LIMIT];
        for (index, widget) in document.widgets.iter().take(WIDGET_LIMIT).enumerate() {
            visible[index] = visibility.hud
                && widget.visibility.evaluate(&data)
                && self.family.visible(widget.binding.as_deref());
        }
        self.tree.clear();
        for (index, widget) in document.widgets.iter().take(WIDGET_LIMIT).enumerate() {
            let _ = self.tree.add(Widget {
                id: WidgetId(index as u32),
                parent: None,
                layout: LayoutKind::Anchored {
                    anchor: widget.anchor,
                    offset: widget.offset,
                },
                size: widget.size,
                visible: visible[index],
                focusable: false,
                scrollable: false,
                opacity: 1.0,
                z: widget.layer,
            });
        }
        let dpi_scale =
            crate::ui_scale::height_scale(viewport[1]).max(2.0 / 3.0) * user_scale.clamp(0.25, 2.0);
        let upper_right_bottom =
            self.upper_right_stack()[2] * crate::ui_scale::height_scale(viewport[1]);
        let rectangles = LayoutEngine.layout(
            &self.tree,
            LayoutContext {
                viewport_physical: Vec2::new(viewport[0], viewport[1]),
                dpi_scale,
                safe_area: Insets::all(0.0),
            },
            &mut self.scratch,
        );
        self.draw_list.clear();
        self.tints.emit(&mut self.draw_list, viewport);
        let mut output = HudLayout::default();
        let low_health = self.values.is_some_and(|value| value.health <= 25);
        let low_ammo = self
            .values
            .is_some_and(|value| value.ammo.is_some_and(|ammo| ammo <= 5));
        let pulse = Tween::pulse(0.68, 1.0, time_ms, self.theme.motion.slow);
        for (index, (widget, rect)) in document.widgets.iter().zip(rectangles).enumerate() {
            if !visible[index] {
                continue;
            }
            widgets::emit(
                &mut self.draw_list,
                self.theme,
                widget,
                if widget.binding.as_deref() == Some("team_rows") {
                    let mut rect = self.family.team_rect(*rect, viewport);
                    if self.family.team[1] == 0.0 {
                        rect.y = rect.y.max(upper_right_bottom);
                    }
                    rect
                } else {
                    *rect
                },
                &widgets::EmitContext {
                    data: &data,
                    family: self.family,
                    targeting: self.targeting.policy,
                    icons: &self.icons,
                    viewport,
                    dpi_scale,
                    hero_scale: crate::ui_scale::height_scale(viewport[1])
                        * user_scale.clamp(0.25, 2.0),
                    low_health,
                    low_ammo,
                    pulse,
                    team_side: self.team_side,
                    team_len: self.team_len,
                    kill_len: self.kill_len,
                    kill_alpha: self.kill_alpha,
                    crosshair_alpha: self.crosshair_alpha,
                    crosshair_teammate: self.crosshair_teammate,
                    lagometer: &self.lagometer,
                },
                &mut output,
            );
        }
        if visibility.hud {
            self.speed.emit(&mut self.draw_list, self.theme, viewport);
            let selector = self
                .selector
                .filter(|s| !s.inventory || self.family.inventory);
            match selector {
                Some(view) if !view.inventory && self.force_wheel_bar => force_wheel::emit(
                    &mut self.draw_list,
                    view,
                    &self.force_wheel_icons,
                    self.flamethrower_shown,
                    viewport,
                    user_scale,
                ),
                view => selection::emit(
                    &mut self.draw_list,
                    view,
                    self.flamethrower_shown,
                    self.theme,
                    viewport,
                    user_scale,
                ),
            }
            self.emit_family(viewport);
            self.icons
                .emit(&mut self.draw_list, viewport, visibility, self.family.upper);
            self.guides.emit(&mut self.draw_list, viewport);
        }
        // CG_DrawWeaponSelect does not follow cg_drawHud: only the row's own
        // conditions (`GpuState::sample_weapon_select`) decide.
        if let Some(shown) = &self.weapon_select.shown {
            crate::weapon_select::emit_icons(&mut self.draw_list, &self.icons, shown, viewport);
        }
        output
    }

    /// Settle the status values and meters on a fixed state, for the layout snapshots.
    #[cfg(test)]
    pub(crate) fn preview_values(&mut self, values: ClientHudData, ratios: [f32; 4]) {
        self.format_values(values);
        self.ratios = [ratios[0], ratios[1], ratios[2]].map(Tween::settled);
        self.ammo_ratio = Tween::settled(ratios[3]);
    }

    pub(crate) fn displayed_ratios(&self) -> [f32; 3] {
        self.displayed_ratios
    }

    /// Append HUD text: what retail drew with a game font goes to that font when
    /// `ui_gameFont` has it loaded ([`text_values::retail_font`]), the rest to
    /// `vertices` with `font`.
    pub(crate) fn append(
        &self,
        fonts: &mut crate::game_font::GameFonts,
        vertices: &mut Vec<TextVertex>,
        font: &UiFont,
        viewport: [f32; 2],
    ) {
        fonts.append_routed(
            &self.draw_list,
            |id| self.resolve_text(id),
            |id, _| text_values::retail_font(id),
            (vertices, font),
            viewport,
            crate::text::TextStyle::NEUTRAL,
        );
    }

    pub(crate) fn draw_list(&self) -> &DrawList {
        &self.draw_list
    }
}

#[cfg(test)]
mod tests {
    use super::{CLASSIC_LAYOUT, DEFAULT_LAYOUT};
    use sjk_ui::{Anchor, Dimension, HudLayoutDocument};

    /// Both layouts put the crosshair name's line top where stock's is:
    /// `CG_DrawCrosshairNames` draws it at y = 170 in the 480-line virtual
    /// screen, 157.5 px above the centre in the HUD's 1080-line frame.
    #[test]
    fn crosshair_name_line_top_matches_stock() {
        for layout in [DEFAULT_LAYOUT, CLASSIC_LAYOUT] {
            let document = HudLayoutDocument::from_json(layout).unwrap();
            let widget = document
                .widgets
                .iter()
                .find(|widget| widget.id == "crosshair_name")
                .unwrap();
            assert_eq!(widget.anchor, Anchor::Center);
            let Dimension::Px(height) = widget.size.height else {
                panic!("crosshair name height is not in px");
            };
            let top = 540.0 - height * 0.5 + widget.offset.y;
            assert!((top - 170.0 * 1_080.0 / 480.0).abs() < 1e-3, "{top}");
        }
    }
}

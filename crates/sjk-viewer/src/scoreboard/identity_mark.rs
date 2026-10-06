//! The mark after a player's name on the scoreboard when the SJK hub knows them
//! (`player_identity.rs`): SJK's emblem, in gold for a player the hub's operator
//! vouches for and in soft white for any other registered player.

use crate::player_identity::Tag;
use sjk_ui::{Color, DrawCommand, Rect};

const VERIFIED: Color = Color::new(1.0, 0.82, 0.25, 1.0);
const REGISTERED: Color = Color::new(0.92, 0.94, 1.0, 0.9);

/// The emblem's tint.
pub(super) fn tint(tag: Tag) -> Color {
    if tag.verified { VERIFIED } else { REGISTERED }
}

/// The emblem as a quad in the right end of `name` (a row's name rectangle),
/// `side` wide, centred vertically.
pub(super) fn logo(name: Rect, side: f32, tag: Tag) -> DrawCommand {
    DrawCommand::TexturedQuad {
        rect: Rect::new(
            name.x + name.width - side,
            name.y + (name.height - side) * 0.5,
            side,
            side,
        ),
        texture: crate::ui_renderer::LOGO_TEXTURE,
        color: tint(tag),
    }
}

/// `name` without the room the emblem takes.
pub(super) fn narrowed(name: Rect, side: f32) -> Rect {
    Rect::new(
        name.x,
        name.y,
        (name.width - side * 1.25).max(0.0),
        name.height,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn verified_players_are_tinted_apart_from_registered_ones() {
        assert_ne!(tint(Tag { verified: true }), tint(Tag { verified: false }));
    }

    #[test]
    fn the_emblem_sits_at_the_right_end_and_the_name_leaves_room() {
        let name = Rect::new(10.0, 20.0, 200.0, 30.0);
        let DrawCommand::TexturedQuad { rect, .. } = logo(name, 20.0, Tag { verified: false })
        else {
            panic!("a textured quad");
        };
        assert_eq!(
            (rect.x + rect.width, rect.y + rect.height * 0.5),
            (210.0, 35.0)
        );
        let text = narrowed(name, 20.0);
        assert!(
            text.x + text.width < rect.x,
            "the name ends before the emblem"
        );
    }
}

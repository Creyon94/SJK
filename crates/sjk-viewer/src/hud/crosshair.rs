//! The classic crosshair: retail's `gfx/2d/crosshaira`..`j` pictures, drawn as
//! EternalJK's `CG_DrawCrosshair` (`cg_draw.c`) draws them. `cg_drawCrosshair`
//! picks the picture (`% NUM_CROSSHAIRS`, 0 hides it), `cg_crosshairSize` sets its
//! size in the 640x480 virtual screen, and the picture is tinted with the target
//! colour [`super::targeting`] already resolves. When the picture is missing the
//! procedural cross in `hud.wgsl` stays as the fallback.

use super::icons::assets::decode;
use sjk_shader::ShaderCatalog;
use sjk_ui::{Color, DrawCommand, DrawList, Rect, TextureId};
use sjk_vfs::VirtualFileSystem;

/// `NUM_CROSSHAIRS` (`cg_local.h`).
pub(crate) const PICTURES: usize = 10;
const _: () = assert!(PICTURES as u32 <= crate::ui_renderer::CROSSHAIR_ICON_CELLS);

/// The image slot `n` draws. EternalJK swaps `crosshaira` and `crosshairj` with
/// `R_RemapShader` after registering them (`cg_main.c` `CG_RegisterGraphics`), so
/// slot 0 shows `j`'s picture and slot 9 shows `a`'s.
fn picture_name(slot: usize) -> String {
    let letter = match slot {
        0 => 'j',
        9 => 'a',
        _ => char::from(b'a' + slot as u8),
    };
    format!("gfx/2d/crosshair{letter}")
}

/// Upload the pictures once per installed world; absent ones stay `None`.
pub(crate) fn load(
    vfs: &VirtualFileSystem,
    shaders: &ShaderCatalog,
    mut upload: impl FnMut(TextureId, &[u8]),
) -> [Option<TextureId>; PICTURES] {
    std::array::from_fn(|slot| {
        let pixels = decode(vfs, shaders, &picture_name(slot))?;
        let id = TextureId(crate::ui_renderer::CROSSHAIR_ICON_FIRST + slot as u32);
        upload(id, pixels.as_raw());
        Some(id)
    })
}

/// What one frame's crosshair needs, sampled from the cvars by the targeting policy.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Look {
    /// `cg_drawCrosshair`; 0 hides the crosshair.
    pub(crate) shape: i64,
    /// `cg_crosshairSize` in virtual units.
    pub(crate) size: f32,
    /// `cg_crosshairSizeScale`: follow the screen height instead of pixels.
    pub(crate) scaled: bool,
}

impl Look {
    /// The picture slot, as `cg_drawCrosshair.integer % NUM_CROSSHAIRS`.
    fn slot(self) -> usize {
        self.shape.rem_euclid(PICTURES as i64) as usize
    }
}

/// The crosshair's rectangle in pixels. `center` is where its middle goes, as a
/// fraction of the viewport from the top left. With `cg_crosshairSizeScale` the
/// side is `size` virtual units of the 480-line screen, kept square on wide
/// screens as `widthRatioCoef` keeps it; without it, or for picture 10, it is
/// `size` pixels.
pub(crate) fn rect(look: Look, center: [f32; 2], viewport: [f32; 2]) -> Rect {
    let side = if look.scaled && look.shape != 10 {
        look.size * viewport[1] / 480.0
    } else {
        look.size
    };
    Rect::new(
        viewport[0] * center[0] - side * 0.5,
        viewport[1] * center[1] - side * 0.5,
        side,
        side,
    )
}

/// Draw the crosshair; false when its picture is missing, so the caller keeps
/// the procedural one.
pub(crate) fn emit(
    list: &mut DrawList,
    pictures: &[Option<TextureId>; PICTURES],
    look: Look,
    center: [f32; 2],
    color: [f32; 4],
    viewport: [f32; 2],
) -> bool {
    let Some(texture) = pictures[look.slot()] else {
        return false;
    };
    if look.shape == 0 || look.size <= 0.0 {
        return true;
    }
    // Picture 10 is EternalJK's pixel-sized white dot, whatever the target.
    let [r, g, b, a] = if look.shape == 10 { [1.0; 4] } else { color };
    let _ = list.push(DrawCommand::TexturedQuad {
        rect: rect(look, center, viewport),
        texture,
        color: Color::new(r, g, b, a),
    });
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slots_follow_the_eternaljk_remap() {
        assert_eq!(picture_name(0), "gfx/2d/crosshairj");
        assert_eq!(picture_name(1), "gfx/2d/crosshairb");
        assert_eq!(picture_name(8), "gfx/2d/crosshairi");
        assert_eq!(picture_name(9), "gfx/2d/crosshaira");
        let look = |shape| Look {
            shape,
            size: 24.0,
            scaled: true,
        };
        assert_eq!(look(1).slot(), 1);
        assert_eq!(look(10).slot(), 0);
        assert_eq!(look(-1).slot(), 9);
    }

    #[test]
    fn size_follows_screen_height_and_stays_square() {
        let look = Look {
            shape: 1,
            size: 24.0,
            scaled: true,
        };
        // 24 of 480 lines on a 1080-line wide screen: 54 pixels each way.
        let r = rect(look, [0.5, 0.5], [1920.0, 1080.0]);
        assert_eq!((r.width, r.height), (54.0, 54.0));
        assert_eq!((r.x, r.y), (933.0, 513.0));
        // Unscaled, and picture 10, are plain pixels.
        let pixels = rect(
            Look {
                scaled: false,
                ..look
            },
            [0.5, 0.5],
            [1920.0, 1080.0],
        );
        assert_eq!(pixels.width, 24.0);
        let dot = rect(Look { shape: 10, ..look }, [0.5, 0.5], [1920.0, 1080.0]);
        assert_eq!(dot.width, 24.0);
    }

    #[test]
    fn missing_picture_keeps_the_fallback() {
        let mut list = DrawList::new(4);
        let look = Look {
            shape: 1,
            size: 24.0,
            scaled: true,
        };
        let none = [None; PICTURES];
        assert!(!emit(
            &mut list,
            &none,
            look,
            [0.5, 0.5],
            [1.0; 4],
            [640.0, 480.0]
        ));
        let mut some = none;
        some[1] = Some(TextureId(7));
        assert!(emit(
            &mut list,
            &some,
            look,
            [0.5, 0.5],
            [1.0; 4],
            [640.0, 480.0]
        ));
        assert_eq!(list.commands().len(), 1);
    }
}

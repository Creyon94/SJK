//! GPU work that may run out of video memory without ending the game.
//!
//! wgpu reports an error that no error scope captures to the device's default
//! handler, which panics ("wgpu error: Out of Memory"). Work done after a world is
//! built, on the render thread (the menu stage's model, a player's textures), runs
//! through [`capture`] instead, so a card short of memory loses that one model and
//! the game goes on. What the failed work made is invalid and must be dropped, not
//! kept in a cache: a later draw that binds it fails validation.

/// Run `create` with wgpu's out-of-memory and validation errors captured: the
/// first one wgpu reported instead of the value.
pub(crate) fn capture<T>(
    device: &wgpu::Device,
    create: impl FnOnce() -> T,
) -> Result<T, wgpu::Error> {
    let memory = device.push_error_scope(wgpu::ErrorFilter::OutOfMemory);
    let validation = device.push_error_scope(wgpu::ErrorFilter::Validation);
    let value = create();
    let invalid = pollster::block_on(validation.pop());
    let exhausted = pollster::block_on(memory.pop());
    match invalid.or(exhausted) {
        Some(error) => Err(error),
        None => Ok(value),
    }
}

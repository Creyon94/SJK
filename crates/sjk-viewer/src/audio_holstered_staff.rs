//! Bounded EJK #267 ignition/shutdown synchronization. No decoding in frame paths.
use super::*;
use crate::holstered_sabers::staff::Phase;

const CLIENTS: usize = 64;
const WAIT: i64 = 150;
const HOLD: i64 = 2000;

#[derive(Clone, Copy, Default)]
struct Slot {
    identity: u64,
    sounds: [[Option<SoundHandle>; 3]; 2],
    enabled: bool,
    origin: [f32; 3],
    phase: Phase,
    lit: bool,
    unholster_time: Option<i64>,
    pending: [Option<(SoundHandle, PlayRequest, i64, bool)>; 3],
    pending_off: [Option<(SoundHandle, PlayRequest)>; 3],
    was_active: bool,
    off_key: Option<(usize, u64, i64)>,
    off_time: Option<i64>,
    seen: bool,
}

impl Slot {
    fn release_ignitions(
        &mut self,
        client: usize,
        origin: [f32; 3],
        phase: Phase,
        now: i64,
    ) -> [Option<(SoundHandle, PlayRequest)>; 3] {
        std::array::from_fn(|layer| {
            let (handle, mut request, started, mut saw_back) = self.pending[layer]?;
            saw_back |= phase == Phase::OnBack;
            if now - started >= HOLD
                || (phase != Phase::OnBack && (saw_back || now - started >= WAIT))
            {
                request.origin = Some(origin);
                request.source = SourceId(client as u32);
                self.pending[layer] = None;
                Some((handle, request))
            } else {
                self.pending[layer] = Some((handle, request, started, saw_back));
                None
            }
        })
    }
}

pub(super) struct Sync {
    slots: [Slot; CLIENTS],
    time: i64,
    pub(super) enabled: bool,
}
impl Default for Sync {
    fn default() -> Self {
        Self {
            slots: [Slot::default(); CLIENTS],
            time: 0,
            enabled: true,
        }
    }
}
impl Sync {
    /// A new snapshot records the instant the server says the staff lit up.
    pub(super) fn observe(&mut self, snapshot: &Snapshot) {
        let now = i64::from(snapshot.server_time);
        if now < self.time {
            self.slots.fill(Slot::default());
        }
        self.time = now;
        for state in &snapshot.entities {
            let Some(slot) = self.slots.get_mut(usize::from(state.number())) else {
                continue;
            };
            let lit = state.weapon() == 3 && state.saber_holstered() == 0;
            if lit && !slot.lit {
                slot.unholster_time = Some(now);
            }
            slot.lit = lit;
        }
        if let Some(slot) = self
            .slots
            .get_mut(usize::from(snapshot.player.client_num()))
        {
            let lit = snapshot.player.weapon() == 3 && snapshot.player.saber_holstered() == 0;
            if lit && !slot.lit {
                slot.unholster_time = Some(now);
            }
            slot.lit = lit;
        }
    }

    pub(super) fn intercept(
        &mut self,
        handle: SoundHandle,
        request: PlayRequest,
        now: i64,
    ) -> bool {
        if !self.enabled {
            return false;
        }
        for (index, slot) in self.slots.iter_mut().enumerate() {
            if !slot.enabled {
                continue;
            }
            let from_client = request.source.0 == index as u32;
            let near = request.origin.is_some_and(|origin| {
                glam::Vec3::from_array(origin).distance_squared(glam::Vec3::from_array(slot.origin))
                    <= 64.0 * 64.0
            });
            if slot.sounds[1].contains(&Some(handle)) && from_client {
                if !slot
                    .off_time
                    .is_some_and(|time| (0..HOLD).contains(&(now - time)))
                {
                    let layer = slot.sounds[1]
                        .iter()
                        .position(|h| *h == Some(handle))
                        .expect("matched sound");
                    slot.pending_off[layer] = Some((handle, request));
                }
                return true; // The current pose decides whether this is the late copy.
            }
            if slot.sounds[0].contains(&Some(handle))
                && (from_client || (near && request.source.0 >= CLIENTS as u32))
                && slot.lit
                && (slot.phase == Phase::OnBack
                    || slot
                        .unholster_time
                        .is_some_and(|time| (0..=WAIT).contains(&(now - time))))
            {
                let layer = slot.sounds[0]
                    .iter()
                    .position(|h| *h == Some(handle))
                    .expect("matched sound");
                slot.pending[layer] = Some((handle, request, now, slot.phase == Phase::OnBack));
                return true;
            }
        }
        false
    }
}

impl GameAudio {
    /// Begin a primary actor pass; portal passes must not own audio state.
    pub(crate) fn begin_holsters(&mut self) {
        for slot in &mut self.holster_sync.slots {
            slot.seen = false;
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn holstered_staff(
        &mut self,
        client: usize,
        identity: u64,
        enabled: bool,
        origin: [f32; 3],
        phase: Phase,
        putting_away: bool,
        key: Option<(usize, u64, i64)>,
        active: bool,
        now: i64,
    ) {
        let handles = self.legacy.as_ref().map_or([[None; 3]; 2], |adapter| {
            adapter.saber_switch_handles(client as u16)
        });
        let Some(slot) = self.holster_sync.slots.get_mut(client) else {
            return;
        };
        if slot.identity != identity || (slot.sounds != handles && slot.sounds != [[None; 3]; 2]) {
            *slot = Slot::default();
        }
        slot.identity = identity;
        slot.sounds = handles;
        slot.enabled = enabled && self.holster_sync.enabled;
        slot.origin = origin;
        slot.phase = phase;
        slot.seen = true;
        if !slot.enabled {
            slot.pending = [None; 3];
            slot.pending_off = [None; 3];
            slot.off_key = None;
            slot.off_time = None;
            return;
        }
        if putting_away
            && phase != Phase::None
            && (active || slot.was_active)
            && key.is_some()
            && slot.off_key != key
        {
            slot.off_key = key;
            for (layer, handle) in handles[1].into_iter().enumerate() {
                let Some(handle) = handle else {
                    continue;
                };
                if handles[1][..layer].contains(&Some(handle)) {
                    continue;
                }
                slot.off_time = Some(now);
                self.output.send(AudioCommand::Play(
                    handle,
                    PlayRequest {
                        origin: Some(origin),
                        source: SourceId(client as u32),
                        channel: ChannelId(0),
                        volume: 1.0,
                        attenuation: sjk_client::legacy_sound_attenuation(0),
                    },
                ));
            }
        }
        slot.was_active = active;
        for pending in &mut slot.pending_off {
            if let Some((handle, request)) = pending.take()
                && !slot
                    .off_time
                    .is_some_and(|time| (0..HOLD).contains(&(now - time)))
            {
                self.output.send(AudioCommand::Play(handle, request));
            }
        }
        for (handle, request) in slot
            .release_ignitions(client, origin, phase, now)
            .into_iter()
            .flatten()
        {
            self.output.send(AudioCommand::Play(handle, request));
        }
    }

    pub(crate) fn end_holsters(&mut self) {
        for slot in &mut self.holster_sync.slots {
            if !slot.seen {
                *slot = Slot::default();
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn ignition_layers_wait_for_reach_and_have_a_bounded_fallback() {
        let request = PlayRequest {
            source: SourceId(3),
            origin: None,
            channel: ChannelId(0),
            volume: 1.,
            attenuation: sjk_audio::Attenuation::None,
        };
        for step in [8, 7, 4, 3] {
            let mut slot = Slot {
                pending: [
                    Some((SoundHandle(4), request, 0, false)),
                    Some((SoundHandle(6), request, 0, false)),
                    Some((SoundHandle(7), request, 0, false)),
                ],
                ..Slot::default()
            };
            for time in (0..600).step_by(step) {
                assert!(
                    slot.release_ignitions(3, [1., 2., 3.], Phase::OnBack, time)
                        .iter()
                        .all(Option::is_none)
                );
            }
            let starts = slot.release_ignitions(3, [1., 2., 3.], Phase::InHand, 600);
            assert_eq!(
                starts.map(|s| s.map(|s| s.0)),
                [
                    Some(SoundHandle(4)),
                    Some(SoundHandle(6)),
                    Some(SoundHandle(7))
                ]
            );
            assert_eq!(starts[0].unwrap().1.origin, Some([1., 2., 3.]));
            assert!(
                slot.release_ignitions(3, [0.; 3], Phase::InHand, 601)
                    .iter()
                    .all(Option::is_none)
            );
        }
        let mut slot = Slot {
            pending: [Some((SoundHandle(4), request, 0, false)), None, None],
            ..Slot::default()
        };
        assert!(slot.release_ignitions(3, [0.; 3], Phase::None, 149)[0].is_none());
        assert!(slot.release_ignitions(3, [0.; 3], Phase::None, 150)[0].is_some());
        slot.pending[0] = Some((SoundHandle(4), request, 0, true));
        assert!(slot.release_ignitions(3, [0.; 3], Phase::OnBack, 1999)[0].is_none());
        assert!(slot.release_ignitions(3, [0.; 3], Phase::OnBack, 2000)[0].is_some());
    }
    #[test]
    fn ignition_interception_requires_ownership_handle_and_recent_draw() {
        let mut sync = Sync::default();
        sync.slots[3] = Slot {
            enabled: true,
            lit: true,
            sounds: [
                [Some(SoundHandle(4)), Some(SoundHandle(6)), None],
                [Some(SoundHandle(5)), None, None],
            ],
            unholster_time: Some(100),
            ..Slot::default()
        };
        let request = PlayRequest {
            source: SourceId(3),
            origin: None,
            channel: ChannelId(0),
            volume: 1.,
            attenuation: sjk_audio::Attenuation::None,
        };
        assert!(!sync.intercept(SoundHandle(8), request, 120));
        assert!(sync.intercept(SoundHandle(4), request, 120));
        assert!(!sync.intercept(SoundHandle(4), request, 300));
        assert!(!sync.intercept(
            SoundHandle(4),
            PlayRequest {
                source: SourceId(9),
                origin: Some([0.; 3]),
                ..request
            },
            120
        ));
        assert!(!sync.intercept(
            SoundHandle(4),
            PlayRequest {
                source: SourceId(9),
                ..request
            },
            120
        ));
        sync.slots[3].off_time = Some(100);
        assert!(sync.intercept(SoundHandle(5), request, 120));
        assert!(sync.slots[3].pending_off[0].is_none());
        assert!(sync.intercept(SoundHandle(5), request, 2100));
        assert!(sync.slots[3].pending_off[0].is_some());
        sync.enabled = false;
        assert!(!sync.intercept(SoundHandle(4), request, 120));
    }
}

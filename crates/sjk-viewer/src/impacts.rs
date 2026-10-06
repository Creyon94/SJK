//! Codemp impact-event to retail-effect presentation mapping.
//!
//! Missile dispatch mirrors `CG_EntityEvent` in `codemp/cgame/cg_event.c`
//! lines 2888-2956 and `CG_MissileHitWall`/`CG_MissileHitPlayer` in
//! `codemp/cgame/cg_weapons.c` lines 1980-2200. Saber selection mirrors
//! `cg_event.c` lines 2147-2379. Disruptor trace shots mirror `cg_event.c`
//! lines 2442-2530 and the code-built lines of `fx_disruptor.c`. The shared
//! client adapter supplies exact event-toggle de-duplication and `ByteToDir`
//! normals.

use sjk_client::{LegacyImpactEvent, LegacyImpactKind, LegacyImpactTracker};
use sjk_protocol::Snapshot;

const MAX_SNAPSHOT_IMPACTS: usize = 1_024;
/// Room for the concussion alt shot: two beam lines, a wall effect, the disruptor
/// miss it borrows and its rings, one every 64 units along an 8192-unit shot.
const MAX_VISUALS_PER_IMPACT: usize = 4 + MAX_CONCUSSION_RINGS;
/// `WP_FireConcussionAlt`'s range (`codemp/game/g_weapon.c:3088`, `shotRange`).
const CONCUSSION_ALT_RANGE: f32 = 8_192.0;
/// One ring per 64 units of the longest shot.
const MAX_CONCUSSION_RINGS: usize = 128;

/// One visual primitive selected by codemp for an impact event.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum Visual {
    Effect {
        name: &'static str,
        direction: [f32; 3],
        /// Where to play it; `None` is the event's own position.
        origin: Option<[f32; 3]>,
    },
    /// `FX_AddLine` with linear size and alpha (`fx_disruptor.c`).
    Line(Line),
}

/// One `FX_AddLine` call: a beam of constant colour whose half-width and
/// alpha fade linearly over `lifetime_millis`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Line {
    pub(crate) start: [f32; 3],
    pub(crate) end: [f32; 3],
    pub(crate) size: [f32; 2],
    pub(crate) alpha: [f32; 2],
    pub(crate) color: [f32; 3],
    pub(crate) lifetime_millis: u32,
    pub(crate) shader: &'static str,
}

const WHITE: [f32; 3] = [1.0; 3];
/// `YELLER` in `fx_disruptor.c:73`.
const YELLER: [f32; 3] = [0.8, 0.7, 0.0];

/// Fixed-size result; effect selection itself never allocates.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Plan {
    visuals: [Option<Visual>; MAX_VISUALS_PER_IMPACT],
    len: usize,
}

impl Plan {
    fn new() -> Self {
        Self {
            visuals: [None; MAX_VISUALS_PER_IMPACT],
            len: 0,
        }
    }

    fn effect(mut self, name: &'static str, direction: [f32; 3]) -> Self {
        if self.len < self.visuals.len() {
            self.visuals[self.len] = Some(Visual::Effect {
                name,
                direction,
                origin: None,
            });
            self.len += 1;
        }
        self
    }

    fn effect_at(mut self, name: &'static str, origin: [f32; 3], direction: [f32; 3]) -> Self {
        if self.len < self.visuals.len() {
            self.visuals[self.len] = Some(Visual::Effect {
                name,
                direction,
                origin: Some(origin),
            });
            self.len += 1;
        }
        self
    }

    fn repeated(self, name: &'static str, direction: [f32; 3], count: usize) -> Self {
        (0..count).fold(self, |plan, _| plan.effect(name, direction))
    }

    fn line(mut self, line: Line) -> Self {
        if self.len < self.visuals.len() {
            self.visuals[self.len] = Some(Visual::Line(line));
            self.len += 1;
        }
        self
    }

    pub(crate) fn iter(&self) -> impl Iterator<Item = Visual> + '_ {
        self.visuals[..self.len].iter().flatten().copied()
    }
}

/// Fixed-capacity decoded impact pool reused for every received snapshot.
pub(crate) struct Pool {
    tracker: LegacyImpactTracker,
    events: Vec<LegacyImpactEvent>,
}

impl Pool {
    pub(crate) fn new() -> Self {
        Self {
            tracker: LegacyImpactTracker::new(),
            events: Vec::with_capacity(MAX_SNAPSHOT_IMPACTS),
        }
    }

    pub(crate) fn observe(&mut self, snapshot: &Snapshot) -> usize {
        self.tracker.observe(snapshot, &mut self.events);
        self.events.len()
    }

    pub(crate) fn event(&self, index: usize) -> Option<LegacyImpactEvent> {
        self.events.get(index).copied()
    }
}

/// Select the exact stock EFX graph(s) used by codemp. A zero-length plan is
/// intentional where codemp has no wall visual (Flechette alt-fire).
pub(crate) fn plan(event: LegacyImpactEvent) -> Plan {
    let direction = event.direction;
    match event.kind {
        LegacyImpactKind::SaberHit => match event.event_parameter {
            0 => Plan::new().effect("saber/saber_cut", direction),
            2 => Plan::new().effect("saber/blood_sparks_50_mp", direction),
            3 => Plan::new().effect("saber/blood_sparks_25_mp", direction),
            16 => Plan::new().repeated("saber/blood_sparks_mp", direction, 6),
            _ => Plan::new().repeated("saber/blood_sparks_mp", direction, 3),
        },
        LegacyImpactKind::SaberBlock => Plan::new().effect(
            if event.event_parameter == 0 {
                "blaster/deflect"
            } else {
                "saber/saber_block"
            },
            direction,
        ),
        // This is a screen-space latch, not an EFX primitive. The exact
        // cg_draw.c envelope and BSP visibility trace live in
        // sjk-client::LegacySaberClashFlare.
        LegacyImpactKind::SaberClashFlare => Plan::new(),
        LegacyImpactKind::DisruptorMainShot => disruptor_main_shot(event),
        LegacyImpactKind::DisruptorSniperShot => disruptor_sniper_shot(event),
        // `FX_DisruptorAltMiss` also lays a 4 s bezier smoke trail
        // (`fx_disruptor.c:89-139`); bezier primitives are not rendered yet.
        LegacyImpactKind::DisruptorSniperMiss => Plan::new().effect(
            if event.weapon != 0 {
                "disruptor/wall_impact"
            } else {
                "disruptor/alt_miss"
            },
            direction,
        ),
        LegacyImpactKind::DisruptorHit => Plan::new().effect(
            if event.weapon != 0 {
                "disruptor/flesh_impact"
            } else {
                "disruptor/wall_impact"
            },
            direction,
        ),
        LegacyImpactKind::ConcussionAltShot => concussion_alt_shot(event, direction),
        LegacyImpactKind::MissileHitPlayer => missile_player(event, direction),
        LegacyImpactKind::MissileHitWall | LegacyImpactKind::MissileHitMetal => {
            missile_wall(event, direction)
        }
    }
}

/// `FX_DisruptorMainShot` (`fx_disruptor.c:35-44`).
fn disruptor_main_shot(event: LegacyImpactEvent) -> Plan {
    Plan::new().line(Line {
        start: event.start,
        end: event.origin,
        size: [0.1, 6.0],
        alpha: [1.0, 0.0],
        color: WHITE,
        lifetime_millis: 150,
        shader: "gfx/effects/redLine",
    })
}

/// `FX_DisruptorAltShot` (`fx_disruptor.c:63-81`): the sniper beam plus a
/// yellow core when the shot was fully charged.
fn disruptor_sniper_shot(event: LegacyImpactEvent) -> Plan {
    let plan = Plan::new().line(Line {
        start: event.start,
        end: event.origin,
        size: [0.1, 10.0],
        alpha: [1.0, 0.0],
        color: WHITE,
        lifetime_millis: 175,
        shader: "gfx/effects/redLine",
    });
    if !event.full_charge {
        return plan;
    }
    plan.line(Line {
        start: event.start,
        end: event.origin,
        size: [0.1, 7.0],
        alpha: [1.0, 0.0],
        color: YELLER,
        lifetime_millis: 150,
        shader: "gfx/misc/whiteline2",
    })
}

/// `EV_CONC_ALT_IMPACT` (`cg_event.c:2860-2881`): rings every 64 units along the
/// shot, the wall hit, `FX_ConcAltShot`'s two lines (`fx_bryarpistol.c:244-259`) to
/// the last ring, and the disruptor's alt miss at the end.
fn concussion_alt_shot(event: LegacyImpactEvent, direction: [f32; 3]) -> Plan {
    let [x, y, z] = event.shot;
    let distance = (x * x + y * y + z * z).sqrt();
    // The game's shot is at most 8192 units (`WP_FireConcussionAlt`). A
    // non-finite or longer vector from a modified server draws nothing.
    if !distance.is_finite()
        || distance > CONCUSSION_ALT_RANGE
        || !event
            .start
            .iter()
            .chain(&event.ring_direction)
            .all(|v| v.is_finite())
    {
        return Plan::new();
    }
    let unit = if distance > 0.0 {
        [x / distance, y / distance, z / distance]
    } else {
        [0.0, 0.0, 0.0]
    };
    let at = |along: f32| std::array::from_fn(|axis| event.start[axis] + unit[axis] * along);
    let mut plan = Plan::new().effect("concussion/explosion", direction);
    let mut spot = event.start;
    // One ring every 64 units, never more than the plan holds.
    for ring in 0..MAX_CONCUSSION_RINGS {
        let along = ring as f32 * 64.0;
        if along >= distance {
            break;
        }
        spot = at(along);
        plan = plan.effect_at("concussion/alt_ring", spot, event.ring_direction);
    }
    plan.line(Line {
        start: event.start,
        end: spot,
        size: [0.1, 10.0],
        alpha: [1.0, 0.0],
        color: WHITE,
        lifetime_millis: 175,
        shader: "gfx/effects/blueLine",
    })
    .line(Line {
        start: event.start,
        end: spot,
        size: [0.1, 7.0],
        alpha: [1.0, 0.0],
        // `BRIGHT` (`fx_bryarpistol.c:242`).
        color: [0.75, 0.5, 1.0],
        lifetime_millis: 150,
        shader: "gfx/misc/whiteline2",
    })
    .effect("disruptor/alt_miss", direction)
}

fn missile_player(event: LegacyImpactEvent, direction: [f32; 3]) -> Plan {
    let name = match (event.weapon, event.alternate) {
        (4 | 16 | 18, _) => "bryar/flesh_impact",
        (5 | 17, _) => "blaster/flesh_impact",
        (6, _) => "disruptor/alt_hit",
        (7, _) => "bowcaster/explosion",
        (8, true) => "repeater/concussion",
        (8, false) => "repeater/flesh_impact",
        (9, true) => "demp2/altdetonate",
        (9, false) => "demp2/flesh_impact",
        (10, _) => "flechette/flesh_impact",
        (11, _) => "rocket/explosion",
        (15, _) => "concussion/explosion",
        (12, _) => {
            return Plan::new()
                .effect("thermal/explosion", direction)
                .effect("thermal/shockwave", [0.0, 0.0, 1.0]);
        }
        _ => return Plan::new(),
    };
    Plan::new().effect(name, direction)
}

fn missile_wall(event: LegacyImpactEvent, direction: [f32; 3]) -> Plan {
    let name = match (event.weapon, event.alternate) {
        (4 | 16, true) if event.charge >= 4 => "bryar/wall_impact3",
        (4 | 16, true) if event.charge >= 2 => "bryar/wall_impact2",
        (4 | 16 | 18, _) => "bryar/wall_impact",
        (5 | 17, _) => "blaster/wall_impact",
        (6, _) => "disruptor/alt_miss",
        (7, _) => "bowcaster/explosion",
        (8, true) => "repeater/concussion",
        (8, false) => "repeater/wall_impact",
        (9, true) => "demp2/altdetonate",
        (9, false) => "demp2/wall_impact",
        (10, true) => return Plan::new(),
        (10, false) => "flechette/wall_impact",
        (11, _) => "rocket/explosion",
        (15, _) => "concussion/explosion",
        (12, _) => {
            return Plan::new()
                .effect("thermal/explosion", direction)
                .effect("thermal/shockwave", [0.0, 0.0, 1.0]);
        }
        _ => return Plan::new(),
    };
    Plan::new().effect(name, direction)
}

#[cfg(test)]
mod concussion_tests {
    use super::*;

    fn shot(shot: [f32; 3]) -> LegacyImpactEvent {
        LegacyImpactEvent {
            entity_number: 1,
            event: 84, // EV_CONC_ALT_IMPACT
            kind: LegacyImpactKind::ConcussionAltShot,
            origin: [0.0; 3],
            start: [0.0; 3],
            direction: [0.0, 0.0, 1.0],
            event_parameter: 0,
            weapon: 15,
            alternate: true,
            charge: 0,
            full_charge: false,
            shot,
            ring_direction: [1.0, 0.0, 0.0],
        }
    }

    fn rings(plan: &Plan) -> usize {
        plan.iter()
            .filter(|visual| {
                matches!(visual, Visual::Effect { name, .. } if *name == "concussion/alt_ring")
            })
            .count()
    }

    #[test]
    fn rings_follow_the_shot_up_to_its_range() {
        assert_eq!(rings(&plan(shot([640.0, 0.0, 0.0]))), 10);
        assert_eq!(
            rings(&plan(shot([8_192.0, 0.0, 0.0]))),
            MAX_CONCUSSION_RINGS
        );
    }

    #[test]
    fn oversized_or_non_finite_shots_draw_nothing() {
        for bad in [
            [1.0e9, 0.0, 0.0],
            [f32::INFINITY, 0.0, 0.0],
            [f32::NAN, 0.0, 0.0],
        ] {
            assert_eq!(plan(shot(bad)).iter().count(), 0, "{bad:?}");
        }
    }
}

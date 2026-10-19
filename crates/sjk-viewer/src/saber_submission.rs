//! Actor-to-hilt, blade, and motion-trail submission.

use crate::actor_instance::ActorInstance;
use crate::saber::{self, Instance};
use crate::saber_hilts::HiltCatalog;
use crate::saber_rgb::BladeColor;
use crate::saber_trail::{Edges, SegmentPool, StateSlab};
use glam::{Quat, Vec3};
use sjk_runtime::{HeldEquipment, HeldItemKind};

#[path = "saber_lights.rs"]
pub(crate) mod lights;

/// Submit both hilts and every numbered blade for one actor. `trails` is
/// `None` with `cg_saberTrail 0`; `skin` is the blade skin the actor wears, which
/// replaces both sabers' colours.
#[allow(clippy::too_many_arguments)]
pub(crate) fn submit(
    entity_id: u64,
    skin: Option<crate::saber_skins::SkinColor>,
    black_sabers: crate::saber_rgb::black::Policy,
    hands: [Option<(&str, saber::Attachment)>; 2],
    equipment: HeldEquipment,
    actor_origin: Vec3,
    actor_rotation: Quat,
    catalog: Option<&HiltCatalog>,
    states: &mut StateSlab,
    segments: &mut SegmentPool,
    object_groups: &mut [Vec<ActorInstance>],
    saber_instances: &mut Vec<Instance>,
    presentation_time: i64,
    mut trails: Option<&mut Edges<'_>>,
    lights: &mut crate::dynamic_lights::PointLightList,
) -> bool {
    if equipment.kind != HeldItemKind::EnergyBlade {
        return false;
    }
    let Some(catalog) = catalog else {
        return true;
    };
    if !equipment.primary_in_flight
        && let Some(tilt) = states.tilt_mut(entity_id)
    {
        // The primary is in the hand: the next throw tilts from its own pitch
        // (`cg_players.c:10429-10449`).
        tilt.reset();
    }
    for (saber_index, hand) in hands.into_iter().enumerate() {
        if saber_index == 0 && equipment.primary_in_flight {
            // The flying hilt carries this saber's blades and trail state
            // (`thrown_saber.rs`), as `CG_AddSaberBlade` shares
            // `client->saber[0].blade[n].trail` between hand and flight.
            continue;
        }
        let Some((name, attachment)) = hand else {
            continue;
        };
        let hilt = catalog.get(name);
        let (grip, weapon_rotation) =
            saber::world_attachment(actor_origin, actor_rotation, attachment);
        object_groups[hilt.mesh_index].push(ActorInstance::new(
            grip.to_array(),
            weapon_rotation.to_array(),
            [1.0; 3],
        ));
        let stock = black_sabers.color(
            entity_id,
            saber_index,
            BladeColor::from_rgb(if saber_index == 0 {
                equipment.color
            } else {
                equipment.secondary_color
            }),
        );
        // A chroma skin takes this saber's colour.
        let color = skin.map_or(stock, |skin| BladeColor::Skin(skin.worn_with(stock)));
        let mut light_blades = [None; 8];
        for blade_index in 0..usize::from(hilt.num_blades) {
            let Some(hilt_blade) = hilt.blade(blade_index) else {
                continue;
            };
            let active = blade_is_active(&equipment, saber_index, blade_index);
            let Some(state) = states.blade_mut(entity_id, saber_index, blade_index) else {
                continue;
            };
            let length = state
                .extension
                .update(active, hilt_blade.length, presentation_time);
            // A design whose words outlast its blade (`SkinColor::linger`): from when the
            // blade begins going in, a whole-length blade that draws only its words, so they
            // float on and can be read (Creyon's round 14). No contact, light or trail.
            if let Some(motion) = skin
                .filter(|skin| skin.linger > 0.0)
                .and_then(|skin| state.motion.lingering(presentation_time, skin.linger))
            {
                let whole = saber::world_blade(
                    grip,
                    weapon_rotation,
                    hilt_blade.socket,
                    hilt_blade.length,
                    hilt_blade.radius,
                );
                let key = entity_id.wrapping_mul(24) + (saber_index * 8 + blade_index) as u64;
                saber_instances.extend(Instance::pair(whole, color).map(|i| {
                    i.with_animation(presentation_time as f64 * 0.001, key as u32)
                        .with_motion(motion)
                }));
            }
            if length < 0.5 {
                continue;
            }
            let blade = saber::world_blade(
                grip,
                weapon_rotation,
                hilt_blade.socket,
                length,
                hilt_blade.radius,
            );
            let key = entity_id.wrapping_mul(24) + (saber_index * 8 + blade_index) as u64;
            let flicker = saber::Flicker::sample(key, presentation_time);
            // The swing is measured in the wearer's own frame (Creyon's round 11): turning the
            // view or running turns the whole body, which is no swing, and the effects that
            // answer a swing must not stir then.
            let body = (actor_rotation.inverse() * Vec3::from_array(blade.direction)).to_array();
            let motion = state.motion.record_turned(
                blade,
                body,
                presentation_time,
                length < hilt_blade.length,
            );
            // A direction across the blade that turns with the hilt, for the signatures'
            // three-dimensional effects (`Instance::with_frame`).
            let across = (weapon_rotation
                * Vec3::from_array(hilt_blade.socket.direction)
                    .normalize_or_zero()
                    .any_orthonormal_vector())
            .to_array();
            saber_instances.extend(
                Instance::pair_flickering(blade, color, hilt_blade.length, flicker).map(|i| {
                    i.with_contact(
                        entity_id,
                        saber_index,
                        blade_index,
                        !hilt.no_wall_marks,
                        hilt.no_dlight,
                    )
                    .with_animation(presentation_time as f64 * 0.001, key as u32)
                    .with_motion(motion)
                    .with_frame(across, hilt_blade.length)
                }),
            );
            saber_instances.extend(saber::skin_extras(
                blade,
                color,
                presentation_time as f64 * 0.001,
                key as u32,
            ));
            if let BladeColor::Skin(skin) = color
                && let Some(spec) = skin.ghosts
            {
                let animate =
                    |i: Instance| i.with_animation(presentation_time as f64 * 0.001, key as u32);
                if spec.wearer {
                    // Kept in the wearer's own frame: turning or running leaves none behind.
                    let origin = actor_origin;
                    let to_body = |b: saber::Blade| saber::Blade {
                        base: (actor_rotation.inverse() * (Vec3::from_array(b.base) - origin))
                            .to_array(),
                        direction: (actor_rotation.inverse() * Vec3::from_array(b.direction))
                            .to_array(),
                        ..b
                    };
                    let to_world = move |b: saber::Blade| saber::Blade {
                        base: (actor_rotation * Vec3::from_array(b.base) + origin).to_array(),
                        direction: (actor_rotation * Vec3::from_array(b.direction)).to_array(),
                        ..b
                    };
                    state.ghosts.record(to_body(blade), presentation_time, spec);
                    saber_instances.extend(
                        state
                            .ghosts
                            .instances_placed(to_body(blade), color, spec, to_world)
                            .map(animate),
                    );
                } else {
                    state.ghosts.record(blade, presentation_time, spec);
                    saber_instances.extend(state.ghosts.instances(blade, color, spec).map(animate));
                }
            }
            light_blades[blade_index] = Some(blade);
            if let Some(edges) = trails.as_deref_mut()
                && let Some(quad) = state.trail.update(
                    edges.edge(blade),
                    presentation_time,
                    equipment.trail_duration_millis,
                    hilt_blade.trail_style,
                    color,
                    equipment.trail_duration_millis > 0,
                )
            {
                segments.insert(quad, presentation_time);
            }
        }
        self::lights::append(
            lights,
            &light_blades,
            color,
            hilt.num_blades,
            hilt.no_dlight,
            presentation_time,
            entity_id.wrapping_add(saber_index as u64),
        );
    }
    true
}

/// Partial holster keeps primary blade 1 but retracts staff extras and saber 2.
pub(crate) fn blade_is_active(
    equipment: &HeldEquipment,
    saber_index: usize,
    blade_index: usize,
) -> bool {
    if saber_index == 0 && blade_index == 0 {
        equipment.active
    } else {
        equipment.secondary_active
    }
}

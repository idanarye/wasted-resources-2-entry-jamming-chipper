use std::time::Duration;

use bevy::prelude::*;
use bevy_rapier2d::prelude::*;

use crate::global_types::{AppState, Chipper, DespawnWithLevel, SpawnsWoodchips, Trunk};
use crate::gltf_spawner::{SpawnCollider, SpawnGltfNode};
use crate::loading::ModelAssets;
use crate::utils::{entities_ordered_by_type, ok_or};

pub struct TrunksPlugin;

impl Plugin for TrunksPlugin {
    fn build(&self, app: &mut App) {
        app.add_system_set({
            SystemSet::on_update(AppState::Game)
                .with_system(spawn_trunk)
                .with_system(handle_trunk_hitting_chipper)
                .with_system(move_kinematic_trunks)
                .with_system(handle_lost_trunks)
        });
    }
}

fn spawn_trunk(
    mut commands: Commands,
    model_assets: Res<ModelAssets>,
    current_logs: Query<(), With<Trunk>>,
) {
    if current_logs.iter().next().is_some() {
        return;
    }
    let mut cmd = commands.spawn();
    cmd.insert_bundle(RigidBodyBundle {
        body_type: RigidBodyType::Dynamic.into(),
        mass_properties: RigidBodyMassProps {
            local_mprops: MassProperties {
                local_com: point![0.0, 0.0],
                inv_mass: 1.0 / 3000.0,
                inv_principal_inertia_sqrt: 1.0 / 300.0,
            },
            ..Default::default()
        }
        .into(),
        position: point![10.0, 5.0].into(),
        velocity: RigidBodyVelocity {
            linvel: vector![-6.0, 3.0],
            angvel: -0.5,
        }
        .into(),
        // damping: RigidBodyDamping {
        // linear_damping: 1.0,
        // angular_damping: 0.0,
        // }
        // .into(),
        ..Default::default()
    });
    cmd.insert(RigidBodyPositionSync::Discrete);
    cmd.insert(SpawnCollider {
        gltf: model_assets.trunk.clone(),
        node_name: "Collider",
        material: ColliderMaterial {
            // friction: 2.0,
            // restitution: todo!(),
            // friction_combine_rule: todo!(),
            // restitution_combine_rule: todo!(),
            ..Default::default()
        },
        flags: Default::default(),
    });
    cmd.insert(Transform::from_xyz(0.0, 2.0, 0.0));
    cmd.insert(GlobalTransform::identity());
    cmd.insert(SpawnGltfNode(model_assets.trunk.clone(), "Trunk"));
    cmd.insert(Trunk::Free);
    cmd.insert(DespawnWithLevel);
}

fn handle_trunk_hitting_chipper(
    mut reader: EventReader<ContactEvent>,
    mut trunks_query: Query<(&mut Trunk, &mut RigidBodyTypeComponent)>,
    mut chippers_query: Query<&mut Chipper>,
    mut commands: Commands,
) {
    for event in reader.iter() {
        match event {
            ContactEvent::Started(handle1, handle2) => {
                if let Some([trunk_entity, chipper_entity]) = entities_ordered_by_type!(
                    [handle1.entity(), handle2.entity()],
                    trunks_query,
                    chippers_query,
                ) {
                    let (mut trunk, mut trunk_rigid_body_type) =
                        ok_or!(trunks_query.get_mut(trunk_entity); continue);
                    let mut chipper = ok_or!(chippers_query.get_mut(chipper_entity); continue);
                    if !matches!(*trunk, Trunk::Free) || !matches!(*chipper, Chipper::Free) {
                        continue;
                    }
                    trunk_rigid_body_type.0 = RigidBodyType::KinematicVelocityBased;
                    *trunk = Trunk::InChipper(chipper_entity);
                    *chipper = Chipper::Chipping;
                    commands
                        .entity(trunk_entity)
                        .insert(SpawnsWoodchips(Timer::new(Duration::ZERO, false)));
                }
            }
            ContactEvent::Stopped(handle1, handle2) => {
                if let Some([trunk_entity, chipper_entity]) = entities_ordered_by_type!(
                    [handle1.entity(), handle2.entity()],
                    trunks_query,
                    chippers_query,
                ) {
                    let trunk = ok_or!(trunks_query.get_component::<Trunk>(trunk_entity); continue);
                    if let Trunk::InChipper(trunk_chipper_entity) = trunk {
                        if *trunk_chipper_entity == chipper_entity {
                            let mut chipper =
                                ok_or!(chippers_query.get_mut(chipper_entity); continue);
                            assert!(matches!(*chipper, Chipper::Chipping));
                            *chipper = Chipper::Free;
                            commands.entity(trunk_entity).despawn_recursive();
                        }
                    }
                }
            }
        }
    }
}

fn move_kinematic_trunks(
    mut trunks: Query<(&RigidBodyTypeComponent, &mut RigidBodyVelocityComponent), With<Trunk>>,
) {
    for (rigid_body_type, mut velocity) in trunks.iter_mut() {
        if rigid_body_type.0 != RigidBodyType::KinematicVelocityBased {
            continue;
        }
        velocity.0.linvel = vector![0.0, -1.0];
    }
}

fn handle_lost_trunks(
    mut commands: Commands,
    trunks: Query<(Entity, &Trunk, &RigidBodyPositionComponent)>,
) {
    for (trunk_entity, trunk, trunk_position) in trunks.iter() {
        if matches!(trunk, Trunk::Free) && trunk_position.position.translation.y < -8.0 {
            commands.entity(trunk_entity).despawn_recursive();
        }
    }
}

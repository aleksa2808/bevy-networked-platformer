//! Main game logic.
//! Based on https://github.com/ErnWong/crystalorb/blob/master/examples/demo/src/lib.rs

use bevy::{
    math::Vec2,
    prelude::debug,
    utils::{HashMap, HashSet},
};
use crystalorb::{
    command::Command,
    fixed_timestepper::Stepper,
    world::{DisplayState, World},
};
use rapier2d::{na::Vector2, prelude::*};
use serde::{Deserialize, Serialize};
use std::fmt::{Debug, Display};

use crate::{
    BOTTOM_POWER_PAD_POSITIONS, BOTTOM_START_POSITION, LAVA_RECTS, PLATFORMS, POWER_PAD_SIZE,
    PROJECTILE_SIZE, STARTING_BOTTOM_POWER_PAD_POSITION, STARTING_TOP_POWER_PAD_POSITION, TIMESTEP,
    TOP_POWER_PAD_POSITIONS, TOP_START_POSITION,
};

pub const PHYSICS_SCALE: f32 = 20.0;
const GRAVITY_SCALE: f32 = 5.0;
const GRAVITY: Vector2<Real> = Vector2::new(0.0, 0.0);

/// Identifies a player. Used as key in maps.
/// Uses the same value as the client's `client_handle`.
#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PlayerId {
    Player1,
    Player2,
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy)]
pub enum AdvantageState {
    Neutral,
    Player1,
    Player2,
}

pub struct GameWorld {
    pipeline: PhysicsPipeline,
    island_manager: IslandManager,
    broad_phase: BroadPhase,
    narrow_phase: NarrowPhase,
    bodies: RigidBodySet,
    colliders: ColliderSet,
    joints: JointSet,
    ccd_solver: CCDSolver,
    player1: Player,
    player2: Player,
    cannon_x_position: f32,
    bottom_power_pad: PowerPad,
    top_power_pad: PowerPad,
    next_projectile_id: u16,
    projectiles: HashMap<u16, Projectile>,
    advantage_state: AdvantageState,
    round: u8,
}

pub struct Player {
    body_handle: RigidBodyHandle,
    _collider_handle: ColliderHandle,
    input: PlayerInput,
}

#[derive(Clone, Copy, Debug)]
pub enum PowerPadStatus {
    Left,
    Right,
}

pub struct PowerPad {
    body_handle: RigidBodyHandle,
    _collider_handle: ColliderHandle,
    status: PowerPadStatus,
}

pub struct Projectile {
    body_handle: RigidBodyHandle,
    _collider_handle: ColliderHandle,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum GameCommand {
    Input(PlayerId, PlayerCommand, bool),
}

impl Command for GameCommand {}

#[derive(Serialize, Deserialize, Default, Debug, Clone, Copy, PartialEq)]
pub struct PlayerInput {
    pub action: bool,
    pub left: bool,
    pub right: bool,
}

impl Display for PlayerId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PlayerId::Player1 => write!(f, "P1"),
            PlayerId::Player2 => write!(f, "P2"),
        }
    }
}

impl PlayerId {
    pub fn as_usize(&self) -> usize {
        match self {
            PlayerId::Player1 => 0,
            PlayerId::Player2 => 1,
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy)]
pub enum PlayerCommand {
    Action,
    Left,
    Right,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct GameSnapshot {
    round: u8,
    advantage_state: AdvantageState,
    player1: PlayerSnapshot,
    player2: PlayerSnapshot,
    cannon_x_position: f32,
    bottom_power_pad_position: Isometry<Real>,
    top_power_pad_position: Isometry<Real>,
    projectiles: HashMap<u16, ProjectileSnapshot>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PlayerSnapshot {
    position: Isometry<Real>,
    linvel: Vector2<Real>,
    angvel: Real,
    input: PlayerInput,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ProjectileSnapshot {
    position: Isometry<Real>,
    linvel: Vector2<Real>,
    angvel: Real,
}

#[derive(Clone, Debug)]
pub struct GameDisplayState {
    pub round: u8,
    pub player1_position: Isometry<Real>,
    pub player2_position: Isometry<Real>,
    pub cannon_x_position: f32,
    pub bottom_power_pad_status: PowerPadStatus,
    pub top_power_pad_status: PowerPadStatus,
    pub projectile_positions: HashMap<u16, Isometry<Real>>,
}

impl Default for GameWorld {
    fn default() -> Self {
        Self::new()
    }
}

impl GameWorld {
    pub fn new() -> Self {
        let mut bodies = RigidBodySet::new();
        let mut colliders = ColliderSet::new();

        // player 1
        let start_position = BOTTOM_START_POSITION;
        let body_handle = bodies.insert(
            RigidBodyBuilder::new_dynamic()
                .translation(vector![
                    start_position.x / PHYSICS_SCALE,
                    start_position.y / PHYSICS_SCALE
                ])
                .ccd_enabled(true)
                .lock_rotations()
                .build(),
        );
        let collider_handle = colliders.insert_with_parent(
            ColliderBuilder::cuboid(10.0 / PHYSICS_SCALE, 10.0 / PHYSICS_SCALE)
                .active_events(ActiveEvents::CONTACT_EVENTS)
                .active_events(ActiveEvents::INTERSECTION_EVENTS)
                .friction(0.0)
                // .density(0.1)
                // .restitution(0.5)
                .build(),
            body_handle,
            &mut bodies,
        );
        let player1 = Player {
            body_handle,
            _collider_handle: collider_handle,
            input: Default::default(),
        };

        // player 2
        let start_position = TOP_START_POSITION;
        let body_handle = bodies.insert(
            RigidBodyBuilder::new_dynamic()
                .translation(vector![
                    start_position.x / PHYSICS_SCALE,
                    start_position.y / PHYSICS_SCALE
                ])
                .ccd_enabled(true)
                .lock_rotations()
                .build(),
        );
        let collider_handle = colliders.insert_with_parent(
            ColliderBuilder::cuboid(10.0 / PHYSICS_SCALE, 10.0 / PHYSICS_SCALE)
                .active_events(ActiveEvents::CONTACT_EVENTS)
                .active_events(ActiveEvents::INTERSECTION_EVENTS)
                .friction(0.0)
                // .density(0.1)
                // .restitution(0.5)
                .build(),
            body_handle,
            &mut bodies,
        );
        let player2 = Player {
            body_handle,
            _collider_handle: collider_handle,
            input: Default::default(),
        };

        // cannon
        let cannon_x_position = 500.0;

        for platform in PLATFORMS.iter() {
            for (x, y) in [
                (platform.x, platform.y),
                (1000.0 - platform.x, 1000.0 - platform.y),
            ]
            .iter()
            {
                let size = Vec2::new(platform.w, platform.h);

                let collider = ColliderBuilder::cuboid(
                    size.x / 2.0 / PHYSICS_SCALE,
                    size.y / 2.0 / PHYSICS_SCALE,
                )
                .translation(vector![x / PHYSICS_SCALE, y / PHYSICS_SCALE])
                .friction(0.0)
                .build();
                colliders.insert(collider);
            }
        }

        for lava_rect in LAVA_RECTS.iter() {
            for (x, y) in [
                (lava_rect.x, lava_rect.y),
                (1000.0 - lava_rect.x, 1000.0 - lava_rect.y),
            ]
            .iter()
            {
                let size = Vec2::new(lava_rect.w, lava_rect.h);

                // let body = RigidBodyBuilder::new_static()
                //     .translation(vector![x / PHYSICS_SCALE, y / PHYSICS_SCALE])
                //     .ccd_enabled(true) // TODO: need this?
                //     .build();
                // let body_handle = bodies.insert(body);

                let collider = ColliderBuilder::cuboid(
                    size.x / 2.0 / PHYSICS_SCALE,
                    size.y / 2.0 / PHYSICS_SCALE,
                )
                .translation(vector![x / PHYSICS_SCALE, y / PHYSICS_SCALE])
                // .density(0.0) // TODO: what does this do?
                .sensor(true)
                .active_events(ActiveEvents::CONTACT_EVENTS)
                .active_events(ActiveEvents::INTERSECTION_EVENTS)
                .build();
                colliders.insert(collider);
            }
        }

        // power pads
        let size = Vec2::new(POWER_PAD_SIZE.w, POWER_PAD_SIZE.h);

        let position = STARTING_BOTTOM_POWER_PAD_POSITION;
        let body = RigidBodyBuilder::new_static()
            .translation(vector![
                position.x / PHYSICS_SCALE,
                position.y / PHYSICS_SCALE
            ])
            .ccd_enabled(true) // TODO: need this?
            .build();
        let body_handle = bodies.insert(body);

        let collider =
            ColliderBuilder::cuboid(size.x / 2.0 / PHYSICS_SCALE, size.y / 2.0 / PHYSICS_SCALE)
                // .density(0.0) // TODO: what does this do?
                // .sensor(true)
                .build();
        let collider_handle = colliders.insert_with_parent(collider, body_handle, &mut bodies);

        let bottom_power_pad = PowerPad {
            body_handle,
            _collider_handle: collider_handle,
            status: PowerPadStatus::Right,
        };

        let position = STARTING_TOP_POWER_PAD_POSITION;
        let body = RigidBodyBuilder::new_static()
            .translation(vector![
                position.x / PHYSICS_SCALE,
                position.y / PHYSICS_SCALE
            ])
            .ccd_enabled(true) // TODO: need this?
            .build();
        let body_handle = bodies.insert(body);

        let collider =
            ColliderBuilder::cuboid(size.x / 2.0 / PHYSICS_SCALE, size.y / 2.0 / PHYSICS_SCALE)
                // .density(0.0) // TODO: what does this do?
                // .sensor(true)
                .build();
        let collider_handle = colliders.insert_with_parent(collider, body_handle, &mut bodies);

        let top_power_pad = PowerPad {
            body_handle,
            _collider_handle: collider_handle,
            status: PowerPadStatus::Left,
        };

        Self {
            pipeline: PhysicsPipeline::new(),
            island_manager: IslandManager::new(),
            broad_phase: BroadPhase::new(),
            narrow_phase: NarrowPhase::new(),
            bodies,
            colliders,
            joints: JointSet::new(),
            ccd_solver: CCDSolver::new(),
            player1,
            player2,
            cannon_x_position,
            bottom_power_pad,
            top_power_pad,
            next_projectile_id: 0,
            projectiles: HashMap::default(),
            advantage_state: AdvantageState::Neutral,
            round: 1,
        }
    }

    fn create_projectile(&mut self, projectile_id: u16, linvel: Option<Vector<Real>>) {
        let body_builder = RigidBodyBuilder::new_dynamic()
            .translation(vector![
                self.cannon_x_position / PHYSICS_SCALE,
                500.0 / PHYSICS_SCALE
            ])
            .ccd_enabled(true)
            .lock_rotations();
        let body_builder = if let Some(linvel) = linvel {
            body_builder.linvel(linvel)
        } else {
            body_builder
        };
        let body_handle = self.bodies.insert(body_builder.build());
        let collider_handle = self.colliders.insert_with_parent(
            ColliderBuilder::cuboid(
                PROJECTILE_SIZE.w / 2.0 / PHYSICS_SCALE,
                PROJECTILE_SIZE.h / 2.0 / PHYSICS_SCALE,
            )
            .active_events(ActiveEvents::INTERSECTION_EVENTS)
            .sensor(true)
            // .density(0.1)
            // .restitution(0.5)
            .build(),
            body_handle,
            &mut self.bodies,
        );

        self.projectiles.insert(
            projectile_id,
            Projectile {
                body_handle,
                _collider_handle: collider_handle,
            },
        );
    }

    fn remove_projectile(&mut self, projectile_id: u16) {
        if let Some(projectile) = self.projectiles.remove(&projectile_id) {
            self.bodies.remove(
                projectile.body_handle,
                &mut self.island_manager,
                &mut self.colliders,
                &mut self.joints,
            );
        }
    }
}

impl World for GameWorld {
    type CommandType = GameCommand;
    type SnapshotType = GameSnapshot;
    type DisplayStateType = GameDisplayState;

    fn command_is_valid(command: &Self::CommandType, client_id: usize) -> bool {
        match command {
            GameCommand::Input(player_id, _, _) => player_id.as_usize() == client_id,
        }
    }

    fn apply_command(&mut self, command: &Self::CommandType) {
        match command {
            GameCommand::Input(player_id, command, value) => {
                let player_input = &mut match player_id {
                    PlayerId::Player1 => &mut self.player1,
                    PlayerId::Player2 => &mut self.player2,
                }
                .input;
                match command {
                    PlayerCommand::Action => player_input.action = *value,
                    PlayerCommand::Left => player_input.left = *value,
                    PlayerCommand::Right => player_input.right = *value,
                }
            }
        }
    }

    fn apply_snapshot(&mut self, snapshot: Self::SnapshotType) {
        self.round = snapshot.round;
        self.advantage_state = snapshot.advantage_state;

        let update_player =
            |player_snapshot: &PlayerSnapshot, bodies: &mut RigidBodySet, player: &mut Player| {
                let body = bodies.get_mut(player.body_handle).unwrap();
                body.set_position(player_snapshot.position, true);
                body.set_linvel(player_snapshot.linvel, true);
                body.set_angvel(player_snapshot.angvel, true);
                player.input = player_snapshot.input;
            };

        update_player(&snapshot.player1, &mut self.bodies, &mut self.player1);
        update_player(&snapshot.player2, &mut self.bodies, &mut self.player2);

        self.cannon_x_position = snapshot.cannon_x_position;

        let update_power_pad =
            |position: &Isometry<Real>, bodies: &mut RigidBodySet, power_pad: &mut PowerPad| {
                let body = bodies.get_mut(power_pad.body_handle).unwrap();
                body.set_position(*position, true);
            };

        update_power_pad(
            &snapshot.bottom_power_pad_position,
            &mut self.bodies,
            &mut self.bottom_power_pad,
        );
        update_power_pad(
            &snapshot.top_power_pad_position,
            &mut self.bodies,
            &mut self.top_power_pad,
        );

        // update projectiles
        let snapshot_projectiles = snapshot
            .projectiles
            .iter()
            .map(|(n, _)| *n)
            .collect::<HashSet<u16>>();
        let current_projectiles = self.projectiles.keys().copied().collect::<HashSet<u16>>();

        // Create objects for all projectiles in the snapshot which are not already in the game world
        for projectile_id in snapshot_projectiles.difference(&current_projectiles) {
            debug!("Creating projectile {} from snapshot", projectile_id);
            self.create_projectile(*projectile_id, None);
        }

        // Remove objects for all projectiles that are in the game world but not in the snapshot
        for projectile_id in current_projectiles.difference(&snapshot_projectiles) {
            debug!("Removing projectile {} not in snapshot", projectile_id);
            self.remove_projectile(*projectile_id);
        }

        for (projectile_id, projectile_snapshot) in snapshot.projectiles.iter() {
            let projectile = self.projectiles.get_mut(projectile_id).unwrap();
            let body = self.bodies.get_mut(projectile.body_handle).unwrap();
            body.set_position(projectile_snapshot.position, true);
            body.set_linvel(projectile_snapshot.linvel, true);
            body.set_angvel(projectile_snapshot.angvel, true);
        }
    }

    fn snapshot(&self) -> Self::SnapshotType {
        let update_player = |player: &Player| {
            let body = self.bodies.get(player.body_handle).unwrap();
            PlayerSnapshot {
                position: *body.position(),
                linvel: *body.linvel(),
                angvel: body.angvel(),
                input: player.input,
            }
        };
        let update_power_pad = |power_pad: &PowerPad| {
            let body = self.bodies.get(power_pad.body_handle).unwrap();
            *body.position()
        };
        GameSnapshot {
            round: self.round,
            advantage_state: self.advantage_state,
            player1: update_player(&self.player1),
            player2: update_player(&self.player2),
            cannon_x_position: self.cannon_x_position,
            bottom_power_pad_position: update_power_pad(&self.bottom_power_pad),
            top_power_pad_position: update_power_pad(&self.top_power_pad),
            projectiles: self
                .projectiles
                .iter()
                .map(|(id, projectile)| {
                    let body = self.bodies.get(projectile.body_handle).unwrap();
                    (
                        *id,
                        ProjectileSnapshot {
                            position: *body.position(),
                            linvel: *body.linvel(),
                            angvel: body.angvel(),
                        },
                    )
                })
                .collect::<HashMap<u16, ProjectileSnapshot>>(),
        }
    }

    fn display_state(&self) -> Self::DisplayStateType {
        let convert_simulation_to_display_scale = |body_handle: RigidBodyHandle| {
            let mut i = *self.bodies.get(body_handle).unwrap().position();
            i.translation.vector *= PHYSICS_SCALE;
            i
        };
        GameDisplayState {
            round: self.round,
            player1_position: convert_simulation_to_display_scale(self.player1.body_handle),
            player2_position: convert_simulation_to_display_scale(self.player2.body_handle),
            cannon_x_position: self.cannon_x_position,
            bottom_power_pad_status: self.bottom_power_pad.status,
            top_power_pad_status: self.top_power_pad.status,
            // TODO: potential caching
            projectile_positions: self
                .projectiles
                .iter()
                .map(|(id, projectile)| {
                    (
                        *id,
                        convert_simulation_to_display_scale(projectile.body_handle),
                    )
                })
                .collect::<HashMap<u16, Isometry<Real>>>(),
        }
    }
}

impl Stepper for GameWorld {
    fn step(&mut self) {
        let mut new_projectiles = vec![];

        for (player_id, player, mirror_multiplier) in [
            (PlayerId::Player1, &self.player1, 1.0),
            (PlayerId::Player2, &self.player2, -1.0),
        ]
        .iter()
        {
            if matches!(
                (self.advantage_state, player_id),
                (AdvantageState::Player1, PlayerId::Player1)
                    | (AdvantageState::Player2, PlayerId::Player2)
            ) {
                const CANNON_SPEED: f32 = 5.0;

                if player.input.left {
                    self.cannon_x_position = (self.cannon_x_position
                        - CANNON_SPEED * mirror_multiplier)
                        .max(100.0)
                        .min(900.0);
                }
                if player.input.right {
                    self.cannon_x_position = (self.cannon_x_position
                        + CANNON_SPEED * mirror_multiplier)
                        .max(100.0)
                        .min(900.0);
                }

                if player.input.action {
                    // TODO: limit firerate
                    if self.projectiles.len() < 10 {
                        const PROJECTILE_SPEED: f32 = 6.0;

                        let id = self.next_projectile_id;
                        self.next_projectile_id += 1;

                        new_projectiles
                            .push((id, vector![0.0, PROJECTILE_SPEED * mirror_multiplier]));
                    }
                }
            } else {
                let body = self.bodies.get_mut(player.body_handle).unwrap();

                let mut x_velocity = 0.0;

                if player.input.left {
                    x_velocity -= 1.0 * *mirror_multiplier;
                }
                if player.input.right {
                    x_velocity += 1.0 * *mirror_multiplier;
                }

                x_velocity *= 15.0;

                let is_grounded = self
                    .narrow_phase
                    .contacts_with(player._collider_handle)
                    .any(|contact_pair| {
                        contact_pair.manifolds.iter().any(|manifold| {
                            manifold.local_n1[0] == 0.0
                                && (f32::abs(manifold.local_n1[1]) - 1.0).abs() < f32::EPSILON
                        })
                    });

                if player.input.action && is_grounded {
                    let y_velocity = 20.0 * *mirror_multiplier;
                    body.set_linvel(vector![x_velocity, y_velocity], true);
                } else {
                    let y_velocity = body.linvel()[1];
                    body.set_linvel(vector![x_velocity, y_velocity], true);
                }

                // apply player specific gravity
                body.apply_force(
                    vector![0.0, *mirror_multiplier * -9.81 * GRAVITY_SCALE],
                    true,
                );
            }
        }

        for (projectile_id, linvel) in new_projectiles {
            self.create_projectile(projectile_id, Some(linvel));
        }

        self.pipeline.step(
            &GRAVITY,
            &IntegrationParameters {
                dt: TIMESTEP as f32,
                ..Default::default()
            },
            &mut self.island_manager,
            &mut self.broad_phase,
            &mut self.narrow_phase,
            &mut self.bodies,
            &mut self.colliders,
            &mut self.joints,
            &mut self.ccd_solver,
            &(),
            &(),
        );

        let mut dead_players = HashSet::default();
        for (player_id, player) in [
            (PlayerId::Player1, &self.player1),
            (PlayerId::Player2, &self.player2),
        ]
        .iter()
        {
            if self
                .narrow_phase
                .intersections_with(player._collider_handle)
                .any(|(c1, c2, intersecting)| {
                    if intersecting {
                        let other_collider = if c1 == player._collider_handle {
                            c2
                        } else {
                            c1
                        };

                        self.colliders.get(other_collider).unwrap().is_sensor()
                    } else {
                        false
                    }
                })
            {
                dead_players.insert(*player_id);
            }
        }

        if !dead_players.is_empty() {
            // TODO: update score

            self.round += 1;
            self.advantage_state = AdvantageState::Neutral;

            // reset players
            let body = self.bodies.get_mut(self.player1.body_handle).unwrap();
            body.set_translation(
                vector![
                    BOTTOM_START_POSITION.x / PHYSICS_SCALE,
                    BOTTOM_START_POSITION.y / PHYSICS_SCALE
                ],
                true,
            );
            body.set_linvel(vector![0.0, 0.0], true);
            let body = self.bodies.get_mut(self.player2.body_handle).unwrap();
            body.set_translation(
                vector![
                    TOP_START_POSITION.x / PHYSICS_SCALE,
                    TOP_START_POSITION.y / PHYSICS_SCALE
                ],
                true,
            );
            body.set_linvel(vector![0.0, 0.0], true);

            // reset cannon
            self.cannon_x_position = 500.0;

            // reset power pads
            self.bodies
                .get_mut(self.bottom_power_pad.body_handle)
                .unwrap()
                .set_translation(
                    vector![
                        STARTING_BOTTOM_POWER_PAD_POSITION.x / PHYSICS_SCALE,
                        STARTING_BOTTOM_POWER_PAD_POSITION.y / PHYSICS_SCALE
                    ],
                    true,
                );
            self.bodies
                .get_mut(self.top_power_pad.body_handle)
                .unwrap()
                .set_translation(
                    vector![
                        STARTING_TOP_POWER_PAD_POSITION.x / PHYSICS_SCALE,
                        STARTING_TOP_POWER_PAD_POSITION.y / PHYSICS_SCALE
                    ],
                    true,
                );

            // clear projectiles
            let projectile_ids = self.projectiles.keys().copied().collect::<Vec<u16>>();
            for projectile_id in projectile_ids {
                self.remove_projectile(projectile_id);
            }
        } else {
            let mut players_reached_pad = 0;
            let mut next_state = self.advantage_state;
            for (player_id, player, power_pad) in [
                (PlayerId::Player1, &self.player1, &mut self.bottom_power_pad),
                (PlayerId::Player2, &self.player2, &mut self.top_power_pad),
            ]
            .iter()
            {
                if let Some(contact_pair) = self
                    .narrow_phase
                    .contact_pair(player._collider_handle, power_pad._collider_handle)
                {
                    if contact_pair.has_any_active_contact {
                        match (self.advantage_state, player_id) {
                            (AdvantageState::Neutral, _)
                            | (AdvantageState::Player1, PlayerId::Player2)
                            | (AdvantageState::Player2, PlayerId::Player1) => {
                                if players_reached_pad > 0 {
                                    next_state = AdvantageState::Neutral;
                                } else {
                                    next_state = match player_id {
                                        PlayerId::Player1 => AdvantageState::Player1,
                                        PlayerId::Player2 => AdvantageState::Player2,
                                    };
                                }

                                players_reached_pad += 1;
                            }
                            (AdvantageState::Player1, PlayerId::Player1)
                            | (AdvantageState::Player2, PlayerId::Player2) => (),
                        }
                    }
                }
            }
            self.advantage_state = next_state;

            match players_reached_pad {
                1 => match self.advantage_state {
                    AdvantageState::Neutral => unreachable!(),
                    AdvantageState::Player1 => {
                        let body = self.bodies.get_mut(self.player1.body_handle).unwrap();
                        body.set_linvel(vector![0.0, 0.0], true);

                        let body = self.bodies.get_mut(self.player2.body_handle).unwrap();
                        let x = body.translation().x * PHYSICS_SCALE;

                        let new_position = if x < 500.0 {
                            self.top_power_pad.status = PowerPadStatus::Right;
                            TOP_POWER_PAD_POSITIONS.right
                        } else {
                            self.top_power_pad.status = PowerPadStatus::Left;
                            TOP_POWER_PAD_POSITIONS.left
                        };

                        let body = self.bodies.get_mut(self.top_power_pad.body_handle).unwrap();
                        body.set_translation(
                            vector![
                                new_position.x / PHYSICS_SCALE,
                                new_position.y / PHYSICS_SCALE
                            ],
                            true,
                        )
                    }
                    AdvantageState::Player2 => {
                        let body = self.bodies.get_mut(self.player2.body_handle).unwrap();
                        body.set_linvel(vector![0.0, 0.0], true);

                        let body = self.bodies.get_mut(self.player1.body_handle).unwrap();
                        let x = body.translation().x * PHYSICS_SCALE;

                        let new_position = if x < 500.0 {
                            self.bottom_power_pad.status = PowerPadStatus::Right;
                            BOTTOM_POWER_PAD_POSITIONS.right
                        } else {
                            self.bottom_power_pad.status = PowerPadStatus::Left;
                            BOTTOM_POWER_PAD_POSITIONS.left
                        };

                        let body = self
                            .bodies
                            .get_mut(self.bottom_power_pad.body_handle)
                            .unwrap();
                        body.set_translation(
                            vector![
                                new_position.x / PHYSICS_SCALE,
                                new_position.y / PHYSICS_SCALE
                            ],
                            true,
                        )
                    }
                },
                2 => {
                    // move both pads
                    todo!();
                }
                _ => (),
            }

            let projectiles_to_remove = if players_reached_pad > 0 {
                // destroy all projectiles
                self.projectiles.keys().copied().collect()
            } else {
                // destroy projectiles that hit solids (non-sensors)
                let mut projectiles_to_remove = vec![];
                for (projectile_id, projectile) in self.projectiles.iter() {
                    if self
                        .narrow_phase
                        .intersections_with(projectile._collider_handle)
                        .any(|(c1, c2, intersecting)| {
                            if intersecting {
                                let other_collider = if c1 == projectile._collider_handle {
                                    c2
                                } else {
                                    c1
                                };

                                !self.colliders.get(other_collider).unwrap().is_sensor()
                            } else {
                                false
                            }
                        })
                    {
                        projectiles_to_remove.push(*projectile_id);
                    }
                }
                projectiles_to_remove
            };

            for projectile_id in projectiles_to_remove {
                self.remove_projectile(projectile_id);
            }
        }
    }
}

impl DisplayState for GameDisplayState {
    fn from_interpolation(state1: &Self, state2: &Self, t: f64) -> Self {
        if state1.round != state2.round {
            state2.clone()
        } else {
            let mut interpolated_projectile_positions = state1.projectile_positions.clone();
            for (projectile_id, p2) in state2.projectile_positions.iter() {
                if let Some(p1) = interpolated_projectile_positions.get_mut(&projectile_id) {
                    p1.translation.vector[0] = p2.translation.vector[0];
                    p1.translation.vector[1] = (1.0 - t as f32) * p1.translation.vector[1]
                        + t as f32 * p2.translation.vector[1];
                    p1.rotation = p2.rotation;
                }
            }

            GameDisplayState {
                round: state2.round,
                player1_position: state1
                    .player1_position
                    .lerp_slerp(&state2.player1_position, t as f32),
                player2_position: state1
                    .player2_position
                    .lerp_slerp(&state2.player2_position, t as f32),
                cannon_x_position: (1.0 - t as f32) * state1.cannon_x_position
                    + t as f32 * state2.cannon_x_position,
                bottom_power_pad_status: state2.bottom_power_pad_status,
                top_power_pad_status: state2.top_power_pad_status,
                projectile_positions: interpolated_projectile_positions,
            }
        }
    }
}

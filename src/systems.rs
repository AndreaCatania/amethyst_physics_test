use amethyst::{
    controls::HideCursor,
    core::{
        math::{Matrix4, UnitQuaternion, Vector3},
        Transform,
    },
    ecs::prelude::*,
    input::{InputEvent, StringBindings},
    renderer::Camera,
    shrev::EventChannel,
};
use amethyst_physics::prelude::*;

use crate::components::*;

const MOUSE_SENSITIVITY: f32 = 0.2;
const MAX_PITCH_ANGLE: f32 = 20.0;
const FORCE_MULTIPLIER: f32 = 600.0;
const JUMP_IMPULSE: f32 = 1.5;

#[derive(Debug)]
pub(crate) struct CameraMotionSystem {
    input_event_reader: Option<ReaderId<InputEvent<StringBindings>>>,
}

impl CameraMotionSystem {
    pub fn new() -> Self {
        CameraMotionSystem {
            input_event_reader: None,
        }
    }
}

impl<'s> System<'s> for CameraMotionSystem {
    #[allow(clippy::type_complexity)]
    type SystemData = (
        ReadExpect<'s, PhysicsTime>,
        ReadExpect<'s, EventChannel<InputEvent<StringBindings>>>,
        WriteExpect<'s, HideCursor>,
        ReadStorage<'s, CameraBoomHandle>,
        WriteStorage<'s, Transform>,
    );

    fn run(
        &mut self,
        (physics_time, input_event_channel, mut hide_cursor, camera_boom_handles, mut transforms): Self::SystemData,
    ) {
        hide_cursor.hide = true;

        // Capture the input
        let motion = {
            let mut m_motion_x = 0.0;
            let mut m_motion_y = 0.0;

            for e in input_event_channel.read(self.input_event_reader.as_mut().unwrap()) {
                if let InputEvent::MouseMoved { delta_x, delta_y } = e {
                    m_motion_x = *delta_y;
                    m_motion_y = *delta_x * -1.0;
                    break;
                }
            }
            (
                m_motion_x * MOUSE_SENSITIVITY,
                m_motion_y * MOUSE_SENSITIVITY,
            )
        };

        for (transform, _) in (&mut transforms, &camera_boom_handles).join() {
            // Clamp the pitch rotation by avoiding further rotations.
            let pitch_clamper = {
                let angles = transform.isometry().rotation.euler_angles();

                let mut pitch_deg = angles.0.to_degrees();

                if angles.2.abs() > std::f32::consts::FRAC_PI_2 {
                    // Invert the pitch
                    if pitch_deg < 0.0 {
                        pitch_deg = pitch_deg + 180.0;
                    } else {
                        pitch_deg = pitch_deg - 180.0;
                    }
                }
                if pitch_deg > MAX_PITCH_ANGLE || pitch_deg < -MAX_PITCH_ANGLE {
                    if pitch_deg.signum() != motion.0.signum() {
                        1.0
                    } else {
                        0.0
                    }
                } else {
                    1.0
                }
            };

            let delta_rotation_pitch = UnitQuaternion::from_axis_angle(
                &Vector3::x_axis(),
                motion.0 * pitch_clamper * physics_time.delta_seconds(),
            );
            let delta_rotation_yaw = UnitQuaternion::from_axis_angle(
                &Vector3::y_axis(),
                motion.1 * physics_time.delta_seconds(),
            );

            transform.isometry_mut().rotation =
                delta_rotation_yaw * transform.isometry().rotation * delta_rotation_pitch;

            break; // Actually is supported only 1 player
        }
    }

    fn setup(&mut self, world: &mut World) {
        Self::SystemData::setup(world);
        let mut ie = world.fetch_mut::<EventChannel<InputEvent<StringBindings>>>();
        self.input_event_reader = Some(ie.register_reader());
    }
}

pub struct CharacterMotionControllerSystem {
    input_event_reader: Option<ReaderId<InputEvent<StringBindings>>>,
    input_direction: Vector3<f32>,
}

impl CharacterMotionControllerSystem {
    pub fn new() -> Self {
        Self {
            input_event_reader: None,
            input_direction: Vector3::zeros(),
        }
    }
}

impl<'s> System<'s> for CharacterMotionControllerSystem {
    type SystemData = (
        ReadExpect<'s, PhysicsWorld<f32>>,
        ReadExpect<'s, PhysicsTime>,
        ReadExpect<'s, EventChannel<InputEvent<StringBindings>>>,
        ReadStorage<'s, CharacterBody>,
        ReadStorage<'s, Camera>,
        ReadStorage<'s, PhysicsHandle<PhysicsRigidBodyTag>>,
        ReadStorage<'s, Transform>,
    );

    fn run(
        &mut self,
        (
            physics_world,
            physics_time,
            input_event_channel,
            character_bodies,
            cameras,
            rigid_body_tags,
            transforms,
        ): Self::SystemData,
    ) {
        for e in input_event_channel.read(self.input_event_reader.as_mut().unwrap()) {
            if let InputEvent::ActionPressed(action) = e {
                match action.as_str() {
                    "Forward" => {
                        self.input_direction.z -= 1.0;
                    }
                    "Backward" => {
                        self.input_direction.z += 1.0;
                    }
                    "Right" => {
                        self.input_direction.x -= 1.0;
                    }
                    "Left" => {
                        self.input_direction.x += 1.0;
                    }
                    "Jump" => {
                        self.input_direction.y += 1.0;
                    }
                    _ => {}
                }
            } else if let InputEvent::ActionReleased(action) = e {
                match action.as_str() {
                    "Forward" => {
                        self.input_direction.z += 1.0;
                    }
                    "Backward" => {
                        self.input_direction.z -= 1.0;
                    }
                    "Right" => {
                        self.input_direction.x += 1.0;
                    }
                    "Left" => {
                        self.input_direction.x -= 1.0;
                    }
                    "Jump" => {
                        self.input_direction.y -= 1.0;
                    }
                    _ => {}
                }
            }
        }

        let mut camera_pos = Matrix4::<f32>::identity();
        for (t, _) in (&transforms, &cameras).join() {
            camera_pos = t.global_matrix().clone();
        }

        for (body_tag, _) in (&rigid_body_tags, &character_bodies).join() {
            // Apply motion force
            let mut force = camera_pos.transform_vector(&self.input_direction);
            force.y = 0.0; // Don't apply any force on Y axis
            physics_world
                .rigid_body_server()
                .apply_force(body_tag.get(), &(force * FORCE_MULTIPLIER));

            // Apply jumping impulse
            physics_world.rigid_body_server().apply_impulse(
                body_tag.get(),
                &Vector3::new(0.0, self.input_direction.y * JUMP_IMPULSE, 0.0),
            );

            // Compute breaking force
            let velocity = physics_world
                .rigid_body_server()
                .linear_velocity(body_tag.get());
            let mut bk_force = (velocity / physics_time.delta_seconds()) * -1.0;
            bk_force.y = 0.0; // Don't slow down the jump
            physics_world
                .rigid_body_server()
                .apply_force(body_tag.get(), &bk_force);

            break; // Actually only 1 player is allowed;
        }
    }

    fn setup(&mut self, world: &mut World) {
        Self::SystemData::setup(world);
        let mut ie = world.fetch_mut::<EventChannel<InputEvent<StringBindings>>>();
        self.input_event_reader = Some(ie.register_reader());
    }
}

use amethyst::{
    controls::{CursorHideSystem, MouseFocusUpdateSystemDesc},
    core::{
        math::Vector3,
        timing::Time,
        transform::{Transform, TransformBundle},
        Parent,
    },
    input::{InputBundle, StringBindings},
    prelude::*,
    renderer::{
        camera::Camera,
        light,
        palette::{LinSrgba, Srgb},
        plugins::{RenderShaded3D, RenderToWindow},
        rendy::mesh::{Normal, Position, Tangent, TexCoord},
        shape::Shape,
        types,
        visibility::BoundingSphere,
        RenderingBundle,
    },
    utils::application_root_dir,
    window::ScreenDimensions,
    Error,
};
use rand::prelude::*;

use amethyst_nphysics::NPhysicsBackend;
use amethyst_physics::{prelude::*, PhysicsBundle};

mod components;
mod systems;
mod visual_utils;

#[derive(Default)]
struct Example {
    time_bank: f32,
}

impl SimpleState for Example {
    fn on_start(&mut self, data: StateData<'_, GameData<'_, '_>>) {
        // Add light
        add_light_entity(
            data.world,
            Srgb::new(1.0, 1.0, 1.0),
            Vector3::new(-0.2, -1.0, -0.2),
            1.0,
        );
        add_light_entity(
            data.world,
            Srgb::new(1.0, 0.8, 0.8),
            Vector3::new(0.2, -1.0, 0.2),
            1.0,
        );

        // Create floor
        create_floor(data.world);

        // Create the character + camera.
        create_character_entity(data.world);

        // Create Box
        add_cube_entity(data.world, Vector3::new(0.0, 6.0, 0.0));
    }

    fn update(&mut self, data: &mut StateData<'_, GameData<'_, '_>>) -> SimpleTrans {
        // TODO this code must go inside a System.
        // Spawn a new cube each X sec.
        {
            let time = data.world.fetch::<Time>();
            self.time_bank += time.delta_seconds();
        }

        let time_threshold = 10.0; // Each 10 sec
        let spawn_scale = 10.0f32; // Scale

        let mut rng = rand::thread_rng();

        while self.time_bank > time_threshold {
            add_cube_entity(
                data.world,
                Vector3::new(
                    rng.gen::<f32>() * spawn_scale - spawn_scale * 0.5,
                    6.0,
                    rng.gen::<f32>() * spawn_scale - spawn_scale * 0.5,
                ),
            );
            self.time_bank -= time_threshold;
        }

        Trans::None
    }
}

fn main() -> Result<(), Error> {
    amethyst::start_logger(Default::default());

    let app_root = application_root_dir()?;

    let assets_dir = app_root.join("assets");
    let display_config_path = app_root.join("config").join("display.ron");

    let game_data = GameDataBuilder::default()
        .with_system_desc(
            MouseFocusUpdateSystemDesc::default(),
            "mouse_focus_update",
            &[],
        )
        .with(
            CursorHideSystem::default(),
            "cursor_hide",
            &["mouse_focus_update"],
        )
        .with_bundle(
            InputBundle::<StringBindings>::new()
                .with_bindings_from_file(assets_dir.join("input_bindings.ron"))
                .unwrap(),
        )?
        .with(
            systems::CameraMotionSystem::new(),
            "camera_motion_system",
            &["input_system"],
        )
        .with_bundle(TransformBundle::new())?
        .with_bundle(
            PhysicsBundle::<f32, NPhysicsBackend>::new().with_pre_physics(
                systems::CharacterMotionControllerSystem::new(),
                String::from("character_motion_controller"),
                vec![],
            ),
        )?
        .with_bundle(
            RenderingBundle::<types::DefaultBackend>::new()
                .with_plugin(
                    RenderToWindow::from_config_path(display_config_path)
                        .with_clear([0.34, 0.36, 0.52, 1.0]),
                )
                .with_plugin(RenderShaded3D::default()),
        )?;
    let mut game = Application::build(assets_dir, Example::default())?.build(game_data)?;
    game.run();
    Ok(())
}

fn add_light_entity(world: &mut World, color: Srgb, direction: Vector3<f32>, intensity: f32) {
    let light: light::Light = light::DirectionalLight {
        color,
        direction: direction.normalize(),
        intensity,
    }
    .into();

    world.create_entity().with(light).build();
}

fn create_floor(world: &mut World) {
    let shape = {
        let desc = ShapeDesc::Cube {
            half_extents: Vector3::new(20.0, 0.2, 20.0),
        };
        let physics_world = world.fetch::<PhysicsWorld<f32>>();
        physics_world.shape_server().create(&desc)
    };

    let rb = {
        let mut rb_desc = RigidBodyDesc::default();
        rb_desc.mode = BodyMode::Static;

        let physics_world = world.fetch::<PhysicsWorld<f32>>();
        physics_world.rigid_body_server().create(&rb_desc)
    };

    let mesh = {
        let mesh_data: types::MeshData = Shape::Cube
            .generate::<(Vec<Position>, Vec<Normal>, Vec<Tangent>, Vec<TexCoord>)>(Some((
                20.0, 0.2, 20.0,
            )))
            .into();

        visual_utils::create_mesh(world, mesh_data)
    };

    let mat = visual_utils::create_material(
        world,
        LinSrgba::new(0.0, 1.0, 0.0, 1.0),
        0.0, // Metallic
        1.0, // Roughness
    );

    world
        .create_entity()
        .with(mesh)
        .with(mat)
        .with(BoundingSphere::origin(20.0))
        .with(Transform::default())
        .with(shape)
        .with(rb)
        .build();
}

fn add_cube_entity(world: &mut World, pos: Vector3<f32>) {
    let shape = {
        let desc = ShapeDesc::Cube {
            half_extents: Vector3::new(1.0, 1.0, 1.0),
        };
        let physics_world = world.fetch::<PhysicsWorld<f32>>();
        physics_world.shape_server().create(&desc)
    };

    let rb = {
        let rb_desc = RigidBodyDesc::default();

        let physics_world = world.fetch::<PhysicsWorld<f32>>();
        physics_world.rigid_body_server().create(&rb_desc)
    };

    let mesh = {
        let mesh_data: types::MeshData = Shape::Cube
            .generate::<(Vec<Position>, Vec<Normal>, Vec<Tangent>, Vec<TexCoord>)>(Some((
                1.0, 1.0, 1.0,
            )))
            .into();

        visual_utils::create_mesh(world, mesh_data)
    };

    let mut rng = rand::thread_rng();
    let mat = visual_utils::create_material(
        world,
        LinSrgba::new(rng.gen(), rng.gen(), rng.gen(), 1.0),
        0.0,
        1.0,
    );

    let mut transf = Transform::default();
    transf.set_translation(pos);

    world
        .create_entity()
        .with(mesh)
        .with(mat)
        .with(BoundingSphere::origin(1.0))
        .with(transf)
        .with(shape)
        .with(rb)
        .build();
}

/// Creates three entities:
/// 1. The character (With RigidBody).
/// 2. The camera boom handle attached to the character.
/// 3. The camera attached to the camera bool handle.
fn create_character_entity(world: &mut World) {
    let character = {
        let shape = {
            let desc = ShapeDesc::Capsule {
                half_height: 1.0,
                radius: 0.5,
            };
            let physics_world = world.fetch::<PhysicsWorld<f32>>();
            physics_world.shape_server().create(&desc)
        };

        let rb = {
            let mut rb_desc = RigidBodyDesc::default();
            rb_desc.lock_rotation_x = true;
            rb_desc.lock_rotation_y = true;
            rb_desc.lock_rotation_z = true;

            let physics_world = world.fetch::<PhysicsWorld<f32>>();
            physics_world.rigid_body_server().create(&rb_desc)
        };

        let mesh = {
            let mesh_data: types::MeshData = Shape::Cube
                .generate::<(Vec<Position>, Vec<Normal>, Vec<Tangent>, Vec<TexCoord>)>(Some((
                    0.5, 1.5, 0.5,
                )))
                .into();

            visual_utils::create_mesh(world, mesh_data)
        };

        let mat =
            visual_utils::create_material(world, LinSrgba::new(0.65, 1.0, 0.90, 1.0), 0.0, 1.0);

        let mut transf = Transform::default();
        transf.set_translation(Vector3::new(-3.0, 2.0, -3.0));

        world
            .create_entity()
            .with(mesh)
            .with(mat)
            .with(BoundingSphere::origin(1.0))
            .with(transf)
            .with(shape)
            .with(rb)
            .with(components::CharacterBody)
            .build()
    };

    let camera_boom_handle = {
        let mut transf = Transform::default();
        transf.set_translation_y(1.5);

        world
            .create_entity()
            .with(transf)
            .with(components::CameraBoomHandle)
            .with(Parent { entity: character })
            .build()
    };

    let _camera = {
        let mut camera_transform = Transform::default();
        camera_transform.set_translation_xyz(0.0, 0.0, 6.0);

        let (width, height) = {
            let dim = world.read_resource::<ScreenDimensions>();
            (dim.width(), dim.height())
        };

        world
            .create_entity()
            .with(camera_transform)
            .with(Camera::standard_3d(width, height))
            .with(Parent {
                entity: camera_boom_handle,
            })
            .build()
    };
}

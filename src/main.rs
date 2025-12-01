use bevy::{
    dev_tools::picking_debug::{DebugPickingMode, DebugPickingPlugin},
    ecs::{system::SystemState, world::CommandQueue},
    picking::prelude::*,
    prelude::*,
    tasks::{AsyncComputeTaskPool, futures},
};

use crate::{
    component::{ComputeMesh, Earth, RotatingLight},
    gui::GuiPlugin,
    math::generate_face,
    observer::{rotate_earth, zoom},
    resource::{BoxMaterialHandle, EarthTexture, LoadingProgress},
    state::GameState,
};

mod component;
mod gui;
mod math;
mod observer;
mod resource;
mod state;

const EARTH_RADIUS: Vec3 = Vec3::new(1000., 1000., 1000.);

const TOTAL_MESH_COUNT: u32 = 800;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(GuiPlugin)
        .add_plugins((MeshPickingPlugin, DebugPickingPlugin))
        .insert_resource(DebugPickingMode::Disabled)
        .init_state::<GameState>()
        .init_resource::<LoadingProgress>()
        .add_systems(Startup, setup_camera)
        .add_systems(OnEnter(GameState::Loading), (add_assets, spawn_task))
        .add_systems(
            Update,
            (check_ready, handle_tasks).run_if(in_state(GameState::Loading)),
        )
        .add_systems(Update, rotate_light.run_if(in_state(GameState::Playing)))
        .add_systems(
            OnEnter(GameState::PostLoading),
            |mut next_state: ResMut<NextState<GameState>>,
             earth: Single<&mut Visibility, With<Earth>>| {
                next_state.set(GameState::Playing);
                *earth.into_inner() = Visibility::Visible;
            },
        )
        .add_systems(
            OnEnter(GameState::Playing),
            |mut mode: ResMut<DebugPickingMode>| *mode = DebugPickingMode::Normal,
        )
        .run();
}

fn setup_camera(mut commands: Commands) {
    // Camera
    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(0.0, 0.0, 3000.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));

    // Light
    commands.spawn((
        DirectionalLight {
            illuminance: 10000.0,
            ..default()
        },
        Transform::from_xyz(2000.0, 1000.0, 2000.0).looking_at(Vec3::ZERO, Vec3::Y),
        RotatingLight,
    ));
}

fn add_assets(
    mut commands: Commands,
    mut materials: ResMut<Assets<StandardMaterial>>,
    asset_server: Res<AssetServer>,
) {
    let textures = EarthTexture {
        // Since the file is too large, so i add it to .gitignore
        // Here is the texture's link, where u can download from it.
        // https://eoimages.gsfc.nasa.gov/images/imagerecords/74000/74167/world.200410.3x21600x10800.png
        base_color: asset_server.load("world.png"),
        metallic_roughness: asset_server.load("specular_map_inverted_8k.png"),

        normal_map: asset_server.load("height.png"),
    };

    let box_material_handle = materials.add(StandardMaterial {
        base_color_texture: Some(textures.base_color.clone()),
        metallic_roughness_texture: Some(textures.metallic_roughness.clone()),
        perceptual_roughness: 1.,
        normal_map_texture: Some(textures.normal_map.clone()),
        ..default()
    });
    commands.insert_resource(BoxMaterialHandle(box_material_handle));

    commands.insert_resource(textures);
}

fn check_ready(
    mut progress: ResMut<LoadingProgress>,
    textures: Res<EarthTexture>,
    asset_server: Res<AssetServer>,
    mut next_state: ResMut<NextState<GameState>>,
) {
    let mut loaded = 0;
    if asset_server.is_loaded_with_dependencies(&textures.base_color) {
        loaded += 1;
    }
    if asset_server.is_loaded_with_dependencies(&textures.metallic_roughness) {
        loaded += 1;
    }
    if asset_server.is_loaded_with_dependencies(&textures.normal_map) {
        loaded += 1;
    }

    progress.texture = loaded;

    if progress.is_complete() {
        next_state.set(GameState::PostLoading);
    }
}

fn rotate_light(time: Res<Time>, mut transform: Single<&mut Transform, With<RotatingLight>>) {
    // rotate around y-axis
    let rotation_speed = 0.5;
    let angle = time.elapsed_secs() * rotation_speed;

    let x = angle.cos() * 2000.0;
    let z = angle.sin() * 2000.0;

    transform.translation = Vec3::new(x, 1000.0, z);
    *transform.into_inner() = transform.looking_at(Vec3::ZERO, Vec3::Y);
}

fn spawn_task(mut commands: Commands) {
    let faces = [
        Vec3::X,
        Vec3::NEG_X,
        Vec3::Y,
        Vec3::NEG_Y,
        Vec3::Z,
        Vec3::NEG_Z,
    ];

    let offsets = [(0.0, 0.0), (0.0, 1.0), (1.0, 0.0), (1.0, 1.0)];

    let id = commands
        .spawn((
            Transform::default(),
            Visibility::Hidden,
            Earth,
            Name::new("Earth"),
        ))
        .observe(rotate_earth)
        .observe(zoom)
        .id();

    let thread_pool = AsyncComputeTaskPool::get();

    for direction in faces {
        for offset in offsets {
            let entity = commands.spawn_empty().id();
            commands.entity(id).add_child(entity);

            let task = thread_pool.spawn(async move {
                let mut command_queue = CommandQueue::default();

                let face = generate_face(direction, TOTAL_MESH_COUNT, offset.0, offset.1);

                command_queue.push(move |world: &mut World| {
                    let (mesh, materal) = {
                        let (mut mesh_handle, materal_handle) =
                            SystemState::<(ResMut<Assets<Mesh>>, Res<BoxMaterialHandle>)>::new(
                                world,
                            )
                            .get_mut(world);

                        (mesh_handle.add(face), materal_handle.clone())
                    };
                    world.entity_mut(entity).insert((
                        Mesh3d(mesh),
                        MeshMaterial3d(materal),
                        Visibility::Inherited,
                    ));
                });

                command_queue
            });

            commands.entity(entity).insert(ComputeMesh(task));
        }
    }
}

fn handle_tasks(
    mut commands: Commands,
    mut transform_tasks: Query<(Entity, &mut ComputeMesh)>,
    mut progress: ResMut<LoadingProgress>,
) {
    // Limit how many tasks we process per frame to avoid freezing the main thread
    // when dealing with large meshes (e.g., TOTAL_MESH_COUNT = 800)
    // const MAX_TASKS_PER_FRAME: usize = 1;
    // let mut processed = 0;

    for (entity, mut task) in &mut transform_tasks {
        // IMPORTANT: Check the limit BEFORE calling check_ready to avoid dropping CommandQueues
        // if processed >= MAX_TASKS_PER_FRAME {
        //     break; // Skip checking this task, leave it for next frame
        // }

        // Use `check_ready` to efficiently poll the task without blocking the main thread.
        if let Some(mut commands_queue) = futures::check_ready(&mut task.0) {
            // Append the returned command queue to execute it later.
            commands.append(&mut commands_queue);
            // Task is complete, so remove the task component from the entity.
            commands.entity(entity).remove::<ComputeMesh>();

            progress.mesh += 1;
            // processed += 1;
        }
    }
}

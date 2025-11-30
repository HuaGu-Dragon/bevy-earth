use std::f32::consts::PI;

use bevy::{
    asset::RenderAssetUsages,
    dev_tools::picking_debug::{DebugPickingMode, DebugPickingPlugin},
    ecs::{system::SystemState, world::CommandQueue},
    mesh::{self, PrimitiveTopology},
    picking::prelude::*,
    prelude::*,
    tasks::{AsyncComputeTaskPool, Task, futures},
};
use bevy_egui::{EguiContexts, EguiPlugin, EguiPrimaryContextPass, egui};
use bevy_inspector_egui::quick::WorldInspectorPlugin;

const EARTH_RADIUS: Vec3 = Vec3::new(1000., 1000., 1000.);

const TOTAL_MESH_COUNT: u32 = 800;

#[derive(States, Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
enum GameState {
    #[default]
    Loading,
    PostLoading,
    Playing,
}

#[derive(Resource)]
struct EarthTexture {
    base_color: Handle<Image>,
    metallic_roughness: Handle<Image>,
    normal_map: Handle<Image>,
}

#[derive(Resource, Default)]
struct LoadingProgress {
    mesh: usize,
    texture: usize,
}

#[derive(Component)]
struct ComputeMesh(Task<CommandQueue>);

#[derive(Resource, Deref)]
struct BoxMaterialHandle(Handle<StandardMaterial>);

impl LoadingProgress {
    fn progress(&self) -> f32 {
        (self.texture as f32 / 3.) * 0.7 + (self.mesh as f32 / 24.) * 0.3
    }

    fn is_complete(&self) -> bool {
        self.texture >= 3 && self.mesh >= 24
    }
}

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(EguiPlugin::default())
        .add_plugins(
            WorldInspectorPlugin::default().run_if(
                bevy::input::common_conditions::input_toggle_active(true, KeyCode::Escape)
                    .and(in_state(GameState::Playing)),
            ),
        )
        .add_plugins((MeshPickingPlugin, DebugPickingPlugin))
        .insert_resource(DebugPickingMode::Disabled)
        .init_state::<GameState>()
        .init_resource::<LoadingProgress>()
        .add_systems(Startup, (add_assets, setup_camera, spawn_task))
        .add_systems(
            EguiPrimaryContextPass,
            display_loading_screen
                .run_if(in_state(GameState::Loading).or(in_state(GameState::PostLoading))),
        )
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

#[derive(Component)]
struct RotatingLight;

#[derive(Component)]
struct Earth;

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

fn display_loading_screen(
    mut contexts: EguiContexts,
    progress: Res<LoadingProgress>,
    mut is_initialized: Local<bool>,
) -> Result {
    let ctx = contexts.ctx_mut()?;

    if !*is_initialized {
        *is_initialized = true;
        egui_extras::install_image_loaders(ctx);
    }

    egui::Area::new("Left".into())
        .anchor(egui::Align2::LEFT_BOTTOM, [0., 0.])
        .show(ctx, |ui| {
            ui.image(egui::include_image!("../assets/loading_left.gif"));
        });

    egui::Window::new("Loading")
        .anchor(egui::Align2::CENTER_CENTER, [0., 0.])
        .collapsible(false)
        .resizable(false)
        .title_bar(false)
        .show(ctx, |ui| {
            ui.vertical_centered(|ui| {
                ui.add_space(10.);

                ui.heading("Loading...");
                ui.add_space(20.);

                let bar = egui::ProgressBar::new(progress.progress()).desired_width(300.);
                ui.add(bar);
                ui.add_space(10.);

                if progress.mesh < 24 {
                    ui.label(format!("Loading meshes ({}/{})", progress.mesh, 24));
                } else if progress.texture < 3 {
                    ui.label(format!("Loading textures ({}/3)", progress.texture));
                } else {
                    ui.label("Loading complete");
                }

                ui.add_space(10.);
            });
        });

    egui::Area::new("Right".into())
        .anchor(egui::Align2::RIGHT_BOTTOM, [0., 0.])
        .show(ctx, |ui| {
            ui.image(egui::include_image!("../assets/loading_right.gif"));
        });
    Ok(())
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

pub fn generate_face(normal: Vec3, resolution: u32, x_offset: f32, y_offset: f32) -> Mesh {
    let axis_a = Vec3::new(normal.y, normal.z, normal.x); // Horizontal
    let axis_b = axis_a.cross(normal); // Vertical

    // Create a vec of verticies and indicies
    let mut verticies: Vec<Vec3> = Vec::new();

    let mut indicies: Vec<u32> = Vec::new();
    let mut normals = Vec::new();
    let mut first_longitude = 0.;

    // Create a new vec containing our uv coords
    let mut uvs = Vec::new();

    for y in 0..(resolution) {
        for x in 0..(resolution) {
            let i = x + y * resolution;

            let percent = Vec2::new(x as f32, y as f32) / (resolution - 1) as f32;
            let point_on_unit_cube =
                normal + (percent.x - x_offset) * axis_a + (percent.y - y_offset) * axis_b;

            // Convert our point_coords into `Coordinates`
            let point_coords: Coordinates = point_on_unit_cube.normalize().into();
            let normalized_point = point_on_unit_cube.normalize() * EARTH_RADIUS;

            verticies.push(normalized_point);

            let (mut u, v) = point_coords.convert_to_uv_mercator();
            let lon = point_coords.longitude;
            let lat = point_coords.latitude;

            if y == 0 && x == 0 {
                first_longitude = lon;
            }

            // In the middle latitudes, if we start on a
            // negative longitude but then wind up crossing to a
            // positive longitude, set u to 0.0 to prevent a seam
            if first_longitude < 0.0 && lon > 0.0 && lat < 89.0 && lat > -89.0 {
                u = 0.0;
            }

            // If we are below -40 degrees latitude and the tile
            // starts at 180 degrees, set u to 0.0 to prevent a seam
            if x == 0 && lon == 180.0 && lat < -40.0 {
                u = 0.0;
            }

            normals.push(-point_on_unit_cube.normalize());

            uvs.push([u, v]);

            if x != resolution - 1 && y != resolution - 1 {
                // First triangle
                indicies.push(i);
                indicies.push(i + resolution);
                indicies.push(i + resolution + 1);

                // Second triangle
                indicies.push(i);
                indicies.push(i + resolution + 1);
                indicies.push(i + 1);
            }
        }
    }
    let indicies = mesh::Indices::U32(indicies);
    let mut mesh = Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::all());
    mesh.insert_indices(indicies);
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, verticies);
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
    // Insert the UV attribute along with our uv vec
    mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs);
    mesh.generate_tangents().unwrap();
    mesh
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
    for (entity, mut task) in &mut transform_tasks {
        // Use `check_ready` to efficiently poll the task without blocking the main thread.
        if let Some(mut commands_queue) = futures::check_ready(&mut task.0) {
            // Append the returned command queue to execute it later.
            commands.append(&mut commands_queue);
            // Task is complete, so remove the task component from the entity.
            commands.entity(entity).remove::<ComputeMesh>();

            progress.mesh += 1;
        }
    }
}

fn rotate_earth(drag: On<Pointer<Drag>>, mut transforms: Query<&mut Transform>) {
    if let Ok(mut transform) = transforms.get_mut(drag.entity) {
        transform.rotate_y(drag.delta.x * 0.02);
        transform.rotate_x(drag.delta.y * 0.02);
    }
}

fn zoom(scroll: On<Pointer<Scroll>>, camera: Single<&mut Projection, With<Camera>>) {
    if let Projection::Perspective(ref mut perspective) = *camera.into_inner() {
        let delta_zoom = -scroll.y * 0.05;

        perspective.fov = (perspective.fov + delta_zoom).clamp(0.05, PI / 4.);
    }
}

fn map(input_range: (f32, f32), output_range: (f32, f32), value: f32) -> f32 {
    let (in_min, in_max) = input_range;
    let (out_min, out_max) = output_range;

    // Linear interpolation: map value from input_range to output_range
    let normalized = (value - in_min) / (in_max - in_min);
    out_min + normalized * (out_max - out_min)
}

fn map_latitude(lat: f32) -> Result<f32, String> {
    // 90 -> 0 maps to 0.0 to 0.5
    // 0 -> -90 maps to 0.5 to 1.0
    // Ensure latitude is valid
    if !(-90.0..=90.0).contains(&lat) {
        return Err("Invalid latitude: {lat:?}".to_string());
    }
    if (0.0..=90.0).contains(&lat) {
        Ok(map((90.0, 0.0), (0.0, 0.5), lat))
    } else {
        Ok(map((0.0, -90.0), (0.5, 1.0), lat))
    }
}

fn map_longitude(lon: f32) -> Result<f32, String> {
    // -180 -> 0 maps to 0.0 to 0.5
    // 0 -> 180 maps to 0.5 to 1.0
    //Ensure longitude is valid
    if !(-180.0..=180.0).contains(&lon) {
        return Err("Invalid longitude: {lon:?}".to_string());
    }
    if (-180.0..=0.0).contains(&lon) {
        Ok(map((-180.0, 0.0), (0.0, 0.5), lon))
    } else {
        Ok(map((0.0, 180.0), (0.5, 1.0), lon))
    }
}

#[derive(Debug)]
pub struct Coordinates {
    // Stored internally in radians
    pub latitude: f32,
    pub longitude: f32,
}

impl From<Vec3> for Coordinates {
    fn from(value: Vec3) -> Self {
        let normalized_point = value.normalize();
        let latitude = normalized_point.y.asin();
        let longitude = normalized_point.x.atan2(normalized_point.z);
        Coordinates {
            latitude,
            longitude,
        }
    }
}

impl Coordinates {
    pub fn as_degrees(&self) -> (f32, f32) {
        let latitude = self.latitude * (180.0 / PI);
        let longitude = self.longitude * (180.0 / PI);
        (latitude, longitude)
    }

    pub fn convert_to_uv_mercator(&self) -> (f32, f32) {
        let (lat, lon) = self.as_degrees();
        let v = map_latitude(lat).unwrap();
        let u = map_longitude(lon).unwrap();
        (u, v)
    }

    pub fn from_degrees(latitude: f32, longitude: f32) -> Result<Self, String> {
        if !(-90.0..=90.0).contains(&latitude) {
            return Err("Invalid latitude: {lat:?}".to_string());
        }
        if !(-180.0..=180.0).contains(&longitude) {
            return Err("Invalid longitude: {lon:?}".to_string());
        }
        let latitude = latitude / (180.0 / PI);
        let longitude = longitude / (180.0 / PI);
        Ok(Coordinates {
            latitude,
            longitude,
        })
    }

    pub fn get_point_on_sphere(&self) -> Vec3 {
        let y = self.latitude.sin();
        let r = self.latitude.cos();
        let x = self.longitude.sin() * -r;
        let z = self.longitude.cos() * r;
        Vec3::new(x, y, z).normalize() * EARTH_RADIUS
    }
}

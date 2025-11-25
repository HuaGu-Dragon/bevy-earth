use bevy::{
    asset::RenderAssetUsages,
    dev_tools::picking_debug::{DebugPickingMode, DebugPickingPlugin},
    mesh::{self, PrimitiveTopology},
    picking::prelude::*,
    prelude::*,
};
use bevy_inspector_egui::{bevy_egui::EguiPlugin, quick::WorldInspectorPlugin};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(EguiPlugin::default())
        .add_plugins(WorldInspectorPlugin::default().run_if(
            bevy::input::common_conditions::input_toggle_active(true, KeyCode::Escape),
        ))
        .add_plugins((MeshPickingPlugin, DebugPickingPlugin))
        .insert_resource(DebugPickingMode::Normal)
        .add_systems(Startup, (setup_camera, generate_faces))
        .add_systems(Update, rotate_light)
        .run();
}

#[derive(Component)]
struct RotatingLight;

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

fn rotate_light(time: Res<Time>, mut query: Query<&mut Transform, With<RotatingLight>>) {
    for mut transform in &mut query {
        // rotate around y-axis
        let rotation_speed = 0.5;
        let angle = time.elapsed_secs() * rotation_speed;

        let x = angle.cos() * 2000.0;
        let z = angle.sin() * 2000.0;

        transform.translation = Vec3::new(x, 1000.0, z);
        *transform = transform.looking_at(Vec3::ZERO, Vec3::Y);
    }
}

pub fn generate_face(normal: Vec3, resolution: u32, x_offset: f32, y_offset: f32) -> Mesh {
    let axis_a = Vec3::new(normal.y, normal.z, normal.x); // Horizontal
    let axis_b = axis_a.cross(normal); // Vertical

    // Create a vec of verticies and indicies
    let mut verticies: Vec<Vec3> = Vec::new();
    let mut indicies: Vec<u32> = Vec::new();
    let mut normals = Vec::new();
    let mut uvs = Vec::new();
    for y in 0..(resolution) {
        for x in 0..(resolution) {
            let i = x + y * resolution;

            let percent = Vec2::new(x as f32, y as f32) / (resolution - 1) as f32;
            let point_on_unit_cube =
                normal + (percent.x - x_offset) * axis_a + (percent.y - y_offset) * axis_b;

            const EARTH_RADIUS: Vec3 = Vec3::new(1000., 1000., 1000.);
            let normalized_point = point_on_unit_cube.normalize() * EARTH_RADIUS;

            verticies.push(normalized_point);

            normals.push(-point_on_unit_cube.normalize());

            uvs.push([percent.x, percent.y]);

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
    mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs);
    mesh.generate_tangents().unwrap();
    mesh
}

pub fn generate_faces(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let faces = vec![
        Vec3::X,
        Vec3::NEG_X,
        Vec3::Y,
        Vec3::NEG_Y,
        Vec3::Z,
        Vec3::NEG_Z,
    ];

    let offsets = vec![(0.0, 0.0), (0.0, 1.0), (1.0, 0.0), (1.0, 1.0)];

    for direction in faces {
        for offset in &offsets {
            commands.spawn((
                Mesh3d(meshes.add(generate_face(direction, 16, offset.0, offset.1))),
                MeshMaterial3d(materials.add(StandardMaterial { ..default() })),
            ));
        }
    }
}

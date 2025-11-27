use std::f32::consts::PI;

use bevy::{
    asset::RenderAssetUsages,
    dev_tools::picking_debug::{DebugPickingMode, DebugPickingPlugin},
    mesh::{self, PrimitiveTopology},
    picking::prelude::*,
    prelude::*,
};
use bevy_inspector_egui::{bevy_egui::EguiPlugin, quick::WorldInspectorPlugin};

const EARTH_RADIUS: Vec3 = Vec3::new(1000., 1000., 1000.);

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
    let mut transform = query.single_mut().unwrap();

    // rotate around y-axis
    let rotation_speed = 0.5;
    let angle = time.elapsed_secs() * rotation_speed;

    let x = angle.cos() * 2000.0;
    let z = angle.sin() * 2000.0;

    transform.translation = Vec3::new(x, 1000.0, z);
    *transform = transform.looking_at(Vec3::ZERO, Vec3::Y);
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

pub fn generate_faces(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    asset: Res<AssetServer>,
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

    commands
        .spawn((Transform::default(), Visibility::default()))
        .with_children(|com| {
            for direction in faces {
                for offset in &offsets {
                    com.spawn((
                        Mesh3d(meshes.add(generate_face(direction, 800, offset.0, offset.1))),
                        MeshMaterial3d(materials.add(StandardMaterial {
                            // Since the file is too large, so i add it to .gitignore
                            // Here is the texture's link, where u can download from it.
                            // https://eoimages.gsfc.nasa.gov/images/imagerecords/74000/74167/world.200410.3x21600x10800.png
                            base_color_texture: Some(asset.load("world.png")),
                            metallic_roughness_texture: Some(
                                asset.load("specular_map_inverted_8k.png"),
                            ),
                            perceptual_roughness: 1.,
                            normal_map_texture: Some(asset.load("height.png")),
                            ..default()
                        })),
                    ));
                }
            }
        })
        .observe(rotate_earth);
}

fn rotate_earth(drag: On<Pointer<Drag>>, mut transforms: Query<&mut Transform>) {
    if let Ok(mut transform) = transforms.get_mut(drag.entity) {
        transform.rotate_y(drag.delta.x * 0.02);
        transform.rotate_x(drag.delta.y * 0.02);
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

    #[allow(dead_code)]
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

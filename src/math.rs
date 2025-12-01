use std::f32::consts::PI;

use bevy::{
    asset::RenderAssetUsages,
    math::Vec3,
    mesh::{self, Mesh, PrimitiveTopology},
};
use bevy_egui::egui::Vec2;

use crate::EARTH_RADIUS;

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

    // pub fn from_degrees(latitude: f32, longitude: f32) -> Result<Self, String> {
    //     if !(-90.0..=90.0).contains(&latitude) {
    //         return Err("Invalid latitude: {lat:?}".to_string());
    //     }
    //     if !(-180.0..=180.0).contains(&longitude) {
    //         return Err("Invalid longitude: {lon:?}".to_string());
    //     }
    //     let latitude = latitude / (180.0 / PI);
    //     let longitude = longitude / (180.0 / PI);
    //     Ok(Coordinates {
    //         latitude,
    //         longitude,
    //     })
    // }

    // pub fn get_point_on_sphere(&self) -> Vec3 {
    //     let y = self.latitude.sin();
    //     let r = self.latitude.cos();
    //     let x = self.longitude.sin() * -r;
    //     let z = self.longitude.cos() * r;
    //     Vec3::new(x, y, z).normalize() * EARTH_RADIUS
    // }
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

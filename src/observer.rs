use std::f32::consts::PI;

use bevy::{
    camera::{Camera, Projection},
    ecs::{
        observer::On,
        query::With,
        system::{Query, Single},
    },
    picking::events::{Drag, Pointer, Scroll},
    transform::components::Transform,
};

pub fn rotate_earth(drag: On<Pointer<Drag>>, mut transforms: Query<&mut Transform>) {
    if let Ok(mut transform) = transforms.get_mut(drag.entity) {
        transform.rotate_y(drag.delta.x * 0.02);
        transform.rotate_x(drag.delta.y * 0.02);
    }
}

pub fn zoom(scroll: On<Pointer<Scroll>>, camera: Single<&mut Projection, With<Camera>>) {
    if let Projection::Perspective(ref mut perspective) = *camera.into_inner() {
        let delta_zoom = -scroll.y * 0.05;

        perspective.fov = (perspective.fov + delta_zoom).clamp(0.05, PI / 4.);
    }
}

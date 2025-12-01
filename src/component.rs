use bevy::{
    ecs::{component::Component, world::CommandQueue},
    tasks::Task,
};

#[derive(Component)]
pub struct ComputeMesh(pub Task<CommandQueue>);

#[derive(Component)]
pub struct RotatingLight;

#[derive(Component)]
pub struct Earth;

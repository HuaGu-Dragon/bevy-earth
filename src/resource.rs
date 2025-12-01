use bevy::{
    asset::Handle, ecs::resource::Resource, image::Image, pbr::StandardMaterial, prelude::Deref,
};

#[derive(Resource)]
pub struct EarthTexture {
    pub base_color: Handle<Image>,
    pub metallic_roughness: Handle<Image>,
    pub normal_map: Handle<Image>,
}

#[derive(Resource, Default)]
pub struct LoadingProgress {
    pub mesh: usize,
    pub texture: usize,
}

#[derive(Resource, Deref)]
pub struct BoxMaterialHandle(pub Handle<StandardMaterial>);

impl LoadingProgress {
    pub fn progress(&self) -> f32 {
        (self.texture as f32 / 3.) * 0.7 + (self.mesh as f32 / 24.) * 0.3
    }

    pub fn is_complete(&self) -> bool {
        self.texture >= 3 && self.mesh >= 24
    }
}

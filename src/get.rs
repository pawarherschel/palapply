use glam::IVec2;

pub trait Get {
    fn get(&self, factor: f32, coord: IVec2) -> f32;
}

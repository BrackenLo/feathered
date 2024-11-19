//====================================================================

pub mod model_renderer;
pub mod texture_renderer;

//====================================================================

pub mod prelude {
    pub use crate::{
        model_renderer::{Mesh, Model, ModelRendererPlugin},
        texture_renderer::{Sprite, TextureRenderer},
    };
}

//====================================================================

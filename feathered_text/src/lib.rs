//====================================================================

use feathered_render_tools::{Device, SetupRendererComponents};
use feathered_shipyard::prelude::*;
use shipyard::{AllStoragesView, SystemModificator, Unique};
use text_atlas::TextAtlas;

pub mod text3d;
pub mod text_atlas;

pub use cosmic_text::{Attrs, Color, Metrics};

//====================================================================

pub struct CoreTextPlugin;
impl Plugin for CoreTextPlugin {
    fn build_plugin(self, builder: &mut WorkloadBuilder) {
        builder
            .add_workload_first(
                Setup,
                sys_setup_text_components.after_all(SetupRendererComponents),
            )
            .add_workload(Last, sys_trim_atlas);
    }
}

//====================================================================

#[derive(Unique)]
pub struct FontSystem(cosmic_text::FontSystem);
impl FontSystem {
    #[inline]
    pub fn inner(&self) -> &cosmic_text::FontSystem {
        &self.0
    }
}

#[derive(Unique)]
pub struct SwashCache(cosmic_text::SwashCache);
impl SwashCache {
    #[inline]
    pub fn inner(&self) -> &cosmic_text::SwashCache {
        &self.0
    }
}

fn sys_setup_text_components(all_storages: AllStoragesView, device: Res<Device>) {
    all_storages
        .insert(FontSystem(cosmic_text::FontSystem::new()))
        .insert(SwashCache(cosmic_text::SwashCache::new()))
        .insert(TextAtlas::new(device.inner()));
}

fn sys_trim_atlas(mut atlas: ResMut<TextAtlas>) {
    atlas.post_render_trim();
}

//====================================================================

//====================================================================

//====================================================================

pub use feathered_common as common;

pub use feathered_physics as physics;

pub use feathered_pipelines as pipelines;

pub use feathered_render_tools as render_tools;

pub use feathered_runner as runner;

pub use feathered_shipyard as shipyard;

pub use feathered_spatial as spatial;

#[cfg(feature = "text")]
pub use feathered_text as text;

pub use feathered_tools as tools;

//====================================================================

pub struct DefaultPlugins;
impl feathered_shipyard::prelude::Plugin for DefaultPlugins {
    fn build_plugin(self, builder: &mut feathered_shipyard::prelude::WorkloadBuilder) {
        builder
            .add_plugin(feathered_common::CommonPlugin)
            .add_plugin(feathered_render_tools::FullRenderToolsPlugin)
            .add_plugin(feathered_spatial::SpatialPlugin)
            .add_plugin(feathered_tools::input::InputPlugin);
    }
}

//====================================================================

//====================================================================

pub mod common {
    pub use feathered_common::{Size, WindowRaw, WindowResizeEvent, WindowSize};
}

pub mod render_tools {
    pub use feathered_render_tools::{
        camera, shared,
        texture::{DepthTexture, Texture},
        tools, ClearColor, Device, FullRenderToolsPlugin, Queue, RenderComponentsPlugin,
        RenderEncoder, RenderPass, RenderPassDesc, RenderUtilsPlugin, Surface, SurfaceConfig,
        Vertex,
    };
}

pub mod runner {
    pub use feathered_runner::{events::WindowInputEvent, window::Window, Runner};
}

pub mod shipyard {
    pub use feathered_shipyard::{
        builder::{
            First, Label, Last, Plugin, Render, Setup, Stage, SubStages, Update, WorkloadBuilder,
        },
        events::{Event, EventBuilder, EventHandle},
        prelude,
        tools::{UniqueTools, WorldTools},
        Res, ResMut,
    };

    pub mod tools {
        pub use feathered_shipyard::{
            builder::{register_main_stages, StageData, WorkloadBuilderInner, WorkloadToBuild},
            runner::WorkloadRunner,
        };
    }
}

pub mod spatial {
    pub use feathered_spatial::Transform;
}

pub mod tools {
    pub use feathered_tools::input::{Input, InputPlugin, KeyboardPlugin, MouseInput, MousePlugin};
}

//====================================================================

pub mod systems {
    pub use feathered_render_tools::{
        shared::sys_setup_shared_resources,
        sys_finish_main_render_pass, sys_setup_encoder, sys_setup_render_pass,
        sys_setup_renderer_components, sys_submit_encoder,
        texture::{sys_resize_depth_texture, sys_setup_depth_texture},
    };
}

//====================================================================

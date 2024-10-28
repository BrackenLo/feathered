//====================================================================

pub mod common {
    pub use feathered_common::{Size, WindowRaw, WindowResizeEvent, WindowSize};
}

pub mod render_tools {
    pub use feathered_render_tools::{
        camera,
        texture::{DepthTexture, Texture},
        tools, ClearColor, Device, Queue, RenderEncoder, RenderPass, RenderPassDesc, Surface,
        SurfaceConfig, Vertex,
    };
}

pub mod runner {
    pub use feathered_runner::{events::WindowInputEvent, window::Window, Runner};
}

pub mod shipyard {
    pub use feathered_shipyard::{
        builder::{
            First, FixedUpdate, Label, Last, Plugin, Render, Setup, Stage, SubStages, Update,
            WorkloadBuilder,
        },
        events::{Event, EventBuilder, EventHandle},
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

pub mod systems {
    pub use feathered_render_tools::{
        sys_finish_main_render_pass, sys_setup_encoder, sys_setup_render_pass,
        sys_setup_renderer_components, sys_submit_encoder,
    };
}

//====================================================================

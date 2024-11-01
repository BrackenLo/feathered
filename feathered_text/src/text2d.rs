//====================================================================

use feathered_common::Size;
use feathered_render_tools::{
    camera::{Camera, OrthographicCamera},
    Device, Queue, Vertex,
};
use feathered_shipyard::{Res, ResMut};
use shipyard::{Component, IntoIter, Unique, ViewMut};

use crate::{
    shared::{TextBuffer, TextVertex},
    text_atlas::TextAtlas,
    FontSystem, SwashCache,
};

//====================================================================

fn sys_prep_text(
    device: Res<Device>,
    queue: Res<Queue>,
    mut font_system: ResMut<FontSystem>,
    mut swash_cache: ResMut<SwashCache>,
    mut text_atlas: ResMut<TextAtlas>,

    mut vm_text_buffer: ViewMut<Text2dBuffer>,
) {
    (&mut vm_text_buffer).iter().for_each(|text_buffer| {
        if let Some(rebuild) = crate::shared::prep(
            device.inner(),
            queue.inner(),
            &mut font_system.0,
            &mut swash_cache.0,
            &mut text_atlas,
            &mut text_buffer.text_buffer,
        ) {
            feathered_render_tools::tools::update_instance_buffer(
                device.inner(),
                queue.inner(),
                "Text3d Vertex Buffer",
                &mut text_buffer.text_buffer.vertex_buffer,
                &mut text_buffer.text_buffer.vertex_count,
                &rebuild,
            );
        }
    });
}

//====================================================================

#[derive(Component)]
pub struct Text2dBuffer {
    text_buffer: TextBuffer,
}

//====================================================================

#[derive(Unique)]
pub struct Text2dRenderer {
    pipeline: wgpu::RenderPipeline,
    instance_buffer_bind_group_layout: wgpu::BindGroupLayout,

    orthographic_projection: OrthographicCamera,
    ui_camera: Camera,
}

impl Text2dRenderer {
    pub fn new(
        device: &wgpu::Device,
        config: &wgpu::SurfaceConfiguration,
        text_atlas: &TextAtlas,
        window_size: Size<u32>,
    ) -> Self {
        let orthographic_projection =
            OrthographicCamera::new_sized(window_size.width as f32, window_size.height as f32);

        let ui_camera = Camera::new(device, &orthographic_projection);

        let instance_buffer_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Text 2d Renderer Instance Buffer Bind Group Layout"),
                entries: &[feathered_render_tools::tools::bgl_uniform_entry(
                    0,
                    wgpu::ShaderStages::VERTEX,
                )],
            });

        let pipeline = feathered_render_tools::tools::create_pipeline(
            device,
            config,
            "Text2dRenderer",
            &[
                ui_camera.bind_group_layout(),
                text_atlas.bind_group_layout(),
                &instance_buffer_bind_group_layout,
            ],
            &[TextVertex::desc()],
            include_str!("text2d.wgsl"),
            feathered_render_tools::tools::RenderPipelineDescriptor {
                primitive: wgpu::PrimitiveState {
                    topology: wgpu::PrimitiveTopology::TriangleStrip,
                    cull_mode: Some(wgpu::Face::Back),
                    ..Default::default()
                },
                fragment_targets: Some(&[Some(wgpu::ColorTargetState {
                    format: config.format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::all(),
                })]),
                ..Default::default()
            }
            .with_backface_culling(),
        );

        Self {
            pipeline,
            instance_buffer_bind_group_layout,

            orthographic_projection,
            ui_camera,
        }
    }
}

//====================================================================

//====================================================================

use feathered_render_tools::{
    camera::MainCamera, Device, Queue, RenderPass, SurfaceConfig, Vertex,
};
use feathered_shipyard::prelude::*;
use feathered_spatial::Transform;
use shipyard::{AllStoragesView, Component, IntoIter, SystemModificator, Unique, View, ViewMut};
use wgpu::util::DeviceExt;

use crate::{
    shared::{TextBuffer, TextBufferDescriptor, TextVertex},
    text_atlas::TextAtlas,
    CoreTextPlugin, FontSystem, SwashCache,
};

//====================================================================

pub struct Text3dPlugin;
impl Plugin for Text3dPlugin {
    fn build_plugin(self, builder: &mut WorkloadBuilder) {
        builder
            .add_plugin(CoreTextPlugin)
            .add_workload_pre(Setup, sys_setup_text_renderer)
            .add_workload_last(Update, (sys_prep_text, sys_prep_text_transform))
            .add_workload(
                Render,
                sys_render_text.skip_if_missing_unique::<RenderPass>(),
            );
    }
}

fn sys_setup_text_renderer(
    all_storages: AllStoragesView,
    device: Res<Device>,
    config: Res<SurfaceConfig>,
    text_atlas: Res<TextAtlas>,
    camera: Res<MainCamera>,
) {
    all_storages.insert(Text3dRenderer::new(
        device.inner(),
        config.inner(),
        &text_atlas,
        camera.bind_group_layout(),
    ));
}

fn sys_prep_text(
    device: Res<Device>,
    queue: Res<Queue>,
    mut font_system: ResMut<FontSystem>,
    mut swash_cache: ResMut<SwashCache>,
    mut text_atlas: ResMut<TextAtlas>,

    mut vm_text_buffer: ViewMut<Text3dBuffer>,
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

fn sys_prep_text_transform(
    queue: Res<Queue>,
    v_text_buffer: View<Text3dBuffer>,
    v_transform: View<Transform>,
) {
    (&v_transform, &v_text_buffer)
        .iter()
        .for_each(|(transform, text_buffer)| {
            text_buffer.update_transform(queue.inner(), transform);
        });
}

fn sys_render_text(
    mut render_pass: ResMut<RenderPass>,
    renderer: Res<Text3dRenderer>,
    text_atlas: Res<TextAtlas>,
    v_text_buffer: View<Text3dBuffer>,
    camera: Res<MainCamera>,
) {
    renderer.render(
        render_pass.pass(),
        &text_atlas,
        camera.bind_group(),
        v_text_buffer.iter(),
    )
}

//====================================================================

#[derive(Component)]
pub struct Text3dBuffer {
    text_buffer: TextBuffer,

    // 3d Transform
    uniform_buffer: wgpu::Buffer,
    uniform_bind_group: wgpu::BindGroup,
}

impl Text3dBuffer {
    pub fn new(
        device: &wgpu::Device,
        text3d_renderer: &mut Text3dRenderer,
        font_system: &mut cosmic_text::FontSystem,
        desc: &TextBufferDescriptor,
        transform: Transform,
    ) -> Self {
        let text_buffer = TextBuffer::new(device, font_system, desc);

        let transform_matrix = transform.to_matrix();

        let uniform_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Text 3d Uniform Buffer"),
            contents: bytemuck::cast_slice(&[transform_matrix]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let uniform_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Text 3d Uniform Bind Group"),
            layout: &text3d_renderer.buffer_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::Buffer(uniform_buffer.as_entire_buffer_binding()),
            }],
        });

        Self {
            text_buffer,
            uniform_buffer,
            uniform_bind_group,
        }
    }

    pub fn update_transform(&self, queue: &wgpu::Queue, transform: &Transform) {
        queue.write_buffer(
            &self.uniform_buffer,
            0,
            bytemuck::cast_slice(&[transform.to_matrix()]),
        );
    }
}

//====================================================================

#[derive(Unique)]
pub struct Text3dRenderer {
    pipeline: wgpu::RenderPipeline,
    buffer_bind_group_layout: wgpu::BindGroupLayout,
}

impl Text3dRenderer {
    pub fn new(
        device: &wgpu::Device,
        config: &wgpu::SurfaceConfiguration,
        text_atlas: &TextAtlas,
        camera_bind_group_layout: &wgpu::BindGroupLayout,
    ) -> Self {
        let instance_buffer_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Text 3d Renderer Instance Buffer Bind Group Layout"),
                entries: &[feathered_render_tools::tools::bgl_uniform_entry(
                    0,
                    wgpu::ShaderStages::VERTEX,
                )],
            });

        let pipeline = feathered_render_tools::tools::create_pipeline(
            device,
            config,
            "Text3dRenderer",
            &[
                camera_bind_group_layout,
                text_atlas.bind_group_layout(),
                &instance_buffer_bind_group_layout,
            ],
            &[TextVertex::desc()],
            include_str!("text3d.wgsl"),
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
            .with_depth_stencil()
            .with_backface_culling(),
        );

        Self {
            pipeline,
            buffer_bind_group_layout: instance_buffer_bind_group_layout,
        }
    }

    // pub fn prep<'a>(
    //     &mut self,
    //     device: &wgpu::Device,
    //     queue: &wgpu::Queue,
    //     font_system: &mut cosmic_text::FontSystem,
    //     swash_cache: &mut cosmic_text::SwashCache,
    //     text_atlas: &mut TextAtlas,
    //     buffers: impl IntoIterator<Item = &'a mut Text3dBuffer>,
    // ) {
    //     buffers.into_iter().for_each(|text3d_buffer| {
    //         if let Some(rebuild) = crate::shared::prep::<TextVertex>(
    //             device,
    //             queue,
    //             font_system,
    //             swash_cache,
    //             text_atlas,
    //             &mut text3d_buffer.text_buffer,
    //         ) {
    //             feathered_render_tools::tools::update_instance_buffer(
    //                 device,
    //                 queue,
    //                 "Text3d Vertex Buffer",
    //                 &mut text3d_buffer.text_buffer.vertex_buffer,
    //                 &mut text3d_buffer.text_buffer.vertex_count,
    //                 &rebuild,
    //             );
    //         }
    //     });
    // }

    pub fn render<'a, B>(
        &self,
        pass: &mut wgpu::RenderPass,
        atlas: &TextAtlas,
        camera_bind_group: &wgpu::BindGroup,
        buffers: B,
    ) where
        B: IntoIterator<Item = &'a Text3dBuffer>,
    {
        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, camera_bind_group, &[]);
        pass.set_bind_group(1, atlas.bind_group(), &[]);

        buffers.into_iter().for_each(|buffer| {
            pass.set_vertex_buffer(0, buffer.text_buffer.vertex_buffer.slice(..));
            pass.set_bind_group(2, &buffer.uniform_bind_group, &[]);
            pass.draw(0..4, 0..buffer.text_buffer.vertex_count);
        });
    }
}

//====================================================================

//====================================================================

use std::collections::{HashMap, HashSet};

use feathered_common::WasmWrapper;
use feathered_render_tools::{
    camera::Camera3d,
    shared::{
        SharedRenderResources, TextureRectVertex, TEXTURE_RECT_INDEX_COUNT, TEXTURE_RECT_INDICES,
        TEXTURE_RECT_VERTICES,
    },
    texture::{LoadedTexture, TextureId},
    tools, Device, Queue, RenderPass, SurfaceConfig, Vertex,
};
use feathered_shipyard::prelude::*;
use feathered_spatial::GlobalTransform;
use shipyard::{AllStoragesView, Component, IntoIter, Unique, View};

//====================================================================

pub struct TextureRendererPlugin;
impl Plugin for TextureRendererPlugin {
    fn build_plugin(self, builder: &mut WorkloadBuilder) {
        builder
            .add_workload_pre(Setup, sys_setup_renderer)
            .add_workload(RenderPrep, sys_prep_renderer)
            .add_workload(Render, sys_render);
    }
}

//====================================================================

#[derive(Component, Debug, Clone)]
pub struct Sprite {
    pub texture: LoadedTexture,
    pub size: glam::Vec2,
    pub color: [f32; 4],
}

#[repr(C)]
#[derive(bytemuck::Pod, bytemuck::Zeroable, Clone, Copy, Debug)]
struct TextureInstance {
    pub size: glam::Vec2,
    pub pad: [f32; 2],
    pub transform: glam::Mat4,
    pub color: glam::Vec4,
}

impl Vertex for TextureInstance {
    fn desc<'a>() -> wgpu::VertexBufferLayout<'a> {
        const VERTEX_ATTRIBUTES: [wgpu::VertexAttribute; 6] = wgpu::vertex_attr_array![
            2 => Float32x4, // Transform
            3 => Float32x4,
            4 => Float32x4,
            5 => Float32x4,
            6 => Float32x4, // Color
            7 => Float32x4, // Size
        ];

        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Self>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Instance,
            attributes: &VERTEX_ATTRIBUTES,
        }
    }
}

//====================================================================

#[derive(Unique, Debug)]
pub struct TextureRenderer {
    pipeline: WasmWrapper<wgpu::RenderPipeline>,

    vertex_buffer: WasmWrapper<wgpu::Buffer>,
    index_buffer: WasmWrapper<wgpu::Buffer>,
    index_count: u32,

    texture_storage: HashMap<TextureId, LoadedTexture>,
    instances: HashMap<TextureId, tools::InstanceBuffer<TextureInstance>>,
}

impl TextureRenderer {
    pub fn new(
        device: &wgpu::Device,
        config: &wgpu::SurfaceConfiguration,
        shared: &SharedRenderResources,
    ) -> Self {
        let pipeline = tools::create_pipeline(
            device,
            config,
            "Texture Pipeline",
            &[
                shared.camera_bind_group_layout(),
                shared.texture_bind_group_layout(),
            ],
            &[TextureRectVertex::desc(), TextureInstance::desc()],
            include_str!("shaders/texture.wgsl"),
            tools::RenderPipelineDescriptor {
                primitive: wgpu::PrimitiveState {
                    topology: wgpu::PrimitiveTopology::TriangleStrip,
                    ..Default::default()
                },
                ..Default::default()
            }
            .with_depth_stencil(),
        );

        let vertex_buffer = tools::buffer(
            device,
            tools::BufferType::Vertex,
            "Texture",
            &TEXTURE_RECT_VERTICES,
        );

        let index_buffer = tools::buffer(
            device,
            tools::BufferType::Index,
            "Texture",
            &TEXTURE_RECT_INDICES,
        );
        let index_count = TEXTURE_RECT_INDEX_COUNT;

        let texture_storage = HashMap::default();
        let instances = HashMap::default();

        Self {
            pipeline: WasmWrapper::new(pipeline),
            vertex_buffer: WasmWrapper::new(vertex_buffer),
            index_buffer: WasmWrapper::new(index_buffer),
            index_count,
            texture_storage,
            instances,
        }
    }
}

fn sys_setup_renderer(
    all_storages: AllStoragesView,
    device: Res<Device>,
    config: Res<SurfaceConfig>,
    shared: Res<SharedRenderResources>,
) {
    all_storages.insert(TextureRenderer::new(
        device.inner(),
        config.inner(),
        &shared,
    ));
}

fn sys_prep_renderer(
    device: Res<Device>,
    queue: Res<Queue>,
    mut renderer: ResMut<TextureRenderer>,
    v_global: View<GlobalTransform>,
    v_sprite: View<Sprite>,
) {
    let mut previous = renderer
        .instances
        .keys()
        .map(|id| *id)
        .collect::<HashSet<_>>();

    let instances =
        (&v_global, &v_sprite)
            .iter()
            .fold(HashMap::new(), |mut acc, (global, sprite)| {
                let instance = TextureInstance {
                    size: sprite.size,
                    pad: [0.; 2],
                    transform: global.to_matrix(),
                    color: sprite.color.into(),
                };

                acc.entry(sprite.texture.id())
                    .or_insert_with(|| {
                        if !renderer.instances.contains_key(&sprite.texture.id()) {
                            renderer
                                .texture_storage
                                .insert(sprite.texture.id(), sprite.texture.clone());
                        }

                        Vec::new()
                    })
                    .push(instance);

                acc
            });

    instances.into_iter().for_each(|(id, raw)| {
        previous.remove(&id);

        renderer
            .instances
            .entry(id)
            .and_modify(|instance| instance.update(device.inner(), queue.inner(), &raw))
            .or_insert_with(|| tools::InstanceBuffer::new(device.inner(), &raw));
    });

    previous.into_iter().for_each(|id| {
        log::trace!("Removing texture instance '{}'", id);
        renderer.instances.remove(&id);
        renderer.texture_storage.remove(&id);
    });
}

fn sys_render(mut pass: ResMut<RenderPass>, renderer: Res<TextureRenderer>, camera: Res<Camera3d>) {
    let pass = pass.pass();

    pass.set_pipeline(renderer.pipeline.inner());
    pass.set_bind_group(0, camera.bind_group(), &[]);

    pass.set_vertex_buffer(0, renderer.vertex_buffer.inner().slice(..));
    pass.set_index_buffer(
        renderer.index_buffer.inner().slice(..),
        wgpu::IndexFormat::Uint16,
    );

    renderer
        .instances
        .iter()
        .for_each(|(texture_id, instance)| {
            let texture = renderer.texture_storage.get(texture_id).unwrap();

            pass.set_bind_group(1, texture.bind_group(), &[]);
            pass.set_vertex_buffer(1, instance.slice(..));
            pass.draw_indexed(0..renderer.index_count, 0, 0..instance.count());
        });
}

//====================================================================

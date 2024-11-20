//====================================================================

use std::{
    collections::{HashMap, HashSet},
    sync::{atomic::AtomicU32, Arc},
};

use feathered_common::WasmWrapper;
use feathered_render_tools::{
    camera::Camera3d,
    shared::{ModelVertex, SharedRenderResources},
    texture::{LoadedTexture, TextureId},
    tools, Device, Queue, RenderPass, SurfaceConfig, Vertex,
};
use feathered_shipyard::prelude::*;
use feathered_spatial::GlobalTransform;
use shipyard::{AllStoragesView, Component, IntoIter, Unique};

//====================================================================

pub struct ModelRendererPlugin;
impl Plugin for ModelRendererPlugin {
    fn build_plugin(self, builder: &mut WorkloadBuilder) {
        builder
            .add_workload_pre(Setup, sys_setup_renderer)
            .add_workload(RenderPrep, sys_prep_renderer)
            .add_workload(Render, sys_render);
    }
}

//====================================================================

pub type MeshId = u32;

static CURRENT_MESH_ID: AtomicU32 = AtomicU32::new(0);

#[derive(Clone, Debug)]
pub struct LoadedMesh {
    id: MeshId,
    mesh: Arc<Mesh>,
}

impl LoadedMesh {
    #[inline]
    pub fn load_mesh(mesh: Mesh) -> Self {
        let id = CURRENT_MESH_ID.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        Self {
            id,
            mesh: Arc::new(mesh),
        }
    }

    #[inline]
    pub fn load_from_data(
        device: &wgpu::Device,
        vertices: &[ModelVertex],
        indices: &[u32],
    ) -> Self {
        Self::load_mesh(Mesh::load_mesh(device, vertices, indices))
    }

    #[inline]
    pub fn id(&self) -> MeshId {
        self.id
    }

    #[inline]
    pub fn mesh(&self) -> &Mesh {
        &self.mesh
    }
}

//--------------------------------------------------

#[derive(Debug)]
pub struct Mesh {
    vertex_buffer: WasmWrapper<wgpu::Buffer>,
    index_buffer: WasmWrapper<wgpu::Buffer>,
    index_count: u32,
}

impl Mesh {
    pub fn load_mesh(device: &wgpu::Device, vertices: &[ModelVertex], indices: &[u32]) -> Self {
        let vertex_buffer = tools::buffer(device, tools::BufferType::Vertex, "Mesh", vertices);
        let index_buffer = tools::buffer(device, tools::BufferType::Index, "Mesh", indices);
        let index_count = indices.len() as u32;

        Self {
            vertex_buffer: WasmWrapper::new(vertex_buffer),
            index_buffer: WasmWrapper::new(index_buffer),
            index_count,
        }
    }
}

//--------------------------------------------------

#[derive(Component, Clone)]
pub struct Model {
    pub meshes: Vec<(LoadedMesh, LoadedTexture)>,
    pub color: [f32; 4],
    pub scale: glam::Vec3,
}

impl Model {
    #[inline]
    pub fn from_mesh(mesh: LoadedMesh, texture: LoadedTexture) -> Self {
        Self {
            meshes: vec![(mesh, texture)],
            color: [1., 1., 1., 1.],
            scale: glam::Vec3::ONE,
        }
    }

    #[inline]
    pub fn with_color(mut self, color: [f32; 4]) -> Self {
        self.color = color;
        self
    }

    #[inline]
    pub fn with_scale(mut self, scale: impl Into<glam::Vec3>) -> Self {
        self.scale = scale.into();
        self
    }
}

//====================================================================

#[repr(C)]
#[derive(bytemuck::Pod, bytemuck::Zeroable, Clone, Copy, Debug)]
struct ModelInstance {
    pub transform: glam::Mat4,
    pub color: glam::Vec4,
    pub normal: glam::Mat3,
    pub scale: glam::Vec3,
}

impl Vertex for ModelInstance {
    fn desc<'a>() -> wgpu::VertexBufferLayout<'a> {
        const VERTEX_ATTRIBUTES: [wgpu::VertexAttribute; 9] = wgpu::vertex_attr_array![
            3 => Float32x4, // Transform
            4 => Float32x4,
            5 => Float32x4,
            6 => Float32x4,
            7 => Float32x4, // Color
            8 => Float32x3, // Normal
            9 => Float32x3,
            10 => Float32x3,
            11 => Float32x3, // Scale
        ];

        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<ModelInstance>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Instance,
            attributes: &VERTEX_ATTRIBUTES,
        }
    }
}

//====================================================================

#[derive(Unique, Debug)]
pub struct ModelRenderer {
    pipeline: WasmWrapper<wgpu::RenderPipeline>,

    texture_storage: HashMap<u32, LoadedTexture>,
    mesh_storage: HashMap<u32, LoadedMesh>,
    instances: HashMap<MeshId, HashMap<TextureId, tools::InstanceBuffer<ModelInstance>>>,
}

impl ModelRenderer {
    fn new(
        device: &wgpu::Device,
        config: &wgpu::SurfaceConfiguration,
        shared: &SharedRenderResources,
    ) -> Self {
        let pipeline = tools::create_pipeline(
            device,
            config,
            "Model Pipeline",
            &[
                shared.camera_bind_group_layout(),
                shared.texture_bind_group_layout(),
            ],
            &[ModelVertex::desc(), ModelInstance::desc()],
            include_str!("shaders/model.wgsl"),
            tools::RenderPipelineDescriptor::default()
                .with_depth_stencil()
                .with_backface_culling(),
        );

        Self {
            pipeline: WasmWrapper::new(pipeline),
            texture_storage: HashMap::default(),
            mesh_storage: HashMap::default(),
            instances: HashMap::default(),
        }
    }
}

fn sys_setup_renderer(
    all_storages: AllStoragesView,
    device: Res<Device>,
    config: Res<SurfaceConfig>,
    shared: Res<SharedRenderResources>,
) {
    all_storages.insert(ModelRenderer::new(device.inner(), config.inner(), &shared));
}

fn sys_prep_renderer(
    device: Res<Device>,
    queue: Res<Queue>,
    mut renderer: ResMut<ModelRenderer>,
    v_global: View<GlobalTransform>,
    v_model: View<Model>,
) {
    let mut previous = renderer
        .instances
        .iter()
        .flat_map(|(mesh_id, textures)| textures.keys().map(|texture_id| (*mesh_id, *texture_id)))
        .collect::<HashSet<_>>();

    let mut meshes_used = HashSet::new();
    let mut textures_used = HashSet::new();

    let instances =
        (&v_global, &v_model)
            .iter()
            .fold(HashMap::new(), |mut acc, (transform, model)| {
                model.meshes.iter().for_each(|(mesh, texture)| {
                    let mesh_entry = acc.entry(mesh.id).or_insert_with(|| {
                        if !renderer.mesh_storage.contains_key(&mesh.id) {
                            renderer.mesh_storage.insert(mesh.id, mesh.clone());
                        }

                        meshes_used.insert(mesh.id);

                        HashMap::new()
                    });

                    let rotation = transform.to_scale_rotation_translation().1;
                    let normal_matrix = glam::Mat3::from_quat(rotation);

                    mesh_entry
                        .entry(texture.id())
                        .or_insert_with(|| {
                            if !renderer.texture_storage.contains_key(&texture.id()) {
                                renderer
                                    .texture_storage
                                    .insert(texture.id(), texture.clone());
                            }

                            textures_used.insert(texture.id());

                            Vec::new()
                        })
                        .push(ModelInstance {
                            transform: transform.to_matrix(),
                            color: model.color.into(),
                            normal: normal_matrix,
                            scale: model.scale,
                        });
                });

                acc
            });

    instances.into_iter().for_each(|(mesh_id, texture_data)| {
        texture_data.into_iter().for_each(|(texture_id, raw)| {
            previous.remove(&(mesh_id, texture_id));

            renderer
                .instances
                .entry(mesh_id)
                .or_insert(HashMap::default())
                .entry(texture_id)
                .and_modify(|instance| instance.update(device.inner(), queue.inner(), &raw))
                .or_insert_with(|| tools::InstanceBuffer::new(device.inner(), &raw));
        });
    });

    previous.into_iter().for_each(|(mesh_id, texture_id)| {
        log::trace!("Removing model instance {} - {}", mesh_id, texture_id);
        renderer
            .instances
            .get_mut(&mesh_id)
            .unwrap()
            .remove(&texture_id);
    });

    renderer
        .texture_storage
        .retain(|texture_id, _| textures_used.contains(texture_id));

    renderer
        .mesh_storage
        .retain(|mesh_id, _| meshes_used.contains(mesh_id));
}

fn sys_render(mut pass: ResMut<RenderPass>, renderer: Res<ModelRenderer>, camera: Res<Camera3d>) {
    let pass = pass.pass();

    pass.set_pipeline(renderer.pipeline.inner());
    pass.set_bind_group(0, camera.bind_group(), &[]);

    renderer.instances.iter().for_each(|(mesh_id, instance)| {
        let mesh = renderer.mesh_storage.get(mesh_id).unwrap();

        pass.set_vertex_buffer(0, mesh.mesh().vertex_buffer.inner().slice(..));
        pass.set_index_buffer(
            mesh.mesh().index_buffer.inner().slice(..),
            wgpu::IndexFormat::Uint32,
        );

        instance.iter().for_each(|(texture_id, instance)| {
            let texture = renderer.texture_storage.get(texture_id).unwrap();

            pass.set_bind_group(1, texture.bind_group(), &[]);
            pass.set_vertex_buffer(1, instance.buffer().slice(..));
            pass.draw_indexed(0..mesh.mesh().index_count, 0, 0..instance.count());
        });
    });
}

//====================================================================

//====================================================================

use feathered_common::{Size, WindowRaw, WindowResizeEvent};
use feathered_shipyard::{
    builder::{First, Plugin, Render, Setup},
    events::EventHandle,
    tools::UniqueTools,
    Res, ResMut,
};
use pollster::FutureExt;
use shipyard::{AllStoragesView, IntoWorkload, SystemModificator, Unique, WorkloadModificator};
use texture::DepthTexture;

pub mod camera;
pub mod shared;
pub mod texture;
pub mod tools;

//====================================================================

#[derive(shipyard::Label, Debug, Clone, PartialEq, Hash)]
pub struct SetupRendererComponents;

#[derive(shipyard::Label, Debug, Clone, PartialEq, Hash)]
pub struct SetupUtils;

#[derive(shipyard::Label, Debug, Clone, PartialEq, Hash)]
pub struct SetupRenderPass;

#[derive(shipyard::Label, Debug, Clone, PartialEq, Hash)]
pub struct FinishMainRenderPass;

#[derive(shipyard::Label, Debug, Clone, PartialEq, Hash)]
pub struct SubmitEncoder;

//--------------------------------------------------

pub struct FullRenderToolsPlugin;
impl Plugin for FullRenderToolsPlugin {
    fn build_plugin(self, builder: &mut feathered_shipyard::builder::WorkloadBuilder) {
        builder
            .add_plugin(RenderComponentsPlugin)
            .add_plugin(RenderUtilsPlugin)
            .add_workload_pre(
                Render,
                (sys_setup_encoder, sys_setup_render_pass)
                    .into_sequential_workload()
                    .tag(SetupRenderPass),
            )
            .add_workload_post(
                Render,
                sys_finish_main_render_pass.tag(FinishMainRenderPass),
            )
            .add_workload_last(Render, sys_submit_encoder.tag(SubmitEncoder));
    }
}

pub struct RenderComponentsPlugin;
impl Plugin for RenderComponentsPlugin {
    fn build_plugin(self, builder: &mut feathered_shipyard::builder::WorkloadBuilder) {
        builder
            .add_workload_first(
                Setup,
                sys_setup_renderer_components.tag(SetupRendererComponents),
            )
            .add_workload_first(First, sys_resize_surface);
    }
}

pub struct RenderUtilsPlugin;
impl Plugin for RenderUtilsPlugin {
    fn build_plugin(self, builder: &mut feathered_shipyard::builder::WorkloadBuilder) {
        builder
            .add_plugin(RenderComponentsPlugin)
            .insert(ClearColor::default())
            .add_workload_first(
                Setup,
                (
                    shared::sys_setup_shared_resources,
                    texture::sys_setup_depth_texture,
                    camera::sys_setup_main_camera,
                )
                    .into_workload()
                    .tag(SetupUtils)
                    .after_all(SetupRendererComponents),
            )
            .add_workload_first(First, texture::sys_resize_depth_texture);
    }
}

//====================================================================

pub trait Vertex: bytemuck::Pod {
    fn desc<'a>() -> wgpu::VertexBufferLayout<'a>;
}

//====================================================================

#[derive(Unique)]
pub struct Device(wgpu::Device);
impl Device {
    #[inline]
    pub fn inner(&self) -> &wgpu::Device {
        &self.0
    }
}

#[derive(Unique)]
pub struct Queue(wgpu::Queue);
impl Queue {
    #[inline]
    pub fn inner(&self) -> &wgpu::Queue {
        &self.0
    }
}

#[derive(Unique)]
pub struct Surface(wgpu::Surface<'static>);
impl Surface {
    #[inline]
    pub fn inner(&self) -> &wgpu::Surface {
        &self.0
    }
}

#[derive(Unique)]
pub struct SurfaceConfig(wgpu::SurfaceConfiguration);
impl SurfaceConfig {
    #[inline]
    pub fn inner(&self) -> &wgpu::SurfaceConfiguration {
        &self.0
    }

    pub fn resize(&mut self, size: Size<u32>) {
        self.0.width = size.width;
        self.0.height = size.height;
    }
}

//====================================================================

pub fn sys_setup_renderer_components(all_storages: AllStoragesView, window: Res<WindowRaw>) {
    log::info!("Creating core wgpu renderer components.");

    let size = window.size();

    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
        backends: wgpu::Backends::PRIMARY,
        ..Default::default()
    });

    let surface = instance.create_surface(window.arc().clone()).unwrap();

    let adapter = instance
        .request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::default(),
            force_fallback_adapter: false,
            compatible_surface: Some(&surface),
        })
        .block_on()
        .unwrap();

    log::debug!("Chosen device adapter: {:#?}", adapter.get_info());

    let (device, queue) = adapter
        .request_device(&wgpu::DeviceDescriptor::default(), None)
        .block_on()
        .unwrap();

    let surface_capabilities = surface.get_capabilities(&adapter);

    let surface_format = surface_capabilities
        .formats
        .iter()
        .find(|format| format.is_srgb())
        .copied()
        .unwrap_or(surface_capabilities.formats[0]);

    let config = wgpu::SurfaceConfiguration {
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
        format: surface_format,
        width: size.width,
        height: size.height,
        present_mode: wgpu::PresentMode::AutoNoVsync,
        desired_maximum_frame_latency: 2,
        alpha_mode: surface_capabilities.alpha_modes[0],
        view_formats: vec![],
    };

    surface.configure(&device, &config);

    all_storages
        .insert(Device(device))
        .insert(Queue(queue))
        .insert(Surface(surface))
        .insert(SurfaceConfig(config));
}

pub fn sys_resize_surface(
    device: Res<Device>,
    surface: Res<Surface>,
    mut config: ResMut<SurfaceConfig>,
    window_resize: Res<EventHandle<WindowResizeEvent>>,
) {
    if let Some(new_size) = window_resize.iter().last() {
        let size = new_size.size();
        config.resize(size);
        surface.inner().configure(device.inner(), config.inner());
    }
}

//====================================================================

#[derive(Unique)]
pub struct ClearColor {
    pub r: f64,
    pub g: f64,
    pub b: f64,
    pub a: f64,
}

impl Default for ClearColor {
    fn default() -> Self {
        Self {
            r: 0.2,
            g: 0.2,
            b: 0.2,
            a: 1.,
        }
    }
}

impl ClearColor {
    #[inline]
    pub fn to_array(&self) -> [f64; 4] {
        [self.r, self.g, self.b, self.a]
    }
}

//--------------------------------------------------

#[derive(Unique)]
pub struct RenderPass(wgpu::RenderPass<'static>);

impl RenderPass {
    #[inline]
    pub fn pass(&mut self) -> &mut wgpu::RenderPass<'static> {
        &mut self.0
    }
}

pub struct RenderPassDesc<'a> {
    pub use_depth: Option<&'a wgpu::TextureView>,
    pub clear_color: Option<[f64; 4]>,
}

impl RenderPassDesc<'_> {
    pub fn none() -> Self {
        Self {
            use_depth: None,
            clear_color: None,
        }
    }
}

impl Default for RenderPassDesc<'_> {
    fn default() -> Self {
        Self {
            use_depth: None,
            clear_color: Some([0.2, 0.2, 0.2, 1.]),
        }
    }
}

//--------------------------------------------------

#[derive(Unique)]
pub struct RenderEncoder {
    surface_texture: wgpu::SurfaceTexture,
    surface_view: wgpu::TextureView,
    encoder: wgpu::CommandEncoder,
}

impl RenderEncoder {
    pub fn new(device: &wgpu::Device, surface: &wgpu::Surface) -> Result<Self, wgpu::SurfaceError> {
        let (surface_texture, surface_view) = match surface.get_current_texture() {
            Ok(texture) => {
                let view = texture
                    .texture
                    .create_view(&wgpu::TextureViewDescriptor::default());
                (texture, view)
            }
            Err(e) => return Err(e),
        };

        let encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Main Command Encoder"),
        });

        Ok(RenderEncoder {
            surface_texture,
            surface_view,
            encoder,
        })
    }

    pub fn finish(self, queue: &wgpu::Queue) {
        queue.submit(Some(self.encoder.finish()));
        self.surface_texture.present();
    }

    pub fn begin_render_pass(&mut self, desc: RenderPassDesc) -> wgpu::RenderPass {
        // Clear the current depth buffer and use it.
        let depth_stencil_attachment = match desc.use_depth {
            Some(view) => Some(wgpu::RenderPassDepthStencilAttachment {
                view,
                depth_ops: Some(wgpu::Operations {
                    load: wgpu::LoadOp::Clear(1.),
                    store: wgpu::StoreOp::Store,
                }),
                stencil_ops: None,
            }),
            None => None,
        };

        let load = match desc.clear_color {
            Some(color) => wgpu::LoadOp::Clear(wgpu::Color {
                r: color[0],
                g: color[1],
                b: color[2],
                a: color[3],
            }),
            None => wgpu::LoadOp::Load,
        };

        let render_pass = self.encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("Render Tools Basic Render Pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &self.surface_view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load,
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment,
            timestamp_writes: None,
            occlusion_query_set: None,
        });

        render_pass
    }
}

pub fn sys_setup_encoder(
    all_storages: AllStoragesView,
    device: Res<Device>,
    surface: Res<Surface>,
) {
    let encoder = match RenderEncoder::new(device.inner(), surface.inner()) {
        Ok(encoder) => encoder,
        Err(_) => todo!(),
    };

    all_storages.insert(encoder);
}

pub fn sys_setup_render_pass(
    all_storages: AllStoragesView,
    mut tools: ResMut<RenderEncoder>,
    clear_color: Res<ClearColor>,
    depth: Res<DepthTexture>,
) {
    let pass = tools
        .begin_render_pass(RenderPassDesc {
            use_depth: Some(&depth.0.view),
            clear_color: Some(clear_color.to_array()),
        })
        .forget_lifetime();

    all_storages.insert(RenderPass(pass));
}

pub fn sys_finish_main_render_pass(all_storages: AllStoragesView) {
    all_storages.remove_unique::<RenderPass>().ok();
}

pub fn sys_submit_encoder(all_storages: AllStoragesView, queue: Res<Queue>) {
    let encoder = all_storages.remove_unique::<RenderEncoder>().unwrap();
    encoder.finish(queue.inner());
}

//====================================================================

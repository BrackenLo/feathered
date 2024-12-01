//====================================================================

use std::sync::{atomic::AtomicU32, Arc};

use feathered_common::{Size, WasmWrapper, WindowResizeEvent, WindowSize};
use feathered_shipyard::{
    events::{EventReader, ReadEvents},
    tools::UniqueTools,
    Res, ResMut,
};
use image::GenericImageView;
use shipyard::AllStoragesView;

use crate::{shared::SharedRenderResources, Device};

//====================================================================

#[derive(shipyard::Unique)]
pub struct DepthTexture(pub WasmWrapper<Texture>);

impl DepthTexture {
    pub fn new(device: &wgpu::Device, size: Size<u32>) -> Self {
        let depth_texture = Texture::create_depth_texture(&device, size, "Main Depth Texture");

        Self(WasmWrapper::new(depth_texture))
    }

    #[inline]
    pub fn main_texture(&self) -> &Texture {
        &self.0
    }

    #[inline]
    fn resize(&mut self, device: &wgpu::Device, size: Size<u32>) {
        *self.0 = Texture::create_depth_texture(device, size, "Main Depth Texture");
    }
}

pub fn sys_setup_depth_texture(
    all_storages: AllStoragesView,
    device: Res<Device>,
    size: Res<WindowSize>,
) {
    let depth_texture = DepthTexture::new(device.inner(), size.size());
    all_storages.insert(depth_texture);
}

pub fn sys_resize_depth_texture(
    device: Res<Device>,
    mut depth_texture: ResMut<DepthTexture>,
    window_resize: EventReader<WindowResizeEvent>,
) {
    if let Some(size) = window_resize.last() {
        depth_texture.resize(device.inner(), size.size());
    }
}

//====================================================================

pub type TextureId = u32;

static CURRENT_TEXTURE_ID: AtomicU32 = AtomicU32::new(0);

#[derive(Debug, Clone)]
pub struct LoadedTexture {
    inner: Arc<LoadedInner>,
}

#[derive(Debug)]
struct LoadedInner {
    id: TextureId,
    texture: WasmWrapper<Texture>,
    bind_group: WasmWrapper<wgpu::BindGroup>,
}

impl LoadedTexture {
    pub fn load_texture(
        device: &wgpu::Device,
        shared: &SharedRenderResources,
        texture: Texture,
    ) -> Self {
        let id = CURRENT_TEXTURE_ID.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let bind_group = shared.create_texture_bind_group(device, &texture, None);
        Self {
            inner: Arc::new(LoadedInner {
                id,
                texture: WasmWrapper::new(texture),
                bind_group: WasmWrapper::new(bind_group),
            }),
        }
    }

    #[inline]
    pub fn load_blank(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        shared: &SharedRenderResources,
    ) -> Self {
        let texture = Texture::from_color(
            device,
            queue,
            [255, 255, 255],
            Some("Default Blank Texture"),
            None,
        );

        Self::load_texture(device, shared, texture)
    }

    #[inline]
    pub fn id(&self) -> TextureId {
        self.inner.id
    }

    #[inline]
    pub fn texture(&self) -> &Texture {
        &self.inner.texture
    }

    #[inline]
    pub fn bind_group(&self) -> &wgpu::BindGroup {
        &self.inner.bind_group
    }
}

impl PartialEq for LoadedTexture {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        self.inner.id == other.inner.id
    }
}

//====================================================================

#[derive(Debug)]
pub struct Texture {
    pub texture: wgpu::Texture,
    pub view: wgpu::TextureView,
    pub sampler: wgpu::Sampler,
}

impl Texture {
    pub const DEPTH_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Depth32Float;

    pub fn create_depth_texture(
        device: &wgpu::Device,
        window_size: Size<u32>,
        label: &str,
    ) -> Self {
        let size = wgpu::Extent3d {
            width: window_size.width,
            height: window_size.height,
            depth_or_array_layers: 1,
        };

        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some(&format!("Depth Texture: {}", label)),
            size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: Self::DEPTH_FORMAT,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[wgpu::TextureFormat::Depth32Float],
        });

        let view = texture.create_view(&wgpu::TextureViewDescriptor {
            label: Some(&format!("Depth Texture View: {}", label)),
            ..Default::default()
        });

        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some(&format!("Depth Texture Sampler: {}", label)),
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::FilterMode::Nearest,
            lod_min_clamp: 0.,
            lod_max_clamp: 100.,
            compare: Some(wgpu::CompareFunction::LessEqual),
            ..Default::default()
        });

        Self {
            texture,
            view,
            sampler,
        }
    }
}

//--------------------------------------------------

impl Texture {
    // Create a wgpu Texture from given RGB values.
    pub fn from_color(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        color: [u8; 3],
        label: Option<&str>,
        sampler: Option<&wgpu::SamplerDescriptor>,
    ) -> Self {
        // Create a 1x1 image which we can set to the provided color
        let mut rgb = image::RgbImage::new(1, 1);
        rgb.pixels_mut().for_each(|pixel| {
            pixel.0[0] = color[0];
            pixel.0[1] = color[1];
            pixel.0[2] = color[2];
        });
        // Convert to generic Dynamic Image format
        let rgba = image::DynamicImage::from(rgb);

        Self::from_image(device, queue, &rgba, label, sampler)
    }

    /// Try to create a wgpu Texture from an array of bytes.
    /// The image crate will return an error if it cannot determine the format
    /// of the image.
    pub fn from_bytes(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        bytes: &[u8],
        label: Option<&str>,
        sampler: Option<&wgpu::SamplerDescriptor>,
    ) -> Result<Self, image::ImageError> {
        let img = image::load_from_memory(bytes)?;
        Ok(Self::from_image(device, queue, &img, label, sampler))
    }

    /// Create a wgpu Texture from an existing image::DynamicImage
    pub fn from_image(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        image: &image::DynamicImage,
        label: Option<&str>,
        sampler: Option<&wgpu::SamplerDescriptor>,
    ) -> Self {
        // Convert from generic dynamic image format to usable rgba8 format
        let rgba = image.to_rgba8();
        let dimensions = image.dimensions();

        let size = wgpu::Extent3d {
            width: dimensions.0,
            height: dimensions.1,
            depth_or_array_layers: 1,
        };

        // Create empty wgpu texture
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label,
            size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });

        // Fill texture with image data
        queue.write_texture(
            wgpu::ImageCopyTexture {
                texture: &texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            &rgba,
            wgpu::ImageDataLayout {
                offset: 0,
                bytes_per_row: Some(4 * dimensions.0),
                rows_per_image: None,
            },
            size,
        );

        // Create a view into the texture and a texture sampler
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        let sampler = device.create_sampler(sampler.unwrap_or(&wgpu::SamplerDescriptor::default()));

        Self {
            texture,
            view,
            sampler,
        }
    }

    pub fn from_size(
        device: &wgpu::Device,
        size: Size<u32>,
        label: Option<&str>,
        sampler: Option<&wgpu::SamplerDescriptor>,
    ) -> Self {
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label,
            size: wgpu::Extent3d {
                width: size.width,
                height: size.height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::R8Unorm,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });

        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        let sampler = device.create_sampler(sampler.unwrap_or(&wgpu::SamplerDescriptor::default()));

        Self {
            texture,
            view,
            sampler,
        }
    }
}

impl Texture {
    pub fn update_area(
        &mut self,
        queue: &wgpu::Queue,
        data: &[u8],
        start_x: u32,
        start_y: u32,
        data_width: u32,
        data_height: u32,
    ) {
        queue.write_texture(
            wgpu::ImageCopyTexture {
                texture: &self.texture,
                mip_level: 0,
                origin: wgpu::Origin3d {
                    x: start_x,
                    y: start_y,
                    z: 0,
                },
                aspect: wgpu::TextureAspect::All,
            },
            data,
            wgpu::ImageDataLayout {
                offset: 0,
                bytes_per_row: Some(data_width),
                rows_per_image: None, //Some(data_height),
            },
            wgpu::Extent3d {
                width: data_width,
                height: data_height,
                depth_or_array_layers: 1,
            },
        );
    }
}

//====================================================================

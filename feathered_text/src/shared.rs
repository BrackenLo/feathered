//====================================================================

use std::hash::{Hash, Hasher};

use cosmic_text::{Attrs, Buffer, CacheKey, Color, Metrics, Shaping, Wrap};
use feathered_render_tools::Vertex;
use rustc_hash::FxHasher;

use crate::text_atlas::TextAtlas;

//====================================================================

#[repr(C)]
#[derive(bytemuck::Pod, bytemuck::Zeroable, Clone, Copy, Debug)]
pub struct TextVertex {
    glyph_pos: [f32; 2],
    glyph_size: [f32; 2],
    uv_start: [f32; 2],
    uv_end: [f32; 2],
    color: u32,
}

impl Vertex for TextVertex {
    fn desc<'a>() -> wgpu::VertexBufferLayout<'a> {
        const VERTEX_ATTRIBUTES: [wgpu::VertexAttribute; 5] = wgpu::vertex_attr_array![
            0 => Float32x2,
            1 => Float32x2,
            2 => Float32x2,
            3 => Float32x2,
            4 => Uint32,
        ];

        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<TextVertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Instance,
            attributes: &VERTEX_ATTRIBUTES,
        }
    }
}

//====================================================================

pub struct TextBuffer {
    pub vertex_buffer: wgpu::Buffer,
    pub vertex_count: u32,
    lines: Vec<TextBufferLine>,

    buffer: Buffer,
    color: Color,
}

pub struct TextBufferDescriptor<'a> {
    pub metrics: Metrics,
    pub word_wrap: Wrap,
    pub attributes: Attrs<'a>,
    pub text: &'a str,
    pub width: Option<f32>,
    pub height: Option<f32>,
    pub color: Color,
}

impl<'a> Default for TextBufferDescriptor<'a> {
    fn default() -> Self {
        Self {
            metrics: Metrics::relative(30., 1.2),
            word_wrap: Wrap::WordOrGlyph,
            attributes: Attrs::new(),
            text: "",
            width: Some(800.),
            height: None,
            color: Color::rgb(0, 0, 0),
        }
    }
}

impl TextBuffer {
    pub fn new(
        device: &wgpu::Device,
        font_system: &mut cosmic_text::FontSystem,
        desc: &TextBufferDescriptor,
    ) -> Self {
        let vertex_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Text Vertex Buffer"),
            size: 0,
            usage: wgpu::BufferUsages::VERTEX,
            mapped_at_creation: false,
        });

        let vertex_count = 0;
        let lines = Vec::new();

        let mut buffer = Buffer::new(font_system, desc.metrics);
        buffer.set_size(font_system, desc.width, desc.height);
        buffer.set_wrap(font_system, desc.word_wrap);
        buffer.set_text(font_system, desc.text, desc.attributes, Shaping::Advanced);

        Self {
            vertex_buffer,
            vertex_count,
            lines,
            buffer,
            color: desc.color,
        }
    }
}

//====================================================================

#[derive(Default)]
struct TextBufferLine {
    hash: u64,
    length: usize,
}

//====================================================================

struct LocalGlyphData {
    x: f32,
    y: f32,
    key: CacheKey,
    color: Color,
}

//====================================================================

pub fn prep(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    font_system: &mut cosmic_text::FontSystem,
    swash_cache: &mut cosmic_text::SwashCache,
    text_atlas: &mut TextAtlas,
    text_buffer: &mut TextBuffer,
) -> Option<Vec<TextVertex>> {
    let mut rebuild_all_lines = false;

    let local_glyph_data = text_buffer
        .buffer
        .layout_runs()
        .enumerate()
        .flat_map(|(index, layout_run)| {
            // Hasher for determining if a line has changed
            let mut hasher = FxHasher::default();

            let mut line_length = 0;

            //--------------------------------------------------

            // Iterate through each glyph in the line - prep and check
            let local_glyph_data = layout_run
                .glyphs
                .iter()
                .map(|glyph| {
                    let physical = glyph.physical((0., 0.), 1.);

                    // Try to prep glyph in atlas
                    if let Err(_) = text_atlas.use_glyph(
                        device,
                        queue,
                        font_system,
                        swash_cache,
                        &physical.cache_key,
                    ) {
                        unimplemented!()
                    }

                    // Check if glyph has specific color to use
                    let color = match glyph.color_opt {
                        Some(color) => color,
                        None => text_buffer.color,
                    };

                    // Hash results to check changes
                    physical.cache_key.hash(&mut hasher);
                    color.hash(&mut hasher);

                    // Count number of glyphs in line
                    line_length += 1;

                    // Data for rebuilding later
                    LocalGlyphData {
                        x: physical.x as f32,
                        y: physical.y as f32 - layout_run.line_y,
                        key: physical.cache_key,
                        color,
                    }
                })
                .collect::<Vec<_>>();

            //--------------------------------------------------

            let line_hash = hasher.finish();

            if text_buffer.lines.len() <= index {
                text_buffer.lines.push(TextBufferLine::default());
            }

            let line_entry = &mut text_buffer.lines[index];

            if line_hash != line_entry.hash {
                log::trace!("Line '{}' hash updated '{}'", index, line_hash);

                line_entry.hash = line_hash;
                line_entry.length = line_length;

                rebuild_all_lines = true;
            }

            local_glyph_data
        })
        .collect::<Vec<_>>();

    // TODO - OPTIMIZE - Only rebuild lines that need rebuilding
    match rebuild_all_lines {
        true => Some(
            local_glyph_data
                .into_iter()
                .map(|local_data| {
                    let data = text_atlas.get_glyph_data(&local_data.key).unwrap();

                    let x = local_data.x + data.left + data.width / 2.;
                    let y = local_data.y + data.top; // TODO - Run Line

                    TextVertex {
                        glyph_pos: [x, y],
                        glyph_size: [data.width, data.height],
                        uv_start: data.uv_start,
                        uv_end: data.uv_end,
                        color: local_data.color.0,
                    }
                })
                .collect::<Vec<_>>(),
        ),

        false => None,
    }
}

//====================================================================

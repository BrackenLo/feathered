//====================================================================
// Uniforms

struct Camera {
    projection: mat4x4<f32>,
    position: vec3<f32>,
}

@group(0) @binding(0) var<uniform> camera: Camera;

@group(1) @binding(0) var texture: texture_2d<f32>;
@group(1) @binding(1) var texture_sampler: sampler;


//====================================================================

struct VertexIn {
    // Vertex
    @location(0) vertex_position: vec3<f32>,
    @location(1) uv: vec2<f32>,
    @location(2) normal: vec3<f32>,

    // Instance
    @location(3) transform_1: vec4<f32>,
    @location(4) transform_2: vec4<f32>,
    @location(5) transform_3: vec4<f32>,
    @location(6) transform_4: vec4<f32>,

    @location(7) color: vec4<f32>,

    @location(8) normal_0: vec3<f32>,
    @location(9) normal_1: vec3<f32>,
    @location(10) normal_2: vec3<f32>,

    @location(11) scale: vec3<f32>,
}

struct VertexOut {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) position: vec3<f32>,
    @location(1) uv: vec2<f32>,
    @location(2) normal: vec3<f32>,
    @location(3) color: vec4<f32>,
}

//====================================================================

@vertex
fn vs_main(in: VertexIn) -> VertexOut {
    var out: VertexOut;
    
    let transform = mat4x4<f32>(
        in.transform_1,
        in.transform_2,
        in.transform_3,
        in.transform_4,
    );

    let normal_matrix = mat3x3<f32>(
        in.normal_0,
        in.normal_1,
        in.normal_2,
    );

    let vertex_position = in.vertex_position * in.scale;

    let world_position = transform * vec4<f32>(vertex_position, 1.);

    out.clip_position =
        camera.projection
        * world_position;

    out.position = world_position.xyz;
    out.uv = in.uv;
    out.normal = normal_matrix * in.normal;
    out.color = in.color;

    return out;
}

//====================================================================

@fragment
fn fs_main(in: VertexOut) -> @location(0) vec4<f32> {
    return in.color * textureSample(texture, texture_sampler, in.uv);
}

//====================================================================


// Vertex shader

struct Camera {
    view_proj: mat4x4<f32>,
}
@group(1) @binding(0)
var<uniform> camera: Camera;

@group(1) @binding(1)
var<uniform> screen_size: vec2<f32>;

@group(2) @binding(0)
var<uniform> texture_atlas_dimensions: vec2<f32>;

struct VertexInput {
    @builtin(vertex_index) idx: u32,
    @location(0) position: vec2<f32>,
    @location(1) tex_coords: vec2<f32>,
}
struct InstanceInput {
    @location(5) sprite_position: vec3<f32>,
    // The size of the sprite in pixels. Since we don't scale our sprites, this is also the size in screen pixels.
    @location(6) sprite_size: vec2<f32>,
    // The offset in pixels into the texture atlas where this sprite begins.
    @location(7) sprite_tex_atlas_offset: vec2<f32>,
}

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) tex_coords: vec2<f32>,
}

@vertex
fn vs_main(
    model: VertexInput,
    instance: InstanceInput,
) -> VertexOutput {
    var out: VertexOutput;
    out.tex_coords = model.tex_coords;

//    let model_pos = vec2<f32>(model.position.x, 1.0 - model.position.y);
    let model_pos = model.position.xy;
//    let sprite_pos = vec2<f32>(instance.sprite_position.x, screen_size.y - instance.sprite_position.y) - vec2<f32>(0.0, instance.sprite_size.y);
    let sprite_pos = instance.sprite_position.xy;

    let pos = model_pos * instance.sprite_size + sprite_pos;
    out.clip_position = camera.view_proj * vec4<f32>(pos, instance.sprite_position.z, 1.0);

    // try and compute a uv frame for the region of a 128x128 texture starting at 20,20 and being 50x50 big
    // TODO: use this to use a sprite atlas
    if texture_atlas_dimensions.x > 0.0 {
        let start = instance.sprite_tex_atlas_offset;
        let size = instance.sprite_size;
        let uv_top_left = start / texture_atlas_dimensions;
        let uv_bottom_right = (start + size) / texture_atlas_dimensions;
        let uv_size = uv_bottom_right - uv_top_left;
        let uv = uv_top_left + model.tex_coords * uv_size;
        out.tex_coords = uv;
    }


    return out;
}

// Fragment shader

@group(0) @binding(0)
var t_diffuse: texture_2d<f32>;
@group(0)@binding(1)
var s_diffuse: sampler;

@group(0) @binding(2)
var t_normal: texture_2d<f32>;
@group(0)@binding(3)
var s_normal: sampler;

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let frag_color = textureSample(t_diffuse, s_diffuse, in.tex_coords);
    // skip entirely transparent pixels
    if (frag_color.a < 0.001) {
        discard;
    }

    // do normal mapping
    // TODO: add uniform for a bunch of lights
    let light_pos = vec3<f32>(100., 100., -20.);
    let normal_raw = textureSample(t_normal, s_normal, in.tex_coords).xyz;
    let normal = normalize(normal_raw * -2.0 + 1.0);
    // QUESTION: do we want z component of input clip position?
//    let in_pos = in.clip_position.xyz;
    let in_pos = vec3<f32>(in.clip_position.xy, 0.0);
    let light_dir = normalize(light_pos - in.clip_position.xyz);
    let light_intensity = max(dot(normal, light_dir), 0.0);
    let light_color = vec3<f32>(1.0, 1.0, 1.0);
    let ambient_color = vec3<f32>(0.1, 0.1, 0.1);
    // TODO: attenuation depending on distance?
    let color = frag_color.rgb * (light_intensity * light_color + ambient_color);


    return vec4<f32>(color, frag_color.a);
}


//@fragment
//fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
//    let x = in.clip_position.x;
//    let y = in.clip_position.y;
//    // draw colors according to position
//    let color = vec3<f32>(
//        x / 200.,
//        y / 120.,
//        0.,
//    );
//    return vec4<f32>(color, 1.0);
//}
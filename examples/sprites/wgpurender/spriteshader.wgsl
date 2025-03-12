// Vertex shader

struct Camera {
    view_proj: mat4x4<f32>,
}
@group(1) @binding(0)
var<uniform> camera: Camera;

@group(1) @binding(1)
var<uniform> screen_size: vec2<f32>;

struct VertexInput {
    @builtin(vertex_index) idx: u32,
    @location(0) position: vec3<f32>,
    @location(1) tex_coords: vec2<f32>,
}
struct InstanceInput {
    @location(5) sprite_position: vec3<f32>,
    @location(6) sprite_scale: vec2<f32>,
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

    // compute the 6 vertices of a quad so we can render the InstanceInput
    const pos = array(
        vec2<f32>(-1.0, 1.0),
        vec2<f32>(-1.0, -1.0),
        vec2<f32>(1.0, -1.0),
        vec2<f32>(-1.0, 1.0),
        vec2<f32>(1.0, -1.0),
        vec2<f32>(1.0, 1.0),
    );
    
    // TODO: fix this and take into account instance pos and scale

//    // move the quad to the position of the sprite, which is in screen coords, so we need to take screen_size into account
//    let sprite_position = instance.sprite_position.xy / screen_size * 2.0 - 1.0;
//    let sprite_scale = instance.sprite_scale;
//    // need to scale down sprite_scale to [0, 2] range as well
//    let new_scale = sprite_scale / screen_size * 2.0;
//    let pos_shifted = pos[model.idx] * sprite_scale + sprite_position;
//    
//    // for all 6 corners, define where each vertex should be according to sprite_position and sprite_scale
//    let scaled_vertex_positions = array(
//        vec2<f32>(-1.0 + sprite_position.x, 1.0 - sprite_position.y),
//        vec2<f32>(-1.0, -1.0),
//        vec2<f32>(1.0, -1.0),
//        vec2<f32>(-1.0, 1.0),
//        vec2<f32>(1.0, -1.0),
//        vec2<f32>(1.0, 1.0),
//    );

    let pos_shifted = pos[model.idx];

    // compute the clip position
    let clip_position = vec4<f32>(
        pos_shifted,
        0.0,
        1.0,
    );
    
    // for tex coords just take full uv
    const tex_coords = array(
        vec2<f32>(0.0, 0.0),
        vec2<f32>(0.0, 1.0),
        vec2<f32>(1.0, 1.0),
        vec2<f32>(0.0, 0.0),
        vec2<f32>(1.0, 1.0),
        vec2<f32>(1.0, 0.0),
    );
    
    var out: VertexOutput;
    
    out.tex_coords = tex_coords[model.idx];
    out.clip_position = clip_position;


    return out;
}

// Fragment shader

@group(0) @binding(0)
var t_diffuse: texture_2d<f32>;
@group(0)@binding(1)
var s_diffuse: sampler;

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    return textureSample(t_diffuse, s_diffuse, in.tex_coords);
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
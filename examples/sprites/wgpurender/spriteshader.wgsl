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

    // TODO: fix this and take into account instance pos and scale


    // Define the quad vertices
    const quad_positions = array(
        vec2<f32>(-0.5, -0.5) + vec2<f32>(0.5, 0.5),
        vec2<f32>(-0.5,  0.5) + vec2<f32>(0.5, 0.5),
        vec2<f32>( 0.5,  0.5) + vec2<f32>(0.5, 0.5),
        vec2<f32>(-0.5, -0.5) + vec2<f32>(0.5, 0.5),
        vec2<f32>( 0.5,  0.5) + vec2<f32>(0.5, 0.5),
        vec2<f32>( 0.5, -0.5) + vec2<f32>(0.5, 0.5),
    );

    // Define the texture coordinates
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

    // Apply instance position and scale
    let pos = quad_positions[model.idx] * instance.sprite_scale + instance.sprite_position.xy;


    out.clip_position = camera.view_proj * vec4<f32>(pos, instance.sprite_position.z, 1.0);
//    out.clip_position = camera.view_proj * vec4<f32>(model.position.xy, 20.0, 1.0);
//    out.clip_position = vec4<f32>(model.position.xy, 20.0, 1.0);

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
use std::collections::HashSet;
use std::iter;

use cgmath::prelude::*;
use crossterm::event::KeyCode;
use image::{DynamicImage, GenericImageView};
use wgpu::{AdapterInfo, BufferSize, TextureView};
use wgpu::util::DeviceExt;
use teng::rendering::color::Color;
use teng::rendering::render::HalfBlockDisplayRender;
use teng::SharedState;
use crate::GameState;
use crate::gpu::animation::Animation;
use crate::gpu::instancewriter::InstanceWriter;
use crate::gpu::sprite::TextureAnimationAtlas;
use crate::gpu::texture;

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct Vertex {
    position: [f32; 2],
    tex_coords: [f32; 2],
}

impl Vertex {
    fn desc() -> wgpu::VertexBufferLayout<'static> {
        use std::mem;
        wgpu::VertexBufferLayout {
            array_stride: mem::size_of::<Vertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            // need the "const" because vertex_attr_array does too complex things for automatic lifetime promotion
            attributes: &const { wgpu::vertex_attr_array![
                0 => Float32x2,
                1 => Float32x2,
            ] },
        }
    }
}

// a quad defined by two triangles
// every instance uses exactly these vertices. the vertex shader scales and translates them.
const VERTICES: &[Vertex] = &[
    Vertex {
        position: [0.0, 0.0],
        tex_coords: [0.0, 0.0],
    },
    Vertex {
        position: [0.0, 1.0],
        tex_coords: [0.0, 1.0],
    },
    Vertex {
        position: [1.0, 1.0],
        tex_coords: [1.0, 1.0],
    },
    Vertex {
        position: [0.0, 0.0],
        tex_coords: [0.0, 0.0],
    },
    Vertex {
        position: [1.0, 1.0],
        tex_coords: [1.0, 1.0],
    },
    Vertex {
        position: [1.0, 0.0],
        tex_coords: [1.0, 0.0],
    },
];

// TODO: switch from cgmath to glam-rs?

struct Camera {
    // center of the screen in world coords
    position: glam::Vec2,
    size: glam::Vec2,
    screen_size: glam::Vec2,
    // A scale of 2x means every screen pixel is 2x2 world pixels.
    // invariant: scale = size / screen_size
    scale: f32,
}

impl Camera {
    fn new() -> Self {
        Self {
            position: glam::Vec2::new(0.0, 0.0),
            size: glam::Vec2::new(1.0, 1.0),
            screen_size: glam::Vec2::new(1.0, 1.0),
            scale: 1.0,
        }
    }

    // Screen coords are input
    fn resize(&mut self, width: f32, height: f32) {
        self.screen_size = glam::Vec2::new(width, height);
        self.size = glam::Vec2::new(width * self.scale, height * self.scale);
    }

    fn set_scale(&mut self, scale: f32) {
        self.scale = scale;
        self.size = glam::Vec2::new(self.screen_size.x * scale, self.screen_size.y * scale);
    }

    fn screen_to_world_coords(&self, screen_x: f32, screen_y: f32) -> (f32, f32) {
        let diff_to_center = glam::Vec2::new(screen_x, screen_y) - self.screen_size / 2.0;
        let world_x = diff_to_center.x * self.scale + self.position.x;
        let world_y = -diff_to_center.y * self.scale + self.position.y;

        return (world_x, world_y);


        // let screen_x = screen_x * self.scale;
        // let screen_y = screen_y * self.scale;
        let world_x = screen_x + self.position.x - self.size.x / 2.0;
        let world_y = (self.screen_size.y - screen_y - 1.0) + self.position.y - self.size.y / 2.0;
        (world_x * self.scale, world_y * self.scale)

        // the inverse:
        // let screen_x = world_x - self.position.x + self.size.x / 2.0;
        // let screen_y =
    }

    fn world_to_screen_coords(&self, world_x: f32, world_y: f32) -> (f32, f32) {
        let diff_to_center = glam::Vec2::new(world_x, world_y) - self.position;
        let screen_x = diff_to_center.x / self.scale + self.screen_size.x / 2.0;
        let screen_y = -diff_to_center.y / self.scale + self.screen_size.y / 2.0;

        (screen_x, screen_y)
    }

    fn to_uniform(&self) -> CameraUniform {
        let mut uniform = CameraUniform::new();
        uniform.update_camera_position(self.position.x, self.position.y);
        uniform.update_camera_size(self.size.x, self.size.y);
        uniform.update_screen_size(self.screen_size.x, self.screen_size.y);
        uniform.update_view_proj(0.0, self.size.x, self.size.y, 0.0);

        uniform
    }
}

#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
struct CameraUniform {
    view_proj: [[f32; 4]; 4],
    camera_size: [f32; 2],
    // center of camera in world pos
    camera_position: [f32; 2],
    // screen size
    screen_size: [f32; 2],
    _pad: [f32; 2],
}

impl CameraUniform {
    fn new() -> Self {
        Self {
            view_proj: cgmath::Matrix4::identity().into(),
            camera_size: [1.0, 1.0],
            camera_position: [0.0, 0.0],
            screen_size: [1.0, 1.0],
            _pad: [0.0, 0.0],
        }
    }

    fn update_camera_position(&mut self, x: f32, y: f32) {
        self.camera_position = [x, y];
    }

    fn update_camera_size(&mut self, width: f32, height: f32) {
        self.camera_size = [width, height];
    }

    fn update_screen_size(&mut self, width: f32, height: f32) {
        self.screen_size = [width, height];
    }

    fn update_view_proj(&mut self, left: f32, right: f32, bottom: f32, top: f32) {
        let znear = -0.1;
        let zfar = 100.0;

        // See below comments for notes on cgmath. cgmath::ortho is right handed, also in OpenGL conventions.
        // here we use glam, which uses wgpu conventions, so we don't need to multiply with OPENGL_TO_WGPU_MATRIX.
        // Additionally, we use the left-handed version of orthographic_lh, because our top/bottom flip
        // performs the right-to-left handedness conversion. So we don't need the orthographic projection to do another
        // right-to-left conversion, and instead we use _lh.
        // See the version below with cgmath for how to use a orthographic_rh projection instead.
        // TODO: if we switch sprites to world coords, maybe swap top/bottom again?
        let glam_ortho = glam::Mat4::orthographic_lh(left, right, bottom, top, znear, zfar);
        self.view_proj = glam_ortho.to_cols_array_2d();

        // NOTE: AHA, the problem seems to be handedness, specifically the c2r2 component should flip sign?
        // maybe this is useful: https://learnopengl.com/In-Practice/2D-Game/Rendering-Sprites
        // ==> ah, I think it's because we flip bottom/top, this is already a change of coordinate system.
        // so. Our input coords (screen coords with z pointing inside screen) are actually right handed coords.
        // so in theory, the right-to-left conversion that cgmath::ortho (and OpenGL) does is good.
        // BUT because we flip bottom/top, we _already_ flipped the coordinate system.
        // so the additional flip by changing z coords is too much, and we can get rid of it.
        // IDEA: I think using a lookat with up = -y might help and be the more idiomatic way? let's try this.
        // ^ I could not get that to work.
        // ^ if anyone can tell me the "right" way to handle this kind of thing, I'd be very grateful.
        // let mut ortho_mat = cgmath::ortho(left, right, bottom, top, znear, zfar);
        // Flip z because cgmath::ortho does a right-to-left hand conversion by flipping z, but
        // we have already done a right-to-left hand conversion by flipping y. We're flipping y because our sprites are in screen coords,
        // where y grows downwards instead of upwards.
        // ortho_mat.z.z = -ortho_mat.z.z;
        // let view_proj_mat = OPENGL_TO_WGPU_MATRIX * ortho_mat;
        // self.view_proj = view_proj_mat.into();
    }
}


#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Instance {
    /// xyz, z being depth for render order but ignored due to orthographic projection, xy in screen coords
    pub position: [f32; 3],
    /// height/width of the desired sprite in pixels
    pub size: [f32; 2],
    /// offset in pixels from the top left corner of the texture atlas
    pub sprite_tex_atlas_offset: [f32; 2],
    /// center offset in pixels from the top left corner
    /// Other way around, if we know the desired center position, top_left_corner = desired_center - center_offset
    pub center_offset: [f32; 2],
}

impl Instance {
    fn desc() -> wgpu::VertexBufferLayout<'static> {
        use std::mem;
        wgpu::VertexBufferLayout {
            array_stride: mem::size_of::<Instance>() as wgpu::BufferAddress,
            // We need to switch from using a step mode of Vertex to Instance
            // This means that our shaders will only change to use the next
            // instance when the shader starts processing a new instance
            step_mode: wgpu::VertexStepMode::Instance,
            // need the "const" because vertex_attr_array does too complex things for automatic lifetime promotion
            attributes: &const { wgpu::vertex_attr_array![
                5 => Float32x3,
                6 => Float32x2,
                7 => Float32x2,
                8 => Float32x2,
            ] },
        }
    }
}

pub fn next_256_multiple(x: u32) -> u32 {
    (x + 255) & !255
}

struct RenderTextures {
    // screen_texture is the texture onto which we draw (the first part) of our render pipeline
    screen_texture_desc: wgpu::TextureDescriptor<'static>,
    screen_texture: wgpu::Texture,
    screen_texture_view: wgpu::TextureView,
    // the depth texture for the first pass, z coords of sprites.
    depth_texture: texture::Texture,
    // output_buffer is the buffer that we read from to get the final image
    output_buffer: wgpu::Buffer,
}

impl RenderTextures {
    fn new(device: &wgpu::Device, (width, height): (u32, u32)) -> Self {
        let screen_texture_desc = wgpu::TextureDescriptor {
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsages::COPY_SRC | wgpu::TextureUsages::RENDER_ATTACHMENT,
            label: Some("screen_texture"),
            view_formats: &[],
        };
        let screen_texture = device.create_texture(&screen_texture_desc);
        let screen_texture_view = screen_texture.create_view(&Default::default());

        let depth_texture = texture::Texture::create_depth_texture(device, (width, height).into(), "depth_texture");

        let output_buffer_size = (next_256_multiple(width) * 4 * height) as wgpu::BufferAddress;
        let output_buffer_desc = wgpu::BufferDescriptor {
            size: output_buffer_size,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            label: Some("output_buffer"),
            mapped_at_creation: false,
        };
        let output_buffer = device.create_buffer(&output_buffer_desc);

        Self {
            screen_texture_desc,
            screen_texture,
            screen_texture_view,
            depth_texture,
            output_buffer,
        }
    }

    fn resize(&mut self, device: &wgpu::Device, (width, height): (u32, u32)) {
        self.screen_texture_desc.size.width = width;
        self.screen_texture_desc.size.height = height;
        self.screen_texture = device.create_texture(&self.screen_texture_desc);
        self.screen_texture_view = self.screen_texture.create_view(&Default::default());
        self.depth_texture = texture::Texture::create_depth_texture(device, (width, height).into(), "depth_texture");
        let output_buffer_size = (next_256_multiple(width) * 4 * height) as wgpu::BufferAddress;
        let output_buffer_desc = wgpu::BufferDescriptor {
            size: output_buffer_size,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            label: Some("output_buffer"),
            mapped_at_creation: false,
        };
        self.output_buffer = device.create_buffer(&output_buffer_desc);
    }
}

pub struct State {
    device: wgpu::Device,
    queue: wgpu::Queue,
    size: (u32, u32),
    render_pipeline: wgpu::RenderPipeline,
    vertex_buffer: wgpu::Buffer,
    #[allow(dead_code)]
    diffuse_texture: texture::Texture,
    diffuse_bind_group_layout: wgpu::BindGroupLayout,
    diffuse_bind_group: wgpu::BindGroup,
    camera: Camera,
    camera_buffer: wgpu::Buffer,
    camera_bind_group: wgpu::BindGroup,
    texture_atlas_size_bind_group: wgpu::BindGroup,
    instances: Vec<Instance>,
    #[allow(dead_code)]
    instance_buffer: wgpu::Buffer,
    adapter_info: AdapterInfo,
    render_textures: RenderTextures,
}

const U32_SIZE: u32 = std::mem::size_of::<u32>() as u32;

impl State {
    fn bytes_per_row_256_aligned(pixel_width: u32) -> u32 {
        let pixels_per_row_aligned = ((pixel_width + 255) / 256) * 256;
        let bytes_per_row_aligned = pixels_per_row_aligned * U32_SIZE;
        bytes_per_row_aligned
    }

    pub async fn new(size: (u32, u32), tex_atlas: DynamicImage) -> State {
        // The instance is a handle to our GPU
        // BackendBit::PRIMARY => Vulkan + Metal + DX12 + Browser WebGPU
        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
            backends: wgpu::Backends::PRIMARY,
            // Allow NVIDIA cards to run inside WSL2 through the kisak-mesa dozen vulkan driver
            flags: wgpu::InstanceFlags::default().union(wgpu::InstanceFlags::ALLOW_UNDERLYING_NONCOMPLIANT_ADAPTER),
            ..Default::default()
        });

        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                // power_preference: wgpu::PowerPreference::default(),
                // Select between low power or high performance GPU
                power_preference: wgpu::PowerPreference::HighPerformance,
                compatible_surface: None,
                // Turn this to false for a real GPU.
                force_fallback_adapter: true,
            })
            .await
            .unwrap();
        let (device, queue) = adapter
            .request_device(
                &Default::default(),
                None, // Trace path
            )
            .await
            .unwrap();

        let render_textures = RenderTextures::new(&device, size);

        // TODO: rename diffuse texture to atlas everywhere
        let diffuse_texture =
            texture::Texture::from_image(&device, &queue, &tex_atlas, Some("crate_diffuse.png"), false).unwrap();

        // TODO: use a separate binding group for this? right now we crash if we render the hbd to a texture since we don't set a normal texture.
        let normal_texture =
            texture::Texture::from_image(&device, &queue, &image::load_from_memory(include_bytes!("crate_normal.png")).unwrap(), Some("crate_normal.png"), true).unwrap();

        let diffuse_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            multisampled: false,
                            view_dimension: wgpu::TextureViewDimension::D2,
                            sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 2,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            multisampled: false,
                            view_dimension: wgpu::TextureViewDimension::D2,
                            sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 3,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                        count: None,
                    },
                ],
                label: Some("texture_bind_group_layout"),
            });

        let diffuse_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &diffuse_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&diffuse_texture.view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&diffuse_texture.sampler),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::TextureView(&normal_texture.view),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: wgpu::BindingResource::Sampler(&normal_texture.sampler),
                },
            ],
            label: Some("diffuse_bind_group"),
        });

        let mut camera = Camera::new();
        camera.resize(size.0 as f32, size.1 as f32);
        let camera_uniform = camera.to_uniform();

        let camera_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Camera Buffer"),
            contents: bytemuck::cast_slice(&[camera_uniform]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let mut instances = vec![
            Instance {
                position: [0.0, 0.0, 3.0],
                size: [30.0, 30.0],
                sprite_tex_atlas_offset: [30.0, 30.0],
                center_offset: [35.0, 35.0],
            },
            Instance {
                position: [0.0, 0.0, 2.0],
                size: [30.0, 30.0],
                sprite_tex_atlas_offset: [0.0, 0.0],
                center_offset: [35.0, 35.0],
            },
            Instance {
                position: [0.0, 0.0, 4.0],
                size: [60.0, 60.0],
                sprite_tex_atlas_offset: [0.0, 0.0],
                center_offset: [35.0, 35.0],
            }];

        // for performance testing
        // for _ in 0..1000000 {
        //     let random_x = rand::random::<f32>() * 100.0;
        //     let random_y = rand::random::<f32>() * 100.0;
        //     instances.push(Instance {
        //         position: [random_x, random_y, 10.0],
        //         scale: [30.0, 30.0],
        //     });
        // }

        let instance_data = instances.clone();
        let instance_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Instance Buffer"),
            contents: bytemuck::cast_slice(&instance_data),
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
        });

        let camera_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::VERTEX,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                ],
                label: Some("camera_bind_group_layout"),
            });

        let camera_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &camera_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: camera_buffer.as_entire_binding(),
                }],
            label: Some("camera_bind_group"),
        });

        let texture_atlas_size_bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
            label: Some("texture_atlas_size_bind_group_layout"),
        });
        // TODO: IMPORTANT LEARNING
        // IF WE JUST USE [128.0, 128.0] WITHOUT TYPE ANNOTATIONS, then we will not get f32 bytes but something else.
        // probably f64. So this is not good, and we should also change it for the other place where we're doing this.
        let tex_size: [f32; 2] = [tex_atlas.dimensions().0 as f32, tex_atlas.dimensions().1 as f32];
        // TODO: don't hardcode this
        let texture_atlas_size_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Texture Atlas Size Buffer"),
            contents: bytemuck::cast_slice(&[tex_size]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });
        let texture_atlas_size_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &texture_atlas_size_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: texture_atlas_size_buffer.as_entire_binding(),
                },
            ],
            label: Some("texture_atlas_size_bind_group"),
        });

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("spriteshader.wgsl").into()),
        });

        let render_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Render Pipeline Layout"),
                bind_group_layouts: &[&diffuse_bind_group_layout, &camera_bind_group_layout, &texture_atlas_size_bind_group_layout],
                push_constant_ranges: &[],
            });

        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Render Pipeline"),
            layout: Some(&render_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                buffers: &[Vertex::desc(), Instance::desc()],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: render_textures.screen_texture_desc.format,
                    blend: Some(wgpu::BlendState {
                        color: wgpu::BlendComponent::REPLACE,
                        alpha: wgpu::BlendComponent::REPLACE,
                    }),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: Some(wgpu::Face::Back),
                // cull_mode: None,
                // Setting this to anything other than Fill requires Features::POLYGON_MODE_LINE
                // or Features::POLYGON_MODE_POINT
                polygon_mode: wgpu::PolygonMode::Fill,
                // Requires Features::DEPTH_CLIP_CONTROL
                unclipped_depth: false,
                // Requires Features::CONSERVATIVE_RASTERIZATION
                conservative: false,
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: texture::Texture::DEPTH_FORMAT,
                depth_write_enabled: true,
                depth_compare: wgpu::CompareFunction::Less, // 1.
                stencil: wgpu::StencilState::default(), // 2.
                bias: wgpu::DepthBiasState::default(),
            }),
            multisample: wgpu::MultisampleState {
                count: 1,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            // If the pipeline will be used with a multiview render pass, this
            // indicates how many array layers the attachments will have.
            multiview: None,
            // Useful for optimizing shader compilation on Android
            cache: None,
        });

        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Vertex Buffer"),
            contents: bytemuck::cast_slice(VERTICES),
            usage: wgpu::BufferUsages::VERTEX,
        });

        Self {
            device,
            queue,
            size,
            render_pipeline,
            vertex_buffer,
            diffuse_texture,
            diffuse_bind_group_layout,
            diffuse_bind_group,
            camera_buffer,
            camera_bind_group,
            texture_atlas_size_bind_group,
            camera,
            instances: instances.clone(),
            instance_buffer,
            adapter_info: adapter.get_info(),
            render_textures,
        }
    }

    pub fn get_size(&self) -> (u32, u32) {
        self.size
    }

    pub fn get_adapter_info(&self) -> &AdapterInfo {
        &self.adapter_info
    }

    pub fn resize(&mut self, new_size: (u32, u32)) {
        if new_size != self.size {
            self.size = new_size;

            // adjust camera uniform
            self.camera.resize(self.size.0 as f32, self.size.1 as f32);
            self.queue.write_buffer(
                &self.camera_buffer,
                0,
                bytemuck::cast_slice(&[self.camera.to_uniform()]),
            );

            self.render_textures.resize(&self.device, new_size);
        }
    }

    fn move_camera(&mut self, shared_state: &SharedState<GameState>) {
        let delta_move = 1.0;
        let mut accum_move = glam::Vec2::new(0.0, 0.0);
        if shared_state.pressed_keys.did_press_char_ignore_case('w') {
            accum_move.y += delta_move;
        }
        if shared_state.pressed_keys.did_press_char_ignore_case('s') {
            accum_move.y -= delta_move;
        }
        if shared_state.pressed_keys.did_press_char_ignore_case('a') {
            accum_move.x -= delta_move;
        }
        if shared_state.pressed_keys.did_press_char_ignore_case('d') {
            accum_move.x += delta_move;
        }

        self.camera.position += accum_move;
        self.queue.write_buffer(
            &self.camera_buffer,
            0,
            bytemuck::cast_slice(&[self.camera.to_uniform()]),
        );
    }

    pub fn update(&mut self, x: usize, y: usize, anim: &mut Animation, atlas: &TextureAnimationAtlas, shared_state: &mut SharedState<GameState>) {
        let x = x as f32;
        let y = y as f32;
        let (x, y) = self.camera.screen_to_world_coords(x, y);
        self.instances[0].position = [x, y, self.instances[0].position[2]];
        shared_state.debug_info.custom.insert("instance0.pos".to_string(), format!("{:?}", self.instances[0].position));
        shared_state.debug_info.custom.insert("world_to_screen".to_string(), format!("{:?}", self.camera.world_to_screen_coords(x, y)));
        shared_state.debug_info.custom.insert("camera.pos".to_string(), format!("{:?}", self.camera.position));
        
        let mut instance_writer = InstanceWriter::new(&mut self.instances);
        // TODO: how to make .clear not necessary? or do we need it? basically, I think it would be nice to have
        // 'preallocated' slots for every animation that a Animation::render call can just write into.
        // Also, layering needs to be determined. A proper animation controller is needed.
        instance_writer.clear();
        anim.render(atlas, [x, y].into(), 0, &mut instance_writer);
        drop(instance_writer);

        // if shared_state.pressed_keys.did_press_char_ignore_case('w') {
        //     self.instances[0].size = [self.instances[0].size[0] + 1.0, self.instances[0].size[1] + 1.0];
        // }
        for instance in &mut self.instances {
            // make sure they're offset by 0.5
            instance.position[0] = instance.position[0].floor() + 0.1;
            instance.position[1] = instance.position[1].floor() + 0.1;
        }
        // TODO: only recreate if size actually changed.
        self.instance_buffer = self.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Instance Buffer"),
            contents: bytemuck::cast_slice(&self.instances),
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
        });
        // self.queue.write_buffer(
        //     &self.instance_buffer,
        //     0,
        //     bytemuck::cast_slice(&self.instances),
        // );
        // try write_buffer_with
        // let writebuf = self.queue.write_buffer_with(&self.instance_buffer, 0, BufferSize::try_from(10).unwrap()).unwrap();
        //

        self.move_camera(shared_state);

        // todo adjust scale using scroll wheel
        if shared_state.pressed_keys.did_press(KeyCode::Up) {
            self.camera.set_scale(self.camera.scale + 0.1);
            self.queue.write_buffer(
                &self.camera_buffer,
                0,
                bytemuck::cast_slice(&[self.camera.to_uniform()]),
            );
        }

        if shared_state.pressed_keys.did_press(KeyCode::Down) {
            self.camera.set_scale(self.camera.scale - 0.1);
            self.queue.write_buffer(
                &self.camera_buffer,
                0,
                bytemuck::cast_slice(&[self.camera.to_uniform()]),
            );
        }

        // self.camera_controller.update_camera(&mut self.camera);
        // self.camera_uniform.update_view_proj(&self.camera);
        // self.queue.write_buffer(
        //     &self.camera_buffer,
        //     0,
        //     bytemuck::cast_slice(&[self.camera_uniform]),
        // );
    }

    pub fn add_instance(&mut self, instance: Instance) {
        self.instances.push(instance);
        // Need to recreate due to changed size
        self.instance_buffer = self.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Instance Buffer"),
            contents: bytemuck::cast_slice(&self.instances),
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
        });
        // self.queue.write_buffer(
        //     &self.instance_buffer,
        //     0,
        //     bytemuck::cast_slice(&self.instances),
        // );
    }

    pub fn render(&mut self, hbd: &mut HalfBlockDisplayRender) -> Result<(), wgpu::SurfaceError> {
        let view = &self.render_textures.screen_texture_view;

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Render Encoder"),
            });

        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 0.01,
                            g: 0.01,
                            b: 0.01,
                            a: 0.0,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &self.render_textures.depth_texture.view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0),
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                occlusion_query_set: None,
                timestamp_writes: None,
            });

            render_pass.set_pipeline(&self.render_pipeline);
            render_pass.set_bind_group(0, &self.diffuse_bind_group, &[]);
            render_pass.set_bind_group(1, &self.camera_bind_group, &[]);
            render_pass.set_bind_group(2, &self.texture_atlas_size_bind_group, &[]);
            render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
            render_pass.set_vertex_buffer(1, self.instance_buffer.slice(..));
            // render_pass.set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint16);
            // UPDATED!
            // render_pass.draw_indexed(0..self.num_indices, 0, 0..self.instances.len() as _);
            render_pass.draw(0..6, 0..self.instances.len() as _);
        }

        let bytes_per_row = next_256_multiple(self.size.0) * U32_SIZE;

        // from windowless example
        encoder.copy_texture_to_buffer(
            wgpu::TexelCopyTextureInfo {
                aspect: wgpu::TextureAspect::All,
                texture: &self.render_textures.screen_texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
            },
            wgpu::TexelCopyBufferInfo {
                buffer: &self.render_textures.output_buffer,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(bytes_per_row),
                    rows_per_image: Some(self.size.1),
                },
            },
            self.render_textures.screen_texture_desc.size,
        );

        self.queue.submit(iter::once(encoder.finish()));

        // Now we can read the buffer {
        {
            let buffer_slice = self.render_textures.output_buffer.slice(..);

            buffer_slice.map_async(wgpu::MapMode::Read, move |result| {
                // using a channel we could get this error outside. but we just want to crash.
                result.unwrap();
            });
            self.device.poll(wgpu::Maintain::Wait);

            let data = buffer_slice.get_mapped_range();

            let mut x = 0;
            let mut y = 0;

            let good_row_size = self.size.0 as usize;
            let row_size_aligned = bytes_per_row as usize / U32_SIZE as usize;
            for window in data.chunks(4) {
                let r = window[0];
                let g = window[1];
                let b = window[2];
                let a = window[3];

                // TODO: iterating over even the not good chunks is inefficient. should just index the data directly.
                if x < good_row_size {
                    // if alpha is not entire solid, render as transparent
                    if a != 255 {
                        // TODO: maybe just don't render at all?
                        hbd.set_color(x, y, Color::Transparent);
                    } else {
                        let color = Color::Rgb([r, g, b]);
                        hbd.set_color(x, y, color);
                    }

                }



                x += 1;
                if x == row_size_aligned{
                    x = 0;
                    y += 1;
                }
            }
        }
        self.render_textures.output_buffer.unmap();

        Ok(())
    }
}


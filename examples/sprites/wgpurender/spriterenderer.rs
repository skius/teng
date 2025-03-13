use std::collections::HashSet;
use std::iter;

use cgmath::prelude::*;
use crossterm::event::KeyCode;
use wgpu::{AdapterInfo, TextureView};
use wgpu::util::DeviceExt;
use teng::rendering::color::Color;
use teng::rendering::render::HalfBlockDisplayRender;
use teng::SharedState;
use crate::GameState;
use crate::wgpurender::texture;

const NUM_INSTANCES_PER_ROW: u32 = 10;
const INSTANCE_DISPLACEMENT: cgmath::Vector3<f32> = cgmath::Vector3::new(
    NUM_INSTANCES_PER_ROW as f32 * 0.5,
    0.0,
    NUM_INSTANCES_PER_ROW as f32 * 0.5,
);

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
            attributes: &[
                wgpu::VertexAttribute {
                    offset: 0,
                    shader_location: 0,
                    format: wgpu::VertexFormat::Float32x2,
                },
                wgpu::VertexAttribute {
                    offset: mem::size_of::<[f32; 2]>() as wgpu::BufferAddress,
                    shader_location: 1,
                    format: wgpu::VertexFormat::Float32x2,
                },
            ],
        }
    }
}

// a quad defined by two triangles
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

const INDICES: &[u16] = &[0, 1, 4, 1, 2, 4, 2, 3, 4, /* padding */ 0];

// this matrix seems very wrong.
#[rustfmt::skip]
pub const OPENGL_TO_WGPU_MATRIX: cgmath::Matrix4<f32> = cgmath::Matrix4::new(
    1.0, 0.0, 0.0, 0.0,
    0.0, 1.0, 0.0, 0.0,
    0.0, 0.0, 0.5, 0.0,
    0.0, 0.0, 0.5, 1.0,
);

#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
struct CameraUniform {
    view_proj: [[f32; 4]; 4],
}

impl CameraUniform {
    fn new() -> Self {
        Self {
            view_proj: cgmath::Matrix4::identity().into(),
        }
    }

    fn update_view_proj(&mut self, left: f32, right: f32, bottom: f32, top: f32) {
        let znear = -0.1;
        let zfar = 100.0;

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
        let mut ortho_mat = cgmath::ortho(left, right, bottom, top, znear, zfar);
        // Flip z because cgmath::ortho does a right-to-left hand conversion by flipping z, but
        // we have already done a right-to-left hand conversion by flipping y. We're flipping y because our sprites are in screen coords,
        // where y grows downwards instead of upwards.
        ortho_mat.z.z = -ortho_mat.z.z;
        let view_proj_mat = OPENGL_TO_WGPU_MATRIX * ortho_mat;
        self.view_proj = view_proj_mat.into();
    }
}


#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
struct Instance {
    /// xyz, z being depth for render order but ignored due to orthographic projection, xy in screen coords
    position: [f32; 3],
    /// height/width of the desired sprite in pixels
    scale: [f32; 2],
}

// NEW!
impl Instance {
    fn desc() -> wgpu::VertexBufferLayout<'static> {
        use std::mem;
        wgpu::VertexBufferLayout {
            array_stride: mem::size_of::<Instance>() as wgpu::BufferAddress,
            // We need to switch from using a step mode of Vertex to Instance
            // This means that our shaders will only change to use the next
            // instance when the shader starts processing a new instance
            step_mode: wgpu::VertexStepMode::Instance,
            attributes: &[
                wgpu::VertexAttribute {
                    offset: 0,
                    // While our vertex shader only uses locations 0, and 1 now, in later tutorials we'll
                    // be using 2, 3, and 4, for Vertex. We'll start at slot 5 not conflict with them later
                    shader_location: 5,
                    format: wgpu::VertexFormat::Float32x3,
                },
                // A mat4 takes up 4 vertex slots as it is technically 4 vec4s. We need to define a slot
                // for each vec4. We don't have to do this in code though.
                wgpu::VertexAttribute {
                    offset: mem::size_of::<[f32; 3]>() as wgpu::BufferAddress,
                    shader_location: 6,
                    format: wgpu::VertexFormat::Float32x2,
                },
            ],
        }
    }
}

pub struct State {
    device: wgpu::Device,
    queue: wgpu::Queue,
    size: (u32, u32),
    render_pipeline: wgpu::RenderPipeline,
    vertex_buffer: wgpu::Buffer,
    index_buffer: wgpu::Buffer,
    num_indices: u32,
    #[allow(dead_code)]
    diffuse_texture: texture::Texture,
    diffuse_bind_group_layout: wgpu::BindGroupLayout,
    diffuse_bind_group: wgpu::BindGroup,
    camera_uniform: CameraUniform,
    camera_buffer: wgpu::Buffer,
    position_buffer: wgpu::Buffer,
    camera_bind_group: wgpu::BindGroup,
    instances: Vec<Instance>,
    #[allow(dead_code)]
    instance_buffer: wgpu::Buffer,
    // from windowless example
    texture_desc: wgpu::TextureDescriptor<'static>,
    texture: wgpu::Texture,
    texture_view: TextureView,
    output_buffer: wgpu::Buffer,
    adapter_info: AdapterInfo,
    depth_texture: texture::Texture,
}

const U32_SIZE: u32 = std::mem::size_of::<u32>() as u32;

impl State {
    fn bytes_per_row_256_aligned(pixel_width: u32) -> u32 {
        let pixels_per_row_aligned = ((pixel_width + 255) / 256) * 256;
        let bytes_per_row_aligned = pixels_per_row_aligned * U32_SIZE;
        bytes_per_row_aligned
    }

    pub async fn new(size: (u32, u32)) -> State {
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

        // From windowless example
        let texture_size = size;
        let texture_desc = wgpu::TextureDescriptor {
            size: wgpu::Extent3d {
                width: texture_size.0,
                height: texture_size.1,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsages::COPY_SRC | wgpu::TextureUsages::RENDER_ATTACHMENT,
            label: None,
            view_formats: &[],
        };
        let texture = device.create_texture(&texture_desc);
        let texture_view = texture.create_view(&Default::default());

        // we need to store this for later
        let output_buffer_size = (State::bytes_per_row_256_aligned(texture_size.0) * texture_size.1) as wgpu::BufferAddress;
        let output_buffer_desc = wgpu::BufferDescriptor {
            size: output_buffer_size,
            usage: wgpu::BufferUsages::COPY_DST
                // this tells wpgu that we want to read this buffer from the cpu
                | wgpu::BufferUsages::MAP_READ,
            label: None,
            mapped_at_creation: false,
        };
        let output_buffer = device.create_buffer(&output_buffer_desc);
        // End windowless example

        let diffuse_bytes = include_bytes!("happy-tree.png");
        let diffuse_texture =
            texture::Texture::from_bytes(&device, &queue, diffuse_bytes, "happy-tree.png").unwrap();

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
            ],
            label: Some("diffuse_bind_group"),
        });

        let mut camera_uniform = CameraUniform::new();
        camera_uniform.update_view_proj(0.0, size.0 as f32, size.1 as f32, 0.0);

        let camera_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Camera Buffer"),
            contents: bytemuck::cast_slice(&[camera_uniform]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let position_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Position Buffer"),
            contents: bytemuck::cast_slice(&[[size.0 as f32, size.1 as f32]]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        // let instances = (0..NUM_INSTANCES_PER_ROW)
        //     .flat_map(|z| {
        //         (0..NUM_INSTANCES_PER_ROW).map(move |x| {
        //             let position = cgmath::Vector3 {
        //                 x: x as f32,
        //                 y: 0.0,
        //                 z: z as f32,
        //             } - INSTANCE_DISPLACEMENT;
        //
        //             let rotation = if position.is_zero() {
        //                 // this is needed so an object at (0, 0, 0) won't get scaled to zero
        //                 // as Quaternions can effect scale if they're not created correctly
        //                 cgmath::Quaternion::from_axis_angle(
        //                     cgmath::Vector3::unit_z(),
        //                     cgmath::Deg(0.0),
        //                 )
        //             } else {
        //                 cgmath::Quaternion::from_axis_angle(position.normalize(), cgmath::Deg(45.0))
        //             };
        //
        //             Instance { position, rotation }
        //         })
        //     })
        //     .collect::<Vec<_>>();

        let instances = [
            Instance {
                position: [0.0, 0.0, 3.0],
                scale: [30.0, 30.0],
            },
            Instance {
                position: [0.0, 0.0, 2.0],
                scale: [30.0, 30.0],
            },
            Instance {
                position: [0.0, 0.0, 4.0],
                scale: [60.0, 60.0],
        }];

        let instance_data = instances.clone().into_iter().collect::<Vec<_>>();
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
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
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
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: position_buffer.as_entire_binding(),
                }],
            label: Some("camera_bind_group"),
        });

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("spriteshader.wgsl").into()),
        });

        let render_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Render Pipeline Layout"),
                bind_group_layouts: &[&diffuse_bind_group_layout, &camera_bind_group_layout],
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
                    format: texture_desc.format,
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
                // cull_mode: Some(wgpu::Face::Back),
                cull_mode: None,
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
        let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Index Buffer"),
            contents: bytemuck::cast_slice(INDICES),
            usage: wgpu::BufferUsages::INDEX,
        });
        let num_indices = INDICES.len() as u32;

        let depth_texture = texture::Texture::create_depth_texture(&device, size, "depth_texture");

        Self {
            device,
            queue,
            size,
            render_pipeline,
            vertex_buffer,
            index_buffer,
            num_indices,
            diffuse_texture,
            diffuse_bind_group_layout,
            diffuse_bind_group,
            camera_buffer,
            camera_bind_group,
            position_buffer,
            camera_uniform,
            instances: instances.to_vec(),
            instance_buffer,
            texture_desc,
            texture,
            texture_view,
            output_buffer,
            depth_texture,
            adapter_info: adapter.get_info(),
        }
    }

    pub fn get_adapter_info(&self) -> &AdapterInfo {
        &self.adapter_info
    }

    pub fn resize(&mut self, new_size: (u32, u32)) {
        if new_size != self.size {
            self.size = new_size;

            // adjust camera uniform
            self.camera_uniform.update_view_proj(0.0, self.size.0 as f32, self.size.1 as f32, 0.0);
            self.queue.write_buffer(
                &self.camera_buffer,
                0,
                bytemuck::cast_slice(&[self.camera_uniform]),
            );

            // adjust position buffer
            self.queue.write_buffer(
                &self.position_buffer,
                0,
                bytemuck::cast_slice(&[[self.size.0 as f32, self.size.1 as f32]]),
            );

            // adjust depth texture
            self.depth_texture = texture::Texture::create_depth_texture(&self.device, self.size, "depth_texture");

            // adjust windowless things
            self.texture_desc.size = wgpu::Extent3d {
                width: self.size.0,
                height: self.size.1,
                depth_or_array_layers: 1,
            };
            self.texture = self.device.create_texture(&self.texture_desc);
            self.texture_view = self.texture.create_view(&Default::default());

            // output
            let output_buffer_size = (State::bytes_per_row_256_aligned(self.size.0) * self.size.1) as wgpu::BufferAddress;
            let output_buffer_desc = wgpu::BufferDescriptor {
                size: output_buffer_size,
                usage: wgpu::BufferUsages::COPY_DST
                    // this tells wpgu that we want to read this buffer from the cpu
                    | wgpu::BufferUsages::MAP_READ,
                label: None,
                mapped_at_creation: false,
            };
            self.output_buffer = self.device.create_buffer(&output_buffer_desc);
        }
    }

    pub fn input(&mut self, event: &HashSet<KeyCode>) -> bool {
        // self.camera_controller.process_events(event)
        true
    }

    pub fn update_texture_to_hbd(&mut self, hbd: &HalfBlockDisplayRender) {
        self.diffuse_texture.update_to_hbd(&mut self.diffuse_bind_group, &self.diffuse_bind_group_layout, &mut self.device, &mut self.queue, Some("hbd texture"), hbd);
    }

    pub fn update(&mut self, x: usize, y: usize, shared_state: &SharedState<GameState>) {
        let x = x as f32;
        let y = y as f32;
        self.instances[0].position = [x, y, self.instances[0].position[2]];
        if shared_state.pressed_keys.did_press_char_ignore_case('w') {
            self.instances[0].scale = [self.instances[0].scale[0] + 1.0, self.instances[0].scale[1] + 1.0];
        }
        self.queue.write_buffer(
            &self.instance_buffer,
            0,
            bytemuck::cast_slice(&self.instances),
        );

        // self.camera_controller.update_camera(&mut self.camera);
        // self.camera_uniform.update_view_proj(&self.camera);
        // self.queue.write_buffer(
        //     &self.camera_buffer,
        //     0,
        //     bytemuck::cast_slice(&[self.camera_uniform]),
        // );
    }

    pub fn render(&mut self, hbd: &mut HalfBlockDisplayRender) -> Result<(), wgpu::SurfaceError> {
        let view = &self.texture_view;

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
                            a: 1.0,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &self.depth_texture.view,
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
            render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
            render_pass.set_vertex_buffer(1, self.instance_buffer.slice(..));
            // render_pass.set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint16);
            // UPDATED!
            // render_pass.draw_indexed(0..self.num_indices, 0, 0..self.instances.len() as _);
            render_pass.draw(0..6, 0..self.instances.len() as _);
        }

        let bytes_per_row = Self::bytes_per_row_256_aligned(self.size.0);

        // from windowless example
        encoder.copy_texture_to_buffer(
            wgpu::TexelCopyTextureInfo {
                aspect: wgpu::TextureAspect::All,
                texture: &self.texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
            },
            wgpu::TexelCopyBufferInfo {
                buffer: &self.output_buffer,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(bytes_per_row),
                    rows_per_image: Some(self.size.1),
                },
            },
            self.texture_desc.size,
        );

        self.queue.submit(iter::once(encoder.finish()));

        // Now we can read the buffer {
        {
            let buffer_slice = self.output_buffer.slice(..);

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
        self.output_buffer.unmap();

        Ok(())
    }
}


use std::collections::HashSet;
use std::iter;

use cgmath::prelude::*;
use crossterm::event::KeyCode;
use wgpu::{AdapterInfo, TextureView};
use wgpu::util::DeviceExt;
use teng::rendering::color::Color;
use teng::rendering::render::HalfBlockDisplayRender;


#[repr(C)]
#[derive(Copy, Default, Clone, bytemuck::Pod, bytemuck::Zeroable)]
struct ShaderToyInputs {
    size: [f32; 4],
    mouse: [f32; 4],
    time: f32,
    frame: i32,
    _padding: [i32; 2],
}


pub struct State {
    device: wgpu::Device,
    queue: wgpu::Queue,
    size: (u32, u32),
    render_pipeline: wgpu::RenderPipeline,
    inputs: ShaderToyInputs,
    inputs_buffer: wgpu::Buffer,
    inputs_bind_group: wgpu::BindGroup,
    // from windowless example
    texture_desc: wgpu::TextureDescriptor<'static>,
    texture: wgpu::Texture,
    texture_view: TextureView,
    output_buffer: wgpu::Buffer,
    adapter_info: AdapterInfo,
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
                power_preference: wgpu::PowerPreference::LowPower,
                // Select between low power or high performance GPU
                // power_preference: wgpu::PowerPreference::HighPerformance,
                compatible_surface: None,
                force_fallback_adapter: false,
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




        let mut inputs = ShaderToyInputs::default();
        inputs.size = [size.0 as f32, size.1 as f32, 1.0, 1.0];


        let inputs_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Inputs Buffer"),
            contents: bytemuck::cast_slice(&[inputs]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });


        let inputs_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                }],
                label: Some("inputs_bind_group_layout"),
            });

        let inputs_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &inputs_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: inputs_buffer.as_entire_binding(),
            }],
            label: Some("inputs_bind_group"),
        });

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("../marble.wgsl").into()),
            // source: wgpu::ShaderSource::Wgsl(include_str!("../shaderart2.wgsl").into()),
        });

        let render_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Render Pipeline Layout"),
                bind_group_layouts: &[&inputs_bind_group_layout],
                push_constant_ranges: &[],
            });

        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Render Pipeline"),
            layout: Some(&render_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                buffers: &[],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: texture_desc.format,
                    // blend: Some(wgpu::BlendState {
                    //     color: wgpu::BlendComponent::REPLACE,
                    //     alpha: wgpu::BlendComponent::REPLACE,
                    // }),
                    blend: Some(wgpu::BlendState {
                        color: wgpu::BlendComponent {
                            operation: wgpu::BlendOperation::Add,
                            src_factor: wgpu::BlendFactor::SrcAlpha,
                            dst_factor: wgpu::BlendFactor::OneMinusSrcAlpha,
                        },
                        alpha: wgpu::BlendComponent {
                            operation: wgpu::BlendOperation::Add,
                            src_factor: wgpu::BlendFactor::SrcAlpha,
                            dst_factor: wgpu::BlendFactor::OneMinusSrcAlpha,
                        },
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
                // Setting this to anything other than Fill requires Features::POLYGON_MODE_LINE
                // or Features::POLYGON_MODE_POINT
                polygon_mode: wgpu::PolygonMode::Fill,
                // Requires Features::DEPTH_CLIP_CONTROL
                unclipped_depth: false,
                // Requires Features::CONSERVATIVE_RASTERIZATION
                conservative: false,
            },
            depth_stencil: None,
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


        Self {
            device,
            queue,
            size,
            render_pipeline,
            inputs,
            inputs_buffer,
            inputs_bind_group,
            texture_desc,
            texture,
            texture_view,
            output_buffer,
            adapter_info: adapter.get_info(),
        }
    }

    pub fn set_mouse_input(&mut self, mouse: (f32, f32), mouse_pressed: (f32, f32)) {
        let input = [mouse.0, mouse.1, mouse_pressed.0, mouse_pressed.1];
        if input != self.inputs.mouse {
            self.inputs.mouse = input;
            self.queue.write_buffer(
                &self.inputs_buffer,
                0,
                bytemuck::cast_slice(&[self.inputs]),
            );
        }
    }

    pub fn get_adapter_info(&self) -> &AdapterInfo {
        &self.adapter_info
    }

    pub fn resize(&mut self, new_size: (u32, u32)) {
        if new_size != self.size {
            self.size = new_size;
            self.inputs.size = [new_size.0 as f32, new_size.1 as f32, 1.0, 1.0];
            self.queue.write_buffer(
                &self.inputs_buffer,
                0,
                bytemuck::cast_slice(&[self.inputs]),
            );

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

    // pub fn input(&mut self, event: &HashSet<KeyCode>) -> bool {
    //     self.camera_controller.process_events(event)
    // }

    pub fn update(&mut self, total_time_secs: f32, frames: i32) {
        self.inputs.frame = frames;
        self.inputs.time = total_time_secs;
        self.queue.write_buffer(
            &self.inputs_buffer,
            0,
            bytemuck::cast_slice(&[self.inputs]),
        );
    }

    pub fn render(&mut self, hbd: &mut HalfBlockDisplayRender, alphablend: bool) -> Result<(), wgpu::SurfaceError> {
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
                            r: 0.1,
                            g: 0.2,
                            b: 0.3,
                            a: 1.0,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                occlusion_query_set: None,
                timestamp_writes: None,
            });

            render_pass.set_pipeline(&self.render_pipeline);
            render_pass.set_bind_group(0, &self.inputs_bind_group, &[]);
            render_pass.draw(0..6, 0..1);
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
                let mut r = window[0];
                let mut g = window[1];
                let mut b = window[2];
                let mut a = window[3];

                // do alpha blend
                if alphablend {
                    let ratio = a as f32 / 255.0;
                    // assume black background
                    let bg = 0;
                    assert!(a == 255 || a == 0);
                    r = (r as f32 * ratio + bg as f32 * (1.0 - ratio)) as u8;
                    g = (g as f32 * ratio + bg as f32 * (1.0 - ratio)) as u8;
                    b = (b as f32 * ratio + bg as f32 * (1.0 - ratio)) as u8;
                    // reset alpha
                    a = 255;
                }

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


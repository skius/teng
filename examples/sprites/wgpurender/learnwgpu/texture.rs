use anyhow::*;
use image::GenericImageView;
use teng::rendering::color::Color;
use teng::rendering::render::HalfBlockDisplayRender;

pub struct Texture {
    #[allow(unused)]
    pub texture: wgpu::Texture,
    pub view: wgpu::TextureView,
    pub sampler: wgpu::Sampler,
}

fn buf_from_hbd(hbd: &HalfBlockDisplayRender) -> (Vec<u8>, u32, u32) {
    let height = hbd.height() as u32;
    let width = hbd.width() as u32;

    let bytes_per_row = 4 * width;
    // align to 256 bytes
    let bytes_per_row = (bytes_per_row + 255) & !255;
    let rows_per_image = height;
    let mut buf = vec![0u8; (bytes_per_row * rows_per_image) as usize];
    let mut x = 0;
    let mut y = 0;
    let mut idx = 0;
    loop {
        let color = hbd.get_color(x, y).unwrap();
        let rgba = match  color {
            Color::Default => {
                [0, 0, 0, 0]
            }
            Color::Transparent => {
                [0, 0, 0, 0]
            }
            Color::Rgb(rgb) => {
                [rgb[0], rgb[1], rgb[2], 255]
            }
        };
        buf[idx..idx + 4].copy_from_slice(&rgba);
        idx += 4;
        x += 1;
        if x == width as usize {
            x = 0;
            y += 1;
            // jump idx to the next row
            idx = y * bytes_per_row as usize;
        }
        if y == height as usize {
            break;
        }
    }
    (buf, bytes_per_row, rows_per_image)
}

impl Texture {
    pub fn update_to_hbd(&mut self, bind_group: &mut wgpu::BindGroup, bind_group_layout: &wgpu::BindGroupLayout, device: &mut wgpu::Device, queue: &mut wgpu::Queue, label: Option<&str>, hbd: &HalfBlockDisplayRender) {
        let (buf, bytes_per_row, rows_per_image) = buf_from_hbd(hbd);
        let size = wgpu::Extent3d {
            width: hbd.width() as u32,
            height: hbd.height() as u32,
            depth_or_array_layers: 1,
        };

        if self.texture.size() != size {
            // recreate the texture, since its size changed
            let format = wgpu::TextureFormat::Rgba8UnormSrgb;

            self.texture = device.create_texture(&wgpu::TextureDescriptor {
                label,
                size,
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format,
                usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
                view_formats: &[],
            });

            self.view = self.texture.create_view(&wgpu::TextureViewDescriptor::default());

            // since we changed the texture, we also need to adjust the bind group.
            *bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
                layout: &bind_group_layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::TextureView(&self.view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::Sampler(&self.sampler),
                    },
                ],
                label: Some("diffuse_bind_group"),
            });
        }
        

        queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                aspect: wgpu::TextureAspect::All,
                texture: &self.texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
            },
            &buf,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(bytes_per_row),
                rows_per_image: Some(rows_per_image),
            },
            size,
        );


    }

    pub fn from_hbd(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        hbd: &HalfBlockDisplayRender,
        label: Option<&str>,
    ) -> Result<Self> {
        let height = hbd.height() as u32;
        let width = hbd.width() as u32;

        let size = wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        };


        let bytes_per_row = 4 * width;
        // align to 256 bytes
        let bytes_per_row = (bytes_per_row + 255) & !255;
        let rows_per_image = height;
        let mut buf = vec![0u8; (bytes_per_row * rows_per_image) as usize];
        let mut x = 0;
        let mut y = 0;
        let mut idx = 0;
        loop {
            let color = hbd.get_color(x, y).unwrap();
            let rgba = match  color {
                Color::Default => {
                    [0, 0, 0, 0]
                }
                Color::Transparent => {
                    [0, 0, 0, 0]
                }
                Color::Rgb(rgb) => {
                    [rgb[0], rgb[1], rgb[2], 255]
                }
            };
            buf[idx..idx + 4].copy_from_slice(&rgba);
            idx += 4;
            x += 1;
            if x == width as usize {
                x = 0;
                y += 1;
                // jump idx to the next row
                idx = y * bytes_per_row as usize;
            }
            if y == height as usize {
                break;
            }
        }


        let format = wgpu::TextureFormat::Rgba8UnormSrgb;
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label,
            size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });

        queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                aspect: wgpu::TextureAspect::All,
                texture: &texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
            },
            &buf,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(bytes_per_row),
                rows_per_image: Some(rows_per_image),
            },
            size,
        );

        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Nearest,
            min_filter: wgpu::FilterMode::Nearest,
            mipmap_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });

        Ok(Self {
            texture,
            view,
            sampler,
        })
    }

    pub fn from_bytes(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        bytes: &[u8],
        label: &str,
    ) -> Result<Self> {
        let img = image::load_from_memory(bytes)?;
        Self::from_image(device, queue, &img, Some(label))
    }

    pub fn from_image(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        img: &image::DynamicImage,
        label: Option<&str>,
    ) -> Result<Self> {
        let rgba = img.to_rgba8();
        let dimensions = img.dimensions();

        let size = wgpu::Extent3d {
            width: dimensions.0,
            height: dimensions.1,
            depth_or_array_layers: 1,
        };
        let format = wgpu::TextureFormat::Rgba8UnormSrgb;
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label,
            size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });

        queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                aspect: wgpu::TextureAspect::All,
                texture: &texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
            },
            &rgba,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(4 * dimensions.0),
                rows_per_image: Some(dimensions.1),
            },
            size,
        );

        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Nearest,
            mipmap_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });

        Ok(Self {
            texture,
            view,
            sampler,
        })
    }
}
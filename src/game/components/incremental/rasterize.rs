use crate::game::{
    Color, Component, HalfBlockDisplayRender, Render, Renderer, SetupInfo, SharedState, UpdateInfo,
};
use crossterm::event::KeyCode;
use std::ops::{AddAssign, SubAssign};

#[derive(Clone, Copy, Debug)]
struct Vec3 {
    x: f32,
    y: f32,
    z: f32,
}

impl Vec3 {
    fn dot(self, other: Vec3) -> f32 {
        self.x * other.x + self.y * other.y + self.z * other.z
    }

    fn normalize(self) -> Vec3 {
        let mag = (self.dot(self)).sqrt();
        Vec3 {
            x: self.x / mag,
            y: self.y / mag,
            z: self.z / mag,
        }
    }

    fn subtract(self, other: Vec3) -> Vec3 {
        Vec3 {
            x: self.x - other.x,
            y: self.y - other.y,
            z: self.z - other.z,
        }
    }

    fn cross(self, other: Vec3) -> Vec3 {
        Vec3 {
            x: self.y * other.z - self.z * other.y,
            y: self.z * other.x - self.x * other.z,
            z: self.x * other.y - self.y * other.x,
        }
    }
}

impl AddAssign<Vec3> for Vec3 {
    fn add_assign(&mut self, other: Vec3) {
        self.x += other.x;
        self.y += other.y;
        self.z += other.z;
    }
}

impl SubAssign<Vec3> for Vec3 {
    fn sub_assign(&mut self, other: Vec3) {
        self.x -= other.x;
        self.y -= other.y;
        self.z -= other.z;
    }
}

#[derive(Clone, Copy, Debug)]
struct Mat4 {
    data: [[f32; 4]; 4],
}

impl Mat4 {
    fn identity() -> Mat4 {
        Mat4 {
            data: [
                [1.0, 0.0, 0.0, 0.0],
                [0.0, 1.0, 0.0, 0.0],
                [0.0, 0.0, 1.0, 0.0],
                [0.0, 0.0, 0.0, 1.0],
            ],
        }
    }

    fn from_cols(c0: [f32; 4], c1: [f32; 4], c2: [f32; 4], c3: [f32; 4]) -> Mat4 {
        Mat4 {
            data: [
                [c0[0], c1[0], c2[0], c3[0]],
                [c0[1], c1[1], c2[1], c3[1]],
                [c0[2], c1[2], c2[2], c3[2]],
                [c0[3], c1[3], c2[3], c3[3]],
            ],
        }
    }

    fn perspective(fov: f32, aspect: f32, near: f32, far: f32) -> Mat4 {
        let f = 1.0 / (fov.to_radians() / 2.0).tan();
        // Mat4 {
        //     data: [
        //         [f / aspect, 0.0, 0.0, 0.0],
        //         [0.0, f, 0.0, 0.0],
        //         [0.0, 0.0, (far + near) / (near - far), (2.0 * far * near) / (near - far)],
        //         [0.0, 0.0, -1.0, 0.0],
        //     ],
        // }
        Mat4::from_cols(
            [f / aspect, 0.0, 0.0, 0.0],
            [0.0, f, 0.0, 0.0],
            [
                0.0,
                0.0,
                (far + near) / (near - far),
                (2.0 * far * near) / (near - far),
            ],
            [0.0, 0.0, -1.0, 0.0],
        )
    }

    fn transform(&self, v: Vec3) -> Vec3 {
        let x =
            v.x * self.data[0][0] + v.y * self.data[1][0] + v.z * self.data[2][0] + self.data[3][0];
        let y =
            v.x * self.data[0][1] + v.y * self.data[1][1] + v.z * self.data[2][1] + self.data[3][1];
        let z =
            v.x * self.data[0][2] + v.y * self.data[1][2] + v.z * self.data[2][2] + self.data[3][2];
        let w =
            v.x * self.data[0][3] + v.y * self.data[1][3] + v.z * self.data[2][3] + self.data[3][3];

        if w != 0.0 {
            Vec3 {
                x: x / w,
                y: y / w,
                z: z / w,
            }
        } else {
            Vec3 { x, y, z }
        }
    }
}

struct Triangle {
    v0: Vec3,
    v1: Vec3,
    v2: Vec3,
    n0: Vec3, // Normal at v0
    n1: Vec3, // Normal at v1
    n2: Vec3, // Normal at v2
}

impl Triangle {
    fn compute_normal(&self) -> Vec3 {
        let edge1 = self.v1.subtract(self.v0);
        let edge2 = self.v2.subtract(self.v0);
        edge1.cross(edge2).normalize()
    }
}

struct Mesh {
    triangles: Vec<Triangle>,
    color: [u8; 3],
}

struct Scene {
    meshes: Vec<Mesh>,
    camera: Camera,
    light_pos: Vec3,
}

struct Camera {
    position: Vec3,
    orientation: Mat4, // For rotation (optional)
    fov: f32,
    aspect: f32,
    near: f32,
    far: f32,
}

impl Camera {
    fn new(
        position: Vec3,
        orientation: Mat4,
        fov: f32,
        width: usize,
        height: usize,
        near: f32,
        far: f32,
    ) -> Self {
        let aspect = width as f32 / height as f32;
        Self {
            position,
            orientation,
            fov,
            aspect,
            near,
            far,
        }
    }

    fn projection_matrix(&self) -> Mat4 {
        Mat4::perspective(self.fov, self.aspect, self.near, self.far)
    }
}

fn edge_function(v0: (usize, usize), v1: (usize, usize), p: (usize, usize)) -> i32 {
    (p.0 as i32 - v0.0 as i32) * (v1.1 as i32 - v0.1 as i32)
        - (p.1 as i32 - v0.1 as i32) * (v1.0 as i32 - v0.0 as i32)
}

fn fill_triangle(
    v0: (usize, usize, f32, Vec3),
    v1: (usize, usize, f32, Vec3),
    v2: (usize, usize, f32, Vec3),
    framebuffer: &mut Vec<[u8; 3]>,
    depth_buffer: &mut Vec<f32>,
    width: usize,
    height: usize,
    color: [u8; 3],
    light_pos: Vec3,
) {
    let min_x = v0.0.min(v1.0).min(v2.0);
    let max_x = v0.0.max(v1.0).max(v2.0);
    let min_y = v0.1.min(v1.1).min(v2.1);
    let max_y = v0.1.max(v1.1).max(v2.1);

    let area = edge_function((v0.0, v0.1), (v1.0, v1.1), (v2.0, v2.1)) as f32;

    for y in min_y..=max_y {
        for x in min_x..=max_x {
            let w0 = edge_function((v1.0, v1.1), (v2.0, v2.1), (x, y)) as f32 / area;
            let w1 = edge_function((v2.0, v2.1), (v0.0, v0.1), (x, y)) as f32 / area;
            let w2 = edge_function((v0.0, v0.1), (v1.0, v1.1), (x, y)) as f32 / area;

            if w0 >= 0.0 && w1 >= 0.0 && w2 >= 0.0 {
                let z = w0 * v0.2 + w1 * v1.2 + w2 * v2.2;
                let idx = y * width + x;

                if z < depth_buffer[idx] {
                    depth_buffer[idx] = z;

                    // Interpolated normal
                    let normal = Vec3 {
                        x: w0 * v0.3.x + w1 * v1.3.x + w2 * v2.3.x,
                        y: w0 * v0.3.y + w1 * v1.3.y + w2 * v2.3.y,
                        z: w0 * v0.3.z + w1 * v1.3.z + w2 * v2.3.z,
                    }
                    .normalize();

                    // Compute lighting
                    let light_dir = light_pos.normalize();
                    let ambient = 0.1;
                    // determine front or backface
                    let diffuse = if normal.dot(light_dir) > 0.0 {
                        normal.dot(light_dir)
                    } else {
                        0.0
                    };
                    let intensity = ambient + (1.0 - ambient) * diffuse;

                    framebuffer[idx] = [
                        (color[0] as f32 * intensity) as u8,
                        (color[1] as f32 * intensity) as u8,
                        (color[2] as f32 * intensity) as u8,
                    ];

                    // actually, just draw distance in b channel
                    let min_dist = 0.0;
                    let max_dist = 1.5;
                    let dist_interp = (z - min_dist) / (max_dist - min_dist);
                    framebuffer[idx] = [0, 0, (dist_interp * 255.0) as u8];
                    // panic!("{z}")
                }
            }
        }
    }
}

fn render(scene: &Scene, width: usize, height: usize) -> Vec<[u8; 3]> {
    let mut framebuffer = vec![[0, 0, 0]; width * height];
    let mut depth_buffer = vec![f32::INFINITY; width * height];

    let projection = scene.camera.projection_matrix();

    for mesh in &scene.meshes {
        for triangle in &mesh.triangles {
            let v0_proj = projection.transform(triangle.v0);
            let v1_proj = projection.transform(triangle.v1);
            let v2_proj = projection.transform(triangle.v2);

            let to_screen = |v: Vec3, n: Vec3| -> (usize, usize, f32, Vec3) {
                let x = ((v.x + 1.0) * 0.5 * width as f32) as usize;
                let y = ((1.0 - v.y) * 0.5 * height as f32) as usize;
                (x.min(width - 1), y.min(height - 1), v.z, n)
            };

            let v0 = to_screen(v0_proj, triangle.n0);
            let v1 = to_screen(v1_proj, triangle.n1);
            let v2 = to_screen(v2_proj, triangle.n2);

            fill_triangle(
                v0,
                v1,
                v2,
                &mut framebuffer,
                &mut depth_buffer,
                width,
                height,
                mesh.color,
                scene.light_pos,
            );
        }
    }

    framebuffer
}

fn create_cube(center: Vec3, size: f32, color: [u8; 3]) -> Mesh {
    let half = size / 2.0;

    // Cube vertices
    let v = [
        Vec3 {
            x: center.x - half,
            y: center.y - half,
            z: center.z - half,
        }, // 0
        Vec3 {
            x: center.x + half,
            y: center.y - half,
            z: center.z - half,
        }, // 1
        Vec3 {
            x: center.x + half,
            y: center.y + half,
            z: center.z - half,
        }, // 2
        Vec3 {
            x: center.x - half,
            y: center.y + half,
            z: center.z - half,
        }, // 3
        Vec3 {
            x: center.x - half,
            y: center.y - half,
            z: center.z + half,
        }, // 4
        Vec3 {
            x: center.x + half,
            y: center.y - half,
            z: center.z + half,
        }, // 5
        Vec3 {
            x: center.x + half,
            y: center.y + half,
            z: center.z + half,
        }, // 6
        Vec3 {
            x: center.x - half,
            y: center.y + half,
            z: center.z + half,
        }, // 7
    ];

    // Normals per vertex (for smooth shading)
    let n = [
        Vec3 {
            x: -1.0,
            y: -1.0,
            z: -1.0,
        }
        .normalize(),
        Vec3 {
            x: 1.0,
            y: -1.0,
            z: -1.0,
        }
        .normalize(),
        Vec3 {
            x: 1.0,
            y: 1.0,
            z: -1.0,
        }
        .normalize(),
        Vec3 {
            x: -1.0,
            y: 1.0,
            z: -1.0,
        }
        .normalize(),
        Vec3 {
            x: -1.0,
            y: -1.0,
            z: 1.0,
        }
        .normalize(),
        Vec3 {
            x: 1.0,
            y: -1.0,
            z: 1.0,
        }
        .normalize(),
        Vec3 {
            x: 1.0,
            y: 1.0,
            z: 1.0,
        }
        .normalize(),
        Vec3 {
            x: -1.0,
            y: 1.0,
            z: 1.0,
        }
        .normalize(),
    ];

    // Define the cube faces (two triangles per face)
    let indices = [
        (0, 1, 2),
        (0, 2, 3), // Front
        (1, 5, 6),
        (1, 6, 2), // Right
        (5, 4, 7),
        (5, 7, 6), // Back
        (4, 0, 3),
        (4, 3, 7), // Left
        (3, 2, 6),
        (3, 6, 7), // Top
        (4, 5, 1),
        (4, 1, 0), // Bottom
    ];

    let triangles = indices
        .iter()
        .map(|&(i0, i1, i2)| Triangle {
            v0: v[i0],
            v1: v[i1],
            v2: v[i2],
            n0: n[i0],
            n1: n[i1],
            n2: n[i2],
        })
        .collect();

    Mesh { triangles, color }
}

fn rotate_mesh_y(mesh: &mut Mesh, angle: f32) {
    let cos_a = angle.cos();
    let sin_a = angle.sin();

    // Compute the center of the mesh
    let mut center = Vec3 {
        x: 0.0,
        y: 0.0,
        z: 0.0,
    };
    let num_vertices = mesh.triangles.len() * 3;

    for triangle in &mesh.triangles {
        center.x += triangle.v0.x + triangle.v1.x + triangle.v2.x;
        center.y += triangle.v0.y + triangle.v1.y + triangle.v2.y;
        center.z += triangle.v0.z + triangle.v1.z + triangle.v2.z;
    }

    center.x /= num_vertices as f32;
    center.y /= num_vertices as f32;
    center.z /= num_vertices as f32;

    // let rotation_matrix = Mat4 {
    //     data: [
    //         [cos_a, 0.0, -sin_a, 0.0],
    //         [0.0, 1.0, 0.0, 0.0],
    //         [sin_a, 0.0, cos_a, 0.0],
    //         [0.0, 0.0, 0.0, 1.0],
    //     ],
    // };

    let rotation_matrix = Mat4::from_cols(
        [cos_a, 0.0, -sin_a, 0.0],
        [0.0, 1.0, 0.0, 0.0],
        [sin_a, 0.0, cos_a, 0.0],
        [0.0, 0.0, 0.0, 1.0],
    );

    for triangle in &mut mesh.triangles {
        // Move the vertices to the origin
        triangle.v0 -= center;
        triangle.v1 -= center;
        triangle.v2 -= center;

        // Rotate
        triangle.v0 = rotation_matrix.transform(triangle.v0);
        triangle.v1 = rotation_matrix.transform(triangle.v1);
        triangle.v2 = rotation_matrix.transform(triangle.v2);

        // Move them back
        triangle.v0 += center;
        triangle.v1 += center;
        triangle.v2 += center;

        // Rotate normals (only rotation, no translation needed)
        triangle.n0 = rotation_matrix.transform(triangle.n0).normalize();
        triangle.n1 = rotation_matrix.transform(triangle.n1).normalize();
        triangle.n2 = rotation_matrix.transform(triangle.n2).normalize();
    }
}

fn rotate_mesh_x(mesh: &mut Mesh, angle: f32) {
    let cos_a = angle.cos();
    let sin_a = angle.sin();

    // Compute the center of the mesh
    let mut center = Vec3 {
        x: 0.0,
        y: 0.0,
        z: 0.0,
    };
    let num_vertices = mesh.triangles.len() * 3;

    for triangle in &mesh.triangles {
        center.x += triangle.v0.x + triangle.v1.x + triangle.v2.x;
        center.y += triangle.v0.y + triangle.v1.y + triangle.v2.y;
        center.z += triangle.v0.z + triangle.v1.z + triangle.v2.z;
    }

    center.x /= num_vertices as f32;
    center.y /= num_vertices as f32;
    center.z /= num_vertices as f32;

    // let rotation_matrix = Mat4 {
    //     data: [
    //         [1.0, 0.0, 0.0, 0.0],
    //         [0.0, cos_a, sin_a, 0.0],
    //         [0.0, -sin_a, cos_a, 0.0],
    //         [0.0, 0.0, 0.0, 1.0],
    //     ],
    // };

    let rotation_matrix = Mat4::from_cols(
        [1.0, 0.0, 0.0, 0.0],
        [0.0, cos_a, sin_a, 0.0],
        [0.0, -sin_a, cos_a, 0.0],
        [0.0, 0.0, 0.0, 1.0],
    );

    for triangle in &mut mesh.triangles {
        // Move the vertices to the origin
        triangle.v0 -= center;
        triangle.v1 -= center;
        triangle.v2 -= center;

        // Rotate
        triangle.v0 = rotation_matrix.transform(triangle.v0);
        triangle.v1 = rotation_matrix.transform(triangle.v1);
        triangle.v2 = rotation_matrix.transform(triangle.v2);

        // Move them back
        triangle.v0 += center;
        triangle.v1 += center;
        triangle.v2 += center;

        // Rotate normals (only rotation, no translation needed)
        triangle.n0 = rotation_matrix.transform(triangle.n0).normalize();
        triangle.n1 = rotation_matrix.transform(triangle.n1).normalize();
        triangle.n2 = rotation_matrix.transform(triangle.n2).normalize();
    }
}

pub struct RasterizeComponent {
    half_block_display_render: HalfBlockDisplayRender,
    rotation_angle: f32,
    rotation_x_angle: f32,
}

impl RasterizeComponent {
    pub fn new() -> Self {
        Self {
            half_block_display_render: HalfBlockDisplayRender::new(0, 0),
            rotation_angle: 0.0,
            rotation_x_angle: 0.0,
        }
    }
}

impl Component for RasterizeComponent {
    fn setup(&mut self, setup_info: &SetupInfo, shared_state: &mut SharedState) {
        self.half_block_display_render
            .resize_discard(setup_info.width, 2 * setup_info.height);
    }

    fn on_resize(&mut self, width: usize, height: usize, shared_state: &mut SharedState) {
        self.half_block_display_render
            .resize_discard(width, 2 * height);
    }

    fn update(&mut self, update_info: UpdateInfo, shared_state: &mut SharedState) {
        if shared_state
            .pressed_keys
            .inner()
            .contains_key(&KeyCode::Left)
        {
            self.rotation_angle += 1.0;
        }
        if shared_state
            .pressed_keys
            .inner()
            .contains_key(&KeyCode::Right)
        {
            self.rotation_angle -= 1.0;
        }
        if shared_state.pressed_keys.inner().contains_key(&KeyCode::Up) {
            self.rotation_x_angle += 1.0;
        }
        if shared_state
            .pressed_keys
            .inner()
            .contains_key(&KeyCode::Down)
        {
            self.rotation_x_angle -= 1.0;
        }

        let height = self.half_block_display_render.height();
        let width = self.half_block_display_render.width();

        // update half_block_display_render
        self.half_block_display_render.clear();
        let triangle = Triangle {
            v0: Vec3 {
                x: -0.5,
                y: -0.5,
                z: 1.0,
            },
            v1: Vec3 {
                x: 0.5,
                y: -0.5,
                z: 1.0,
            },
            v2: Vec3 {
                x: 0.0,
                y: 0.5,
                z: 1.0,
            },
            n0: Vec3 {
                x: -0.5,
                y: -0.5,
                z: 1.0,
            }
            .normalize(),
            n1: Vec3 {
                x: 0.5,
                y: -0.5,
                z: 1.0,
            }
            .normalize(),
            n2: Vec3 {
                x: 0.0,
                y: 0.5,
                z: 1.0,
            }
            .normalize(),
        };

        let mesh = Mesh {
            triangles: vec![triangle],
            color: [255, 100, 50], // Orange color
        };

        let mut cube = create_cube(
            Vec3 {
                x: 0.0,
                y: 0.0,
                z: 0.0,
            },
            100.0,
            [255, 0, 0],
        ); // Red cube
        rotate_mesh_y(&mut cube, self.rotation_angle.to_radians());
        rotate_mesh_x(&mut cube, self.rotation_x_angle.to_radians());

        let camera = Camera::new(
            Vec3 {
                x: 0.0,
                y: 0.0,
                z: 200.0,
            },
            Mat4::identity(),
            90.0,
            width,
            height,
            0.1,
            100.0,
        );

        let scene = Scene {
            meshes: vec![cube],
            camera,
            light_pos: Vec3 {
                x: 1.0,
                y: 10.0,
                z: -1.0,
            },
        };

        let framebuffer = render(&scene, width, height);
        for y in 0..height {
            for x in 0..width {
                let color_top = framebuffer[y * width + x];

                self.half_block_display_render
                    .set_color(x, height - y - 1, Color::Rgb(color_top));
            }
        }
        // self.half_block_display_render.set_color(0, 0, Color::Rgb([255, 255, 255]));
    }

    fn render(&self, mut renderer: &mut dyn Renderer, shared_state: &SharedState, depth_base: i32) {
        let depth_base = i32::MAX - 200;
        self.half_block_display_render
            .render(&mut renderer, 0, 0, depth_base);
    }
}

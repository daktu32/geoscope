// renderer/globe.rs — Globe (3D sphere) renderer

use eframe::CreationContext;
use egui::epaint;
use wgpu::util::DeviceExt;

use super::common::{
    build_view_proj, colormap_lut, identity_mat4, CameraUniform, Vertex,
};

// ---------------------------------------------------------------------------
// GPU resources stored in CallbackResources
// ---------------------------------------------------------------------------

struct GlobeGpuResources {
    pipeline: wgpu::RenderPipeline,
    vertex_buffer: wgpu::Buffer,
    index_buffer: wgpu::Buffer,
    num_indices: u32,
    camera_buffer: wgpu::Buffer,
    camera_bind_group: wgpu::BindGroup,
    data_texture: wgpu::Texture,
    data_bind_group: wgpu::BindGroup,
    data_width: u32,
    data_height: u32,
}

// ---------------------------------------------------------------------------
// Paint callback
// ---------------------------------------------------------------------------

struct GlobePaintCallback {
    camera_uniform: CameraUniform,
}

impl egui_wgpu::CallbackTrait for GlobePaintCallback {
    fn prepare(
        &self,
        _device: &wgpu::Device,
        queue: &wgpu::Queue,
        _screen_descriptor: &egui_wgpu::ScreenDescriptor,
        _egui_encoder: &mut wgpu::CommandEncoder,
        callback_resources: &mut egui_wgpu::CallbackResources,
    ) -> Vec<wgpu::CommandBuffer> {
        if let Some(res) = callback_resources.get::<GlobeGpuResources>() {
            queue.write_buffer(
                &res.camera_buffer,
                0,
                bytemuck::cast_slice(&[self.camera_uniform]),
            );
        }
        Vec::new()
    }

    fn paint(
        &self,
        info: epaint::PaintCallbackInfo,
        render_pass: &mut wgpu::RenderPass<'static>,
        callback_resources: &egui_wgpu::CallbackResources,
    ) {
        let Some(res) = callback_resources.get::<GlobeGpuResources>() else {
            return;
        };

        let viewport = info.viewport_in_pixels();
        render_pass.set_viewport(
            viewport.left_px as f32,
            viewport.top_px as f32,
            viewport.width_px as f32,
            viewport.height_px as f32,
            0.0,
            1.0,
        );
        let clip = info.clip_rect_in_pixels();
        render_pass.set_scissor_rect(
            clip.left_px as u32,
            clip.top_px as u32,
            clip.width_px as u32,
            clip.height_px as u32,
        );

        render_pass.set_pipeline(&res.pipeline);
        render_pass.set_bind_group(0, &res.camera_bind_group, &[]);
        render_pass.set_bind_group(1, &res.data_bind_group, &[]);
        render_pass.set_vertex_buffer(0, res.vertex_buffer.slice(..));
        render_pass.set_index_buffer(res.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
        render_pass.draw_indexed(0..res.num_indices, 0, 0..1);
    }
}

// ---------------------------------------------------------------------------
// GlobeRenderer — public API
// ---------------------------------------------------------------------------

pub struct GlobeRenderer {
    pub cam_lon: f32,
    pub cam_lat: f32,
    pub zoom: f32,
    data_min: f32,
    data_max: f32,
    interpolated: bool,
    initialized: bool,
}

impl GlobeRenderer {
    pub fn new(cc: &CreationContext<'_>) -> Self {
        if let Some(render_state) = &cc.wgpu_render_state {
            Self::init_gpu_resources(render_state);
        }

        Self {
            cam_lon: 0.0,
            cam_lat: 0.0,
            zoom: 1.0,
            data_min: 0.0,
            data_max: 1.0,
            interpolated: true,
            initialized: cc.wgpu_render_state.is_some(),
        }
    }

    fn init_gpu_resources(render_state: &egui_wgpu::RenderState) {
        let device = &render_state.device;
        let target_format = render_state.target_format;

        let (vertices, indices) = generate_uv_sphere(64, 32);

        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Globe Vertex Buffer"),
            contents: bytemuck::cast_slice(&vertices),
            usage: wgpu::BufferUsages::VERTEX,
        });

        let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Globe Index Buffer"),
            contents: bytemuck::cast_slice(&indices),
            usage: wgpu::BufferUsages::INDEX,
        });

        let camera_uniform = CameraUniform {
            view_proj: identity_mat4(),
            view: identity_mat4(),
            data_range: [0.0, 1.0],
            params: [0.0; 2],
        };

        let camera_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Globe Camera Buffer"),
            contents: bytemuck::cast_slice(&[camera_uniform]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let camera_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Globe Camera BGL"),
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                }],
            });

        let camera_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Globe Camera BG"),
            layout: &camera_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: camera_buffer.as_entire_binding(),
            }],
        });

        let data_texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Globe Data Texture"),
            size: wgpu::Extent3d {
                width: 1,
                height: 1,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::R32Float,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });

        let data_texture_view = data_texture.create_view(&wgpu::TextureViewDescriptor::default());

        let data_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("Globe Data Sampler"),
            address_mode_u: wgpu::AddressMode::Repeat,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Nearest,
            min_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });

        let colormap_data = colormap_lut(crate::ui::Colormap::Viridis);
        let colormap_texture = device.create_texture_with_data(
            &render_state.queue,
            &wgpu::TextureDescriptor {
                label: Some("Globe Colormap LUT"),
                size: wgpu::Extent3d {
                    width: 256,
                    height: 1,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: wgpu::TextureFormat::Rgba8UnormSrgb,
                usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
                view_formats: &[],
            },
            wgpu::util::TextureDataOrder::LayerMajor,
            &colormap_data,
        );

        let colormap_view = colormap_texture.create_view(&wgpu::TextureViewDescriptor::default());

        let colormap_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("Globe Colormap Sampler"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });

        let data_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Globe Data BGL"),
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            sample_type: wgpu::TextureSampleType::Float { filterable: false },
                            view_dimension: wgpu::TextureViewDimension::D2,
                            multisampled: false,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::NonFiltering),
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 2,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            sample_type: wgpu::TextureSampleType::Float { filterable: true },
                            view_dimension: wgpu::TextureViewDimension::D2,
                            multisampled: false,
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
            });

        let data_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Globe Data BG"),
            layout: &data_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&data_texture_view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&data_sampler),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::TextureView(&colormap_view),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: wgpu::BindingResource::Sampler(&colormap_sampler),
                },
            ],
        });

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Globe Shader"),
            source: wgpu::ShaderSource::Wgsl(GLOBE_SHADER_WGSL.into()),
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Globe Pipeline Layout"),
            bind_group_layouts: &[&camera_bind_group_layout, &data_bind_group_layout],
            push_constant_ranges: &[],
        });

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Globe Pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                buffers: &[wgpu::VertexBufferLayout {
                    array_stride: std::mem::size_of::<Vertex>() as wgpu::BufferAddress,
                    step_mode: wgpu::VertexStepMode::Vertex,
                    attributes: &[
                        wgpu::VertexAttribute {
                            offset: 0,
                            shader_location: 0,
                            format: wgpu::VertexFormat::Float32x3,
                        },
                        wgpu::VertexAttribute {
                            offset: 12,
                            shader_location: 1,
                            format: wgpu::VertexFormat::Float32x2,
                        },
                    ],
                }],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: target_format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: None,
                ..Default::default()
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
            cache: None,
        });

        let resources = GlobeGpuResources {
            pipeline,
            vertex_buffer,
            index_buffer,
            num_indices: indices.len() as u32,
            camera_buffer,
            camera_bind_group,
            data_texture,
            data_bind_group,
            data_width: 1,
            data_height: 1,
        };

        render_state
            .renderer
            .write()
            .callback_resources
            .insert(resources);
    }

    #[allow(dead_code)]
    pub fn upload_field_data(
        &mut self,
        render_state: &egui_wgpu::RenderState,
        data: &[f32],
        width: usize,
        height: usize,
        colormap: crate::ui::Colormap,
        interpolated: bool,
    ) {
        let mut min = f32::INFINITY;
        let mut max = f32::NEG_INFINITY;
        for &v in data {
            if v.is_finite() {
                min = min.min(v);
                max = max.max(v);
            }
        }
        if min >= max {
            max = min + 1.0;
        }
        self.upload_field_data_with_range(render_state, data, width, height, min, max, colormap, interpolated);
    }

    pub fn upload_field_data_with_range(
        &mut self,
        render_state: &egui_wgpu::RenderState,
        data: &[f32],
        width: usize,
        height: usize,
        min: f32,
        max: f32,
        colormap: crate::ui::Colormap,
        interpolated: bool,
    ) {
        let device = &render_state.device;
        let queue = &render_state.queue;

        self.data_min = min;
        self.data_max = max;
        self.interpolated = interpolated;

        let new_texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Globe Data Texture"),
            size: wgpu::Extent3d {
                width: width as u32,
                height: height as u32,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::R32Float,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });

        queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &new_texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            bytemuck::cast_slice(data),
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some((width * 4) as u32),
                rows_per_image: None,
            },
            wgpu::Extent3d {
                width: width as u32,
                height: height as u32,
                depth_or_array_layers: 1,
            },
        );

        let new_view = new_texture.create_view(&wgpu::TextureViewDescriptor::default());

        let mut renderer = render_state.renderer.write();
        let res = renderer
            .callback_resources
            .get_mut::<GlobeGpuResources>()
            .expect("GlobeGpuResources not initialized");

        let data_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("Globe Data Sampler"),
            address_mode_u: wgpu::AddressMode::Repeat,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Nearest,
            min_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });

        let colormap_data = colormap_lut(colormap);
        let colormap_texture = device.create_texture_with_data(
            queue,
            &wgpu::TextureDescriptor {
                label: Some("Globe Colormap LUT"),
                size: wgpu::Extent3d {
                    width: 256,
                    height: 1,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: wgpu::TextureFormat::Rgba8UnormSrgb,
                usage: wgpu::TextureUsages::TEXTURE_BINDING,
                view_formats: &[],
            },
            wgpu::util::TextureDataOrder::LayerMajor,
            &colormap_data,
        );
        let colormap_view = colormap_texture.create_view(&wgpu::TextureViewDescriptor::default());
        let colormap_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("Globe Colormap Sampler"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });

        let data_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Globe Data BGL"),
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            sample_type: wgpu::TextureSampleType::Float { filterable: false },
                            view_dimension: wgpu::TextureViewDimension::D2,
                            multisampled: false,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::NonFiltering),
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 2,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            sample_type: wgpu::TextureSampleType::Float { filterable: true },
                            view_dimension: wgpu::TextureViewDimension::D2,
                            multisampled: false,
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
            });

        let new_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Globe Data BG"),
            layout: &data_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&new_view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&data_sampler),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::TextureView(&colormap_view),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: wgpu::BindingResource::Sampler(&colormap_sampler),
                },
            ],
        });

        res.data_texture = new_texture;
        res.data_bind_group = new_bind_group;
        res.data_width = width as u32;
        res.data_height = height as u32;
    }

    pub fn paint(&mut self, ui: &mut egui::Ui) {
        let available = ui.available_size();
        // Add padding so the globe doesn't touch the edges
        let pad_x = (available.x * 0.05).max(8.0);
        let pad_y = (available.y * 0.05).max(8.0);
        let padded_size = egui::vec2(available.x - pad_x * 2.0, available.y - pad_y * 2.0);
        let (full_rect, _) = ui.allocate_exact_size(available, egui::Sense::hover());
        let rect = egui::Rect::from_center_size(full_rect.center(), padded_size);

        // Stylish background — radial vignette from deep blue center to dark edges
        paint_viewport_background(ui.painter(), full_rect);

        let response = ui.interact(rect, ui.id().with("globe_interact"), egui::Sense::click_and_drag());

        if !self.initialized {
            ui.painter().text(
                rect.center(),
                egui::Align2::CENTER_CENTER,
                "Globe Viewport\n(wgpu not available)",
                egui::FontId::proportional(18.0),
                egui::Color32::from_rgb(100, 160, 200),
            );
            return;
        }

        if response.dragged() {
            let delta = response.drag_delta();
            let sensitivity = 0.005 / self.zoom;
            self.cam_lon += delta.x * sensitivity;
            self.cam_lat += delta.y * sensitivity;
            self.cam_lat = self.cam_lat.clamp(
                -std::f32::consts::FRAC_PI_2 + 0.01,
                std::f32::consts::FRAC_PI_2 - 0.01,
            );
        }

        if response.hovered() {
            let scroll = ui.input(|i| i.smooth_scroll_delta.y);
            if scroll != 0.0 {
                self.zoom *= 1.0 + scroll * 0.002;
                self.zoom = self.zoom.clamp(0.3, 5.0);
            }
        }

        let (view, view_proj) = build_view_proj(self.cam_lon, self.cam_lat, self.zoom, rect);
        let camera_uniform = CameraUniform {
            view_proj,
            view,
            data_range: [self.data_min, self.data_max],
            params: [if self.interpolated { 1.0 } else { 0.0 }, 0.0],
        };

        let callback = egui_wgpu::Callback::new_paint_callback(
            rect,
            GlobePaintCallback { camera_uniform },
        );
        ui.painter().add(callback);
    }
}

// ---------------------------------------------------------------------------
// Viewport background — radial vignette with subtle star-field feel
// ---------------------------------------------------------------------------

/// Paint a stylish dark background with radial vignette gradient.
/// Center: deep blue-black, edges: darker. Creates a space-like feel.
pub fn paint_viewport_background(painter: &egui::Painter, rect: egui::Rect) {
    // Base fill — very dark blue-black
    painter.rect_filled(rect, 0.0, egui::Color32::from_rgb(12, 14, 22));

    // Radial vignette via concentric oval mesh
    let center = rect.center();
    let rx = rect.width() * 0.5;
    let ry = rect.height() * 0.5;

    let mut mesh = egui::Mesh::default();
    let n_rings = 5;
    let n_segments = 32;

    // Center vertex — slightly lighter
    mesh.colored_vertex(center, egui::Color32::from_rgb(22, 24, 38));

    for ring in 1..=n_rings {
        let t = ring as f32 / n_rings as f32;
        // Exponential falloff for smoother vignette
        let brightness = (1.0 - t * t).max(0.0);
        let r = (12.0 + brightness * 10.0) as u8;
        let g = (14.0 + brightness * 10.0) as u8;
        let b = (22.0 + brightness * 16.0) as u8;
        let color = egui::Color32::from_rgb(r, g, b);

        for seg in 0..n_segments {
            let angle = seg as f32 / n_segments as f32 * std::f32::consts::TAU;
            let x = center.x + angle.cos() * rx * t;
            let y = center.y + angle.sin() * ry * t;
            mesh.colored_vertex(egui::pos2(x, y), color);
        }
    }

    // Triangles: center to first ring
    for seg in 0..n_segments {
        let next = (seg + 1) % n_segments;
        mesh.add_triangle(0, 1 + seg as u32, 1 + next as u32);
    }

    // Triangles: ring to ring
    for ring in 0..(n_rings - 1) {
        let base_inner = 1 + ring as u32 * n_segments as u32;
        let base_outer = base_inner + n_segments as u32;
        for seg in 0..n_segments {
            let next = (seg + 1) % n_segments;
            let i0 = base_inner + seg as u32;
            let i1 = base_inner + next as u32;
            let o0 = base_outer + seg as u32;
            let o1 = base_outer + next as u32;
            mesh.add_triangle(i0, o0, i1);
            mesh.add_triangle(i1, o0, o1);
        }
    }

    // Clip to rect
    painter.with_clip_rect(rect).add(egui::Shape::mesh(mesh));
}

// ---------------------------------------------------------------------------
// UV sphere mesh generation
// ---------------------------------------------------------------------------

fn generate_uv_sphere(lon_segments: u32, lat_segments: u32) -> (Vec<Vertex>, Vec<u32>) {
    let mut vertices = Vec::new();
    let mut indices = Vec::new();

    for lat in 0..=lat_segments {
        let v = lat as f32 / lat_segments as f32;
        let phi = std::f32::consts::PI * v;

        for lon in 0..=lon_segments {
            let u = lon as f32 / lon_segments as f32;
            let theta = 2.0 * std::f32::consts::PI * u;

            let sin_phi = phi.sin();
            let cos_phi = phi.cos();
            let sin_theta = theta.sin();
            let cos_theta = theta.cos();

            let x = sin_phi * cos_theta;
            let y = cos_phi;
            let z = sin_phi * sin_theta;

            vertices.push(Vertex {
                position: [x, y, z],
                uv: [u, v],
            });
        }
    }

    for lat in 0..lat_segments {
        for lon in 0..lon_segments {
            let first = lat * (lon_segments + 1) + lon;
            let second = first + lon_segments + 1;

            indices.push(first);
            indices.push(second);
            indices.push(first + 1);

            indices.push(second);
            indices.push(second + 1);
            indices.push(first + 1);
        }
    }

    (vertices, indices)
}

// ---------------------------------------------------------------------------
// WGSL Shader
// ---------------------------------------------------------------------------

const GLOBE_SHADER_WGSL: &str = r#"
struct CameraUniform {
    view_proj: mat4x4<f32>,
    view: mat4x4<f32>,
    data_range: vec2<f32>,
    params: vec2<f32>, // x: interpolated (0=grid, 1=smooth), y: unused
};

@group(0) @binding(0) var<uniform> camera: CameraUniform;

@group(1) @binding(0) var data_tex: texture_2d<f32>;
@group(1) @binding(1) var data_sampler: sampler;
@group(1) @binding(2) var colormap_tex: texture_2d<f32>;
@group(1) @binding(3) var colormap_sampler: sampler;

struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) uv: vec2<f32>,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) view_normal: vec3<f32>,
};

@vertex
fn vs_main(in: VertexInput) -> VertexOutput {
    var out: VertexOutput;
    out.clip_position = vec4<f32>(in.position, 1.0) * camera.view_proj;
    out.uv = in.uv;
    let rotated = vec4<f32>(in.position, 0.0) * camera.view;
    out.view_normal = rotated.xyz;
    return out;
}

fn sample_bilinear(uv: vec2<f32>) -> f32 {
    let dims = vec2<f32>(textureDimensions(data_tex));
    let texel = uv * dims - 0.5;
    let ix = floor(texel.x);
    let iy = floor(texel.y);
    let fx = texel.x - ix;
    let fy = texel.y - iy;

    let x0 = i32(ix) % i32(dims.x);
    let x1 = (x0 + 1) % i32(dims.x); // wrap longitude
    let y0 = clamp(i32(iy), 0, i32(dims.y) - 1);
    let y1 = clamp(y0 + 1, 0, i32(dims.y) - 1);

    let v00 = textureLoad(data_tex, vec2<i32>(x0, y0), 0).r;
    let v10 = textureLoad(data_tex, vec2<i32>(x1, y0), 0).r;
    let v01 = textureLoad(data_tex, vec2<i32>(x0, y1), 0).r;
    let v11 = textureLoad(data_tex, vec2<i32>(x1, y1), 0).r;

    return mix(mix(v00, v10, fx), mix(v01, v11, fx), fy);
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let nz = in.view_normal.z;

    // Atmospheric glow halo — render even on backface with nz in [-0.08, 0]
    if nz <= -0.08 {
        discard;
    }

    // Atmosphere glow color (teal-ish, matching the app primary)
    let atmo_color = vec3<f32>(0.0, 0.45, 0.42);

    // Backface halo zone: nz in [-0.08, 0] → pure atmospheric glow fading out
    if nz <= 0.0 {
        let halo_t = smoothstep(-0.08, 0.0, nz);
        let glow_alpha = halo_t * 0.25;
        return vec4<f32>(atmo_color * 0.6, glow_alpha);
    }

    var data_val: f32;
    if camera.params.x > 0.5 {
        data_val = sample_bilinear(in.uv);
    } else {
        data_val = textureSample(data_tex, data_sampler, in.uv).r;
    }

    let range = camera.data_range;
    let normalized = clamp((data_val - range.x) / max(range.y - range.x, 0.0001), 0.0, 1.0);

    let cmap_color = textureSample(colormap_tex, colormap_sampler, vec2<f32>(normalized, 0.5));

    let limb = pow(nz, 0.3);

    var color = cmap_color.rgb * limb;

    // Atmospheric rim light — subtle glow near the limb
    let rim = 1.0 - nz;
    let rim_intensity = pow(rim, 3.0) * 0.4;
    color = mix(color, atmo_color, rim_intensity);

    // Graticule lines
    let lon_deg = in.uv.x * 360.0;
    let lat_deg = in.uv.y * 180.0;
    let grid_lon = abs(lon_deg % 30.0);
    let grid_lat = abs(lat_deg % 30.0);
    let line_width = 0.25;
    if grid_lon < line_width || grid_lon > (30.0 - line_width) ||
       grid_lat < line_width || grid_lat > (30.0 - line_width) {
        color = mix(color, vec3<f32>(0.4, 0.4, 0.4), 0.35);
    }

    return vec4<f32>(color, 1.0);
}
"#;

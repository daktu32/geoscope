// renderer/map.rs — Equirectangular Map (2D) renderer

use egui::epaint;
use wgpu::util::DeviceExt;

use super::common::{
    generate_rdbu_r_lut, generate_viridis_lut, identity_mat4, CameraUniform, Vertex,
};

// ---------------------------------------------------------------------------
// GPU resources stored in CallbackResources
// ---------------------------------------------------------------------------

struct MapGpuResources {
    pipeline: wgpu::RenderPipeline,
    vertex_buffer: wgpu::Buffer,
    index_buffer: wgpu::Buffer,
    num_indices: u32,
    camera_buffer: wgpu::Buffer,
    camera_bind_group: wgpu::BindGroup,
    data_texture: wgpu::Texture,
    data_bind_group: wgpu::BindGroup,
    #[allow(dead_code)]
    data_width: u32,
    #[allow(dead_code)]
    data_height: u32,
}

// ---------------------------------------------------------------------------
// Paint callback
// ---------------------------------------------------------------------------

struct MapPaintCallback {
    camera_uniform: CameraUniform,
}

impl egui_wgpu::CallbackTrait for MapPaintCallback {
    fn prepare(
        &self,
        _device: &wgpu::Device,
        queue: &wgpu::Queue,
        _screen_descriptor: &egui_wgpu::ScreenDescriptor,
        _egui_encoder: &mut wgpu::CommandEncoder,
        callback_resources: &mut egui_wgpu::CallbackResources,
    ) -> Vec<wgpu::CommandBuffer> {
        if let Some(res) = callback_resources.get::<MapGpuResources>() {
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
        let Some(res) = callback_resources.get::<MapGpuResources>() else {
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
// MapRenderer — public API
// ---------------------------------------------------------------------------

pub struct MapRenderer {
    pub pan_x: f32,
    pub pan_y: f32,
    pub zoom: f32,
    data_min: f32,
    data_max: f32,
    interpolated: bool,
    initialized: bool,
}

impl MapRenderer {
    pub fn new() -> Self {
        Self {
            pan_x: 0.0,
            pan_y: 0.0,
            zoom: 1.0,
            data_min: 0.0,
            data_max: 1.0,
            interpolated: true,
            initialized: false,
        }
    }

    pub fn ensure_initialized(&mut self, render_state: &egui_wgpu::RenderState) {
        if self.initialized {
            return;
        }
        Self::init_gpu_resources(render_state);
        self.initialized = true;
    }

    fn init_gpu_resources(render_state: &egui_wgpu::RenderState) {
        let device = &render_state.device;
        let target_format = render_state.target_format;

        let (vertices, indices) = generate_flat_grid(128, 64);

        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Map Vertex Buffer"),
            contents: bytemuck::cast_slice(&vertices),
            usage: wgpu::BufferUsages::VERTEX,
        });

        let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Map Index Buffer"),
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
            label: Some("Map Camera Buffer"),
            contents: bytemuck::cast_slice(&[camera_uniform]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let camera_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Map Camera BGL"),
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
            label: Some("Map Camera BG"),
            layout: &camera_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: camera_buffer.as_entire_binding(),
            }],
        });

        let data_texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Map Data Texture"),
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
            label: Some("Map Data Sampler"),
            address_mode_u: wgpu::AddressMode::Repeat,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Nearest,
            min_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });

        let colormap_data = generate_viridis_lut();
        let colormap_texture = device.create_texture_with_data(
            &render_state.queue,
            &wgpu::TextureDescriptor {
                label: Some("Map Colormap LUT"),
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
            label: Some("Map Colormap Sampler"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });

        let data_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Map Data BGL"),
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
            label: Some("Map Data BG"),
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
            label: Some("Map Shader"),
            source: wgpu::ShaderSource::Wgsl(MAP_SHADER_WGSL.into()),
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Map Pipeline Layout"),
            bind_group_layouts: &[&camera_bind_group_layout, &data_bind_group_layout],
            push_constant_ranges: &[],
        });

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Map Pipeline"),
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

        let resources = MapGpuResources {
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

    pub fn upload_field_data(
        &mut self,
        render_state: &egui_wgpu::RenderState,
        data: &[f32],
        width: usize,
        height: usize,
        colormap: crate::ui::Colormap,
        interpolated: bool,
    ) {
        let device = &render_state.device;
        let queue = &render_state.queue;

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
        self.data_min = min;
        self.data_max = max;
        self.interpolated = interpolated;

        let new_texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Map Data Texture"),
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
            .get_mut::<MapGpuResources>()
            .expect("MapGpuResources not initialized");

        let data_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("Map Data Sampler"),
            address_mode_u: wgpu::AddressMode::Repeat,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Nearest,
            min_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });

        let colormap_data = match colormap {
            crate::ui::Colormap::Viridis => generate_viridis_lut(),
            crate::ui::Colormap::RdBuR => generate_rdbu_r_lut(),
        };
        let colormap_texture = device.create_texture_with_data(
            queue,
            &wgpu::TextureDescriptor {
                label: Some("Map Colormap LUT"),
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
            label: Some("Map Colormap Sampler"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });

        let data_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Map Data BGL"),
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
            label: Some("Map Data BG"),
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
        let (rect, response) = ui.allocate_exact_size(
            ui.available_size(),
            egui::Sense::click_and_drag(),
        );

        // Stylish background
        super::globe::paint_viewport_background(ui.painter(), rect);

        if !self.initialized {
            ui.painter().text(
                rect.center(),
                egui::Align2::CENTER_CENTER,
                "Map Viewport\n(wgpu not available)",
                egui::FontId::proportional(18.0),
                egui::Color32::from_rgb(100, 160, 200),
            );
            return;
        }

        if response.dragged() {
            let delta = response.drag_delta();
            let sensitivity = 2.0 / (rect.width().max(1.0) * self.zoom);
            self.pan_x -= delta.x * sensitivity;
            self.pan_y += delta.y * sensitivity;
        }

        if response.hovered() {
            let scroll = ui.input(|i| i.smooth_scroll_delta.y);
            if scroll != 0.0 {
                self.zoom *= 1.0 + scroll * 0.002;
                self.zoom = self.zoom.clamp(0.3, 10.0);
            }
        }

        let view_proj = build_ortho_view_proj(self.pan_x, self.pan_y, self.zoom, rect);
        let camera_uniform = CameraUniform {
            view_proj,
            view: identity_mat4(),
            data_range: [self.data_min, self.data_max],
            params: [if self.interpolated { 1.0 } else { 0.0 }, 0.0],
        };

        let callback = egui_wgpu::Callback::new_paint_callback(
            rect,
            MapPaintCallback { camera_uniform },
        );
        ui.painter().add(callback);
    }
}

// ---------------------------------------------------------------------------
// Orthographic projection for 2D map with pan + zoom
// ---------------------------------------------------------------------------

fn build_ortho_view_proj(
    pan_x: f32,
    pan_y: f32,
    zoom: f32,
    rect: egui::Rect,
) -> [[f32; 4]; 4] {
    let aspect = rect.width() / rect.height().max(1.0);
    let (sx, sy) = if aspect > 1.0 {
        (zoom / aspect, zoom)
    } else {
        (zoom, zoom * aspect)
    };

    // Orthographic: scale + translate (pan)
    [
        [sx, 0.0, 0.0, 0.0],
        [0.0, sy, 0.0, 0.0],
        [0.0, 0.0, 1.0, 0.0],
        [-pan_x * sx, -pan_y * sy, 0.0, 1.0],
    ]
}

// ---------------------------------------------------------------------------
// Flat grid mesh generation (equirectangular quad)
// ---------------------------------------------------------------------------

fn generate_flat_grid(lon_segments: u32, lat_segments: u32) -> (Vec<Vertex>, Vec<u32>) {
    let mut vertices = Vec::new();
    let mut indices = Vec::new();

    for lat in 0..=lat_segments {
        let v = lat as f32 / lat_segments as f32;
        // y maps from -1 (bottom, south) to +1 (top, north)
        let y = 1.0 - 2.0 * v;

        for lon in 0..=lon_segments {
            let u = lon as f32 / lon_segments as f32;
            // x maps from -1 (left) to +1 (right)
            let x = -1.0 + 2.0 * u;

            vertices.push(Vertex {
                position: [x, y, 0.0],
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
// WGSL Shader (equirectangular map — no backface discard, no limb darkening)
// ---------------------------------------------------------------------------

const MAP_SHADER_WGSL: &str = r#"
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
};

@vertex
fn vs_main(in: VertexInput) -> VertexOutput {
    var out: VertexOutput;
    out.clip_position = vec4<f32>(in.position, 1.0) * camera.view_proj;
    out.uv = in.uv;
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
    let x1 = (x0 + 1) % i32(dims.x);
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
    var data_val: f32;
    if camera.params.x > 0.5 {
        data_val = sample_bilinear(in.uv);
    } else {
        data_val = textureSample(data_tex, data_sampler, in.uv).r;
    }

    let range = camera.data_range;
    let normalized = clamp((data_val - range.x) / max(range.y - range.x, 0.0001), 0.0, 1.0);

    let cmap_color = textureSample(colormap_tex, colormap_sampler, vec2<f32>(normalized, 0.5));

    var color = cmap_color.rgb;

    // Graticule lines every 30 degrees
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

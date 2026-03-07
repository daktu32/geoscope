use eframe::CreationContext;
use egui::epaint;
use wgpu::util::DeviceExt;

// ---------------------------------------------------------------------------
// Vertex type for the UV sphere mesh
// ---------------------------------------------------------------------------

#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
struct Vertex {
    position: [f32; 3],
    uv: [f32; 2],
}

// ---------------------------------------------------------------------------
// Camera uniform sent to the GPU
// ---------------------------------------------------------------------------

#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
struct CameraUniform {
    view_proj: [[f32; 4]; 4],
    view: [[f32; 4]; 4],
    data_range: [f32; 2], // [min, max]
    _padding: [f32; 2],
}

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

/// Globe renderer using wgpu.
pub struct GlobeRenderer {
    /// Camera longitude in radians.
    pub cam_lon: f32,
    /// Camera latitude in radians.
    pub cam_lat: f32,
    /// Zoom level (1.0 = default).
    pub zoom: f32,
    /// Data range for normalization.
    data_min: f32,
    data_max: f32,
    /// Whether GPU resources have been initialized.
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
            initialized: cc.wgpu_render_state.is_some(),
        }
    }

    fn init_gpu_resources(render_state: &egui_wgpu::RenderState) {
        let device = &render_state.device;
        let target_format = render_state.target_format;

        // --- Sphere mesh ---
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

        // --- Camera uniform ---
        let camera_uniform = CameraUniform {
            view_proj: identity_mat4(),
            view: identity_mat4(),
            data_range: [0.0, 1.0],
            _padding: [0.0; 2],
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

        // --- Data texture (placeholder 1x1) ---
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

        // --- Colormap LUT texture ---
        let colormap_data = generate_viridis_lut();
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

        // --- Data + colormap bind group ---
        let data_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Globe Data BGL"),
                entries: &[
                    // Data texture
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
                    // Data sampler
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::NonFiltering),
                        count: None,
                    },
                    // Colormap texture
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
                    // Colormap sampler
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

        // --- Shader ---
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Globe Shader"),
            source: wgpu::ShaderSource::Wgsl(GLOBE_SHADER_WGSL.into()),
        });

        // --- Pipeline ---
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
                        // position
                        wgpu::VertexAttribute {
                            offset: 0,
                            shader_location: 0,
                            format: wgpu::VertexFormat::Float32x3,
                        },
                        // uv
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
                cull_mode: None, // We discard back-facing fragments in shader
                ..Default::default()
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
            cache: None,
        });

        // --- Store in callback_resources ---
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

    /// Upload field data to the GPU data texture.
    /// `data` is row-major [lat][lon], `width` = lon count, `height` = lat count.
    pub fn upload_field_data(
        &mut self,
        render_state: &egui_wgpu::RenderState,
        data: &[f32],
        width: usize,
        height: usize,
        colormap: crate::ui::Colormap,
    ) {
        let device = &render_state.device;
        let queue = &render_state.queue;

        // Compute min/max
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

        // Create new texture
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

        // We need to recreate the bind group with the new texture view.
        // Access existing resources to get sampler references.
        let mut renderer = render_state.renderer.write();
        let res = renderer
            .callback_resources
            .get_mut::<GlobeGpuResources>()
            .expect("GlobeGpuResources not initialized");

        // Recreate data sampler (same config)
        let data_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("Globe Data Sampler"),
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

    /// Paint the globe into the given UI rect.
    pub fn paint(&mut self, ui: &mut egui::Ui) {
        let (rect, response) = ui.allocate_exact_size(
            ui.available_size(),
            egui::Sense::click_and_drag(),
        );

        if !self.initialized {
            ui.painter()
                .rect_filled(rect, 0.0, egui::Color32::from_rgb(20, 25, 35));
            ui.painter().text(
                rect.center(),
                egui::Align2::CENTER_CENTER,
                "Globe Viewport\n(wgpu not available)",
                egui::FontId::proportional(18.0),
                egui::Color32::from_rgb(100, 160, 200),
            );
            return;
        }

        // Handle mouse drag for rotation
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

        // Handle scroll for zoom
        if response.hovered() {
            let scroll = ui.input(|i| i.smooth_scroll_delta.y);
            if scroll != 0.0 {
                self.zoom *= 1.0 + scroll * 0.002;
                self.zoom = self.zoom.clamp(0.3, 5.0);
            }
        }

        // Build camera uniform
        let (view, view_proj) = build_view_proj(self.cam_lon, self.cam_lat, self.zoom, rect);
        let camera_uniform = CameraUniform {
            view_proj,
            view,
            data_range: [self.data_min, self.data_max],
            _padding: [0.0; 2],
        };

        let callback = egui_wgpu::Callback::new_paint_callback(
            rect,
            GlobePaintCallback { camera_uniform },
        );
        ui.painter().add(callback);
    }
}

// ---------------------------------------------------------------------------
// UV sphere mesh generation
// ---------------------------------------------------------------------------

fn generate_uv_sphere(lon_segments: u32, lat_segments: u32) -> (Vec<Vertex>, Vec<u32>) {
    let mut vertices = Vec::new();
    let mut indices = Vec::new();

    for lat in 0..=lat_segments {
        let v = lat as f32 / lat_segments as f32;
        let phi = std::f32::consts::PI * v; // 0 (north pole) to PI (south pole)

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
// Camera math
// ---------------------------------------------------------------------------

fn build_view_proj(cam_lon: f32, cam_lat: f32, zoom: f32, rect: egui::Rect) -> ([[f32; 4]; 4], [[f32; 4]; 4]) {
    // View matrix: rotate world so camera looks at (cam_lon, cam_lat)
    let (sin_lon, cos_lon) = cam_lon.sin_cos();
    let (sin_lat, cos_lat) = cam_lat.sin_cos();

    // Rotation: R_y(lon) * R_x(lat) — rotate world so camera looks at (lon, lat) from outside
    let rot_y = [
        [cos_lon, 0.0, sin_lon, 0.0],
        [0.0, 1.0, 0.0, 0.0],
        [-sin_lon, 0.0, cos_lon, 0.0],
        [0.0, 0.0, 0.0, 1.0],
    ];

    let rot_x = [
        [1.0, 0.0, 0.0, 0.0],
        [0.0, cos_lat, -sin_lat, 0.0],
        [0.0, sin_lat, cos_lat, 0.0],
        [0.0, 0.0, 0.0, 1.0],
    ];

    let view = mat4_mul(&rot_x, &rot_y);

    // Orthographic projection with aspect ratio correction
    let aspect = rect.width() / rect.height().max(1.0);
    let scale = zoom;
    let (sx, sy) = if aspect > 1.0 {
        (scale / aspect, scale)
    } else {
        (scale, scale * aspect)
    };

    let proj = [
        [sx,  0.0, 0.0, 0.0],
        [0.0, sy,  0.0, 0.0],
        [0.0, 0.0, 0.5, 0.5],
        [0.0, 0.0, 0.0, 1.0],
    ];

    (view, mat4_mul(&proj, &view))
}

fn mat4_mul(a: &[[f32; 4]; 4], b: &[[f32; 4]; 4]) -> [[f32; 4]; 4] {
    let mut out = [[0.0f32; 4]; 4];
    for i in 0..4 {
        for j in 0..4 {
            for k in 0..4 {
                out[i][j] += a[i][k] * b[k][j];
            }
        }
    }
    out
}

fn identity_mat4() -> [[f32; 4]; 4] {
    [
        [1.0, 0.0, 0.0, 0.0],
        [0.0, 1.0, 0.0, 0.0],
        [0.0, 0.0, 1.0, 0.0],
        [0.0, 0.0, 0.0, 1.0],
    ]
}

// ---------------------------------------------------------------------------
// Colormap LUT generation (256 entries, RGBA8)
// ---------------------------------------------------------------------------

fn generate_viridis_lut() -> Vec<u8> {
    // 5-point viridis control points
    let stops: [(f32, [u8; 3]); 5] = [
        (0.0, [68, 1, 84]),
        (0.25, [59, 82, 139]),
        (0.5, [33, 145, 140]),
        (0.75, [94, 201, 98]),
        (1.0, [253, 231, 37]),
    ];
    interpolate_lut(&stops)
}

fn generate_rdbu_r_lut() -> Vec<u8> {
    let stops: [(f32, [u8; 3]); 5] = [
        (0.0, [5, 48, 97]),
        (0.25, [67, 147, 195]),
        (0.5, [247, 247, 247]),
        (0.75, [214, 96, 77]),
        (1.0, [178, 24, 43]),
    ];
    interpolate_lut(&stops)
}

fn interpolate_lut(stops: &[(f32, [u8; 3]); 5]) -> Vec<u8> {
    let mut data = Vec::with_capacity(256 * 4);
    for i in 0..256 {
        let t = i as f32 / 255.0;
        // Find segment
        let mut seg = 0;
        for s in 0..4 {
            if t >= stops[s].0 && t <= stops[s + 1].0 {
                seg = s;
                break;
            }
        }
        let t0 = stops[seg].0;
        let t1 = stops[seg + 1].0;
        let frac = if (t1 - t0).abs() < 1e-6 {
            0.0
        } else {
            (t - t0) / (t1 - t0)
        };
        let c0 = stops[seg].1;
        let c1 = stops[seg + 1].1;
        let r = (c0[0] as f32 + (c1[0] as f32 - c0[0] as f32) * frac) as u8;
        let g = (c0[1] as f32 + (c1[1] as f32 - c0[1] as f32) * frac) as u8;
        let b = (c0[2] as f32 + (c1[2] as f32 - c0[2] as f32) * frac) as u8;
        data.extend_from_slice(&[r, g, b, 255]);
    }
    data
}

// ---------------------------------------------------------------------------
// WGSL Shader
// ---------------------------------------------------------------------------

const GLOBE_SHADER_WGSL: &str = r#"
struct CameraUniform {
    view_proj: mat4x4<f32>,
    view: mat4x4<f32>,
    data_range: vec2<f32>,
    _padding: vec2<f32>,
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
    // Transform normal by view (rotation only) — no aspect ratio distortion
    let rotated = vec4<f32>(in.position, 0.0) * camera.view;
    out.view_normal = rotated.xyz;
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    // Discard back-facing fragments (facing away from camera)
    let nz = in.view_normal.z;
    if nz <= 0.0 {
        discard;
    }

    // Sample data texture
    let data_val = textureSample(data_tex, data_sampler, in.uv).r;

    // Normalize to [0, 1]
    let range = camera.data_range;
    let normalized = clamp((data_val - range.x) / max(range.y - range.x, 0.0001), 0.0, 1.0);

    // Sample colormap LUT (1D texture stored as 256x1 2D texture)
    let cmap_color = textureSample(colormap_tex, colormap_sampler, vec2<f32>(normalized, 0.5));

    // Limb darkening
    let limb = pow(nz, 0.3);

    var color = cmap_color.rgb * limb;

    // Graticule (every 30° for longitude, every 30° for latitude)
    let lon_deg = in.uv.x * 360.0;  // 0..360
    let lat_deg = in.uv.y * 180.0;  // 0..180 (north to south)
    let grid_lon = abs(lon_deg % 30.0);
    let grid_lat = abs(lat_deg % 30.0);
    let line_width = 0.6; // in degrees
    if grid_lon < line_width || grid_lon > (30.0 - line_width) ||
       grid_lat < line_width || grid_lat > (30.0 - line_width) {
        color = mix(color, vec3<f32>(0.3, 0.3, 0.3), 0.6);
    }

    return vec4<f32>(color, 1.0);
}
"#;

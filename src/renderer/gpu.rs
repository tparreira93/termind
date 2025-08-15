// GPU Renderer implementation using WGPU

use thiserror::Error;
use wgpu::util::DeviceExt;
use std::collections::HashMap;
use fontdue::{Font, FontSettings};

#[derive(Error, Debug)]
pub enum RenderError {
    #[error("GPU initialization failed: {0}")]
    GpuInit(String),
    
    #[error("Render operation failed: {0}")]
    RenderFailed(String),
    
    #[error("Window error: {0}")]
    Window(String),
    
    #[error("Font error: {0}")]
    Font(String),
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct Vertex {
    position: [f32; 3],
    tex_coords: [f32; 2],
    color: [f32; 4],
}

impl Vertex {
    fn desc<'a>() -> wgpu::VertexBufferLayout<'a> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Vertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[
                wgpu::VertexAttribute {
                    offset: 0,
                    shader_location: 0,
                    format: wgpu::VertexFormat::Float32x3,
                },
                wgpu::VertexAttribute {
                    offset: std::mem::size_of::<[f32; 3]>() as wgpu::BufferAddress,
                    shader_location: 1,
                    format: wgpu::VertexFormat::Float32x2,
                },
                wgpu::VertexAttribute {
                    offset: std::mem::size_of::<[f32; 5]>() as wgpu::BufferAddress,
                    shader_location: 2,
                    format: wgpu::VertexFormat::Float32x4,
                },
            ],
        }
    }
}

struct FontAtlas {
    texture: wgpu::Texture,
    texture_view: wgpu::TextureView,
    sampler: wgpu::Sampler,
    bind_group: wgpu::BindGroup,
    char_map: HashMap<char, (f32, f32, f32, f32)>, // (u, v, width, height) in normalized coords
    char_width: f32,
    char_height: f32,
}

pub struct GpuRenderer {
    surface: wgpu::Surface<'static>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    size: winit::dpi::PhysicalSize<u32>,
    
    render_pipeline: wgpu::RenderPipeline,
    font_atlas: FontAtlas,
    vertex_buffer: wgpu::Buffer,
    index_buffer: wgpu::Buffer,
    
    vertices: Vec<Vertex>,
    indices: Vec<u16>,
}

impl GpuRenderer {
    pub async fn new(window: &winit::window::Window) -> Result<Self, RenderError> {
        let size = window.inner_size();
        
        // Create WGPU instance
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::all(),
            ..Default::default()
        });
        
        // Create surface - we need to extend the lifetime here since the window
        // will outlive the renderer for the application's lifetime
        let surface = unsafe {
            // SAFETY: The window will outlive the renderer in our application architecture
            let window_static: &'static winit::window::Window = std::mem::transmute(window);
            instance.create_surface(window_static)
        }.map_err(|e| RenderError::GpuInit(format!("Failed to create surface: {}", e)))?;
        
        // Request adapter
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::default(),
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            })
            .await
            .ok_or_else(|| RenderError::GpuInit("Failed to find an appropriate adapter".to_string()))?;
        
        // Request device and queue
        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    required_features: wgpu::Features::empty(),
                    required_limits: wgpu::Limits::default(),
                    label: None,
                },
                None,
            )
            .await
            .map_err(|e| RenderError::GpuInit(format!("Failed to create device: {}", e)))?;
        
        // Configure surface
        let surface_caps = surface.get_capabilities(&adapter);
        let surface_format = surface_caps
            .formats
            .iter()
            .copied()
            .find(|f| f.is_srgb())
            .unwrap_or(surface_caps.formats[0]);
        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface_format,
            width: size.width,
            height: size.height,
            present_mode: surface_caps.present_modes[0],
            alpha_mode: surface_caps.alpha_modes[0],
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };
        surface.configure(&device, &config);
        
        // Create font atlas
        let font_atlas = Self::create_font_atlas(&device, &queue)?;
        
        // Create shader
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Text Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("../shaders/text.wgsl").into()),
        });
        
        // Create bind group layout
        let texture_bind_group_layout =
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
        
        // Create render pipeline
        let render_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Render Pipeline Layout"),
                bind_group_layouts: &[&texture_bind_group_layout],
                push_constant_ranges: &[],
            });
        
        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Render Pipeline"),
            layout: Some(&render_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: "vs_main",
                buffers: &[Vertex::desc()],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: "fs_main",
                targets: &[Some(wgpu::ColorTargetState {
                    format: config.format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: Some(wgpu::Face::Back),
                polygon_mode: wgpu::PolygonMode::Fill,
                unclipped_depth: false,
                conservative: false,
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState {
                count: 1,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            multiview: None,
        });
        
        // Create initial buffers
        let vertices = Vec::new();
        let indices = Vec::new();
        
        let vertex_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Vertex Buffer"),
            size: (vertices.len() * std::mem::size_of::<Vertex>()).max(64) as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        
        let index_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Index Buffer"),
            size: (indices.len() * std::mem::size_of::<u16>()).max(64) as u64,
            usage: wgpu::BufferUsages::INDEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        
        Ok(Self {
            surface,
            device,
            queue,
            config,
            size,
            render_pipeline,
            font_atlas,
            vertex_buffer,
            index_buffer,
            vertices,
            indices,
        })
    }
    
    fn create_font_atlas(device: &wgpu::Device, queue: &wgpu::Queue) -> Result<FontAtlas, RenderError> {
        // Load system monospace font for terminal rendering
        tracing::info!("🔤 Starting font atlas creation...");
        
        let font_data = Self::load_system_font()?
            .or_else(|| Self::load_fallback_font())
            .ok_or_else(|| RenderError::Font("No suitable font found".to_string()))?;
        
        tracing::info!("📝 Loaded font data: {} bytes", font_data.len());
        
        let font = Font::from_bytes(font_data, FontSettings::default())
            .map_err(|e| RenderError::Font(format!("Failed to load font: {}", e)))?;
            
        tracing::info!("✅ Font parsed successfully");
        
        const ATLAS_SIZE: u32 = 512;
        const FONT_SIZE: f32 = 16.0;
        const CHARS_PER_ROW: u32 = 16; // 16x8 grid for 96 printable ASCII chars
        
        // Create texture data - RGBA format
        let mut texture_data = vec![0u8; (ATLAS_SIZE * ATLAS_SIZE * 4) as usize];
        let mut char_map = HashMap::new();
        
        // Calculate character cell size - make cells more square
        let cell_width = ATLAS_SIZE / CHARS_PER_ROW;  // 32 pixels
        let cell_height = ATLAS_SIZE / 8; // 64 pixels (8 rows instead of 6)
        
        // Generate font atlas with actual glyphs
        tracing::info!("🖼️  Creating font atlas: {}x{} pixels, cell size: {}x{}", ATLAS_SIZE, ATLAS_SIZE, cell_width, cell_height);
        
        let mut chars_processed = 0;
        for c in 32u8..127u8 { // ASCII printable characters
            let char_idx = (c - 32) as u32;
            let row = char_idx / CHARS_PER_ROW;
            let col = char_idx % CHARS_PER_ROW;
            
            let start_x = col * cell_width;
            let start_y = row * cell_height;
            
            // Rasterize the character using fontdue
            let (metrics, bitmap) = font.rasterize(c as char, FONT_SIZE);
            
            if chars_processed < 5 {
                tracing::debug!("  Char '{}' ({}): metrics {}x{}, bitmap {} bytes", c as char, c, metrics.width, metrics.height, bitmap.len());
            }
            chars_processed += 1;
            
            // Copy glyph bitmap to atlas
            for y in 0..metrics.height {
                for x in 0..metrics.width {
                    let src_idx = y * metrics.width + x;
                    if src_idx < bitmap.len() {
                        let atlas_x = start_x + x as u32 + (cell_width - metrics.width as u32) / 2;
                        let atlas_y = start_y + y as u32 + (cell_height - metrics.height as u32) / 2;
                        
                        if atlas_x < ATLAS_SIZE && atlas_y < ATLAS_SIZE {
                            let dst_idx = ((atlas_y * ATLAS_SIZE + atlas_x) * 4) as usize;
                            
                            if dst_idx + 3 < texture_data.len() {
                                let alpha = bitmap[src_idx];
                                texture_data[dst_idx] = 255;     // R
                                texture_data[dst_idx + 1] = 255; // G
                                texture_data[dst_idx + 2] = 255; // B
                                texture_data[dst_idx + 3] = alpha; // A
                            }
                        }
                    }
                }
            }
            
            // Store character UV coordinates
            let u = start_x as f32 / ATLAS_SIZE as f32;
            let v = start_y as f32 / ATLAS_SIZE as f32;
            let w = cell_width as f32 / ATLAS_SIZE as f32;
            let h = cell_height as f32 / ATLAS_SIZE as f32;
            
            char_map.insert(c as char, (u, v, w, h));
        }
        
        tracing::info!("✅ Font atlas created with {} characters", chars_processed);
        tracing::debug!("📊 Character map contains {} entries", char_map.len());
        
        // Create texture
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            size: wgpu::Extent3d {
                width: ATLAS_SIZE,
                height: ATLAS_SIZE,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            label: Some("Font Atlas Texture"),
            view_formats: &[],
        });
        
        queue.write_texture(
            wgpu::ImageCopyTexture {
                aspect: wgpu::TextureAspect::All,
                texture: &texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
            },
            &texture_data,
            wgpu::ImageDataLayout {
                offset: 0,
                bytes_per_row: Some(4 * ATLAS_SIZE),
                rows_per_image: Some(ATLAS_SIZE),
            },
            wgpu::Extent3d {
                width: ATLAS_SIZE,
                height: ATLAS_SIZE,
                depth_or_array_layers: 1,
            },
        );
        
        let texture_view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Nearest,
            min_filter: wgpu::FilterMode::Nearest,
            mipmap_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });
        
        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
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
        
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&texture_view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&sampler),
                },
            ],
            label: Some("diffuse_bind_group"),
        });
        
        Ok(FontAtlas {
            texture,
            texture_view,
            sampler,
            bind_group,
            char_map,
            char_width: cell_width as f32,
            char_height: cell_height as f32,
        })
    }
    
    fn load_system_font() -> Result<Option<Vec<u8>>, RenderError> {
        // Try to load system monospace fonts on macOS
        let font_paths = [
            "/System/Library/Fonts/Monaco.ttf",
            "/System/Library/Fonts/Menlo.ttc",
            "/Library/Fonts/SF Mono Regular.otf",
            "/System/Library/Fonts/Courier New.ttf",
        ];
        
        tracing::debug!("🔍 Searching for system fonts...");
        
        for path in &font_paths {
            tracing::debug!("  Trying: {}", path);
            if let Ok(data) = std::fs::read(path) {
                tracing::info!("✅ Found font: {} ({} bytes)", path, data.len());
                return Ok(Some(data));
            } else {
                tracing::debug!("  ❌ Not found: {}", path);
            }
        }
        
        tracing::warn!("⚠️  No system fonts found");
        Ok(None)
    }
    
    fn load_fallback_font() -> Option<Vec<u8>> {
        // Embedded fallback font - a simple bitmap-style font
        // This is a basic fallback when no system fonts are available
        None // For now, we'll rely on system fonts
    }
    
    pub fn render_text(&mut self, _text: &str, lines: &[String]) -> Result<(), RenderError> {
        self.vertices.clear();
        self.indices.clear();
        
        tracing::debug!("🔤 render_text called with {} lines", lines.len());
        
        let screen_width = self.size.width as f32;
        let screen_height = self.size.height as f32;
        
        let char_width_screen = self.font_atlas.char_width / screen_width * 2.0;
        let char_height_screen = self.font_atlas.char_height / screen_height * 2.0;
        
        let mut vertex_count = 0u16;
        
        for (line_idx, line) in lines.iter().enumerate() {
            // Start from top of screen and move down
            let y = 1.0 - (line_idx as f32 + 1.0) * char_height_screen;
            
            for (char_idx, ch) in line.chars().enumerate() {
                if let Some(&(u, v, w, h)) = self.font_atlas.char_map.get(&ch) {
                    let x = -1.0 + char_idx as f32 * char_width_screen;
                    
                    if line_idx == 0 && char_idx < 5 {
                        tracing::debug!("  Rendering char '{}' at ({:.3}, {:.3}) with UV ({:.3}, {:.3}, {:.3}, {:.3})", ch, x, y, u, v, w, h);
                    }
                    
                    // Create quad for character
                    let vertices = [
                        Vertex {
                            position: [x, y, 0.0],
                            tex_coords: [u, v],
                            color: [1.0, 1.0, 1.0, 1.0],
                        },
                        Vertex {
                            position: [x + char_width_screen, y, 0.0],
                            tex_coords: [u + w, v],
                            color: [1.0, 1.0, 1.0, 1.0],
                        },
                        Vertex {
                            position: [x + char_width_screen, y - char_height_screen, 0.0],
                            tex_coords: [u + w, v + h],
                            color: [1.0, 1.0, 1.0, 1.0],
                        },
                        Vertex {
                            position: [x, y - char_height_screen, 0.0],
                            tex_coords: [u, v + h],
                            color: [1.0, 1.0, 1.0, 1.0],
                        },
                    ];
                    
                    let indices = [
                        vertex_count, vertex_count + 1, vertex_count + 2,
                        vertex_count, vertex_count + 2, vertex_count + 3,
                    ];
                    
                    self.vertices.extend_from_slice(&vertices);
                    self.indices.extend_from_slice(&indices);
                    vertex_count += 4;
                } else {
                    if line_idx == 0 && char_idx < 5 {
                        tracing::debug!("  ❌ No mapping found for char '{}' (code: {})", ch, ch as u32);
                    }
                }
            }
        }
        
        tracing::debug!("📊 Generated {} vertices, {} indices", self.vertices.len(), self.indices.len());
        
        Ok(())
    }
    
    pub fn render_frame(&mut self, grid: &crate::TextGrid) -> Result<(), RenderError> {
        // Convert grid to lines
        let mut lines = Vec::new();
        let mut non_empty_lines = 0;
        let mut total_chars = 0;
        
        for row in 0..grid.rows {
            if let Some(row_data) = grid.row(row) {
                let mut line = String::new();
                for col in 0..row_data.len().min(grid.cols as usize) {
                    if let Some(cell) = grid.cell_at(row, col as u16) {
                        if cell.ch != '\0' && cell.ch != ' ' {
                            line.push(cell.ch);
                            total_chars += 1;
                        } else {
                            line.push(' ');
                        }
                    } else {
                        line.push(' ');
                    }
                }
                if !line.trim().is_empty() {
                    non_empty_lines += 1;
                }
                lines.push(line);
            }
        }
        
        // Debug: log what we're trying to render
        if total_chars > 0 {
            tracing::debug!("🎨 Rendering {} non-empty lines with {} total chars", non_empty_lines, total_chars);
            if non_empty_lines <= 3 {
                for (i, line) in lines.iter().enumerate().take(3) {
                    if !line.trim().is_empty() {
                        tracing::debug!("   Line {}: '{}'", i, line.trim());
                    }
                }
            }
        } else {
            // Always render some debug text to test the renderer
            lines[0] = "Termind Terminal Ready".to_string();
            if lines.len() > 1 {
                lines[1] = "Type commands here...".to_string();
            }
            total_chars = lines[0].len() + lines.get(1).map(|l| l.len()).unwrap_or(0);
            if total_chars > 0 {
                tracing::debug!("🎨 Rendering debug text with {} chars", total_chars);
            }
        }
        
        // Prepare text for rendering
        let full_text = lines.join("\n");
        tracing::debug!("🎯 Preparing to render {} lines, {} total chars", lines.len(), total_chars);
        self.render_text(&full_text, &lines)?;
        
        tracing::debug!("🔧 Buffer update: {} vertices, {} indices", self.vertices.len(), self.indices.len());
        
        // Update buffers if needed
        if !self.vertices.is_empty() {
            // Recreate vertex buffer if needed
            let vertex_size = self.vertices.len() * std::mem::size_of::<Vertex>();
            if vertex_size > self.vertex_buffer.size() as usize {
                self.vertex_buffer = self.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("Vertex Buffer"),
                    contents: bytemuck::cast_slice(&self.vertices),
                    usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                });
            } else {
                self.queue.write_buffer(&self.vertex_buffer, 0, bytemuck::cast_slice(&self.vertices));
            }
            
            // Recreate index buffer if needed
            let index_size = self.indices.len() * std::mem::size_of::<u16>();
            if index_size > self.index_buffer.size() as usize {
                self.index_buffer = self.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("Index Buffer"),
                    contents: bytemuck::cast_slice(&self.indices),
                    usage: wgpu::BufferUsages::INDEX | wgpu::BufferUsages::COPY_DST,
                });
            } else {
                self.queue.write_buffer(&self.index_buffer, 0, bytemuck::cast_slice(&self.indices));
            }
        }
        
        // Render
        let output = self.surface.get_current_texture()
            .map_err(|e| RenderError::RenderFailed(format!("Failed to get surface texture: {}", e)))?;
        let view = output.texture.create_view(&wgpu::TextureViewDescriptor::default());
        
        let mut encoder = self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
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
                            r: 0.0,
                            g: 0.0,
                            b: 0.0,
                            a: 1.0,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                occlusion_query_set: None,
                timestamp_writes: None,
            });
            
            if !self.vertices.is_empty() {
                tracing::debug!("🎮 Drawing {} indexed vertices ({} indices)", self.vertices.len(), self.indices.len());
                render_pass.set_pipeline(&self.render_pipeline);
                render_pass.set_bind_group(0, &self.font_atlas.bind_group, &[]);
                render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
                render_pass.set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint16);
                render_pass.draw_indexed(0..self.indices.len() as u32, 0, 0..1);
            } else {
                tracing::debug!("⚠️ No vertices to draw - rendering black screen");
            }
        }
        
        self.queue.submit(std::iter::once(encoder.finish()));
        output.present();
        
        Ok(())
    }
    
    pub fn resize(&mut self, new_size: winit::dpi::PhysicalSize<u32>) -> Result<(), RenderError> {
        if new_size.width > 0 && new_size.height > 0 {
            self.size = new_size;
            self.config.width = new_size.width;
            self.config.height = new_size.height;
            self.surface.configure(&self.device, &self.config);
        }
        Ok(())
    }
    
    pub fn char_width(&self) -> u32 {
        self.font_atlas.char_width as u32
    }
    
    pub fn char_height(&self) -> u32 {
        self.font_atlas.char_height as u32
    }
}

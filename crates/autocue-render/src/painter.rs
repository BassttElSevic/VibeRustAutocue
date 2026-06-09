//! wgpu 绘制器 — 将 cosmic-text Buffer 渲染到屏幕

use crate::layout::LayoutEngine;
use crate::Result;

pub struct Painter {
    pipeline: wgpu::RenderPipeline,
    swash_cache: cosmic_text::SwashCache,
    vertex_buffer: wgpu::Buffer,
    index_buffer: wgpu::Buffer,
}

impl Painter {
    pub fn new(device: &wgpu::Device, surface_format: wgpu::TextureFormat) -> Self {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("autocue text shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shader.wgsl").into()),
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("autocue pipeline layout"),
            bind_group_layouts: &[],
            push_constant_ranges: &[],
        });

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("autocue text pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                buffers: &[wgpu::VertexBufferLayout {
                    array_stride: 20,
                    step_mode: wgpu::VertexStepMode::Vertex,
                    attributes: &[
                        wgpu::VertexAttribute {
                            offset: 0,
                            format: wgpu::VertexFormat::Float32x2,
                            shader_location: 0,
                        },
                        wgpu::VertexAttribute {
                            offset: 8,
                            format: wgpu::VertexFormat::Float32x2,
                            shader_location: 1,
                        },
                        wgpu::VertexAttribute {
                            offset: 16,
                            format: wgpu::VertexFormat::Unorm8x4,
                            shader_location: 2,
                        },
                    ],
                }],
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format: surface_format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                ..Default::default()
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
            cache: None,
        });

        let vertex_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("autocue vertex buffer"),
            size: 131072,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let index_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("autocue index buffer"),
            size: 65536,
            usage: wgpu::BufferUsages::INDEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        Self {
            pipeline,
            swash_cache: cosmic_text::SwashCache::new(),
            vertex_buffer,
            index_buffer,
        }
    }

    pub fn draw(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        view: &wgpu::TextureView,
        layout: &mut LayoutEngine,
        width: u32,
        height: u32,
    ) -> Result<()> {
        let mut vertex_data: Vec<u8> = Vec::new();
        let mut index_data: Vec<u32> = Vec::new();
        let mut vertex_count: u32 = 0;

        let fs = layout.font_size();
        layout.buffer.set_size(
            &mut layout.font_system,
            Some(fs),
            Some(fs * 1.5),
        );

        // cosmic-text 0.13: draw callback 接收 Color (tuple struct wrapping u32)
        layout.buffer.draw(
            &mut layout.font_system,
            &mut self.swash_cache,
            cosmic_text::Color::rgb(255, 255, 255),
            |x, y, w, h, color| {
                let rgba = color.0.to_le_bytes();

                let x0 = x as f32 / width as f32 * 2.0 - 1.0;
                let y0 = 1.0 - y as f32 / height as f32 * 2.0;
                let x1 = (x + w as i32) as f32 / width as f32 * 2.0 - 1.0;
                let y1 = 1.0 - (y + h as i32) as f32 / height as f32 * 2.0;

                let col_u32 = u32::from_le_bytes(rgba);
                let vtx = [
                    (x0, y0, 0.0f32, 0.0f32, col_u32),
                    (x1, y0, 1.0, 0.0, col_u32),
                    (x1, y1, 1.0, 1.0, col_u32),
                    (x0, y1, 0.0, 1.0, col_u32),
                ];

                for &(px, py, u, v, col) in &vtx {
                    vertex_data.extend_from_slice(&px.to_le_bytes());
                    vertex_data.extend_from_slice(&py.to_le_bytes());
                    vertex_data.extend_from_slice(&u.to_le_bytes());
                    vertex_data.extend_from_slice(&v.to_le_bytes());
                    vertex_data.extend_from_slice(&col.to_le_bytes());
                }

                let base = vertex_count;
                index_data.extend_from_slice(&[base, base + 1, base + 2, base, base + 2, base + 3]);
                vertex_count += 4;
            },
        );

        if vertex_data.is_empty() {
            return Ok(());
        }

        if vertex_data.len() as u64 <= self.vertex_buffer.size() {
            queue.write_buffer(&self.vertex_buffer, 0, &vertex_data);
        }
        if (index_data.len() * 4) as u64 <= self.index_buffer.size() {
            let index_bytes: Vec<u8> = index_data.iter().flat_map(|i| i.to_le_bytes()).collect();
            queue.write_buffer(&self.index_buffer, 0, &index_bytes);
        }

        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("autocue render encoder"),
        });

        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("autocue render pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view,
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

            pass.set_pipeline(&self.pipeline);
            pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
            pass.set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
            pass.draw_indexed(0..index_data.len() as u32, 0, 0..1);
        }

        queue.submit(std::iter::once(encoder.finish()));

        Ok(())
    }
}

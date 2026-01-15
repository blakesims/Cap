use wgpu::{self, util::DeviceExt};

use crate::{ConvertError, GpuConverterError, util::read_buffer_to_vec};

pub struct RGBAToNV12 {
    device: wgpu::Device,
    queue: wgpu::Queue,
    pipeline: wgpu::ComputePipeline,
    bind_group_layout: wgpu::BindGroupLayout,
}

impl RGBAToNV12 {
    pub async fn new() -> Result<Self, GpuConverterError> {
        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor::default());

        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                force_fallback_adapter: false,
                compatible_surface: None,
            })
            .await?;

        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor::default())
            .await?;

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("RGBA to NV12 Converter"),
            source: wgpu::ShaderSource::Wgsl(std::borrow::Cow::Borrowed(include_str!(
                "./shader.wgsl"
            ))),
        });

        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("RGBA to NV12 Bind Group Layout"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: false },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: false },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: false },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 3,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("RGBA to NV12 Pipeline Layout"),
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });

        let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("RGBA to NV12 Pipeline"),
            layout: Some(&pipeline_layout),
            module: &shader,
            entry_point: Some("main"),
            compilation_options: Default::default(),
            cache: None,
        });

        Ok(Self {
            device,
            queue,
            pipeline,
            bind_group_layout,
        })
    }

    pub fn convert(
        &self,
        rgba_data: &[u8],
        width: u32,
        height: u32,
    ) -> Result<(Vec<u8>, Vec<u8>), ConvertError> {
        if !width.is_multiple_of(2) {
            return Err(ConvertError::OddWidth { width });
        }

        if !height.is_multiple_of(2) {
            return Err(ConvertError::OddHeight { height });
        }

        let expected_size = (width as usize) * (height as usize) * 4;
        if rgba_data.len() != expected_size {
            return Err(ConvertError::BufferSizeMismatch {
                expected: expected_size,
                actual: rgba_data.len(),
            });
        }

        let rgba_texture = self.device.create_texture_with_data(
            &self.queue,
            &wgpu::TextureDescriptor {
                label: Some("RGBA Input Texture"),
                size: wgpu::Extent3d {
                    width,
                    height,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: wgpu::TextureFormat::Rgba8Unorm,
                usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
                view_formats: &[],
            },
            wgpu::util::TextureDataOrder::MipMajor,
            rgba_data,
        );

        let width_u64 = u64::from(width);
        let height_u64 = u64::from(height);
        let y_plane_size = width_u64 * height_u64;
        let uv_plane_size = (width_u64 * height_u64) / 2;

        let y_buffer_size = (y_plane_size + 3) / 4 * 4;
        let uv_buffer_size = (uv_plane_size + 3) / 4 * 4;

        let y_write_buffer = self
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("RGBA to NV12 Y Plane Buffer"),
                contents: &vec![0u8; y_buffer_size as usize],
                usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
            });

        let uv_write_buffer = self
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("RGBA to NV12 UV Plane Buffer"),
                contents: &vec![0u8; uv_buffer_size as usize],
                usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
            });

        let dimensions_buffer = self
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("RGBA to NV12 Dimensions Buffer"),
                contents: [width.to_ne_bytes(), height.to_ne_bytes()].as_flattened(),
                usage: wgpu::BufferUsages::UNIFORM,
            });

        let bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("RGBA to NV12 Bind Group"),
            layout: &self.bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(
                        &rgba_texture.create_view(&Default::default()),
                    ),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Buffer(
                        y_write_buffer.as_entire_buffer_binding(),
                    ),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::Buffer(
                        uv_write_buffer.as_entire_buffer_binding(),
                    ),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: wgpu::BindingResource::Buffer(
                        dimensions_buffer.as_entire_buffer_binding(),
                    ),
                },
            ],
        });

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("RGBA to NV12 Encoder"),
            });

        {
            let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("RGBA to NV12 Pass"),
                ..Default::default()
            });
            compute_pass.set_pipeline(&self.pipeline);
            compute_pass.set_bind_group(0, &bind_group, &[]);
            compute_pass.dispatch_workgroups(width.div_ceil(8), height.div_ceil(8), 1);
        }

        let y_read_buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("RGBA to NV12 Y Read Buffer"),
            size: y_buffer_size,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });

        let uv_read_buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("RGBA to NV12 UV Read Buffer"),
            size: uv_buffer_size,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });

        encoder.copy_buffer_to_buffer(&y_write_buffer, 0, &y_read_buffer, 0, y_buffer_size);
        encoder.copy_buffer_to_buffer(&uv_write_buffer, 0, &uv_read_buffer, 0, uv_buffer_size);

        let _submission = self.queue.submit(std::iter::once(encoder.finish()));

        let mut y_data =
            read_buffer_to_vec(&y_read_buffer, &self.device).map_err(ConvertError::Poll)?;
        let mut uv_data =
            read_buffer_to_vec(&uv_read_buffer, &self.device).map_err(ConvertError::Poll)?;

        y_data.truncate(y_plane_size as usize);
        uv_data.truncate(uv_plane_size as usize);

        Ok((y_data, uv_data))
    }
}

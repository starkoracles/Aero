use std::borrow::Cow;
use std::mem;
use wasm_bindgen::prelude::*;
use wgpu::util::DeviceExt;
use wgpu::Instance;

#[wasm_bindgen]
pub struct BaseElementGpu(pub u64);

impl BaseElementGpu {
    pub const M: u64 = 0xFFFFFFFF00000001;
}

#[wasm_bindgen]
pub struct GpuWrapper {
    compute_pipeline: wgpu::ComputePipeline,
    input_buffer: wgpu::Buffer,
    device: wgpu::Device,
}

impl GpuWrapper {
    pub async fn init() -> Self {
        let device = Self::create_device().await;
        let compute_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: None,
            source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(include_str!("field_ops.wgsl"))),
        });
        let mul_input_data = vec![(0u64, 0u64); 1024];
        let mul_input_vec_tuple_1024 =
            device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Mul input Buffer"),
                contents: bytemuck::cast_slice(&mul_input_data),
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            });
        let compute_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: wgpu::BufferSize::new(
                                (mul_input_vec_tuple_1024.len() * mem::size_of::<u64>() * 2) as _,
                            ),
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Storage { read_only: false },
                            has_dynamic_offset: false,
                            min_binding_size: wgpu::BufferSize::new(
                                (mul_input_vec_tuple_1024.len() * mem::size_of::<u64>()) as _,
                            ),
                        },
                        count: None,
                    },
                ],
                label: None,
            });
        let compute_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("compute"),
                bind_group_layouts: &[&compute_bind_group_layout],
                push_constant_ranges: &[],
            });
        let compute_pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("Compute pipeline"),
            layout: Some(&compute_pipeline_layout),
            module: &compute_shader,
            entry_point: "main",
        });
        GpuWrapper {
            compute_pipeline,
            input_buffer: mul_input_vec_tuple_1024,
            device,
        }
    }

    pub fn mul_g(&mut self, a: BaseElementGpu, b: BaseElementGpu) -> BaseElementGpu {
        self.input_buffer[0] = [a.0, b.0];
        let mut command_encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });
        let mut cpass =
            command_encoder.begin_compute_pass(&wgpu::ComputePassDescriptor { label: None });
        cpass.set_pipeline(&self.compute_pipeline);
        cpass.dispatch_workggroups();
        cpass.end_pass();
        BaseElementGpu(0)
    }

    async fn create_device() -> wgpu::Device {
        let instance = wgpu::Instance::default();
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                ..Default::default()
            })
            .await
            .expect("Failed to find an appropriate adapter");
        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    label: None,
                    features: wgpu::Features::empty(),
                    limits: wgpu::Limits::downlevel_defaults(),
                },
                None,
            )
            .await
            .expect("Failed to create device");
        device
    }
}

//! Opt-in GPU appearance executor beside Vello, not a vector renderer.
//!
//! The caller supplies its existing wgpu device/queue. Outputs remain GPU textures; only tests
//! read them back. Values are premultiplied in the caller's working space, with no implicit
//! ICC/gamma conversion. There is deliberately no desktop routing or export change yet.
//! See docs/GPU-EFFECTS-COMPANION-SCOPE-2026-10-11.md for the integration gates.

use std::sync::Arc;
use wgpu::util::DeviceExt;

/// Per-image cap; the eventual canvas router must also account for aggregate resident memory.
pub const MAX_PIXELS: u64 = 4_194_304;
/// Bounded pilot kernel: up to 61 taps per axis at sigma 10.
pub const MAX_SIGMA: f32 = 10.0;

/// Texture created by this executor. Private ownership prevents cross-device input mistakes.
#[derive(Clone)]
pub struct EffectImage {
    texture: wgpu::Texture,
    owner: Arc<()>,
}

impl EffectImage {
    /// GPU-resident output, suitable for subsequent GPU passes or a presentation adapter.
    pub fn texture(&self) -> &wgpu::Texture {
        &self.texture
    }

    pub fn dimensions(&self) -> [u32; 2] {
        [self.texture.width(), self.texture.height()]
    }
}

/// Pipelines are built once; a new executor does not request a second adapter/device.
pub struct GpuEffects {
    device: wgpu::Device,
    queue: wgpu::Queue,
    blur: wgpu::ComputePipeline,
    owner: Arc<()>,
}

/// Capture GPU validation/allocation errors. No process-wide error handler is replaced.
async fn checked<T>(device: &wgpu::Device, work: impl FnOnce() -> T) -> Result<T, String> {
    let validation = device.push_error_scope(wgpu::ErrorFilter::Validation);
    let allocation = device.push_error_scope(wgpu::ErrorFilter::OutOfMemory);
    let internal = device.push_error_scope(wgpu::ErrorFilter::Internal);
    let output = work();
    // Pop every scope before awaiting, including on an error; do not leak scopes into the host.
    let internal = internal.pop();
    let allocation = allocation.pop();
    let validation = validation.pop();
    let errors = [internal.await, allocation.await, validation.await];
    if let Some(error) = errors.into_iter().flatten().next() { Err(format!("GPU effects: {error}")) } else { Ok(output) }
}

fn dimensions(width: u32, height: u32, limit: u32) -> Result<(), String> {
    if width == 0 || height == 0 || width > limit || height > limit {
        return Err("GPU effects: image dimensions exceed device limits or are zero".into());
    }
    if u64::from(width) * u64::from(height) > MAX_PIXELS {
        return Err("GPU effects: image exceeds the pilot pixel budget; use padded tiles".into());
    }
    Ok(())
}

fn kernel(sigma: f32) -> Result<Vec<f32>, String> {
    if !sigma.is_finite() || !(0.0..=MAX_SIGMA).contains(&sigma) {
        return Err(format!("GPU effects: sigma must be finite and between 0 and {MAX_SIGMA}"));
    }
    if sigma == 0.0 {
        return Ok(vec![1.0]);
    }
    let radius = (sigma * 3.0).ceil() as i32;
    // For subnormal sigma every noncentral tap is effectively zero; avoid 0/0 at the center.
    let mut weights: Vec<f32> = (-radius..=radius).map(|x| if x == 0 { 1.0 } else { (-0.5 * (x as f32 / sigma).powi(2)).exp() }).collect();
    let total: f32 = weights.iter().sum();
    for weight in &mut weights {
        *weight /= total;
    }
    Ok(weights)
}

impl GpuEffects {
    /// The device and queue must be a pair from the same host graphics context.
    pub async fn new(device: wgpu::Device, queue: wgpu::Queue) -> Result<Self, String> {
        let limits = device.limits();
        if limits.max_compute_workgroups_per_dimension == 0
            || limits.max_compute_invocations_per_workgroup < 64
            || limits.max_compute_workgroup_size_x < 8
            || limits.max_compute_workgroup_size_y < 8
            || limits.max_storage_textures_per_shader_stage < 1
            || limits.max_storage_buffers_per_shader_stage < 1
        {
            return Err("GPU effects: this device lacks required compute limits; retain CPU rendering".into());
        }
        let blur = checked(&device, || {
            let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("VectorCraft effects Gaussian blur"),
                source: wgpu::ShaderSource::Wgsl(include_str!("gpu_fx/blur.wgsl").into()),
            });
            device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: Some("VectorCraft effects Gaussian blur"),
                layout: None,
                module: &module,
                entry_point: Some("blur"),
                compilation_options: Default::default(),
                cache: None,
            })
        })
        .await?;
        Ok(Self { device, queue, blur, owner: Arc::new(()) })
    }

    fn image(&self, width: u32, height: u32, format: wgpu::TextureFormat) -> EffectImage {
        let texture = self.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("VectorCraft resident effect tile"),
            size: wgpu::Extent3d { width, height, depth_or_array_layers: 1 },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format,
            usage: wgpu::TextureUsages::TEXTURE_BINDING
                | wgpu::TextureUsages::COPY_DST
                | wgpu::TextureUsages::COPY_SRC
                | if format == wgpu::TextureFormat::Rgba16Float { wgpu::TextureUsages::STORAGE_BINDING } else { wgpu::TextureUsages::empty() },
            view_formats: &[],
        });
        EffectImage { texture, owner: self.owner.clone() }
    }

    /// Upload a premultiplied RGBA8 effect source. This is not an sRGB texture or an ICC converter.
    pub async fn upload_rgba8(&self, width: u32, height: u32, rgba: &[u8]) -> Result<EffectImage, String> {
        dimensions(width, height, self.device.limits().max_texture_dimension_2d)?;
        if rgba.len() as u64 != u64::from(width) * u64::from(height) * 4 {
            return Err("GPU effects: RGBA byte length does not match dimensions".into());
        }
        if rgba.chunks_exact(4).any(|p| p.iter().take(3).any(|c| *c > p.get(3).copied().unwrap_or(0))) {
            return Err("GPU effects: RGBA input is not premultiplied".into());
        }
        checked(&self.device, || {
            let image = self.image(width, height, wgpu::TextureFormat::Rgba8Unorm);
            self.queue.write_texture(
                image.texture.as_image_copy(),
                rgba,
                wgpu::TexelCopyBufferLayout { offset: 0, bytes_per_row: Some(width * 4), rows_per_image: Some(height) },
                image.texture.size(),
            );
            image
        })
        .await
    }

    /// Separable blur; sigma is in source pixels, not document units. No readback or blocking poll.
    /// The caller must pad content by ceil(3*sigma) on every side before cropping its output.
    pub async fn gaussian_blur(&self, source: &EffectImage, sigma: f32) -> Result<EffectImage, String> {
        if !Arc::ptr_eq(&self.owner, &source.owner) {
            return Err("GPU effects: source belongs to another executor".into());
        }
        let weights = kernel(sigma)?;
        if sigma == 0.0 {
            return Ok(source.clone());
        }
        let [width, height] = source.dimensions();
        checked(&self.device, || {
            let intermediate = self.image(width, height, wgpu::TextureFormat::Rgba16Float);
            let output = self.image(width, height, wgpu::TextureFormat::Rgba16Float);
            let weight_bytes: Vec<u8> = weights.iter().flat_map(|w| w.to_le_bytes()).collect();
            let weight_buffer = self.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("VectorCraft Gaussian weights"),
                contents: &weight_bytes,
                usage: wgpu::BufferUsages::STORAGE,
            });
            let layout = self.blur.get_bind_group_layout(0);
            let mut encoder =
                self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: Some("VectorCraft effects two-pass blur") });
            for (axis, input, target) in [(0u32, source, &intermediate), (1, &intermediate, &output)] {
                let params: Vec<u8> = [width, height, ((weights.len() - 1) / 2) as u32, axis].iter().flat_map(|v| v.to_le_bytes()).collect();
                let params = self.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("VectorCraft blur axis"),
                    contents: &params,
                    usage: wgpu::BufferUsages::UNIFORM,
                });
                let input = input.texture.create_view(&Default::default());
                let target = target.texture.create_view(&Default::default());
                let bindings = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
                    label: Some("VectorCraft blur bindings"),
                    layout: &layout,
                    entries: &[
                        wgpu::BindGroupEntry { binding: 0, resource: wgpu::BindingResource::TextureView(&input) },
                        wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::TextureView(&target) },
                        wgpu::BindGroupEntry { binding: 2, resource: params.as_entire_binding() },
                        wgpu::BindGroupEntry { binding: 3, resource: weight_buffer.as_entire_binding() },
                    ],
                });
                let mut pass =
                    encoder.begin_compute_pass(&wgpu::ComputePassDescriptor { label: Some("VectorCraft blur pass"), timestamp_writes: None });
                pass.set_pipeline(&self.blur);
                pass.set_bind_group(0, &bindings, &[]);
                pass.dispatch_workgroups(width.div_ceil(8), height.div_ceil(8), 1);
            }
            self.queue.submit([encoder.finish()]);
            output
        })
        .await
    }
}

#[cfg(test)]
mod tests;

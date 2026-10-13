use super::*;

#[test]
fn bounded_normalized_symmetric_kernel() {
    for sigma in [0.0, f32::MIN_POSITIVE, 0.25, 1.0, 3.0, MAX_SIGMA] {
        let weights = kernel(sigma).unwrap();
        assert!(weights.len() <= 61);
        assert!((weights.iter().sum::<f32>() - 1.0).abs() < 0.00001);
        assert!(weights.iter().all(|w| w.is_finite() && *w >= 0.0));
        assert!(weights.iter().zip(weights.iter().rev()).all(|(a, b)| a == b));
    }
}

#[test]
fn invalid_parameters_return_errors() {
    for sigma in [-1.0, MAX_SIGMA + 0.1, f32::NAN, f32::INFINITY] {
        assert!(kernel(sigma).is_err());
    }
    for (width, height) in [(0, 1), (1, 0), (8193, 1), (4096, 4096)] {
        assert!(dimensions(width, height, 8192).is_err());
    }
    assert!(dimensions(2048, 2048, 8192).is_ok());
}

fn reference(input: &[[f32; 4]], width: u32, height: u32, sigma: f32) -> Vec<[f32; 4]> {
    let weights = kernel(sigma).unwrap();
    let radius = (weights.len() / 2) as i32;
    let mut source = input.to_vec();
    for axis in 0..2 {
        let mut output = vec![[0.0; 4]; source.len()];
        for y in 0..height as i32 {
            for x in 0..width as i32 {
                let result = &mut output[(y as u32 * width + x as u32) as usize];
                for offset in -radius..=radius {
                    let (sx, sy) = if axis == 0 { (x + offset, y) } else { (x, y + offset) };
                    if sx < 0 || sy < 0 || sx >= width as i32 || sy >= height as i32 {
                        continue;
                    }
                    let color = source[(sy as u32 * width + sx as u32) as usize];
                    let weight = weights[(offset + radius) as usize];
                    for (dst, src) in result.iter_mut().zip(color) {
                        *dst += src * weight;
                    }
                }
            }
        }
        source = output;
    }
    source
}

#[test]
fn transparent_edges_do_not_duplicate_color() {
    let a = reference(&[[0.25, 0.0, 0.0, 0.5]], 1, 1, 1.0);
    assert!(a[0][3] < 0.5 && a[0][3] > 0.0);
    assert!((a[0][0] * 2.0 - a[0][3]).abs() < 0.00001);
}

#[test]
fn impulse_spreads_symmetrically_and_retains_premultiplication() {
    let mut image = vec![[0.0; 4]; 25 * 25];
    image[12 * 25 + 12] = [0.25, 0.125, 0.0, 0.5];
    let output = reference(&image, 25, 25, 2.0);
    let alpha: f32 = output.iter().map(|c| c[3]).sum();
    assert!((alpha - 0.5).abs() < 0.00001);
    assert!(output.iter().all(|c| c[0] <= c[3] && c[1] <= c[3]));
    assert_eq!(output[12 * 25 + 10], output[12 * 25 + 14]);
}

#[cfg(not(target_arch = "wasm32"))]
fn readback(effects: &GpuEffects, image: &EffectImage) -> Vec<[f32; 4]> {
    let [width, height] = image.dimensions();
    let half_float = image.texture.format() == wgpu::TextureFormat::Rgba16Float;
    let bytes_per_pixel = if half_float { 8 } else { 4 };
    let row = (width * bytes_per_pixel).div_ceil(256) * 256;
    let buffer = effects.device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("Effects correctness readback only"),
        size: u64::from(row) * u64::from(height),
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });
    let mut encoder = effects.device.create_command_encoder(&Default::default());
    encoder.copy_texture_to_buffer(
        image.texture.as_image_copy(),
        wgpu::TexelCopyBufferInfo {
            buffer: &buffer,
            layout: wgpu::TexelCopyBufferLayout { offset: 0, bytes_per_row: Some(row), rows_per_image: Some(height) },
        },
        image.texture.size(),
    );
    let submission = effects.queue.submit([encoder.finish()]);
    let (send, recv) = std::sync::mpsc::channel();
    buffer.slice(..).map_async(wgpu::MapMode::Read, move |result| {
        // A timed-out test may already have closed the receiver.
        let _ = send.send(result);
    });
    effects.device.poll(wgpu::PollType::Wait { submission_index: Some(submission), timeout: Some(std::time::Duration::from_secs(10)) }).unwrap();
    recv.recv_timeout(std::time::Duration::from_secs(10)).unwrap().unwrap();
    let mapped = buffer.slice(..).get_mapped_range().unwrap();
    let mut pixels = Vec::with_capacity((width * height) as usize);
    for line in mapped.chunks_exact(row as usize) {
        for p in line[..(width * bytes_per_pixel) as usize].chunks_exact(bytes_per_pixel as usize) {
            let mut color = [0.0; 4];
            for (channel, value) in color.iter_mut().enumerate() {
                *value = if half_float {
                    half::f16::from_bits(u16::from_le_bytes([p[channel * 2], p[channel * 2 + 1]])).to_f32()
                } else {
                    p[channel] as f32 / 255.0
                };
            }
            pixels.push(color);
        }
    }
    drop(mapped);
    buffer.unmap();
    pixels
}

/// Explicit GPU acceptance: no silent CPU-adapter fallback or passing skip on missing hardware.
#[cfg(not(target_arch = "wasm32"))]
#[test]
#[ignore = "requires a real GPU; run explicitly with --ignored --nocapture"]
fn hardware_gaussian_parity() {
    pollster::block_on(async {
        let backends = if cfg!(windows) { wgpu::Backends::DX12 } else { wgpu::Backends::PRIMARY };
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor { backends, ..wgpu::InstanceDescriptor::new_without_display_handle() });
        let adapters = instance.enumerate_adapters(backends).await;
        let adapter = adapters
            .iter()
            .find(|a| a.get_info().name.contains("NVIDIA"))
            .or_else(|| adapters.iter().find(|a| a.get_info().device_type == wgpu::DeviceType::DiscreteGpu))
            .expect("explicit hardware test requires a discrete GPU");
        let info = adapter.get_info();
        println!("Effects hardware adapter: {} ({:?}, {:?})", info.name, info.backend, info.device_type);
        let (device, queue) = adapter.request_device(&wgpu::DeviceDescriptor::default()).await.unwrap();
        let start = std::time::Instant::now();
        let effects = GpuEffects::new(device.clone(), queue.clone()).await.unwrap();
        println!("Cold effects pipeline setup: {:?}", start.elapsed());
        for (width, height, sigma) in [(1, 1, 1.0), (17, 13, 0.0), (17, 13, 0.25), (25, 25, 2.0), (33, 19, 10.0), (512, 512, 3.0)] {
            let mut bytes = Vec::with_capacity(width * height * 4);
            for index in 0..width * height {
                let alpha = ((index * 53 + 197) % 256) as u8;
                let green = ((index * 29) % (usize::from(alpha) + 1)) as u8;
                bytes.extend([alpha / 2, green, alpha / 8, alpha]);
            }
            let input: Vec<[f32; 4]> = bytes.chunks_exact(4).map(|p| std::array::from_fn(|i| p[i] as f32 / 255.0)).collect();
            let start = std::time::Instant::now();
            let cpu = reference(&input, width as u32, height as u32, sigma);
            let cpu_time = start.elapsed();
            let start = std::time::Instant::now();
            let source = effects.upload_rgba8(width as u32, height as u32, &bytes).await.unwrap();
            let blurred = effects.gaussian_blur(&source, sigma).await.unwrap();
            let gpu = readback(&effects, &blurred);
            let total = start.elapsed();
            let worst = gpu.iter().flatten().zip(cpu.iter().flatten()).map(|(a, b)| (a - b).abs()).fold(0.0f32, f32::max);
            println!("{width}x{height}, sigma {sigma}: worst={worst:.7}, CPU oracle={cpu_time:?}, upload+GPU+readback={total:?}");
            assert!(worst <= 0.002, "GPU parity failed: {worst}");
            assert!(gpu.iter().all(|p| p.iter().all(|c| c.is_finite()) && p[..3].iter().all(|c| *c <= p[3] + 0.001)));
            let chained = effects.gaussian_blur(&blurred, 1.0).await.unwrap();
            let chained = readback(&effects, &chained);
            let reference = reference(&cpu, width as u32, height as u32, 1.0);
            let worst = chained.iter().flatten().zip(reference.iter().flatten()).map(|(a, b)| (a - b).abs()).fold(0.0f32, f32::max);
            assert!(worst <= 0.002, "resident chained output parity failed: {worst}");
            assert!(effects.gaussian_blur(&source, f32::NAN).await.is_err());
            let other = GpuEffects::new(device.clone(), queue.clone()).await.unwrap();
            assert!(other.gaussian_blur(&source, 1.0).await.is_err());
        }
        assert!(effects.upload_rgba8(0, 1, &[]).await.is_err());
        assert!(effects.upload_rgba8(1, 1, &[]).await.is_err());
        assert!(effects.upload_rgba8(1, 1, &[255, 0, 0, 0]).await.is_err());
        assert!(effects.upload_rgba8(4096, 4096, &[]).await.is_err());
    });
}

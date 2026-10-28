//! Device ownership is outside paint-core. CPU data remains authoritative.
use paint_ops::{Cpu, Executor, Operator, Rgba};
use std::collections::BTreeMap;
use std::sync::{
    atomic::{AtomicBool, AtomicU64, Ordering},
    Arc, Mutex, OnceLock, Weak,
};

#[derive(Clone, Debug)]
pub struct Hardware {
    pub id: String,
    pub name: String,
    pub backend: String,
}
struct Adapter {
    hardware: Hardware,
    adapter: wgpu::Adapter,
}
static ADAPTERS: OnceLock<Vec<Adapter>> = OnceLock::new();
fn adapters() -> &'static [Adapter] {
    ADAPTERS.get_or_init(|| {
        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
            backends: wgpu::Backends::PRIMARY,
            ..Default::default()
        });
        let mut devices = instance.enumerate_adapters(wgpu::Backends::PRIMARY);
        devices.sort_by_key(|a| match a.get_info().device_type {
            wgpu::DeviceType::DiscreteGpu => 0,
            wgpu::DeviceType::IntegratedGpu => 1,
            _ => 2,
        });
        devices
            .into_iter()
            .filter(|a| a.get_info().device_type != wgpu::DeviceType::Cpu)
            .map(|adapter| {
                let info = adapter.get_info();
                Adapter {
                    hardware: Hardware {
                        id: format!(
                            "{:?}:{:08x}:{:08x}:{}",
                            info.backend, info.vendor, info.device, info.name
                        ),
                        name: info.name,
                        backend: format!("{:?}", info.backend),
                    },
                    adapter,
                }
            })
            .collect()
    })
}
pub fn hardware() -> Vec<Hardware> {
    adapters().iter().map(|a| a.hardware.clone()).collect()
}

#[derive(Debug)]
pub struct Gpu {
    device: wgpu::Device,
    queue: wgpu::Queue,
    pipeline: wgpu::ComputePipeline,
    label: String,
    failed: Arc<AtomicBool>,
    serial: Mutex<Option<Buffers>>,
    sources: Mutex<SourceCache>,
    submissions: AtomicU64,
}
#[derive(Debug)]
struct Buffers {
    input: wgpu::Buffer,
    output: wgpu::Buffer,
    readback: wgpu::Buffer,
    group: wgpu::BindGroup,
}
#[derive(Debug, Default)]
struct SourceCache {
    buffer: Option<wgpu::Buffer>,
    slots: BTreeMap<usize, u32>,
    // Weak identities prevent address reuse without pinning a completed session's RAM.
    identities: Vec<Weak<paint_ops::SourcePixels>>,
}
impl SourceCache {
    fn prepare(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        tiles: &[paint_ops::SourceTile],
    ) -> Result<(Vec<u32>, bool), String> {
        if tiles.iter().any(|tile| !tile.is_valid()) {
            return Err("invalid source tile".into());
        }
        let limit = paint_ops::MAX_SOURCE_TILES
            .min(device.limits().max_storage_buffer_binding_size as usize / 65536);
        if tiles.len() > limit {
            return Err("GPU source cache budget".into());
        }
        let needed = tiles
            .iter()
            .filter(|t| !self.slots.contains_key(&(Arc::as_ptr(&t.pixels) as usize)))
            .count();
        if self.slots.len() + needed > limit {
            self.slots.clear();
            self.identities.clear();
        }
        let required = (self.slots.len() + tiles.len())
            .min(limit)
            .max(1)
            .next_power_of_two()
            .min(limit) as u64
            * 65536;
        let mut recreated = false;
        if self.buffer.as_ref().is_none_or(|b| b.size() < required) {
            let buffer = device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("Immutable liquify sources"),
                size: required,
                usage: wgpu::BufferUsages::STORAGE
                    | wgpu::BufferUsages::COPY_DST
                    | wgpu::BufferUsages::COPY_SRC,
                mapped_at_creation: false,
            });
            if let Some(old) = &self.buffer {
                let mut encoder = device.create_command_encoder(&Default::default());
                encoder.copy_buffer_to_buffer(old, 0, &buffer, 0, old.size());
                queue.submit([encoder.finish()]);
            }
            self.buffer = Some(buffer);
            recreated = true;
        }
        let mut result = Vec::with_capacity(tiles.len());
        for tile in tiles {
            let key = Arc::as_ptr(&tile.pixels) as usize;
            let slot = if let Some(slot) = self.slots.get(&key) {
                *slot
            } else {
                let slot = self.slots.len() as u32;
                queue.write_buffer(
                    self.buffer.as_ref().expect("source buffer"),
                    u64::from(slot) * 65536,
                    bytemuck::cast_slice(&tile.pixels[..]),
                );
                self.slots.insert(key, slot);
                self.identities.push(Arc::downgrade(&tile.pixels));
                slot
            };
            result.push(slot);
        }
        Ok((result, recreated))
    }
}
impl Gpu {
    pub fn open(id: &str) -> Result<Self, String> {
        let adapter = adapters()
            .iter()
            .find(|a| id == "auto" || a.hardware.id == id)
            .ok_or("没有可用的 GPU 计算硬件")?;
        let (device, queue) =
            pollster::block_on(adapter.adapter.request_device(&wgpu::DeviceDescriptor {
                label: Some("DrawVerse compute"),
                ..Default::default()
            }))
            .map_err(|e| e.to_string())?;
        let failed = Arc::new(AtomicBool::new(false));
        let signal = failed.clone();
        device.on_uncaptured_error(Box::new(move |_| {
            signal.store(true, Ordering::Release);
        }));
        let signal = failed.clone();
        device.set_device_lost_callback(move |_, _| {
            signal.store(true, Ordering::Release);
        });
        device.push_error_scope(wgpu::ErrorFilter::Validation);
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Shared numeric operators"),
            source: wgpu::ShaderSource::Wgsl(include_str!("operators.wgsl").into()),
        });
        let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("Elastic / remap"),
            layout: None,
            module: &shader,
            entry_point: Some("main"),
            compilation_options: Default::default(),
            cache: None,
        });
        if let Some(error) = pollster::block_on(device.pop_error_scope()) {
            return Err(error.to_string());
        }
        Ok(Self {
            device,
            queue,
            pipeline,
            label: format!("{} ({})", adapter.hardware.name, adapter.hardware.backend),
            failed,
            serial: Mutex::new(None),
            sources: Mutex::new(SourceCache::default()),
            submissions: AtomicU64::new(0),
        })
    }
    pub fn submissions(&self) -> u64 {
        self.submissions.load(Ordering::Relaxed)
    }
}
impl Executor for Gpu {
    fn prepare_sources(&self, tiles: &[paint_ops::SourceTile]) -> Result<(), String> {
        let mut buffers = self.serial.lock().map_err(|_| "GPU queue poisoned")?;
        let mut sources = self.sources.lock().map_err(|_| "GPU sources poisoned")?;
        let (_, recreated) = sources.prepare(&self.device, &self.queue, tiles)?;
        if recreated {
            *buffers = None;
        }
        self.queue.submit([]);
        Ok(())
    }
    fn name(&self) -> String {
        self.label.clone()
    }
    fn execute(&self, op: &Operator) -> Result<Vec<Rgba>, String> {
        let mut buffers = self.serial.lock().map_err(|_| "GPU queue poisoned")?;
        if self.failed.load(Ordering::Acquire) {
            return Err("GPU device unavailable".into());
        }
        op.validate()?;
        let tiles = match op {
            Operator::Adjustment(_) | Operator::Elastic(_) => &[][..],
            Operator::Remap(v) | Operator::RemapBlocks(v) => &v.source,
        };
        let mut sources = self.sources.lock().map_err(|_| "GPU sources poisoned")?;
        let (slots, recreated) = sources.prepare(&self.device, &self.queue, tiles)?;
        if recreated {
            *buffers = None;
        }
        let packed = paint_ops::pack_with_sources(op, &slots)?;
        let bytes = bytemuck::cast_slice::<Rgba, u8>(&packed);
        let output_size = (op.count() * 16) as u64;
        let limits = self.device.limits();
        if bytes.len() as u64 > u64::from(limits.max_storage_buffer_binding_size)
            || output_size > u64::from(limits.max_storage_buffer_binding_size)
        {
            return Err("GPU storage binding budget".into());
        }
        if buffers
            .as_ref()
            .is_none_or(|b| b.input.size() < bytes.len() as u64 || b.output.size() < output_size)
        {
            let input = self.device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("Operator input"),
                size: (bytes.len() as u64)
                    .next_power_of_two()
                    .min(u64::from(limits.max_storage_buffer_binding_size))
                    .max(buffers.as_ref().map_or(0, |b| b.input.size())),
                usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            });
            let capacity = output_size
                .next_power_of_two()
                .min(u64::from(limits.max_storage_buffer_binding_size))
                .max(buffers.as_ref().map_or(0, |b| b.output.size()));
            let output = self.device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("Operator output"),
                size: capacity,
                usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
                mapped_at_creation: false,
            });
            let readback = self.device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("Bounded readback"),
                size: capacity,
                usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            });
            let group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: None,
                layout: &self.pipeline.get_bind_group_layout(0),
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: input.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: output.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 2,
                        resource: sources
                            .buffer
                            .as_ref()
                            .expect("source buffer")
                            .as_entire_binding(),
                    },
                ],
            });
            *buffers = Some(Buffers {
                input,
                output,
                readback,
                group,
            });
        }
        let Buffers {
            input,
            output,
            readback,
            group,
        } = buffers.as_ref().expect("buffers");
        self.queue.write_buffer(input, 0, bytes);
        let mut encoder = self.device.create_command_encoder(&Default::default());
        {
            let mut pass = encoder.begin_compute_pass(&Default::default());
            pass.set_pipeline(&self.pipeline);
            pass.set_bind_group(0, group, &[]);
            let count = match op {
                Operator::Remap(v) => op.count() / (v.stride * v.stride) as usize,
                _ => op.count(),
            };
            let groups = (count as u32).div_ceil(64);
            pass.dispatch_workgroups(groups.min(65535), groups.div_ceil(65535), 1);
        }
        encoder.copy_buffer_to_buffer(output, 0, readback, 0, output_size);
        self.queue.submit([encoder.finish()]);
        self.submissions.fetch_add(1, Ordering::Relaxed);
        let (send, receive) = std::sync::mpsc::sync_channel(1);
        readback
            .slice(..output_size)
            .map_async(wgpu::MapMode::Read, move |r| {
                let _ = send.send(r);
            });
        // Poll with a finite timeout; device loss cannot hold a document actor forever.
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
        loop {
            self.device
                .poll(wgpu::PollType::Poll)
                .map_err(|e| e.to_string())?;
            match receive.try_recv() {
                Ok(result) => {
                    result.map_err(|e| e.to_string())?;
                    break;
                }
                Err(std::sync::mpsc::TryRecvError::Empty)
                    if std::time::Instant::now() < deadline =>
                {
                    std::thread::yield_now()
                }
                _ => {
                    self.failed.store(true, Ordering::Release);
                    return Err("GPU readback timeout/device loss".into());
                }
            }
        }
        let mapped = readback.slice(..output_size).get_mapped_range();
        let result = bytemuck::try_cast_slice::<u8, Rgba>(&mapped)
            .map_err(|e| e.to_string())?
            .to_vec();
        drop(mapped);
        readback.unmap();
        if self.failed.load(Ordering::Acquire) {
            return Err("GPU device failed during execution".into());
        }
        if result.iter().flatten().any(|v| !v.is_finite()) {
            return Err("non-finite GPU result".into());
        }
        Ok(result)
    }
}

/// Retains a visible fallback reason. A failed backend is disabled for this executor.
#[derive(Debug)]
pub struct Accelerated {
    gpu: Option<Gpu>,
    reason: Mutex<String>,
    disabled: AtomicBool,
}
impl Accelerated {
    pub fn new(id: &str) -> Self {
        match Gpu::open(id) {
            Ok(gpu) => Self {
                gpu: Some(gpu),
                reason: Mutex::new(String::new()),
                disabled: AtomicBool::new(false),
            },
            Err(reason) => Self {
                gpu: None,
                reason: Mutex::new(reason),
                disabled: AtomicBool::new(true),
            },
        }
    }
    pub fn status(&self) -> String {
        let reason = self.reason.lock().unwrap_or_else(|e| e.into_inner());
        if self.disabled.load(Ordering::Acquire) {
            format!("CPU：{reason}")
        } else {
            format!("GPU：{}", self.gpu.as_ref().expect("enabled GPU").name())
        }
    }
}
impl Executor for Accelerated {
    fn prepare_sources(&self, tiles: &[paint_ops::SourceTile]) -> Result<(), String> {
        if !self.disabled.load(Ordering::Acquire) {
            if let Some(gpu) = &self.gpu {
                if let Err(reason) = gpu.prepare_sources(tiles) {
                    *self.reason.lock().unwrap_or_else(|e| e.into_inner()) = reason;
                    self.disabled.store(true, Ordering::Release);
                }
            }
        }
        Ok(())
    }
    fn name(&self) -> String {
        self.status()
    }
    fn execute(&self, op: &Operator) -> Result<Vec<Rgba>, String> {
        if !self.disabled.load(Ordering::Acquire) {
            if let Some(gpu) = &self.gpu {
                match gpu.execute(op) {
                    Ok(result) => return Ok(result),
                    Err(reason) => {
                        // An invalid request is not a device fault. Successful requests
                        // are validated once by pack, without rescanning source pixels.
                        op.validate()?;
                        *self.reason.lock().unwrap_or_else(|e| e.into_inner()) = reason;
                        self.disabled.store(true, Ordering::Release);
                    }
                }
            }
        }
        Cpu.execute(op)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn adjustment_shader_matches_cpu_on_transparent_and_translucent_colors() {
        let gpu = Gpu::open("auto").expect("GPU required for adjustment acceptance");
        let pixels = (0..8192)
            .map(|i| {
                let a = (i % 29) as f32 / 28.;
                [
                    (i % 97) as f32 / 96. * a,
                    (i % 47) as f32 / 46. * a,
                    (i % 19) as f32 / 18. * a,
                    a,
                ]
            })
            .collect::<Vec<_>>();
        for (kind, parameters) in [
            (0, [0.; 4]),
            (1, [0.5, 0.5, -0.2, 0.]),
            (1, [-0.5, -1., 1., 0.]),
            (1, [0.1, 1., 0., 0.]),
            (1, [0.7, 0.25, -0.3, 1.]),
            (1, [0.3, 0., 0.3, 1.]),
        ] {
            let op = Operator::Adjustment(paint_ops::AdjustmentOp {
                pixels: pixels.clone(),
                kind,
                parameters,
                lut: (0..=4096)
                    .map(|i| {
                        let x = i as f32 / 4096.;
                        [x * x, 1. - x, x.sqrt(), 1.]
                    })
                    .collect(),
            });
            let expected = Cpu.execute(&op).unwrap();
            let actual = gpu.execute(&op).unwrap();
            let error = expected
                .iter()
                .zip(&actual)
                .flat_map(|(a, b)| a.iter().zip(b).map(|(x, y)| (x - y).abs()))
                .fold(0., f32::max);
            assert!(
                error < 0.00002,
                "adjustment kind {kind} maximum error {error}"
            );
            assert!(gpu.submissions() > 0);
        }
    }
    #[test]
    fn lost_device_replays_immutable_input_and_stays_on_cpu() {
        if hardware().is_empty() {
            eprintln!("No hardware; device-loss replay test not run");
            return;
        }
        let gpu = Gpu::open("auto").unwrap();
        // Exercise the same signal written by wgpu's device-loss/error callbacks.
        gpu.failed.store(true, Ordering::Release);
        let executor = Accelerated {
            gpu: Some(gpu),
            reason: Mutex::new(String::new()),
            disabled: AtomicBool::new(false),
        };
        let op = Operator::Remap(paint_ops::RemapOp {
            grid: paint_ops::Grid {
                origin: [0, 0],
                width: 2,
                height: 2,
                cell: 2.,
                vectors: vec![[0.; 2]; 4],
            },
            source: vec![],
            white: [64, 64],
            tiles: vec![[0, 0]],
            selection: vec![],
            stride: 1,
            cubic: true,
        });
        let expected = Cpu.execute(&op).unwrap();
        assert_eq!(executor.execute(&op).unwrap(), expected);
        assert!(executor.status().contains("GPU device unavailable"));
        assert_eq!(executor.execute(&op).unwrap(), expected);
        assert_eq!(executor.gpu.as_ref().unwrap().submissions(), 0);
    }
}

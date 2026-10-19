//! Device ownership is outside paint-core. CPU data remains authoritative.
use paint_ops::{Cpu, Executor, Operator, Rgba};
use std::sync::{
    atomic::{AtomicBool, AtomicU64, Ordering},
    Arc, Mutex, OnceLock,
};
use wgpu::util::DeviceExt;

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
    serial: Mutex<()>,
    submissions: AtomicU64,
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
            serial: Mutex::new(()),
            submissions: AtomicU64::new(0),
        })
    }
    pub fn submissions(&self) -> u64 {
        self.submissions.load(Ordering::Relaxed)
    }
}
impl Executor for Gpu {
    fn name(&self) -> String {
        self.label.clone()
    }
    fn execute(&self, op: &Operator) -> Result<Vec<Rgba>, String> {
        let _guard = self.serial.lock().map_err(|_| "GPU queue poisoned")?;
        if self.failed.load(Ordering::Acquire) {
            return Err("GPU device unavailable".into());
        }
        let packed = paint_ops::pack(op)?;
        let bytes: Vec<u8> = packed
            .iter()
            .flatten()
            .flat_map(|v| v.to_le_bytes())
            .collect();
        let output_size = (op.count() * 16) as u64;
        let limits = self.device.limits();
        if bytes.len() as u64 > u64::from(limits.max_storage_buffer_binding_size)
            || output_size > u64::from(limits.max_storage_buffer_binding_size)
        {
            return Err("GPU storage binding budget".into());
        }
        let input = self
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Operator input"),
                contents: &bytes,
                usage: wgpu::BufferUsages::STORAGE,
            });
        let output = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Operator output"),
            size: output_size,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });
        let readback = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Bounded readback"),
            size: output_size,
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
            ],
        });
        let mut encoder = self.device.create_command_encoder(&Default::default());
        {
            let mut pass = encoder.begin_compute_pass(&Default::default());
            pass.set_pipeline(&self.pipeline);
            pass.set_bind_group(0, &group, &[]);
            let groups = (op.count() as u32).div_ceil(64);
            pass.dispatch_workgroups(groups.min(65535), groups.div_ceil(65535), 1);
        }
        encoder.copy_buffer_to_buffer(&output, 0, &readback, 0, output_size);
        self.queue.submit([encoder.finish()]);
        self.submissions.fetch_add(1, Ordering::Relaxed);
        let (send, receive) = std::sync::mpsc::sync_channel(1);
        readback.slice(..).map_async(wgpu::MapMode::Read, move |r| {
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
        let mapped = readback.slice(..).get_mapped_range();
        let result = mapped
            .chunks_exact(16)
            .map(|p| {
                std::array::from_fn(|k| {
                    f32::from_le_bytes(p[k * 4..k * 4 + 4].try_into().expect("RGBA word"))
                })
            })
            .collect::<Vec<Rgba>>();
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
    fn name(&self) -> String {
        self.status()
    }
    fn execute(&self, op: &Operator) -> Result<Vec<Rgba>, String> {
        op.validate()?;
        if !self.disabled.load(Ordering::Acquire) {
            if let Some(gpu) = &self.gpu {
                match gpu.execute(op) {
                    Ok(result) => return Ok(result),
                    Err(reason) => {
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

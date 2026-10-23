//! Opt-in, bounded local task diagnostics. No ABI or scheduling changes.
use std::{
    path::PathBuf,
    sync::{
        atomic::{AtomicU64, Ordering},
        Mutex, OnceLock,
    },
    thread::ThreadId,
    time::Instant,
};

#[derive(Clone, Copy, Default)]
pub(crate) struct Context {
    pub session: u64,
    pub task: u64,
    pub sequence: u64,
    pub view: u32,
    pub generation: u64,
    pub request: u64,
    pub revision: u64,
    pub frame: u64,
}
struct Event {
    name: &'static str,
    ns: u64,
    thread: ThreadId,
    context: Context,
    value: i64,
}
struct Buffer {
    events: Vec<Event>,
    dropped: u64,
    closed: bool,
}
struct Recorder {
    origin: Instant,
    path: PathBuf,
    buffer: Mutex<Buffer>,
}
static RECORDER: OnceLock<Option<Recorder>> = OnceLock::new();
static IDS: AtomicU64 = AtomicU64::new(1);
const CAPACITY: usize = 50_000;
fn recorder() -> Option<&'static Recorder> {
    RECORDER
        .get_or_init(|| {
            let path = std::env::var_os("DRAWVERSE_MOVE_COMPLETION_TRACE")?;
            if path.is_empty() {
                return None;
            }
            Some(Recorder {
                origin: Instant::now(),
                path: path.into(),
                buffer: Mutex::new(Buffer {
                    events: Vec::with_capacity(CAPACITY),
                    dropped: 0,
                    closed: false,
                }),
            })
        })
        .as_ref()
}
pub(crate) fn enabled() -> bool {
    recorder().is_some()
}
pub(crate) fn id() -> u64 {
    if enabled() {
        IDS.fetch_add(1, Ordering::Relaxed)
    } else {
        0
    }
}
pub(crate) fn point(name: &'static str, context: Context, value: i64) {
    let Some(recorder) = recorder() else {
        return;
    };
    recorder.record(name, context, value);
}
impl Recorder {
    fn record(&self, name: &'static str, context: Context, value: i64) {
        let ns = self.origin.elapsed().as_nanos() as u64;
        let mut buffer = self.buffer.lock().unwrap_or_else(|e| e.into_inner());
        if buffer.closed {
            return;
        }
        if buffer.events.len() == CAPACITY {
            buffer.dropped += 1;
            return;
        }
        buffer.events.push(Event {
            name,
            ns,
            thread: std::thread::current().id(),
            context,
            value,
        });
    }
    fn freeze(&self) -> Option<(Vec<Event>, u64)> {
        let mut buffer = self.buffer.lock().unwrap_or_else(|e| e.into_inner());
        if buffer.closed {
            return None;
        }
        buffer.closed = true;
        Some((std::mem::take(&mut buffer.events), buffer.dropped))
    }
}
/// Called after core destruction has joined actor and render workers. Freeze before I/O.
pub(crate) fn flush() {
    let Some(recorder) = recorder() else {
        return;
    };
    let Some((events, dropped)) = recorder.freeze() else {
        return;
    };
    let events: Vec<_> = events.into_iter().map(|e| {
        let c=e.context;
        serde_json::json!({"name":e.name,"ns":e.ns,"thread":format!("{:?}",e.thread),
            "session":c.session,"task":c.task,"sequence":c.sequence,"view":c.view,
            "generation":c.generation,"request":c.request,"revision":c.revision,"frame":c.frame,"value":e.value})
    }).collect();
    let result = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&recorder.path)
        .and_then(|file| {
            serde_json::to_writer(
                file,
                &serde_json::json!({"schema":1,
            "clock":"Rust Instant local origin; not C++ steady_clock", "capacity":CAPACITY,
            "dropped":dropped,"closed":true,"events":events}),
            )
            .map_err(std::io::Error::other)
        });
    if let Err(error) = result {
        eprintln!("DrawVerse completion trace export failed: {error}");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn bounded_freeze_rejects_tail_and_second_export() {
        let recorder = Recorder {
            origin: Instant::now(),
            path: PathBuf::new(),
            buffer: Mutex::new(Buffer {
                events: Vec::new(),
                dropped: 0,
                closed: false,
            }),
        };
        for _ in 0..CAPACITY + 1 {
            recorder.record("accepted", Context::default(), 1);
        }
        let (events, dropped) = recorder.freeze().unwrap();
        assert_eq!(events.len(), CAPACITY);
        assert_eq!(dropped, 1);
        let barrier = std::sync::Barrier::new(2);
        std::thread::scope(|scope| {
            scope.spawn(|| {
                barrier.wait();
                recorder.record("rejected_after_freeze", Context::default(), 2);
            });
            barrier.wait();
        });
        assert!(recorder.freeze().is_none());
        assert!(recorder.buffer.lock().unwrap().events.is_empty());
    }
}

//! One retained file result per session. Heavy snapshots/results stay on workers.
use crate::{
    api::*,
    error::{ApiError, ApiResult},
};
use paint_core::Document;
use paint_task::{CancellationToken, Priority, TaskError, TaskPool, TaskTicket};
use std::{
    mem::size_of,
    path::PathBuf,
    sync::{Arc, Mutex},
};
#[derive(Clone)]
pub(crate) struct Request {
    pub path: PathBuf,
    pub kind: u32,
    pub format: u32,
    pub generation: u64,
    pub options: paint_io::ExportOptions,
}
pub(crate) struct FileState {
    pub info: PaintFileJobInfo,
    pub message: String,
    pub cancel: CancellationToken,
}
impl Default for FileState {
    fn default() -> Self {
        Self {
            info: PaintFileJobInfo {
                struct_size: size_of::<PaintFileJobInfo>() as u32,
                ..Default::default()
            },
            message: String::new(),
            cancel: CancellationToken::default(),
        }
    }
}
impl FileState {
    pub fn reserve(&mut self, request: &Request) -> ApiResult<u64> {
        if matches!(self.info.state, PAINT_FILE_QUEUED | PAINT_FILE_RUNNING) {
            return Err(ApiError::new(
                PAINT_BUSY,
                "a file operation is already pending",
            ));
        }
        let id = self
            .info
            .job_id
            .checked_add(1)
            .ok_or_else(ApiError::internal)?;
        self.info = PaintFileJobInfo {
            struct_size: size_of::<PaintFileJobInfo>() as u32,
            state: PAINT_FILE_QUEUED,
            job_id: id,
            source_generation: request.generation,
            kind: request.kind,
            format: request.format,
            ..Default::default()
        };
        self.message.clear();
        self.cancel = CancellationToken::default();
        Ok(id)
    }
    pub fn finish(&mut self, error: Option<ApiError>, generation: u64, revision: u64) {
        self.info.result_generation = generation;
        self.info.result_revision = revision;
        if let Some(error) = error {
            self.info.status = error.status;
            self.message = error.message;
            self.info.state = if error.status == PAINT_CANCELLED {
                PAINT_FILE_CANCELLED
            } else {
                PAINT_FILE_FAILED
            };
        } else {
            self.info.status = PAINT_OK;
            self.info.state = PAINT_FILE_SUCCEEDED;
        }
    }
}
enum Outcome {
    Opened(Box<Document>, paint_io::Format),
    Saved,
}
struct Job {
    ticket: TaskTicket<ApiResult<Outcome>>,
    generation: u64,
    revision: u64,
    format: u32,
}
pub(crate) struct FileService {
    pub state: Arc<Mutex<FileState>>,
    pub pool: Arc<TaskPool>,
    job: Option<Job>,
}
impl FileService {
    pub fn new(state: Arc<Mutex<FileState>>, pool: Arc<TaskPool>) -> Self {
        Self {
            state,
            pool,
            job: None,
        }
    }
    pub fn start(&mut self, request: Request, doc: &Document, generation: u64) {
        let mut state = self.state.lock().unwrap_or_else(|e| e.into_inner());
        let fail = if state.cancel.is_cancelled() {
            Some(ApiError::new(PAINT_CANCELLED, "file operation cancelled"))
        } else if request.generation != generation || doc.stroke_active() {
            Some(ApiError::new(
                PAINT_BUSY,
                "document changed or a stroke is active",
            ))
        } else {
            None
        };
        if let Some(error) = fail {
            state.finish(Some(error), generation, doc.revision());
            return;
        }
        state.info.state = PAINT_FILE_RUNNING;
        state.info.source_revision = doc.revision();
        let cancel = state.cancel.clone();
        let snapshot = doc.snapshot();
        let document_options = doc.document_options();
        let storage = doc.storage();
        let active = doc.active_layer();
        let revision = doc.revision();
        let format = request.format;
        let result = self.pool.submit(Priority::Normal, move |_| {
            Ok(if request.kind == PAINT_FILE_OPEN {
                paint_io::load_with_storage(&request.path, &cancel, document_options, storage)
                    .map(|(doc, format)| Outcome::Opened(Box::new(doc), format))
                    .map_err(io_error)
            } else {
                paint_io::save(&request.path, &snapshot, active, request.options, &cancel)
                    .map(|()| Outcome::Saved)
                    .map_err(io_error)
            })
        });
        match result {
            Ok(ticket) => {
                self.job = Some(Job {
                    ticket,
                    generation,
                    revision,
                    format,
                })
            }
            Err(e) => state.finish(
                Some(ApiError::new(PAINT_INTERNAL_ERROR, e.to_string())),
                generation,
                revision,
            ),
        }
    }
    /// Return (new document, clear modified), after checking the captured edit generation.
    pub fn poll(
        &mut self,
        generation: u64,
        revision: u64,
        stroke_active: bool,
    ) -> Option<(Option<Document>, bool)> {
        let result = self.job.as_ref()?.ticket.try_take()?;
        let job = self.job.take().unwrap();
        let result = result.unwrap_or_else(|e| {
            Err(ApiError::new(
                if e == TaskError::Cancelled {
                    PAINT_CANCELLED
                } else {
                    PAINT_INTERNAL_ERROR
                },
                e.to_string(),
            ))
        });
        let mut state = self.state.lock().unwrap_or_else(|e| e.into_inner());
        match result {
            Ok(Outcome::Opened(doc, format))
                if job.generation == generation && job.revision == revision && !stroke_active =>
            {
                state.info.format = match format {
                    paint_io::Format::Png => PAINT_FILE_PNG,
                    paint_io::Format::Jpeg => PAINT_FILE_JPEG,
                    paint_io::Format::WebP => PAINT_FILE_WEBP,
                    paint_io::Format::OpenRaster => PAINT_FILE_OPENRASTER,
                };
                state.finish(None, generation + 1, doc.revision());
                Some((Some(*doc), true))
            }
            Ok(Outcome::Opened(..)) => {
                state.finish(
                    Some(ApiError::new(
                        PAINT_BUSY,
                        "document was edited during open; original document retained",
                    )),
                    generation,
                    revision,
                );
                None
            }
            Ok(Outcome::Saved) => {
                state.finish(None, job.generation, job.revision);
                Some((
                    None,
                    job.format == PAINT_FILE_OPENRASTER
                        && job.generation == generation
                        && job.revision == revision
                        && !stroke_active,
                ))
            }
            Err(error) => {
                state.finish(Some(error), generation, revision);
                None
            }
        }
    }
}
fn io_error(error: paint_io::Error) -> ApiError {
    let status = match &error {
        paint_io::Error::Io(_) => PAINT_IO_ERROR,
        paint_io::Error::Unsupported(_) => PAINT_UNSUPPORTED,
        paint_io::Error::Limit(_) => PAINT_LIMIT_EXCEEDED,
        paint_io::Error::Cancelled => PAINT_CANCELLED,
        paint_io::Error::Core(e) => return e.clone().into(),
        _ => PAINT_INVALID_ARGUMENT,
    };
    ApiError::new(status, error.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    fn request(path: PathBuf, kind: u32, format: u32) -> Request {
        Request {
            path,
            kind,
            format,
            generation: 1,
            options: paint_io::ExportOptions {
                format: if format == PAINT_FILE_OPENRASTER {
                    paint_io::Format::OpenRaster
                } else {
                    paint_io::Format::Png
                },
                quality: 95,
                background: [1.; 3],
            },
        }
    }
    fn poll(
        service: &mut FileService,
        generation: u64,
        revision: u64,
    ) -> Option<(Option<Document>, bool)> {
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
        loop {
            let result = service.poll(generation, revision, false);
            if service.state.lock().unwrap().info.state >= PAINT_FILE_SUCCEEDED {
                return result;
            }
            assert!(std::time::Instant::now() < deadline);
            std::thread::sleep(std::time::Duration::from_millis(1));
        }
    }
    #[test]
    fn queued_cancellation_busy_and_expiration_are_explicit() {
        let state = Arc::new(Mutex::new(FileState::default()));
        let pool = Arc::new(TaskPool::new(1, 1).unwrap());
        let mut service = FileService::new(state.clone(), pool);
        let r = request(
            PathBuf::from("not-created.png"),
            PAINT_FILE_SAVE,
            PAINT_FILE_PNG,
        );
        let first = state.lock().unwrap().reserve(&r).unwrap();
        assert_eq!(
            state.lock().unwrap().reserve(&r).unwrap_err().status,
            PAINT_BUSY
        );
        state.lock().unwrap().cancel.cancel();
        service.start(r.clone(), &Document::new(16, 16).unwrap(), 1);
        assert_eq!(state.lock().unwrap().info.state, PAINT_FILE_CANCELLED);
        assert_eq!(state.lock().unwrap().reserve(&r).unwrap(), first + 1);
    }
    #[test]
    fn stale_open_and_save_never_replace_or_clean_newer_document() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("a.ora");
        let doc = Document::new(16, 16).unwrap();
        paint_io::save(
            &path,
            &doc.snapshot(),
            doc.active_layer(),
            paint_io::ExportOptions {
                format: paint_io::Format::OpenRaster,
                quality: 95,
                background: [1.; 3],
            },
            &CancellationToken::default(),
        )
        .unwrap();
        let state = Arc::new(Mutex::new(FileState::default()));
        let pool = Arc::new(TaskPool::new(1, 1).unwrap());
        let mut service = FileService::new(state.clone(), pool);
        let r = request(path.clone(), PAINT_FILE_OPEN, PAINT_FILE_AUTO);
        state.lock().unwrap().reserve(&r).unwrap();
        service.start(r, &doc, 1);
        assert!(poll(&mut service, 2, 0).is_none());
        assert_eq!(state.lock().unwrap().info.status, PAINT_BUSY);
        let r = request(path, PAINT_FILE_SAVE, PAINT_FILE_OPENRASTER);
        state.lock().unwrap().reserve(&r).unwrap();
        service.start(r, &doc, 1);
        let (new, clear) = poll(&mut service, 1, 99).unwrap();
        assert!(new.is_none());
        assert!(!clear);
        assert_eq!(state.lock().unwrap().info.state, PAINT_FILE_SUCCEEDED);
    }
}

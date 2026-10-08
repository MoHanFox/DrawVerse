use crate::session::Session;
use crate::{
    api::*,
    error::{ApiError, ApiResult},
};
use paint_core::Document;
use std::{
    cell::Cell,
    collections::HashMap,
    ffi::c_void,
    sync::{Arc, Condvar, Mutex, MutexGuard, OnceLock},
};

const MAX_TOKENS: usize = 65_536;
thread_local! { static IN_CALLBACK: Cell<bool> = const { Cell::new(false) }; }

pub(crate) fn outside_callback() -> ApiResult<()> {
    if IN_CALLBACK.with(Cell::get) {
        return Err(ApiError::new(
            PAINT_BUSY,
            "mutating/lifecycle operations are forbidden inside callbacks",
        ));
    }
    Ok(())
}

pub(crate) struct DocumentState {
    pub owner: usize,
    pub id: u64,
    pub model: Mutex<Document>,
}

struct CallbackGate {
    enabled: bool,
    in_flight: usize,
}
pub(crate) struct SubscriptionState {
    owner: usize,
    callback: unsafe extern "C" fn(*const PaintEvent, *mut c_void),
    // Retains exposed pointer provenance. User data remains caller-owned until unsubscribe returns.
    user_data: usize,
    gate: Mutex<CallbackGate>,
    drained: Condvar,
}

#[derive(Default)]
pub(crate) struct Registry {
    cores: HashMap<usize, Arc<crate::storage_api::CoreStorage>>,
    documents: HashMap<usize, Arc<DocumentState>>,
    subscriptions: HashMap<usize, Arc<SubscriptionState>>,
    sessions: HashMap<usize, (usize, Arc<Session>)>,
    // Stable addresses are the ABI tokens. Keeping tombstones prevents stale-pointer ABA.
    #[allow(clippy::vec_box)]
    tokens: Vec<Box<u8>>,
}

static REGISTRY: OnceLock<Mutex<Registry>> = OnceLock::new();

pub(crate) fn registry() -> ApiResult<MutexGuard<'static, Registry>> {
    REGISTRY
        .get_or_init(|| Mutex::new(Registry::default()))
        .lock()
        .map_err(|_| ApiError::internal())
}

impl Registry {
    fn token<T>(&mut self) -> ApiResult<*mut T> {
        if self.tokens.len() >= MAX_TOKENS {
            return Err(ApiError::new(
                PAINT_LIMIT_EXCEEDED,
                "process lifetime handle budget exhausted",
            ));
        }
        let mut token = Box::new(0_u8);
        let pointer = (&mut *token as *mut u8).cast::<T>();
        self.tokens.push(token);
        Ok(pointer)
    }

    pub fn core(&self, core: *mut PaintCore) -> ApiResult<usize> {
        let key = core.addr();
        if !self.cores.contains_key(&key) {
            return Err(ApiError::handle());
        }
        Ok(key)
    }

    pub fn storage(&self, core: *mut PaintCore) -> ApiResult<Arc<crate::storage_api::CoreStorage>> {
        let key = self.core(core)?;
        Ok(Arc::clone(&self.cores[&key]))
    }
    pub fn create_core(
        &mut self,
        storage: Arc<crate::storage_api::CoreStorage>,
    ) -> ApiResult<*mut PaintCore> {
        let pointer = self.token::<PaintCore>()?;
        self.cores.insert(pointer.addr(), storage);
        Ok(pointer)
    }

    pub fn destroy_core(
        &mut self,
        core: *mut PaintCore,
    ) -> ApiResult<Arc<crate::storage_api::CoreStorage>> {
        let key = self.core(core)?;
        if self.documents.values().any(|d| d.owner == key)
            || self.subscriptions.values().any(|s| s.owner == key)
            || self.sessions.values().any(|s| s.0 == key)
        {
            return Err(ApiError::new(
                PAINT_BUSY,
                "core still owns documents or subscriptions",
            ));
        }
        self.cores.remove(&key).ok_or_else(ApiError::handle)
    }

    pub fn create_session(
        &mut self,
        core: *mut PaintCore,
        session: Arc<Session>,
    ) -> ApiResult<*mut PaintSession> {
        let owner = self.core(core)?;
        let pointer = self.token::<PaintSession>()?;
        self.sessions.insert(pointer.addr(), (owner, session));
        Ok(pointer)
    }
    pub fn session(
        &self,
        core: *mut PaintCore,
        session: *mut PaintSession,
    ) -> ApiResult<Arc<Session>> {
        let owner = self.core(core)?;
        let entry = self
            .sessions
            .get(&session.addr())
            .ok_or_else(ApiError::handle)?;
        if entry.0 != owner {
            return Err(ApiError::handle());
        }
        Ok(Arc::clone(&entry.1))
    }
    pub fn destroy_session(
        &mut self,
        core: *mut PaintCore,
        session: *mut PaintSession,
    ) -> ApiResult<Arc<Session>> {
        self.session(core, session)?;
        self.sessions
            .remove(&session.addr())
            .map(|entry| entry.1)
            .ok_or_else(ApiError::handle)
    }

    pub fn create_document(
        &mut self,
        core: *mut PaintCore,
        model: Document,
    ) -> ApiResult<*mut PaintDocument> {
        let owner = self.core(core)?;
        let pointer = self.token::<PaintDocument>()?;
        let id = self.tokens.len() as u64;
        self.documents.insert(
            pointer.addr(),
            Arc::new(DocumentState {
                owner,
                id,
                model: Mutex::new(model),
            }),
        );
        Ok(pointer)
    }

    pub fn document(
        &self,
        core: *mut PaintCore,
        doc: *mut PaintDocument,
    ) -> ApiResult<Arc<DocumentState>> {
        let owner = self.core(core)?;
        let state = self
            .documents
            .get(&doc.addr())
            .ok_or_else(ApiError::handle)?;
        if state.owner != owner {
            return Err(ApiError::handle());
        }
        Ok(Arc::clone(state))
    }

    pub fn destroy_document(
        &mut self,
        core: *mut PaintCore,
        doc: *mut PaintDocument,
    ) -> ApiResult<Arc<DocumentState>> {
        self.document(core, doc)?;
        self.documents
            .remove(&doc.addr())
            .ok_or_else(ApiError::handle)
    }

    pub fn subscribe(
        &mut self,
        core: *mut PaintCore,
        callback: PaintEventCallback,
        user_data: *mut c_void,
    ) -> ApiResult<*mut PaintSubscription> {
        let owner = self.core(core)?;
        let callback =
            callback.ok_or_else(|| ApiError::invalid("event callback must not be NULL"))?;
        let pointer = self.token::<PaintSubscription>()?;
        self.subscriptions.insert(
            pointer.addr(),
            Arc::new(SubscriptionState {
                owner,
                callback,
                user_data: user_data as usize,
                gate: Mutex::new(CallbackGate {
                    enabled: true,
                    in_flight: 0,
                }),
                drained: Condvar::new(),
            }),
        );
        Ok(pointer)
    }
}

pub(crate) fn document(
    core: *mut PaintCore,
    doc: *mut PaintDocument,
) -> ApiResult<Arc<DocumentState>> {
    registry()?.document(core, doc)
}

pub(crate) fn read_document<T>(
    core: *mut PaintCore,
    doc: *mut PaintDocument,
    operation: impl FnOnce(&DocumentState, &Document) -> ApiResult<T>,
) -> ApiResult<T> {
    let state = document(core, doc)?;
    let model = state.model.lock().map_err(|_| ApiError::internal())?;
    operation(&state, &model)
}

pub(crate) fn mutate_document<T>(
    core: *mut PaintCore,
    doc: *mut PaintDocument,
    operation: impl FnOnce(&mut Document) -> ApiResult<T>,
) -> ApiResult<T> {
    outside_callback()?;
    let state = document(core, doc)?;
    let mut model = state.model.lock().map_err(|_| ApiError::internal())?;
    let before = model.revision();
    let before_state = (
        model.history_depth(),
        model.active_layer(),
        model.stroke_active(),
    );
    let result = operation(&mut model);
    let pixels_changed = before != model.revision();
    let changed = pixels_changed
        || before_state
            != (
                model.history_depth(),
                model.active_layer(),
                model.stroke_active(),
            );
    let (width, height) = model.dimensions();
    let event = PaintEvent {
        struct_size: std::mem::size_of::<PaintEvent>() as u32,
        kind: if pixels_changed {
            PAINT_EVENT_DOCUMENT_CHANGED
        } else {
            PAINT_EVENT_DOCUMENT_STATE_CHANGED
        },
        document_id: state.id,
        revision: model.revision(),
        width: if pixels_changed { width } else { 0 },
        height: if pixels_changed { height } else { 0 },
        status: result.as_ref().err().map_or(PAINT_OK, |e| e.status),
        ..Default::default()
    };
    drop(model);
    if changed {
        emit(state.owner, &event)?;
    }
    result
}

struct CallbackScope(bool);
impl CallbackScope {
    fn enter() -> Self {
        Self(IN_CALLBACK.with(|value| value.replace(true)))
    }
}
impl Drop for CallbackScope {
    fn drop(&mut self) {
        IN_CALLBACK.with(|value| value.set(self.0));
    }
}

fn emit(owner: usize, event: &PaintEvent) -> ApiResult<()> {
    let callbacks: Vec<_> = registry()?
        .subscriptions
        .values()
        .filter(|s| s.owner == owner)
        .cloned()
        .collect();
    for subscription in callbacks {
        {
            let mut gate = subscription.gate.lock().map_err(|_| ApiError::internal())?;
            if !gate.enabled {
                continue;
            }
            gate.in_flight += 1;
        }
        let scope = CallbackScope::enter();
        // SAFETY: subscribe contract: callback/user_data valid until unsubscribe drains all calls.
        // No library locks are held. Foreign callbacks must neither throw nor unwind.
        unsafe { (subscription.callback)(event, subscription.user_data as *mut c_void) };
        drop(scope);
        let mut gate = subscription.gate.lock().map_err(|_| ApiError::internal())?;
        gate.in_flight -= 1;
        if gate.in_flight == 0 {
            subscription.drained.notify_all();
        }
    }
    Ok(())
}

pub(crate) fn unsubscribe(core: *mut PaintCore, handle: *mut PaintSubscription) -> ApiResult<()> {
    outside_callback()?;
    let state = {
        let registry = registry()?;
        let owner = registry.core(core)?;
        let state = registry
            .subscriptions
            .get(&handle.addr())
            .ok_or_else(ApiError::handle)?;
        if state.owner != owner {
            return Err(ApiError::handle());
        }
        Arc::clone(state)
    };
    let mut gate = state.gate.lock().map_err(|_| ApiError::internal())?;
    gate.enabled = false;
    while gate.in_flight != 0 {
        gate = state.drained.wait(gate).map_err(|_| ApiError::internal())?;
    }
    drop(gate);
    registry()?
        .subscriptions
        .remove(&handle.addr())
        .ok_or_else(ApiError::handle)?;
    Ok(())
}

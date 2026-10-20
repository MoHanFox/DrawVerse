//! Ordered continuation and mailbox credit. No pixel or pressure algorithms live here.
use super::*;
#[derive(Clone, Copy)]
pub(super) struct Active {
    pub(super) sequence: u64,
    pub(super) finish: bool,
}
impl Engine {
    pub(super) fn receive(&mut self, command: Command) {
        if self.active_input.is_none() && self.deferred.is_empty() {
            self.execute(command);
            return;
        }
        let cancel = matches!(command.operation, Operation::Cancel(_))
            && !self.active_input.is_some_and(|a| a.finish)
            && self.document.stroke_active()
            && self
                .deferred
                .iter()
                .all(|c| matches!(c.operation, Operation::Move(_)));
        let reset = matches!(
            command.operation,
            Operation::New(..) | Operation::NewWhite(..)
        ) && self.deferred.iter().all(|c| {
            matches!(
                c.operation,
                Operation::Move(_)
                    | Operation::End
                    | Operation::Begin(..)
                    | Operation::BeginPrepared(..)
            )
        });
        if cancel || reset {
            self.discard_active_input();
            self.deferred.clear();
            self.execute(command);
        } else {
            // The serial mailbox charges this retained command against its original capacity.
            self.deferred.push_back(command);
        }
    }
    fn discard_active_input(&mut self) {
        if self.document.stroke_active() {
            let _ = self.document.cancel_stroke();
        }
        self.wet_lease.take();
        self.active_input = None;
        self.aborted = true;
        self.floor = self.document.revision();
        self.shared.floor.store(self.floor, Ordering::Release);
    }
    pub(super) fn resume_input(&mut self) {
        // A previously deferred cancel may now refer to the current stroke rather than an
        // earlier completed one. Only its preceding Move prefix may be discarded.
        if self.document.stroke_active() && !self.active_input.is_some_and(|a| a.finish) {
            if let Some(index) = self
                .deferred
                .iter()
                .position(|c| !matches!(c.operation, Operation::Move(_)))
            {
                if matches!(self.deferred[index].operation, Operation::Cancel(_)) {
                    let command = self.deferred.remove(index).expect("cancel position");
                    self.deferred.drain(..index);
                    self.discard_active_input();
                    self.execute(command);
                    return;
                }
            }
        }
        let Some(active) = self.active_input else {
            if let Some(command) = self.deferred.pop_front() {
                self.execute(command);
            }
            return;
        };
        match self
            .document
            .continue_stroke_page(paint_core::InputScheduleBudget::default())
        {
            Ok(paint_core::StrokePageResult::Pending) => {}
            Ok(paint_core::StrokePageResult::InputComplete) => {
                debug_assert!(!active.finish);
                self.active_input = None;
                self.completed = active.sequence;
            }
            Ok(paint_core::StrokePageResult::StrokeComplete { changed, wet_ink }) => {
                debug_assert!(active.finish);
                if let Some(ink) = wet_ink {
                    let lease = self
                        .wet_lease
                        .take()
                        .expect("wet stroke retains its lease until history succeeds");
                    if ink.secondary().is_some() {
                        lease.commit_ink(ink);
                    } else {
                        lease.commit(ink.primary().clone());
                    }
                } else {
                    debug_assert!(self.wet_lease.is_none());
                }
                self.modified |= changed;
                if changed {
                    let committed = Arc::make_mut(&mut self.committed_strokes);
                    if committed.len() == 4096 {
                        committed.pop_front();
                    }
                    committed.push_back(active.sequence);
                }
                self.active_input = None;
                self.completed = active.sequence;
            }
            Err(error) => {
                self.command_failed(active.sequence, error.into(), true);
                self.active_input = None;
                self.completed = active.sequence;
            }
        }
        self.dirty = true;
    }
    pub(super) fn command_failed(&mut self, sequence: u64, error: ApiError, stroke: bool) {
        if error.status != PAINT_CANCELLED || self.error.is_none() {
            self.error = Some((sequence, error));
        }
        if stroke && self.document.stroke_active() {
            let _ = self.document.cancel_stroke();
        }
        if stroke {
            self.wet_lease.take();
            self.aborted = true;
            self.floor = self.document.revision();
            self.shared.floor.store(self.floor, Ordering::Release);
        }
    }
}

#[cfg(test)]
#[path = "session_input_paging_tests.rs"]
mod tests;

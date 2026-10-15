//! Shared tool ink; document history and preset state never own this reservoir.
use crate::{Error, Pixel, Result, WetCarrier, WetMix};
use std::sync::{Arc, Mutex, MutexGuard};
#[derive(Clone, Default)]
pub struct WetTool {
    state: Arc<Mutex<State>>,
}
#[derive(Default)]
struct State {
    carrier: Option<WetCarrier>,
    leased: bool,
    serial: u64,
}
/// Exclusive, non-cloneable reservation. Dropping cancels without adopting ink.
pub struct WetLease {
    tool: WetTool,
    retained: Option<WetCarrier>,
    serial: u64,
}
impl WetTool {
    // No callbacks or document mutations run under this lock. All fallible work
    // precedes state replacement, so recovering a poisoned guard preserves ink.
    fn lock(&self) -> MutexGuard<'_, State> {
        self.state.lock().unwrap_or_else(|e| e.into_inner())
    }
    pub fn is_busy(&self) -> bool {
        self.lock().leased
    }
    pub fn snapshot(&self) -> Result<Option<WetCarrier>> {
        let state = self.lock();
        if state.leased {
            return Err(Error::Busy);
        }
        Ok(state.carrier.clone())
    }
    pub fn lease(&self) -> Result<WetLease> {
        let mut state = self.lock();
        if state.leased {
            return Err(Error::Busy);
        }
        let serial = state
            .serial
            .checked_add(1)
            .ok_or(Error::ResourceLimit("wet tool lease serial exhausted"))?;
        let retained = state.carrier.clone();
        state.serial = serial;
        state.leased = true;
        Ok(WetLease {
            tool: self.clone(),
            retained,
            serial,
        })
    }
    pub fn clean(&self) -> Result<()> {
        let mut state = self.lock();
        if state.leased {
            return Err(Error::Busy);
        }
        if let Some(previous) = &state.carrier {
            let mut next = previous.clone();
            next.clean()?;
            state.carrier = Some(next);
        }
        Ok(())
    }
    pub fn reload(&self, settings: WetMix, diameter: f32, color: Pixel) -> Result<()> {
        let mut state = self.lock();
        if state.leased {
            return Err(Error::Busy);
        }
        let mut next = if let Some(previous) = &state.carrier {
            previous.reconfigured(settings, diameter)?
        } else {
            WetCarrier::new(settings, diameter, Pixel::TRANSPARENT)?
        };
        next.reload(color)?;
        state.carrier = Some(next);
        Ok(())
    }
}
impl WetLease {
    pub fn retained(&self) -> Option<&WetCarrier> {
        self.retained.as_ref()
    }
    /// Infallible adoption after the document/history transaction has succeeded.
    /// Only this non-cloneable reservation can release its serial while alive.
    pub fn commit(self, carrier: WetCarrier) {
        let mut state = self.tool.lock();
        assert!(
            state.leased && state.serial == self.serial,
            "exclusive wet lease invariant"
        );
        state.carrier = Some(carrier);
        state.leased = false;
    }
}
impl Drop for WetLease {
    fn drop(&mut self) {
        let mut state = self.tool.lock();
        if state.serial == self.serial {
            state.leased = false;
        }
    }
}

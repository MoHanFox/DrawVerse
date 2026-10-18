//! Shared tool ink; document history and preset state never own this reservoir.
use crate::{Error, Pixel, Result, WetCarrier, WetInk, WetMix, WetSettings};
use std::sync::{Arc, Mutex, MutexGuard};
#[derive(Clone, Default)]
pub struct WetTool {
    state: Arc<Mutex<State>>,
}
#[derive(Default)]
struct State {
    ink: Option<WetInk>,
    leased: bool,
    serial: u64,
}
/// Exclusive, non-cloneable reservation. Dropping cancels without adopting ink.
pub struct WetLease {
    tool: WetTool,
    retained: Option<WetInk>,
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
        Ok(self.snapshot_ink()?.map(|ink| ink.primary))
    }
    pub fn snapshot_ink(&self) -> Result<Option<WetInk>> {
        let state = self.lock();
        if state.leased {
            return Err(Error::Busy);
        }
        Ok(state.ink.clone())
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
        let retained = state.ink.clone();
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
        if let Some(previous) = &state.ink {
            let mut next = previous.clone();
            next.clean()?;
            state.ink = Some(next);
        }
        Ok(())
    }
    pub fn reload(&self, settings: WetMix, diameter: f32, color: Pixel) -> Result<()> {
        let mut state = self.lock();
        if state.leased {
            return Err(Error::Busy);
        }
        let mut next = if let Some(previous) = &state.ink {
            previous.primary.reconfigured(settings, diameter)?
        } else {
            WetCarrier::new(settings, diameter, Pixel::TRANSPARENT)?
        };
        next.reload(color)?;
        let secondary = state.ink.as_ref().and_then(|ink| ink.secondary.clone());
        state.ink = Some(WetInk::new(next, secondary));
        Ok(())
    }
    pub fn reload_dual(
        &self,
        settings: WetMix,
        diameters: [f32; 2],
        colors: [Pixel; 2],
    ) -> Result<()> {
        let mut state = self.lock();
        if state.leased {
            return Err(Error::Busy);
        }
        let policy = WetSettings::new(true, false, false, true, settings);
        let next = WetInk::for_stroke(
            policy,
            diameters[0],
            colors[0],
            Some((diameters[1], colors[1])),
            state.ink.as_ref(),
        )?;
        state.ink = Some(next);
        Ok(())
    }
}
impl WetLease {
    pub fn retained(&self) -> Option<&WetCarrier> {
        self.retained.as_ref().map(WetInk::primary)
    }
    pub fn retained_ink(&self) -> Option<&WetInk> {
        self.retained.as_ref()
    }
    /// Infallible adoption after the document/history transaction has succeeded.
    /// Only this non-cloneable reservation can release its serial while alive.
    pub fn commit(self, carrier: WetCarrier) {
        let secondary = self.retained.as_ref().and_then(|ink| ink.secondary.clone());
        self.commit_ink(WetInk::new(carrier, secondary));
    }
    pub fn commit_ink(self, ink: WetInk) {
        let mut state = self.tool.lock();
        assert!(
            state.leased && state.serial == self.serial,
            "exclusive wet lease invariant"
        );
        state.ink = Some(ink);
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

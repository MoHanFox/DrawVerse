//! Role-preserving reservoirs. This is transient tool ink, never a preset.
use crate::{Pixel, Result, WetCarrier, WetSettings};

#[derive(Clone, Debug)]
pub struct WetInk {
    pub(crate) primary: WetCarrier,
    pub(crate) secondary: Option<WetCarrier>,
}
impl WetInk {
    pub fn new(primary: WetCarrier, secondary: Option<WetCarrier>) -> Self {
        Self { primary, secondary }
    }
    pub fn primary(&self) -> &WetCarrier {
        &self.primary
    }
    pub fn primary_mut(&mut self) -> &mut WetCarrier {
        &mut self.primary
    }
    pub fn secondary(&self) -> Option<&WetCarrier> {
        self.secondary.as_ref()
    }
    pub fn secondary_mut(&mut self) -> Option<&mut WetCarrier> {
        self.secondary.as_mut()
    }

    /// Prepare both roles privately. A bad second configuration never publishes
    /// primary cleaning, reloading, revision changes, or resampling.
    pub fn for_stroke(
        settings: WetSettings,
        diameter: f32,
        foreground: Pixel,
        secondary: Option<(f32, Pixel)>,
        retained: Option<&Self>,
    ) -> Result<Self> {
        let configure = |previous: Option<&WetCarrier>, diameter, color| -> Result<WetCarrier> {
            let mut next = if let Some(previous) = previous {
                previous.reconfigured(settings.mix(), diameter)?
            } else {
                WetCarrier::new(settings.mix(), diameter, Pixel::TRANSPARENT)?
            };
            if settings.auto_clean() {
                next.clean()?;
            }
            if settings.auto_load() {
                next.reload(color)?;
            }
            Ok(next)
        };
        let primary = configure(retained.map(Self::primary), diameter, foreground)?;
        let secondary = secondary
            .map(|(diameter, color)| configure(retained.and_then(Self::secondary), diameter, color))
            .transpose()?;
        Ok(Self::new(primary, secondary))
    }
    pub fn clean(&mut self) -> Result<()> {
        let mut next = self.clone();
        next.primary.clean()?;
        if let Some(secondary) = &mut next.secondary {
            secondary.clean()?;
        }
        *self = next;
        Ok(())
    }
}

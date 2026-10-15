//! Portable preset policy; the actual reservoir remains tool-owned state.
use crate::WetMix;
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct WetSettings {
    enabled: bool,
    all_layers: bool,
    auto_clean: bool,
    auto_load: bool,
    mix: WetMix,
}
impl Default for WetSettings {
    fn default() -> Self {
        Self::new(false, false, true, true, WetMix::default())
    }
}
impl WetSettings {
    pub fn new(
        enabled: bool,
        all_layers: bool,
        auto_clean: bool,
        auto_load: bool,
        mix: WetMix,
    ) -> Self {
        Self {
            enabled,
            all_layers,
            auto_clean,
            auto_load,
            mix,
        }
    }
    pub fn enabled(self) -> bool {
        self.enabled
    }
    pub fn all_layers(self) -> bool {
        self.all_layers
    }
    pub fn auto_clean(self) -> bool {
        self.auto_clean
    }
    pub fn auto_load(self) -> bool {
        self.auto_load
    }
    pub fn mix(self) -> WetMix {
        self.mix
    }
    pub fn is_default(self) -> bool {
        self == Self::default()
    }
}

//! Dual-color composition models. Pixel fields remain paged by Document.
use crate::{BlendMode, Error, Pixel, Result};
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum DualColorModel {
    /// Accumulate both colors for the entire stroke, then compose once.
    #[default]
    SeparatedStroke,
    /// Compose each primary/secondary stamp pair before the next pair.
    InterleavedStamp,
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum DualColorOrder {
    #[default]
    SecondBehind,
    SecondInFront,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DualColor {
    model: DualColorModel,
    enabled: bool,
    use_background: bool,
    overflow: bool,
    order: DualColorOrder,
    primary_blend: BlendMode,
    secondary_blend: BlendMode,
}
impl Default for DualColor {
    fn default() -> Self {
        Self {
            model: DualColorModel::default(),
            enabled: false,
            use_background: false,
            overflow: false,
            order: DualColorOrder::default(),
            primary_blend: BlendMode::Normal,
            secondary_blend: BlendMode::Normal,
        }
    }
}
impl DualColor {
    pub fn new(
        enabled: bool,
        use_background: bool,
        overflow: bool,
        order: DualColorOrder,
        primary_blend: BlendMode,
        secondary_blend: BlendMode,
    ) -> Result<Self> {
        Self::new_with_model(
            DualColorModel::SeparatedStroke,
            enabled,
            use_background,
            overflow,
            order,
            primary_blend,
            secondary_blend,
        )
    }
    #[allow(clippy::too_many_arguments)]
    pub fn new_with_model(
        model: DualColorModel,
        enabled: bool,
        use_background: bool,
        overflow: bool,
        order: DualColorOrder,
        primary_blend: BlendMode,
        secondary_blend: BlendMode,
    ) -> Result<Self> {
        if primary_blend == BlendMode::PassThrough || secondary_blend == BlendMode::PassThrough {
            return Err(Error::InvalidArgument(
                "pass-through is not a dual-color contribution mode",
            ));
        }
        Ok(Self {
            model,
            enabled,
            use_background,
            overflow,
            order,
            primary_blend,
            secondary_blend,
        })
    }
    pub fn model(self) -> DualColorModel {
        self.model
    }
    pub fn with_model(mut self, model: DualColorModel) -> Self {
        self.model = model;
        self
    }
    pub fn enabled(self) -> bool {
        self.enabled
    }
    pub fn use_background(self) -> bool {
        self.use_background
    }
    pub fn overflow(self) -> bool {
        self.overflow
    }
    pub fn order(self) -> DualColorOrder {
        self.order
    }
    pub fn primary_blend(self) -> BlendMode {
        self.primary_blend
    }
    pub fn secondary_blend(self) -> BlendMode {
        self.secondary_blend
    }
    pub(crate) fn random_active(self) -> bool {
        self.enabled
            && (self.primary_blend == BlendMode::Dissolve
                || self.secondary_blend == BlendMode::Dissolve)
    }
    pub(crate) fn compose(
        self,
        primary: Pixel,
        primary_coverage: f32,
        secondary_color: Pixel,
        secondary_coverage: f32,
        position: (u32, u32),
        seed: u64,
    ) -> Pixel {
        let coverage = if self.overflow {
            secondary_coverage
        } else {
            secondary_coverage.min(primary_coverage)
        };
        let secondary = secondary_color.scaled(coverage);
        self.compose_fields(primary, secondary, position, seed)
    }
    pub(crate) fn compose_wet(
        self,
        primary: Pixel,
        primary_coverage: f32,
        secondary: Pixel,
        secondary_coverage: f32,
        position: (u32, u32),
        seed: u64,
    ) -> Pixel {
        let coverage = if self.overflow {
            secondary_coverage
        } else {
            secondary_coverage.min(primary_coverage)
        };
        let secondary = secondary.scaled(if secondary_coverage > 0. {
            coverage / secondary_coverage
        } else {
            0.
        });
        self.compose_fields(primary, secondary, position, seed)
    }
    fn compose_fields(
        self,
        primary: Pixel,
        secondary: Pixel,
        position: (u32, u32),
        seed: u64,
    ) -> Pixel {
        let (x, y) = position;
        let main = |dst| {
            crate::blend_pixel(
                primary,
                dst,
                self.primary_blend,
                x,
                y,
                seed ^ 0x6475616c2d6d6169,
            )
        };
        let second = |dst| {
            crate::blend_pixel(
                secondary,
                dst,
                self.secondary_blend,
                x,
                y,
                seed ^ 0x6475616c2d736563,
            )
        };
        match self.order {
            DualColorOrder::SecondBehind => main(second(Pixel::TRANSPARENT)),
            DualColorOrder::SecondInFront => second(main(Pixel::TRANSPARENT)),
        }
    }
}

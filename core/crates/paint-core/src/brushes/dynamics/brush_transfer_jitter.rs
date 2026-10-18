use crate::{
    brush_random::{mix, unit},
    Error, Result,
};

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TransferSample {
    pub size: f32,
    pub flow: f32,
    pub opacity: f32,
}
impl TransferSample {
    pub const IDENTITY: Self = Self {
        size: 1.,
        flow: 1.,
        opacity: 1.,
    };
}
impl Default for TransferSample {
    fn default() -> Self {
        Self::IDENTITY
    }
}

/// Deterministic per-stamp jitter for the transfer channels. The stroke serial
/// is supplied by the document; no global random state or ABR seed is used.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct TransferJitter {
    size: f32,
    flow: f32,
    opacity: f32,
}
impl TransferJitter {
    pub fn new(size: f32, flow: f32, opacity: f32) -> Result<Self> {
        if [size, flow, opacity]
            .iter()
            .any(|value| !value.is_finite() || !(0. ..=1.).contains(value))
        {
            return Err(Error::InvalidArgument("transfer jitter bounds"));
        }
        Ok(Self {
            size,
            flow,
            opacity,
        })
    }
    pub fn size(self) -> f32 {
        self.size
    }
    pub fn flow(self) -> f32 {
        self.flow
    }
    pub fn opacity(self) -> f32 {
        self.opacity
    }
    pub fn is_default(self) -> bool {
        self == Self::default()
    }
    pub(crate) fn active(self) -> bool {
        self.size != 0. || self.flow != 0. || self.opacity != 0.
    }
    pub(crate) fn sample(self, stroke: u64, index: u64, child: u32) -> TransferSample {
        if !self.active() {
            return TransferSample::IDENTITY;
        }
        let key = mix(stroke ^ mix(index.wrapping_add(0x9e3779b97f4a7c15)) ^ 0x7472616e73666572);
        let factor = |amount: f32, lane| 1. - amount * unit(key, child, lane) as f32;
        TransferSample {
            size: factor(self.size, 0),
            flow: factor(self.flow, 1),
            opacity: factor(self.opacity, 2),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn transfer_jitter_is_bounded_deterministic_and_channel_specific() {
        let jitter = TransferJitter::new(0.75, 0.5, 0.25).unwrap();
        let first = jitter.sample(7, 3, 0);
        assert_eq!(first, jitter.sample(7, 3, 0));
        assert_ne!(first, jitter.sample(7, 4, 0));
        assert_ne!(first.size, first.flow);
        for index in 0..1000 {
            let sample = jitter.sample(9, index, 0);
            assert!((0.25..=1.).contains(&sample.size));
            assert!((0.5..=1.).contains(&sample.flow));
            assert!((0.75..=1.).contains(&sample.opacity));
        }
        assert_eq!(
            TransferJitter::default().sample(1, 1, 1),
            TransferSample::IDENTITY
        );
    }
}

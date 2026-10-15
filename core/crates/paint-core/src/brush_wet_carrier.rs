//! Bounded linear-premultiplied two-dimensional brush reservoir.
//! Preparing is read-only; the document commits only after its pixel transaction succeeds.
use crate::{Error, Pixel, Result};
use std::sync::Arc;

pub const MAX_WET_CARRIER_EDGE: usize = 64;
pub const WET_CARRIER_MODEL: &str = "linear-premul-grid-v1";

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct WetMix {
    wetness: f32,
    pickup: f32,
    mix: f32,
    load: f32,
    deposit: f32,
    consumption: f32,
}
impl Default for WetMix {
    fn default() -> Self {
        Self {
            wetness: 0.5,
            pickup: 0.5,
            mix: 0.5,
            load: 1.,
            deposit: 1.,
            consumption: 0.1,
        }
    }
}
impl WetMix {
    pub fn new(
        wetness: f32,
        pickup: f32,
        mix: f32,
        load: f32,
        deposit: f32,
        consumption: f32,
    ) -> Result<Self> {
        if [wetness, pickup, mix, load, deposit, consumption]
            .iter()
            .any(|v| !v.is_finite() || !(0. ..=1.).contains(v))
        {
            return Err(Error::InvalidArgument("wet mix parameter"));
        }
        Ok(Self {
            wetness,
            pickup,
            mix,
            load,
            deposit,
            consumption,
        })
    }
    pub fn wetness(self) -> f32 {
        self.wetness
    }
    pub fn pickup(self) -> f32 {
        self.pickup
    }
    pub fn mix(self) -> f32 {
        self.mix
    }
    pub fn load(self) -> f32 {
        self.load
    }
    pub fn deposit(self) -> f32 {
        self.deposit
    }
    pub fn consumption(self) -> f32 {
        self.consumption
    }
}

#[derive(Clone, Copy, Debug)]
pub struct WetSample {
    pub color: Pixel,
    pub pickup_weight: f32,
}
#[derive(Clone, Copy, Debug)]
pub struct WetDabGeometry {
    pub x: f64,
    pub y: f64,
    pub diameter: f32,
    pub roundness: f32,
    pub angle: f32,
}
impl WetDabGeometry {
    fn validate(self) -> Result<()> {
        if !self.x.is_finite()
            || !self.y.is_finite()
            || !self.diameter.is_finite()
            || !(1. ..=512.).contains(&self.diameter)
            || !self.roundness.is_finite()
            || !(0.01..=1.).contains(&self.roundness)
            || !self.angle.is_finite()
        {
            return Err(Error::InvalidArgument("wet dab geometry"));
        }
        Ok(())
    }
}
#[derive(Clone, Debug)]
pub struct WetCarrier {
    settings: WetMix,
    edge: usize,
    cells: Arc<[Pixel]>,
    last: Option<(f64, f64)>,
    revision: u64,
}
#[derive(Debug)]
pub struct WetDab {
    base: Arc<[Pixel]>,
    revision: u64,
    edge: usize,
    next: Arc<[Pixel]>,
    output: Arc<[Pixel]>,
    last: (f64, f64),
}
impl WetCarrier {
    pub fn new(settings: WetMix, diameter: f32, color: Pixel) -> Result<Self> {
        if !diameter.is_finite() || !(1. ..=512.).contains(&diameter) {
            return Err(Error::InvalidArgument("wet carrier diameter"));
        }
        let edge = (diameter.ceil() as usize).clamp(2, MAX_WET_CARRIER_EDGE);
        Ok(Self {
            settings,
            edge,
            cells: vec![color.scaled(settings.load); edge * edge].into(),
            last: None,
            revision: 0,
        })
    }
    pub fn settings(&self) -> WetMix {
        self.settings
    }
    pub fn edge(&self) -> usize {
        self.edge
    }
    pub fn revision(&self) -> u64 {
        self.revision
    }
    pub fn sample(&self, u: f64, v: f64) -> Pixel {
        sample(&self.cells, self.edge, u, v)
    }
    pub fn clean(&mut self) -> Result<()> {
        self.replace(Pixel::TRANSPARENT)
    }
    pub fn reload(&mut self, color: Pixel) -> Result<()> {
        self.replace(color.scaled(self.settings.load))
    }
    fn replace(&mut self, color: Pixel) -> Result<()> {
        let revision = self.next_revision()?;
        self.cells = vec![color; self.edge * self.edge].into();
        self.last = None;
        self.revision = revision;
        Ok(())
    }
    pub fn reset_path(&mut self) -> Result<()> {
        let revision = self.next_revision()?;
        self.last = None;
        self.revision = revision;
        Ok(())
    }
    fn next_revision(&self) -> Result<u64> {
        self.revision
            .checked_add(1)
            .ok_or(Error::ResourceLimit("wet carrier revision exhausted"))
    }
    pub fn prepare_dab<F>(&self, geometry: WetDabGeometry, mut read: F) -> Result<WetDab>
    where
        F: FnMut(f64, f64) -> Result<WetSample>,
    {
        geometry.validate()?;
        self.next_revision()?;
        let distance = self
            .last
            .map_or(0., |(x, y)| (geometry.x - x).hypot(geometry.y - y));
        if !distance.is_finite() {
            return Err(Error::InvalidArgument("wet carrier distance"));
        }
        let decay = if distance == 0. || self.settings.consumption == 0. {
            1.
        } else {
            (1. - f64::from(self.settings.consumption))
                .powf(distance / f64::from(geometry.diameter)) as f32
        };
        let needs_read =
            self.settings.wetness > 0. && (self.settings.pickup > 0. || self.settings.mix > 0.);
        let mut source = Vec::with_capacity(if needs_read { self.edge * self.edge } else { 0 });
        if needs_read {
            let (sine, cosine) = f64::from(geometry.angle).to_radians().sin_cos();
            for row in 0..self.edge {
                for col in 0..self.edge {
                    let x = ((col as f64 + 0.5) / self.edge as f64 - 0.5)
                        * f64::from(geometry.diameter);
                    let y = ((row as f64 + 0.5) / self.edge as f64 - 0.5)
                        * f64::from(geometry.diameter)
                        * f64::from(geometry.roundness);
                    let value = read(
                        geometry.x + x * cosine - y * sine,
                        geometry.y + x * sine + y * cosine,
                    )?;
                    if !value.pickup_weight.is_finite()
                        || !(0. ..=1.).contains(&value.pickup_weight)
                    {
                        return Err(Error::InvalidArgument("wet pickup weight"));
                    }
                    source.push(value);
                }
            }
        }
        let mut next = Vec::with_capacity(self.cells.len());
        let mut output = Vec::with_capacity(self.cells.len());
        for (i, cell) in self.cells.iter().enumerate() {
            let carry = cell.scaled(decay);
            let (value, deposit) = if needs_read {
                let picked = source[i];
                let wet = self.settings.wetness * picked.pickup_weight;
                let value = carry.mix(picked.color, wet * self.settings.pickup);
                (value, value.mix(picked.color, wet * self.settings.mix))
            } else {
                (carry, carry)
            };
            next.push(value);
            output.push(deposit.scaled(self.settings.deposit));
        }
        drop(source);
        Ok(WetDab {
            base: self.cells.clone(),
            revision: self.revision,
            edge: self.edge,
            next: next.into(),
            output: output.into(),
            last: (geometry.x, geometry.y),
        })
    }
    pub fn commit(&mut self, dab: WetDab) -> Result<()> {
        if self.revision != dab.revision
            || self.edge != dab.edge
            || !Arc::ptr_eq(&self.cells, &dab.base)
        {
            return Err(Error::InvalidArgument("stale or foreign wet dab"));
        }
        let revision = self.next_revision()?;
        self.cells = dab.next;
        self.last = Some(dab.last);
        self.revision = revision;
        Ok(())
    }
}
impl WetDab {
    pub fn sample(&self, u: f64, v: f64) -> Pixel {
        sample(&self.output, self.edge, u, v)
    }
    pub fn edge(&self) -> usize {
        self.edge
    }
}
fn sample(cells: &[Pixel], edge: usize, u: f64, v: f64) -> Pixel {
    if !u.is_finite() || !v.is_finite() || !(0. ..=1.).contains(&u) || !(0. ..=1.).contains(&v) {
        return Pixel::TRANSPARENT;
    }
    let x = (u * edge as f64 - 0.5).clamp(0., (edge - 1) as f64);
    let y = (v * edge as f64 - 0.5).clamp(0., (edge - 1) as f64);
    let x0 = x.floor() as usize;
    let y0 = y.floor() as usize;
    let x1 = (x0 + 1).min(edge - 1);
    let y1 = (y0 + 1).min(edge - 1);
    cells[y0 * edge + x0]
        .mix(cells[y0 * edge + x1], (x - x0 as f64) as f32)
        .mix(
            cells[y1 * edge + x0].mix(cells[y1 * edge + x1], (x - x0 as f64) as f32),
            (y - y0 as f64) as f32,
        )
}

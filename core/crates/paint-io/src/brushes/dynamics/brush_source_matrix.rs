use paint_core::{DynamicsSource, DynamicsSourceMatrix};
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
enum Source {
    All,
    Speed,
    Tangent,
    Distance,
    Tilt,
    Rotation,
    Direction,
}
impl From<Source> for DynamicsSource {
    fn from(value: Source) -> Self {
        match value {
            Source::All => Self::All,
            Source::Speed => Self::Speed,
            Source::Tangent => Self::Tangent,
            Source::Distance => Self::Distance,
            Source::Tilt => Self::Tilt,
            Source::Rotation => Self::Rotation,
            Source::Direction => Self::Direction,
        }
    }
}
impl From<DynamicsSource> for Source {
    fn from(value: DynamicsSource) -> Self {
        match value {
            DynamicsSource::All => Self::All,
            DynamicsSource::Speed => Self::Speed,
            DynamicsSource::Tangent => Self::Tangent,
            DynamicsSource::Distance => Self::Distance,
            DynamicsSource::Tilt => Self::Tilt,
            DynamicsSource::Rotation => Self::Rotation,
            DynamicsSource::Direction => Self::Direction,
        }
    }
}
#[derive(Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Data {
    model: String,
    size: Source,
    flow: Source,
    opacity: Source,
}
#[derive(Clone, Debug)]
pub struct SourceMatrixParameters {
    core: DynamicsSourceMatrix,
}
impl SourceMatrixParameters {
    pub fn from_core(core: DynamicsSourceMatrix) -> Self {
        Self { core }
    }
    pub fn core(&self) -> DynamicsSourceMatrix {
        self.core
    }
}
pub fn present<'de, D: serde::Deserializer<'de>>(
    d: D,
) -> Result<Option<SourceMatrixParameters>, D::Error> {
    SourceMatrixParameters::deserialize(d).map(Some)
}
impl Serialize for SourceMatrixParameters {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        Data {
            model: "source-matrix-v1".into(),
            size: self.core.size().into(),
            flow: self.core.flow().into(),
            opacity: self.core.opacity().into(),
        }
        .serialize(s)
    }
}
impl<'de> Deserialize<'de> for SourceMatrixParameters {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let data = Data::deserialize(d)?;
        if data.model != "source-matrix-v1" {
            return Err(serde::de::Error::custom("dynamics source matrix model"));
        }
        Ok(Self {
            core: DynamicsSourceMatrix::new(
                data.size.into(),
                data.flow.into(),
                data.opacity.into(),
            ),
        })
    }
}

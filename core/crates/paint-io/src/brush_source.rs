//! Non-executable, content-addressed import provenance. No paths from metadata are opened.
use crate::{
    abr_conversion::{ConvertedPreset, Status},
    brush_resource::{encode_mask, hash, valid_id},
    check,
    vbr::Parameters,
    Error, Result,
};
use paint_core::BrushProgram;
use paint_task::CancellationToken;
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs::File,
    io::{Read, Write},
    path::PathBuf,
    sync::{Arc, Mutex, Weak},
};
const MAX_ORIGINAL: usize = 128 * 1024 * 1024;
const MAX_RECORD: usize = 2 * 1024 * 1024;
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum Support {
    Mapped,
    Approximate,
    NotExecuted,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ReportItem {
    pub path: String,
    pub status: Support,
    pub reason: String,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SourceInfo {
    pub version: u32,
    pub format: String,
    pub original_hash: String,
    pub file_version: u16,
    pub file_subversion: Option<u16>,
    pub preset_index: u32,
    pub original_name: String,
    pub converter: String,
    pub converter_version: u32,
    pub baseline: Parameters,
    pub tip_resource: Option<String>,
    pub report: Vec<ReportItem>,
}
impl SourceInfo {
    pub fn validate(&self) -> Result<()> {
        if self.version != 1
            || self.format != "abr"
            || self.converter != "drawverse.abr.basic"
            || !matches!(self.converter_version, 1..=3)
        {
            return Err(Error::Unsupported(
                "brush source format/converter version".into(),
            ));
        }
        if !valid_id(&self.original_hash)
            || self.tip_resource.as_ref().is_some_and(|id| !valid_id(id))
            || self.preset_index >= 1024
            || self.original_name.trim().is_empty()
            || self.original_name.chars().count() > 80
            || self.original_name.contains('\0')
        {
            return Err(Error::Invalid("brush source identity/name"));
        }
        match (self.file_version, self.file_subversion) {
            (1 | 2, None) | (6 | 10, Some(1 | 2)) => {}
            _ => return Err(Error::Unsupported("brush source ABR version".into())),
        }
        if self.report.len() > 4096
            || self.report.iter().map(|i| i.path.len()).sum::<usize>() > 1024 * 1024
            || self.report.iter().any(|i| {
                i.path.is_empty()
                    || i.path.contains('\0')
                    || i.reason.is_empty()
                    || i.reason.len() > 1024
                    || i.reason.contains('\0')
            })
        {
            return Err(Error::Limit("brush source report bounds"));
        }
        if (self.converter_version == 3 && self.baseline.pressure_dynamics.is_none())
            || (self.converter_version < 3 && self.baseline.pressure_dynamics.is_some())
            || self.baseline.tip_dynamics.is_some()
            || self.baseline.scatter_dynamics.is_some()
            || self.baseline.shape_jitter.is_some()
            || self.baseline.color_dynamics.is_some()
        {
            return Err(Error::Unsupported(
                "converter v1 baseline dynamic controls".into(),
            ));
        }
        BrushProgram::compile(self.baseline.definition())?;
        Ok(())
    }
}
pub struct SourceArchive {
    pub info: SourceInfo,
    original: Arc<Vec<u8>>,
}
impl SourceArchive {
    pub fn original(&self) -> &[u8] {
        &self.original
    }
    pub fn capture(
        original: Arc<Vec<u8>>,
        preset_index: u32,
        preset: &ConvertedPreset<'_>,
    ) -> Result<Self> {
        let version = header(&original)?;
        let tip_resource = preset
            .settings
            .mask
            .as_ref()
            .map(|mask| encode_mask(mask).map(|png| hash(&png)))
            .transpose()?;
        let info =
            Self::capture_info(hash(&original), version, preset_index, preset, tip_resource)?;
        Ok(Self { info, original })
    }
    fn capture_info(
        original_hash: String,
        version: (u16, Option<u16>),
        preset_index: u32,
        preset: &ConvertedPreset<'_>,
        tip_resource: Option<String>,
    ) -> Result<SourceInfo> {
        let settings = &preset.settings;
        if !settings.wet.is_default() {
            return Err(Error::Unsupported(
                "wet source baseline is not represented".into(),
            ));
        }
        if !settings.smoothing.is_default() {
            return Err(Error::Unsupported(
                "smoothing source baseline is not supported".into(),
            ));
        }
        if !settings.dual.is_default() {
            return Err(Error::Unsupported(
                "source dual brush is not represented by converter v1".into(),
            ));
        }
        let mut non_pressure = settings.dynamics.clone();
        non_pressure.size = Default::default();
        non_pressure.flow = Default::default();
        non_pressure.opacity = Default::default();
        non_pressure.pressure_opacity = false;
        if non_pressure != paint_core::BrushDynamics::default()
            || (preset.converter_version < 3
                && settings.dynamics != paint_core::BrushDynamics::default())
        {
            return Err(Error::Unsupported(
                "source pressure dynamics are not represented by converter v1".into(),
            ));
        }
        let info = SourceInfo {
            version: 1,
            format: "abr".into(),
            original_hash,
            file_version: version.0,
            file_subversion: version.1,
            preset_index,
            original_name: preset.name.into(),
            converter: "drawverse.abr.basic".into(),
            converter_version: preset.converter_version,
            baseline: Parameters {
                scatter_dynamics: None,
                shape_jitter: None,
                color_dynamics: None,
                tip_dynamics: None,
                pressure_dynamics: crate::abr_conversion::pressure_parameters(
                    settings,
                    preset.converter_version,
                ),
                diameter: settings.diameter,
                spacing: settings.spacing,
                dab_opacity: settings.opacity,
                hardness: settings.tip.hardness,
                roundness: settings.tip.roundness,
                angle: settings.tip.angle,
                flow: settings.tip.flow,
                pressure_size: settings.tip.pressure_size,
                pressure_flow: settings.tip.pressure_flow,
            },
            tip_resource,
            report: preset
                .issues
                .iter()
                .map(|i| ReportItem {
                    path: i.path.clone(),
                    status: match i.status {
                        Status::Mapped => Support::Mapped,
                        Status::Approximate => Support::Approximate,
                        Status::NotExecuted => Support::NotExecuted,
                    },
                    reason: i.reason.into(),
                })
                .collect(),
        };
        info.validate()?;
        Ok(info)
    }
    pub fn record(&self) -> Result<Vec<u8>> {
        self.info.validate()?;
        if header(&self.original)? != (self.info.file_version, self.info.file_subversion)
            || hash(&self.original) != self.info.original_hash
        {
            return Err(Error::Invalid("brush source original identity/version"));
        }
        let record = serde_json::to_vec(&self.info).map_err(|e| Error::Codec(e.to_string()))?;
        if record.len() > MAX_RECORD {
            return Err(Error::Limit("brush source record bytes"));
        }
        Ok(record)
    }
}
/// Fully validated batch, sharing original bytes and deduplicating tip encoding.
/// Private records cannot be changed after capture; not a filesystem transaction.
pub struct SourceBatch {
    original: Arc<Vec<u8>>,
    original_id: String,
    records: Vec<(String, Vec<u8>)>,
    tip_ids: BTreeSet<String>,
    converter_version: u32,
}
impl SourceBatch {
    pub fn capture(
        original: Arc<Vec<u8>>,
        presets: &[ConvertedPreset<'_>],
        token: &CancellationToken,
    ) -> Result<Self> {
        check(token)?;
        if presets.is_empty() || presets.len() > 1024 {
            return Err(Error::Limit("brush source batch count"));
        }
        let version = header(&original)?;
        let original_id = hash(&original);
        let mut tips: BTreeMap<usize, String> = BTreeMap::new();
        let mut records = Vec::new();
        let mut tip_ids = BTreeSet::new();
        let mut total = 0usize;
        let mut converter_version = 1;
        for (index, preset) in presets.iter().enumerate() {
            converter_version = converter_version.max(preset.converter_version);
            check(token)?;
            let tip_resource = if let Some(mask) = &preset.settings.mask {
                let key = Arc::as_ptr(mask) as usize;
                let identity = if let Some(id) = tips.get(&key) {
                    id.clone()
                } else {
                    let id = hash(&encode_mask(mask)?);
                    tips.insert(key, id.clone());
                    id
                };
                Some(identity)
            } else {
                None
            };
            let info = SourceArchive::capture_info(
                original_id.clone(),
                version,
                index as u32,
                preset,
                tip_resource,
            )?;
            if let Some(id) = &info.tip_resource {
                tip_ids.insert(id.clone());
            }
            let record = serde_json::to_vec(&info).map_err(|e| Error::Codec(e.to_string()))?;
            total = total.saturating_add(record.len());
            if record.len() > MAX_RECORD || total > 32 * 1024 * 1024 {
                return Err(Error::Limit("brush source batch record bytes"));
            }
            records.push((hash(&record), record));
        }
        check(token)?;
        Ok(Self {
            original,
            original_id,
            records,
            tip_ids,
            converter_version,
        })
    }
    /// Verify portable records against one shared original, preserving record bytes.
    pub fn from_records(
        original: Arc<Vec<u8>>,
        records: BTreeMap<String, Vec<u8>>,
        token: &CancellationToken,
    ) -> Result<Self> {
        check(token)?;
        if records.is_empty() || records.len() > 1024 {
            return Err(Error::Limit("brush source batch count"));
        }
        let version = header(&original)?;
        let original_id = hash(&original);
        let mut tip_ids = BTreeSet::new();
        let mut total = 0usize;
        let mut converter_version = 1;
        for (id, bytes) in &records {
            check(token)?;
            total = total.saturating_add(bytes.len());
            if bytes.len() > MAX_RECORD || total > 32 * 1024 * 1024 {
                return Err(Error::Limit("brush source batch record bytes"));
            }
            if !valid_id(id) || hash(bytes) != *id {
                return Err(Error::Invalid("brush source record identity"));
            }
            let info: SourceInfo =
                serde_json::from_slice(bytes).map_err(|e| Error::Codec(e.to_string()))?;
            info.validate()?;
            converter_version = converter_version.max(info.converter_version);
            if info.original_hash != original_id
                || (info.file_version, info.file_subversion) != version
            {
                return Err(Error::Invalid("brush source original identity/version"));
            }
            if let Some(id) = info.tip_resource {
                tip_ids.insert(id);
            }
        }
        check(token)?;
        Ok(Self {
            original,
            original_id,
            records: records.into_iter().collect(),
            tip_ids,
            converter_version,
        })
    }
    pub fn converter_version(&self) -> u32 {
        self.converter_version
    }
    pub fn original_id(&self) -> &str {
        &self.original_id
    }
    pub fn original_bytes(&self) -> &[u8] {
        &self.original
    }
    pub fn records(&self) -> impl Iterator<Item = (&str, &[u8])> {
        self.records
            .iter()
            .map(|(id, bytes)| (id.as_str(), bytes.as_slice()))
    }
    pub fn tip_ids(&self) -> impl Iterator<Item = &str> {
        self.tip_ids.iter().map(String::as_str)
    }
    pub fn ids(&self) -> impl Iterator<Item = &str> {
        self.records.iter().map(|(id, _)| id.as_str())
    }
}
fn header(bytes: &[u8]) -> Result<(u16, Option<u16>)> {
    if bytes.len() > MAX_ORIGINAL {
        return Err(Error::Limit("brush source original bytes"));
    }
    let prefix = bytes
        .get(..4)
        .ok_or(Error::Invalid("brush source ABR header"))?;
    let version = u16::from_be_bytes([prefix[0], prefix[1]]);
    let second = u16::from_be_bytes([prefix[2], prefix[3]]);
    match (version, second) {
        (1 | 2, 1..=1024) => Ok((version, None)),
        (6 | 10, 1 | 2) => Ok((version, Some(second))),
        _ => Err(Error::Unsupported("brush source ABR header".into())),
    }
}
pub struct SourceStore {
    directory: PathBuf,
    writer: Mutex<()>,
    verified: Mutex<BTreeMap<String, Weak<ValidatedSource>>>,
}
/// Namespace-local proof of a captured identity, not an execution instruction.
#[derive(Debug)]
pub struct ValidatedSource {
    id: String,
}
impl ValidatedSource {
    pub fn id(&self) -> &str {
        &self.id
    }
}
impl SourceStore {
    pub fn open(directory: PathBuf) -> Result<Self> {
        std::fs::create_dir_all(&directory)?;
        Ok(Self {
            directory,
            writer: Mutex::new(()),
            verified: Mutex::new(BTreeMap::new()),
        })
    }
    fn path(&self, id: &str, extension: &str) -> Result<PathBuf> {
        if !valid_id(id) {
            return Err(Error::Invalid("brush source hash ID"));
        }
        Ok(self.directory.join(format!("{id}.{extension}")))
    }
    fn read(&self, id: &str, extension: &str, limit: usize) -> Result<Vec<u8>> {
        let path = self.path(id, extension)?;
        if std::fs::symlink_metadata(&path)?.file_type().is_symlink() {
            return Err(Error::Invalid("brush source symlink"));
        }
        let mut file = File::open(path)?;
        if file.metadata()?.len() > limit as u64 {
            return Err(Error::Limit("brush source file bytes"));
        }
        let mut bytes = Vec::new();
        Read::by_ref(&mut file)
            .take(limit as u64 + 1)
            .read_to_end(&mut bytes)?;
        if bytes.len() > limit || hash(&bytes) != id {
            return Err(Error::Invalid("brush source content identity"));
        }
        Ok(bytes)
    }
    fn write(
        &self,
        id: &str,
        extension: &str,
        bytes: &[u8],
        limit: usize,
        token: &CancellationToken,
        check_capacity: bool,
    ) -> Result<()> {
        check(token)?;
        if bytes.len() > limit || hash(bytes) != id {
            return Err(Error::Invalid("brush source write identity"));
        }
        let path = self.path(id, extension)?;
        if path.exists() {
            self.read(id, extension, limit)?;
            return Ok(());
        }
        if check_capacity {
            self.check_capacity(1)?;
        }
        let mut temp = tempfile::Builder::new()
            .prefix(".brush-source-")
            .tempfile_in(&self.directory)?;
        temp.write_all(bytes)?;
        temp.flush()?;
        temp.as_file().sync_all()?;
        check(token)?;
        match temp.persist_noclobber(&path) {
            Ok(_) => Ok(()),
            Err(e) if e.error.kind() == std::io::ErrorKind::AlreadyExists => {
                self.read(id, extension, limit)?;
                Ok(())
            }
            Err(e) => Err(Error::Io(e.error)),
        }
    }
    fn check_capacity(&self, additional: usize) -> Result<()> {
        let mut count = 0usize;
        for entry in std::fs::read_dir(&self.directory)? {
            if entry?
                .path()
                .extension()
                .is_some_and(|e| e == "abr" || e == "json")
            {
                count += 1;
            }
        }
        if count.saturating_add(additional) > 4096 {
            return Err(Error::Limit("brush source file count"));
        }
        Ok(())
    }
    pub fn save(&self, archive: &SourceArchive, token: &CancellationToken) -> Result<String> {
        check(token)?;
        let record = archive.record()?;
        let id = hash(&record);
        let _writer = self
            .writer
            .lock()
            .map_err(|_| Error::Invalid("brush source writer lock"))?;
        self.write(
            &archive.info.original_hash,
            "abr",
            archive.original(),
            MAX_ORIGINAL,
            token,
            true,
        )?;
        self.write(&id, "json", &record, MAX_RECORD, token, true)?;
        Ok(id)
    }
    /// Validate all records before capture, then store one original and all records.
    /// IO failure may leave unreferenced files; no partial catalog is returned.
    pub fn save_batch(
        &self,
        batch: &SourceBatch,
        token: &CancellationToken,
    ) -> Result<Vec<String>> {
        check(token)?;
        let _writer = self
            .writer
            .lock()
            .map_err(|_| Error::Invalid("brush source writer lock"))?;
        let targets = batch
            .ids()
            .map(|id| self.path(id, "json"))
            .chain(std::iter::once(self.path(&batch.original_id, "abr")))
            .collect::<Result<BTreeSet<_>>>()?;
        self.check_capacity(targets.iter().filter(|path| !path.exists()).count())?;
        self.write(
            &batch.original_id,
            "abr",
            &batch.original,
            MAX_ORIGINAL,
            token,
            false,
        )?;
        for (id, record) in &batch.records {
            self.write(id, "json", record, MAX_RECORD, token, false)?;
        }
        check(token)?;
        Ok(batch.ids().map(str::to_owned).collect())
    }
    /// Fresh export, grouped by original identity; no per-preset original copies.
    pub fn export_batches(
        &self,
        ids: &BTreeSet<String>,
        token: &CancellationToken,
    ) -> Result<Vec<SourceBatch>> {
        check(token)?;
        if ids.is_empty() || ids.len() > 1024 {
            return Err(Error::Limit("brush source reference count"));
        }
        let mut groups: BTreeMap<String, BTreeMap<String, Vec<u8>>> = BTreeMap::new();
        let mut total = 0usize;
        for id in ids {
            check(token)?;
            let record = self.read(id, "json", MAX_RECORD)?;
            total = total.saturating_add(record.len());
            if total > 32 * 1024 * 1024 {
                return Err(Error::Limit("brush source batch record bytes"));
            }
            let info: SourceInfo =
                serde_json::from_slice(&record).map_err(|e| Error::Codec(e.to_string()))?;
            info.validate()?;
            groups
                .entry(info.original_hash)
                .or_default()
                .insert(id.clone(), record);
        }
        let mut batches = Vec::new();
        let mut originals = 0usize;
        for (id, records) in groups {
            check(token)?;
            let bytes = self.read(&id, "abr", MAX_ORIGINAL)?;
            originals = originals.saturating_add(bytes.len());
            if originals > 256 * 1024 * 1024 {
                return Err(Error::Limit("brush source original batch bytes"));
            }
            batches.push(SourceBatch::from_records(Arc::new(bytes), records, token)?);
        }
        Ok(batches)
    }
    pub fn load(&self, id: &str, token: &CancellationToken) -> Result<SourceArchive> {
        check(token)?;
        let bytes = self.read(id, "json", MAX_RECORD)?;
        let info: SourceInfo =
            serde_json::from_slice(&bytes).map_err(|e| Error::Codec(e.to_string()))?;
        info.validate()?;
        let original = Arc::new(self.read(&info.original_hash, "abr", MAX_ORIGINAL)?);
        if header(&original)? != (info.file_version, info.file_subversion) {
            return Err(Error::Invalid("brush source original version"));
        }
        check(token)?;
        Ok(SourceArchive { info, original })
    }
    /// Export original record bytes; never re-encode a referenced content identity.
    pub fn encoded_record(&self, id: &str, token: &CancellationToken) -> Result<Vec<u8>> {
        check(token)?;
        self.read(id, "json", MAX_RECORD)
    }
    /// Freshly verify the same record bytes returned to the inspector.
    pub fn verified_record(&self, id: &str, token: &CancellationToken) -> Result<Vec<u8>> {
        check(token)?;
        let bytes = self.read(id, "json", MAX_RECORD)?;
        let info: SourceInfo =
            serde_json::from_slice(&bytes).map_err(|e| Error::Codec(e.to_string()))?;
        info.validate()?;
        let original = self.read(&info.original_hash, "abr", MAX_ORIGINAL)?;
        if header(&original)? != (info.file_version, info.file_subversion) {
            return Err(Error::Invalid("brush source original version"));
        }
        check(token)?;
        Ok(bytes)
    }
    /// Verify one restore batch without rereading a shared original per preset.
    /// Only verified identities/headers survive between records, not large originals.
    pub fn validate_references(
        &self,
        ids: &BTreeSet<String>,
        token: &CancellationToken,
    ) -> Result<()> {
        check(token)?;
        if ids.len() > 1024 {
            return Err(Error::Limit("brush source reference count"));
        }
        let mut originals = BTreeMap::new();
        for id in ids {
            check(token)?;
            let record = self.read(id, "json", MAX_RECORD)?;
            let info: SourceInfo =
                serde_json::from_slice(&record).map_err(|e| Error::Codec(e.to_string()))?;
            info.validate()?;
            let actual = if let Some(version) = originals.get(&info.original_hash) {
                *version
            } else {
                let bytes = self.read(&info.original_hash, "abr", MAX_ORIGINAL)?;
                let version = header(&bytes)?;
                originals.insert(info.original_hash.clone(), version);
                version
            };
            if actual != (info.file_version, info.file_subversion) {
                return Err(Error::Invalid("brush source original version"));
            }
        }
        check(token)?;
        Ok(())
    }
    pub fn resolve_references(
        &self,
        ids: &BTreeSet<String>,
        token: &CancellationToken,
    ) -> Result<Vec<Arc<ValidatedSource>>> {
        check(token)?;
        if ids.len() > 1024 {
            return Err(Error::Limit("brush source reference count"));
        }
        let mut cache = self
            .verified
            .lock()
            .map_err(|_| Error::Invalid("brush source validation cache"))?;
        cache.retain(|_, weak| weak.strong_count() > 0);
        let retained = ids
            .iter()
            .filter_map(|id| {
                cache
                    .get(id)
                    .and_then(Weak::upgrade)
                    .map(|receipt| (id.clone(), receipt))
            })
            .collect::<BTreeMap<_, _>>();
        let missing = ids
            .iter()
            .filter(|id| !retained.contains_key(*id))
            .cloned()
            .collect::<BTreeSet<_>>();
        self.validate_references(&missing, token)?;
        let mut output = Vec::with_capacity(ids.len());
        for id in ids {
            check(token)?;
            let receipt = retained
                .get(id)
                .cloned()
                .unwrap_or_else(|| Arc::new(ValidatedSource { id: id.clone() }));
            if cache.len() >= 4096 && !cache.contains_key(id) {
                cache.pop_first();
            }
            cache.insert(id.clone(), Arc::downgrade(&receipt));
            output.push(receipt);
        }
        Ok(output)
    }
    /// Import a portable source pair only after complete identity/schema validation.
    pub fn put_verified(
        &self,
        id: &str,
        record: &[u8],
        original: Arc<Vec<u8>>,
        token: &CancellationToken,
    ) -> Result<SourceArchive> {
        check(token)?;
        if record.len() > MAX_RECORD || !valid_id(id) || hash(record) != id {
            return Err(Error::Invalid("brush source record identity"));
        }
        let info: SourceInfo =
            serde_json::from_slice(record).map_err(|e| Error::Codec(e.to_string()))?;
        let archive = SourceArchive { info, original };
        archive.record()?;
        let _writer = self
            .writer
            .lock()
            .map_err(|_| Error::Invalid("brush source writer lock"))?;
        self.write(
            &archive.info.original_hash,
            "abr",
            archive.original(),
            MAX_ORIGINAL,
            token,
            true,
        )?;
        self.write(id, "json", record, MAX_RECORD, token, true)?;
        Ok(archive)
    }
}

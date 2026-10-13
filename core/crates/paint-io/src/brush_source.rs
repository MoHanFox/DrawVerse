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
            || self.converter_version != 1
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
        let settings = &preset.settings;
        let info = SourceInfo {
            version: 1,
            format: "abr".into(),
            original_hash: hash(&original),
            file_version: version.0,
            file_subversion: version.1,
            preset_index,
            original_name: preset.name.into(),
            converter: "drawverse.abr.basic".into(),
            converter_version: 1,
            baseline: Parameters {
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
            tip_resource: settings
                .mask
                .as_ref()
                .map(|mask| encode_mask(mask).map(|png| hash(&png)))
                .transpose()?,
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
        Ok(Self { info, original })
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
        let mut count = 0;
        for entry in std::fs::read_dir(&self.directory)? {
            let path = entry?.path();
            if path.extension().is_some_and(|e| e == "abr" || e == "json") {
                count += 1;
            }
        }
        if count >= 4096 {
            return Err(Error::Limit("brush source file count"));
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
        )?;
        self.write(&id, "json", &record, MAX_RECORD, token)?;
        Ok(id)
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
        )?;
        self.write(id, "json", record, MAX_RECORD, token)?;
        Ok(archive)
    }
}

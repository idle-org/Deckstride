//! Read-only document loading, natural ordering, notes and bounded cached rendering.
use crate::settings;
use hayro::{
    RenderCache,
    hayro_syntax::{
        Pdf,
        object::{Array, Dict, String as PdfString},
    },
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    cmp::Ordering,
    collections::BTreeMap,
    path::{Path, PathBuf},
};
pub const MAX_FILE: u64 = 512 * 1024 * 1024;
pub const MAX_PAGES: usize = 10000;
pub const MAX_FRAME: usize = 4096 * 4096 * 4;
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DeckInfo {
    pub path: PathBuf,
    pub key: String,
    pub title: String,
    pub names: Vec<String>,
    pub notes: Vec<String>,
    pub warnings: Vec<String>,
    pub sha256: String,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FrameInfo {
    pub page: usize,
    pub width: u32,
    pub height: u32,
    pub path: PathBuf,
    pub sha256: String,
}
pub struct Frame {
    pub info: FrameInfo,
    pub rgba: Vec<u8>,
}
pub struct Document {
    pub info: DeckInfo,
    source: Source,
    cache: PathBuf,
    _cache_lease: crate::cache::Lease,
}
enum Source {
    Pdf(Pdf),
    Images(Vec<PathBuf>),
}
pub fn natural_cmp(a: &str, b: &str) -> Ordering {
    let (aa, bb) = (a.as_bytes(), b.as_bytes());
    let (mut i, mut j) = (0, 0);
    while i < aa.len() && j < bb.len() {
        if aa[i].is_ascii_digit() && bb[j].is_ascii_digit() {
            let (si, sj) = (i, j);
            while i < aa.len() && aa[i].is_ascii_digit() {
                i += 1;
            }
            while j < bb.len() && bb[j].is_ascii_digit() {
                j += 1;
            }
            let a = a[si..i].trim_start_matches('0');
            let b = b[sj..j].trim_start_matches('0');
            let cmp = a.len().cmp(&b.len()).then_with(|| a.cmp(b));
            if cmp != Ordering::Equal {
                return cmp;
            }
        } else {
            let cmp = aa[i].to_ascii_lowercase().cmp(&bb[j].to_ascii_lowercase());
            if cmp != Ordering::Equal {
                return cmp;
            }
            i += 1;
            j += 1;
        }
    }
    (aa.len() - i).cmp(&(bb.len() - j)).then_with(|| a.cmp(b))
}
pub(crate) fn read_limited(path: &Path, limit: u64) -> Result<Vec<u8>, String> {
    use std::io::Read;
    let file = std::fs::File::open(path).map_err(|e| e.to_string())?;
    if file.metadata().map_err(|e| e.to_string())?.len() > limit {
        return Err(format!("{} exceeds file size limit", path.display()));
    }
    let mut bytes = Vec::new();
    file.take(limit + 1)
        .read_to_end(&mut bytes)
        .map_err(|e| e.to_string())?;
    if bytes.len() as u64 > limit {
        return Err("File grew beyond limit".into());
    }
    Ok(bytes)
}
fn text(path: &Path) -> Result<String, String> {
    String::from_utf8(read_limited(path, 256 * 1024)?).map_err(|e| e.to_string())
}
fn pdf_text(value: PdfString<'_>) -> String {
    let bytes = value.as_bytes();
    if bytes.starts_with(&[254, 255]) {
        String::from_utf16_lossy(
            &bytes[2..]
                .as_chunks::<2>()
                .0
                .iter()
                .map(|c| u16::from_be_bytes([c[0], c[1]]))
                .collect::<Vec<_>>(),
        )
    } else {
        String::from_utf8_lossy(bytes).into_owned()
    }
}
impl Document {
    pub fn open(path: &Path, cache: PathBuf) -> Result<Self, String> {
        Self::open_inner(path, cache, true)
    }
    pub fn open_slides(path: &Path, cache: PathBuf) -> Result<Self, String> {
        Self::open_inner(path, cache, false)
    }
    fn open_inner(path: &Path, cache: PathBuf, load_notes: bool) -> Result<Self, String> {
        let path = std::fs::canonicalize(path).map_err(|e| e.to_string())?;
        let key = settings::project_key(&path);
        let cache_lease = crate::cache::lease(&cache, &key)?;
        let mut warnings = vec![];
        let mut notes = vec![];
        let names;
        let source;
        let hash;
        if path.is_dir() {
            let mut files = Vec::new();
            for entry in std::fs::read_dir(&path).map_err(|e| e.to_string())? {
                let e = entry.map_err(|e| e.to_string())?;
                let p = e.path();
                let ext = p
                    .extension()
                    .and_then(|s| s.to_str())
                    .unwrap_or("")
                    .to_ascii_lowercase();
                if ["png", "jpg", "jpeg", "webp", "bmp", "gif", "tif", "tiff"]
                    .contains(&ext.as_str())
                    && e.file_type().map_err(|e| e.to_string())?.is_file()
                {
                    files.push(p);
                }
                if files.len() > MAX_PAGES {
                    return Err("Too many slides (limit 10000)".into());
                }
            }
            files.sort_by(|a, b| {
                natural_cmp(
                    &a.file_name().unwrap().to_string_lossy(),
                    &b.file_name().unwrap().to_string_lossy(),
                )
            });
            if files.is_empty() {
                return Err("Folder contains no supported images".into());
            }
            names = files
                .iter()
                .map(|p| p.file_name().unwrap().to_string_lossy().into_owned())
                .collect::<Vec<_>>();
            if load_notes {
                let json = path.join("notes.json");
                let toml = path.join("notes.toml");
                let mut global = BTreeMap::<String, String>::new();
                if json.exists() && toml.exists() {
                    warnings.push("Both notes.json and notes.toml exist; using sidecars only. Remove one global notes file to select it.".into());
                } else if json.exists() || toml.exists() {
                    let result = text(if json.exists() { &json } else { &toml }).and_then(|s| {
                        if json.exists() {
                            serde_json::from_str(&s).map_err(|e| e.to_string())
                        } else {
                            toml::from_str(&s).map_err(|e| e.to_string())
                        }
                    });
                    match result {
                        Ok(map) => global = map,
                        Err(e) => warnings.push(format!("Global notes: {e}")),
                    }
                }
                let mut total = 0;
                for (file, name) in files.iter().zip(&names) {
                    let sidecar = file.with_extension("txt");
                    let note = if sidecar.exists() {
                        match text(&sidecar) {
                            Ok(s) => s,
                            Err(e) => {
                                warnings.push(format!("{name} notes: {e}"));
                                String::new()
                            }
                        }
                    } else {
                        global.get(name).cloned().unwrap_or_default()
                    };
                    total += note.len();
                    if total > 8 * 1024 * 1024 {
                        warnings.push("Notes exceed 8 MiB; remaining notes omitted".into());
                        notes.resize(names.len(), String::new());
                        break;
                    }
                    notes.push(note);
                }
            } else {
                notes.resize(names.len(), String::new());
            }
            let mut identity = Sha256::new();
            for name in &names {
                identity.update(name.as_bytes());
                identity.update([0]);
            }
            hash = format!("{:x}", identity.finalize());
            source = Source::Images(files);
        } else {
            let bytes = read_limited(&path, MAX_FILE)?;
            hash = format!("{:x}", Sha256::digest(&bytes));
            let pdf=Pdf::new(bytes).map_err(|e|format!("Cannot open PDF (encrypted/unsupported PDFs may need an unlocked export): {e:?}"))?;
            if pdf.pages().is_empty() || pdf.pages().len() > MAX_PAGES {
                return Err("PDF must have 1–10000 pages".into());
            }
            names = (1..=pdf.pages().len())
                .map(|n| format!("Slide {n}"))
                .collect();
            if load_notes {
                for page in pdf.pages().iter() {
                    let mut note = String::new();
                    if let Some(annotations) = page.raw().get::<Array<'_>>(b"Annots") {
                        for annotation in annotations.iter::<Dict<'_>>() {
                            if let Some(content) = annotation.get::<PdfString<'_>>(b"Contents") {
                                let s = pdf_text(content);
                                if note.len() + s.len() < 256 * 1024 {
                                    note.push_str(&s);
                                    note.push('\n');
                                }
                            }
                        }
                    }
                    notes.push(note);
                }
                if notes.iter().map(String::len).sum::<usize>() > 8 * 1024 * 1024 {
                    notes.fill(String::new());
                    warnings.push("PDF annotation notes exceed 8 MiB; notes omitted".into());
                }
            } else {
                notes.resize(pdf.pages().len(), String::new());
            }
            source = Source::Pdf(pdf);
        }
        if load_notes && let Err(error) = crate::notes::overlay(&path, &names, &mut notes) {
            warnings.push(format!("Presentation notes: {error}"));
        }
        let title = path
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .into_owned();
        let info = DeckInfo {
            path,
            key,
            title,
            names,
            notes,
            warnings,
            sha256: hash,
        };
        Ok(Self {
            info,
            source,
            cache,
            _cache_lease: cache_lease,
        })
    }
    pub fn render(&self, page: usize, edge: u32) -> Result<Frame, String> {
        self.render_inner(page, edge, false)?
            .ok_or_else(|| "Missing rendered frame".into())
    }
    pub fn cached(&self, page: usize, edge: u32) -> Result<Option<Frame>, String> {
        self.render_inner(page, edge, true)
    }
    fn render_inner(
        &self,
        page: usize,
        edge: u32,
        cached_only: bool,
    ) -> Result<Option<Frame>, String> {
        if page >= self.info.names.len() {
            return Err("Slide out of range".into());
        }
        let edge = edge.clamp(160, 4096);
        let (identity, bytes) = match &self.source {
            Source::Pdf(_) => (self.info.sha256.clone(), None),
            Source::Images(files) => {
                let bytes = read_limited(&files[page], MAX_FILE)?;
                (format!("{:x}", Sha256::digest(&bytes)), Some(bytes))
            }
        };
        let dir = self.cache.join(&self.info.key).join(&identity);
        let renderer_key = match self.source {
            Source::Pdf(_) => "hayro09-straight",
            Source::Images(_) => "hayro09",
        };
        let path = dir.join(format!("{page}-{edge}-{renderer_key}.rgba"));
        if let Ok((width, height, rgba)) = read_frame(&path) {
            return Ok(Some(Frame {
                info: FrameInfo {
                    page,
                    width,
                    height,
                    path,
                    sha256: identity,
                },
                rgba,
            }));
        }
        if cached_only {
            return Ok(None);
        }
        let (width, height, rgba) = match &self.source {
            Source::Pdf(pdf) => {
                let p = &pdf.pages()[page];
                let (w, h) = p.render_dimensions();
                if !w.is_finite() || !h.is_finite() || w <= 0. || h <= 0. {
                    return Err("Invalid PDF page geometry".into());
                }
                let scale = edge as f32 / w.max(h);
                let pixmap = hayro::render(
                    p,
                    &RenderCache::new(),
                    &Default::default(),
                    &Default::default(),
                    &hayro::PixmapSettings {
                        x_scale: scale,
                        y_scale: scale,
                        ..Default::default()
                    },
                );
                let w = pixmap.width() as u32;
                let h = pixmap.height() as u32;
                // Hayro stores premultiplied pixels; our cache/UI contract is straight RGBA.
                (
                    w,
                    h,
                    pixmap.take_rgba8(hayro::vello_cpu::peniko::ImageAlphaType::Alpha),
                )
            }
            Source::Images(_) => {
                let mut reader = image::ImageReader::new(std::io::Cursor::new(bytes.unwrap()))
                    .with_guessed_format()
                    .map_err(|e| e.to_string())?;
                let mut limits = image::Limits::default();
                limits.max_alloc = Some(512 * 1024 * 1024);
                limits.max_image_width = Some(32768);
                limits.max_image_height = Some(32768);
                reader.limits(limits);
                let image = reader.decode().map_err(|e| e.to_string())?;
                let image = image
                    .resize(edge, edge, image::imageops::FilterType::Triangle)
                    .to_rgba8();
                let (w, h) = image.dimensions();
                (w, h, image.into_raw())
            }
        };
        validate_frame(width, height, rgba.len())?;
        let mut data = Vec::with_capacity(rgba.len() + 40);
        data.extend(width.to_le_bytes());
        data.extend(height.to_le_bytes());
        data.extend(&rgba);
        data.extend(Sha256::digest(&data));
        settings::atomic_write(&path, &data)?;
        let manifest = serde_json::json!({"version":1,"source":self.info.path,"sha256":identity,"renderer":"hayro-0.9/image-0.25","page":page,"edge":edge,"width":width,"height":height,"artifact":path.file_name().unwrap().to_string_lossy()});
        settings::atomic_write(
            &dir.join(format!("{page}-{edge}-extraction.json")),
            &serde_json::to_vec_pretty(&manifest).unwrap(),
        )?;
        Ok(Some(Frame {
            info: FrameInfo {
                page,
                width,
                height,
                path,
                sha256: identity,
            },
            rgba,
        }))
    }
}
pub fn validate_frame(w: u32, h: u32, len: usize) -> Result<(), String> {
    if w == 0
        || h == 0
        || w > 4096
        || h > 4096
        || w as u64 * h as u64 * 4 != len as u64
        || len > MAX_FRAME
    {
        Err("Invalid frame dimensions or length".into())
    } else {
        Ok(())
    }
}
pub fn read_frame(path: &Path) -> Result<(u32, u32, Vec<u8>), String> {
    let data = read_limited(path, (MAX_FRAME + 40) as u64)?;
    if data.len() < 40 {
        return Err("Truncated frame".into());
    }
    let w = u32::from_le_bytes(data[..4].try_into().unwrap());
    let h = u32::from_le_bytes(data[4..8].try_into().unwrap());
    validate_frame(w, h, data.len() - 40)?;
    let end = data.len() - 32;
    if Sha256::digest(&data[..end]).as_slice() != &data[end..] {
        return Err("Frame checksum mismatch".into());
    }
    Ok((w, h, data[8..end].to_vec()))
}

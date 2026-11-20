//! Bounded, worker-only preparation of the optional blank-screen picture.
use crate::{
    document::{Frame, FrameInfo},
    worker::{Bridge, Request},
};
use sha2::{Digest, Sha256};
use std::{
    path::{Path, PathBuf},
    sync::Arc,
    time::Duration,
};

const EDGE: u32 = 2048;
const MAX_RGBA: usize = EDGE as usize * EDGE as usize * 4;

/// Copy/normalize a selected image, or load the existing managed copy.
/// Call only in a supervised worker (or tests), never on an input/UI path.
pub fn prepare(source: Option<&Path>, path: &Path) -> Result<Frame, String> {
    if let Some(source) = source {
        let bytes = crate::document::read_limited(source, 64 * 1024 * 1024)?;
        let mut reader = image::ImageReader::new(std::io::Cursor::new(bytes))
            .with_guessed_format()
            .map_err(|e| e.to_string())?;
        let mut limits = image::Limits::default();
        limits.max_alloc = Some(256 * 1024 * 1024);
        limits.max_image_width = Some(32768);
        limits.max_image_height = Some(32768);
        reader.limits(limits);
        let image = reader.decode().map_err(|e| e.to_string())?;
        let image = if image.width() > EDGE || image.height() > EDGE {
            image.resize(EDGE, EDGE, image::imageops::FilterType::Triangle)
        } else {
            image
        }
        .to_rgba8();
        let mut data = Vec::with_capacity(image.len() + 40);
        data.extend(image.width().to_le_bytes());
        data.extend(image.height().to_le_bytes());
        data.extend(image.as_raw());
        data.extend(Sha256::digest(&data));
        crate::settings::atomic_write(path, &data)?;
    }
    let data = crate::document::read_limited(path, (MAX_RGBA + 40) as u64)?;
    if data.len() < 40 {
        return Err("Truncated blank picture".into());
    }
    let width = u32::from_le_bytes(data[..4].try_into().unwrap());
    let height = u32::from_le_bytes(data[4..8].try_into().unwrap());
    crate::document::validate_frame(width, height, data.len() - 40)?;
    if width > EDGE || height > EDGE {
        return Err("Blank picture exceeds 2048 pixels per edge".into());
    }
    let end = data.len() - 32;
    let hash = Sha256::digest(&data[..end]);
    if hash.as_slice() != &data[end..] {
        return Err("Blank picture checksum mismatch".into());
    }
    Ok(Frame {
        info: FrameInfo {
            page: 0,
            width,
            height,
            path: path.into(),
            sha256: format!("{hash:x}"),
        },
        rgba: data[8..end].to_vec(),
    })
}

/// Output-side loading is independent of slide reads and never reports a picture as a slide.
#[derive(Default)]
pub struct Output {
    requested: Option<FrameInfo>,
    loader: Option<Bridge>,
    pub picture: Option<Arc<Frame>>,
}
impl Output {
    pub fn set(&mut self, info: Option<&FrameInfo>) -> bool {
        if self.requested.as_ref().map(identity) == info.map(identity) {
            return false;
        }
        self.requested = info.cloned();
        self.picture = None;
        self.loader = info.map(|info| {
            let loader = Bridge::new(Duration::from_secs(10));
            loader.submit(
                0,
                0,
                Request::BlankPicture {
                    source: None,
                    path: info.path.clone(),
                },
            );
            loader
        });
        true
    }
    pub fn poll(&mut self) -> Option<Result<(), String>> {
        let completion = self.loader.as_ref()?.rx.try_recv().ok()?;
        self.loader = None;
        Some((|| {
            let packet = completion.result?;
            if let Some(error) = packet.reply.error {
                return Err(error);
            }
            let info = packet.reply.frame.ok_or("Missing blank picture")?;
            if self.requested.as_ref().map(identity) != Some(identity(&info)) {
                return Err(
                    "Blank picture changed while loading; choose it again to retry.".into(),
                );
            }
            self.picture = Some(Arc::new(Frame {
                info,
                rgba: packet.data,
            }));
            Ok(())
        })())
    }
}
fn identity(info: &FrameInfo) -> (&PathBuf, &str, u32, u32) {
    (&info.path, &info.sha256, info.width, info.height)
}

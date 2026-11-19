#![allow(dead_code)]
use std::path::Path;
pub fn pdf(path: &Path) {
    let objects = [
        "<< /Type /Catalog /Pages 2 0 R >>",
        "<< /Type /Pages /Kids [3 0 R 5 0 R] /Count 2 >>",
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 640 360] /Contents 4 0 R /Annots [7 0 R] >>",
        "<< /Length 30 >>\nstream\n0.1 0.7 0.5 rg 0 0 640 360 re f\nendstream",
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 640 360] /Contents 6 0 R >>",
        "<< /Length 30 >>\nstream\n0.7 0.2 0.1 rg 0 0 640 360 re f\nendstream",
        "<< /Type /Annot /Subtype /Text /Rect [0 0 10 10] /Contents (Speaker note) >>",
    ];
    let mut bytes = b"%PDF-1.4\n".to_vec();
    let mut offsets = vec![0];
    for (i, object) in objects.iter().enumerate() {
        offsets.push(bytes.len());
        bytes.extend(format!("{} 0 obj\n{}\nendobj\n", i + 1, object).bytes());
    }
    let start = bytes.len();
    bytes.extend(format!("xref\n0 {}\n0000000000 65535 f \n", offsets.len()).bytes());
    for offset in offsets.iter().skip(1) {
        bytes.extend(format!("{offset:010} 00000 n \n").bytes());
    }
    bytes.extend(
        format!(
            "trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{start}\n%%EOF\n",
            offsets.len()
        )
        .bytes(),
    );
    std::fs::write(path, bytes).unwrap();
}
pub fn image(path: &Path, color: [u8; 4]) {
    image::RgbaImage::from_pixel(64, 36, image::Rgba(color))
        .save(path)
        .unwrap();
}

# Local renderer patch

Upstream: crates.io `egui_software_backend` 0.0.3, MIT OR Apache-2.0. Source and
license texts were copied from the Cargo registry for reproducible offline builds.
The Cargo example entries were removed because those example files are not bundled.
No generated binaries or downloaded runtime components are included.

Local changes:

- `fast_image.rs`, registered in lib.rs and called from render.rs: guarded scanline
  sampling for axis-aligned, white, clamped cropped image quads. Other geometry,
  font texture, full images and tint/wrapping keep the upstream raster path.
- Sampling caches two horizontally filtered rows and O(viewport width) index data.
  Fixed-point bilinear interpolation preserves premultiplied color/alpha, with at
  most two channel values of rounding difference versus the floating-point path.
  No separate slide/zoom image is allocated.
- egui_texture.rs records opacity at upload; lib.rs conservatively clears it for
  transparent partial updates. Opaque scanlines avoid unnecessary blending.
- lib.rs initializes the optional Rayon pool lazily, bounded to at most four
  available CPU threads per process. Renderer workers are not created by PDF/IO
  worker processes. The application splits large moving crops into four disjoint
  clip bands; all bands reference the same texture.
- Tile occupancy lookup uses binary search in the existing row-major sorted list.
  Cached primitives, alpha compositing, canvas buffers and general UI drawing keep
  their upstream representation. No new coverage exclusions were added.

Tests: `crates/pdf-presenter/tests/zoom.rs`, `slide_colors.rs`, and native tests.
They compare optimized pixels with the general raster path, exercise clipping/DPI,
alpha and crop bounds, and check that panning does not upload another texture.
Benchmark: `cargo test --release --test zoom buffered_zoom_drawing_measurement --
--ignored --nocapture`. Physical multicreen acceptance is separate.

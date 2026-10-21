//! Raster grids for an image behind a drawing (clause 9.10): clip, resample, shaded relief and JPEG.
//!
//! The standard does not make, reproject or clip images (clause 9.10). This module does that part for a
//! tool, in model coordinates, so that every tool makes a backdrop the same way. The caller reads the
//! file (a DEM or an orthophoto) and gives a grid; the module gives JPEG bytes and the `image` entity
//! that places them. It opens no file. Every transcendental goes through `libm`, and the JPEG encoder
//! uses integer arithmetic only, so a grid gives the same bytes on every platform.

use crate::entities::num;
use crate::image::sha256_hex;
use crate::pipeline::Files;
use serde_json::{Map, Value};
use std::collections::BTreeMap;

/// A grid of cells in model space, row by row from the top (`ymax`) down, each row from `xmin` to the
/// right. The box is the outer edge of the cells: `[xmin, ymin, xmax, ymax]`, in metres.
#[derive(Debug, Clone, PartialEq)]
pub struct Grid<T> {
    pub cols: usize,
    pub rows: usize,
    pub bbox_m: [f64; 4],
    pub cells: Vec<T>,
}

/// Why a grid cannot be used.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RasterError {
    /// `cells` does not hold `cols x rows` values, or the box is empty.
    BadGrid,
    /// The window does not hold one whole cell of the grid.
    NoCells,
    /// A JPEG holds at most 65535 x 65535 pixels.
    TooLarge,
}

impl std::fmt::Display for RasterError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            RasterError::BadGrid => "the grid does not hold cols x rows cells over a box",
            RasterError::NoCells => "the window holds no whole cell of the grid",
            RasterError::TooLarge => "a JPEG holds at most 65535 x 65535 pixels",
        })
    }
}

impl std::error::Error for RasterError {}

impl<T: Copy> Grid<T> {
    /// A grid, checked: `cells` holds `cols x rows` values and the box is not empty.
    pub fn new(
        cols: usize,
        rows: usize,
        bbox_m: [f64; 4],
        cells: Vec<T>,
    ) -> Result<Grid<T>, RasterError> {
        let [x0, y0, x1, y1] = bbox_m;
        if cols == 0 || rows == 0 || cells.len() != cols * rows || !(x1 > x0 && y1 > y0) {
            return Err(RasterError::BadGrid);
        }
        Ok(Grid {
            cols,
            rows,
            bbox_m,
            cells,
        })
    }

    /// The cell at a column and a row (row 0 is the top).
    pub fn at(&self, col: usize, row: usize) -> T {
        self.cells[row * self.cols + col]
    }

    /// The size of one cell: `(dx, dy)` in metres.
    pub fn cell_size(&self) -> (f64, f64) {
        let [x0, y0, x1, y1] = self.bbox_m;
        ((x1 - x0) / self.cols as f64, (y1 - y0) / self.rows as f64)
    }

    /// The whole cells that lie inside a window `[xmin, ymin, xmax, ymax]`, with their own box. A cell
    /// edge within a millionth of a cell of the window edge counts as on it. Use it where the pixels
    /// must stay as they are; the box then lies inside the window, so the image lies inside the frame
    /// of a viewport with that window (clause 9.10.6).
    pub fn clip(&self, window: [f64; 4]) -> Result<Grid<T>, RasterError> {
        let [x0, _, _, y1] = self.bbox_m;
        let (dx, dy) = self.cell_size();
        let eps = 1e-6;
        let lo = |v: f64, n: usize| (libm::ceil(v - eps).max(0.0) as usize).min(n);
        let hi = |v: f64, n: usize| (libm::floor(v + eps).max(0.0) as usize).min(n);
        let (c0, c1) = (
            lo((window[0] - x0) / dx, self.cols),
            hi((window[2] - x0) / dx, self.cols),
        );
        let (r0, r1) = (
            lo((y1 - window[3]) / dy, self.rows),
            hi((y1 - window[1]) / dy, self.rows),
        );
        if c1 <= c0 || r1 <= r0 {
            return Err(RasterError::NoCells);
        }
        let cells = (r0..r1)
            .flat_map(|r| (c0..c1).map(move |c| (c, r)))
            .map(|(c, r)| self.at(c, r))
            .collect();
        Grid::new(
            c1 - c0,
            r1 - r0,
            [
                x0 + c0 as f64 * dx,
                y1 - r1 as f64 * dy,
                x0 + c1 as f64 * dx,
                y1 - r0 as f64 * dy,
            ],
            cells,
        )
    }

    /// A new grid of `cols x rows` cells over `bbox_m`, each the cell of this grid under its centre
    /// (nearest neighbour). A centre outside this grid takes `outside`. Use it to cover a window
    /// exactly, or to fix the number of pixels of a sheet.
    pub fn resample(
        &self,
        bbox_m: [f64; 4],
        cols: usize,
        rows: usize,
        outside: T,
    ) -> Result<Grid<T>, RasterError> {
        let [bx0, by0, bx1, by1] = bbox_m;
        if cols == 0 || rows == 0 || !(bx1 > bx0 && by1 > by0) {
            return Err(RasterError::BadGrid);
        }
        let [x0, _, _, y1] = self.bbox_m;
        let (dx, dy) = self.cell_size();
        let (sx, sy) = ((bx1 - bx0) / cols as f64, (by1 - by0) / rows as f64);
        let mut cells = Vec::with_capacity(cols * rows);
        for r in 0..rows {
            let y = by1 - (r as f64 + 0.5) * sy;
            let row = libm::floor((y1 - y) / dy);
            for c in 0..cols {
                let x = bx0 + (c as f64 + 0.5) * sx;
                let col = libm::floor((x - x0) / dx);
                let inside = col >= 0.0
                    && row >= 0.0
                    && (col as usize) < self.cols
                    && (row as usize) < self.rows;
                cells.push(if inside {
                    self.at(col as usize, row as usize)
                } else {
                    outside
                });
            }
        }
        Grid::new(cols, rows, bbox_m, cells)
    }
}

/// The light of a shaded relief.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Light {
    /// Where the light comes from, clockwise from north (grid north: +y), in degrees.
    pub azimuth_deg: f64,
    /// The height of the light above the horizon, in degrees.
    pub altitude_deg: f64,
    /// The vertical scale: a height is multiplied by it before the slope is taken.
    pub z_factor: f64,
}

impl Default for Light {
    /// The cartographic default: from the north-west, 45 degrees up, no vertical scale.
    fn default() -> Self {
        Light {
            azimuth_deg: 315.0,
            altitude_deg: 45.0,
            z_factor: 1.0,
        }
    }
}

/// A shaded relief of a DEM (heights in metres; `NaN` is no data), one grey value per cell, on the
/// same box. The slope is Horn's: the 3 x 3 cells around a cell, the edge cells repeated. A missing
/// neighbour takes the height of the cell; a cell with no height is white (255). The grey is
/// `round(255 x max(0, n . l))`, with `n` the unit normal of the ground and `l` the direction to the
/// light.
pub fn shaded_relief(dem: &Grid<f64>, light: &Light) -> Grid<u8> {
    let (dx, dy) = dem.cell_size();
    let z = light.z_factor;
    let (sa, ca) = (
        libm::sin(light.azimuth_deg.to_radians()),
        libm::cos(light.azimuth_deg.to_radians()),
    );
    let (sh, ch) = (
        libm::sin(light.altitude_deg.to_radians()),
        libm::cos(light.altitude_deg.to_radians()),
    );
    let (lx, ly, lz) = (sa * ch, ca * ch, sh);
    let mut cells = Vec::with_capacity(dem.cells.len());
    for r in 0..dem.rows {
        for c in 0..dem.cols {
            let centre = dem.at(c, r);
            if centre.is_nan() {
                cells.push(255);
                continue;
            }
            let h = |dc: isize, dr: isize| {
                let cc = (c as isize + dc).clamp(0, dem.cols as isize - 1) as usize;
                let rr = (r as isize + dr).clamp(0, dem.rows as isize - 1) as usize;
                let v = dem.at(cc, rr);
                if v.is_nan() { centre } else { v }
            };
            let (a, b, cc) = (h(-1, -1), h(0, -1), h(1, -1));
            let (d, f) = (h(-1, 0), h(1, 0));
            let (g, hh, i) = (h(-1, 1), h(0, 1), h(1, 1));
            // dz/dx to the east, dz/dy to the north (row 0 is the top)
            let p = ((cc + 2.0 * f + i) - (a + 2.0 * d + g)) / (8.0 * dx);
            let q = ((a + 2.0 * b + cc) - (g + 2.0 * hh + i)) / (8.0 * dy);
            let (nx, ny) = (-z * p, -z * q);
            let shade = (nx * lx + ny * ly + lz) / libm::sqrt(nx * nx + ny * ny + 1.0);
            cells.push(libm::round(255.0 * shade.clamp(0.0, 1.0)) as u8);
        }
    }
    Grid {
        cols: dem.cols,
        rows: dem.rows,
        bbox_m: dem.bbox_m,
        cells,
    }
}

/// The default JPEG quality: high enough for line work to read over it.
pub const JPEG_QUALITY: u8 = 85;

fn dimensions<T>(grid: &Grid<T>) -> Result<(u16, u16), RasterError> {
    Ok((
        u16::try_from(grid.cols).map_err(|_| RasterError::TooLarge)?,
        u16::try_from(grid.rows).map_err(|_| RasterError::TooLarge)?,
    ))
}

/// A grey grid as a baseline JPEG of one component (clause 9.10.1).
pub fn jpeg_grey(grid: &Grid<u8>, quality: u8) -> Result<Vec<u8>, RasterError> {
    let (w, h) = dimensions(grid)?;
    let mut out = Vec::new();
    jpeg_encoder::Encoder::new(&mut out, quality)
        .encode(&grid.cells, w, h, jpeg_encoder::ColorType::Luma)
        .map_err(|_| RasterError::BadGrid)?;
    Ok(out)
}

/// A colour grid (an orthophoto, a satellite image) as a baseline JPEG of three components.
pub fn jpeg_rgb(grid: &Grid<[u8; 3]>, quality: u8) -> Result<Vec<u8>, RasterError> {
    let (w, h) = dimensions(grid)?;
    let data: Vec<u8> = grid.cells.iter().flatten().copied().collect();
    let mut out = Vec::new();
    jpeg_encoder::Encoder::new(&mut out, quality)
        .encode(&data, w, h, jpeg_encoder::ColorType::Rgb)
        .map_err(|_| RasterError::BadGrid)?;
    Ok(out)
}

/// JPEG bytes and the model box they cover: what a drawing document needs to place an image.
#[derive(Debug, Clone, PartialEq)]
pub struct Backdrop {
    /// The path the document names, relative to its folder (clause 9.10).
    pub path: String,
    pub jpeg: Vec<u8>,
    pub bbox_m: [f64; 4],
}

impl Backdrop {
    /// A shaded relief of a DEM, as a JPEG.
    pub fn relief(
        path: &str,
        dem: &Grid<f64>,
        light: &Light,
        quality: u8,
    ) -> Result<Backdrop, RasterError> {
        Ok(Backdrop {
            path: path.to_string(),
            jpeg: jpeg_grey(&shaded_relief(dem, light), quality)?,
            bbox_m: dem.bbox_m,
        })
    }

    /// A colour grid, as a JPEG.
    pub fn colour(path: &str, grid: &Grid<[u8; 3]>, quality: u8) -> Result<Backdrop, RasterError> {
        Ok(Backdrop {
            path: path.to_string(),
            jpeg: jpeg_rgb(grid, quality)?,
            bbox_m: grid.bbox_m,
        })
    }

    /// The SHA-256 of the JPEG (clause 9.10.2).
    pub fn sha256(&self) -> String {
        sha256_hex(&self.jpeg)
    }

    /// The `image` entity of a drawing document that places it (clause 9.10). An opacity of 1 is
    /// left out, as it is the default.
    pub fn entity(&self, layer: &str, opacity: f64, id: Option<&str>) -> Value {
        let mut m = Map::new();
        m.insert("type".into(), "image".into());
        m.insert("layer".into(), layer.into());
        if let Some(id) = id {
            m.insert("id".into(), id.into());
        }
        m.insert("path".into(), self.path.clone().into());
        m.insert("sha256".into(), self.sha256().into());
        m.insert(
            "bbox_m".into(),
            Value::Array(self.bbox_m.iter().map(|v| num(*v)).collect()),
        );
        if opacity != 1.0 {
            m.insert("opacity".into(), num(opacity));
        }
        Value::Object(m)
    }
}

/// The files of a document, with images held in memory: a tool that makes a backdrop runs the document
/// without writing the JPEG first. Other files come from `inner`.
pub struct WithImages<'a> {
    pub inner: &'a dyn Files,
    /// The JPEG bytes by the path the document names.
    pub images: BTreeMap<String, Vec<u8>>,
}

impl<'a> WithImages<'a> {
    pub fn new(inner: &'a dyn Files, backdrops: &[Backdrop]) -> WithImages<'a> {
        WithImages {
            inner,
            images: backdrops
                .iter()
                .map(|b| (b.path.clone(), b.jpeg.clone()))
                .collect(),
        }
    }
}

impl Files for WithImages<'_> {
    fn read(&self, relative_path: &str) -> Option<String> {
        self.inner.read(relative_path)
    }

    fn read_bytes(&self, relative_path: &str) -> Option<Vec<u8>> {
        self.images
            .get(relative_path)
            .cloned()
            .or_else(|| self.inner.read_bytes(relative_path))
    }
}

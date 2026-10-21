//! Fetched imagery for a drawing backdrop: EOxCloudless 2016 tiles, cached on disk, sampled into a
//! raster grid in the caller's model coordinates.
//!
//! EOxCloudless 2016 is a cloudless mosaic of Sentinel-2 at 10 m, licensed CC BY 4.0, so commercial use
//! is allowed with attribution. It is the only source and the only year: the mosaics of 2018 and later
//! are CC BY-NC-SA 4.0 and need a paid licence for commercial use, so this crate does not offer them.
//! Every sheet that shows the imagery must print [`ATTRIBUTION`], in full.
//!
//! The tiles are cached on disk, in a folder the caller names. They are never shipped, and must not be
//! passed on as a dataset of their own. The free endpoint has no service level and a fair-use limit:
//! a failed or slow fetch comes back as a [`FetchError`] value, which the caller turns into a
//! fallback, never a panic.
//!
//! The caller gives the box in model coordinates and a function from a model point to longitude and
//! latitude, so the projection of the drawing stays with the tool. The result is a
//! [`otd::raster::Grid`] ready for [`otd::raster::Backdrop::colour`].

use otd::raster::Grid;
use std::collections::BTreeMap;
use std::fmt;
use std::path::PathBuf;
use std::time::Duration;

/// The attribution that every sheet showing the imagery prints, word for word.
pub const ATTRIBUTION: &str = "EOxCloudless https://cloudless.eox.at by EOX IT Services GmbH (Contains modified Copernicus Sentinel data 2016)";

/// The tile template of the 2016 layer, in Web Mercator (layer `s2cloudless_3857`, tile matrix set `g`).
const TILE_URL: &str = "https://tiles.maps.eox.at/wmts/1.0.0/s2cloudless_3857/default/g";

/// The size of a tile, in pixels.
pub const TILE_PX: u32 = 256;

/// The highest zoom fetched. The mosaic has 10 m pixels; zoom 15 is about 4.8 m at the equator.
pub const MAX_ZOOM: u8 = 15;

/// The most tiles one request may fetch, to stay inside the fair-use limit.
pub const MAX_TILES: usize = 400;

/// A source of fetched imagery. There is one.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Source {
    /// EOxCloudless 2016 (CC BY 4.0).
    EoxCloudless2016,
}

impl Source {
    /// The source of a name, as a CLI flag gives it: `eox-cloudless-2016`. Any other name, any other
    /// year included, is an error that says why.
    pub fn parse(name: &str) -> Result<Source, FetchError> {
        match name {
            "eox-cloudless-2016" => Ok(Source::EoxCloudless2016),
            other => Err(FetchError::Unsupported(other.to_string())),
        }
    }

    /// The name of the source.
    pub fn name(self) -> &'static str {
        "eox-cloudless-2016"
    }

    /// The year of the imagery, for the image note of the sheet.
    pub fn year(self) -> u16 {
        2016
    }

    /// The attribution to print on the sheet.
    pub fn attribution(self) -> &'static str {
        ATTRIBUTION
    }

    fn tile_url(self, t: Tile) -> String {
        format!("{TILE_URL}/{}/{}/{}.jpg", t.z, t.y, t.x)
    }
}

/// Why imagery could not be had. Each is a value to fall back on, never a panic.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FetchError {
    /// A source that is not offered, with the reason in [`fmt::Display`].
    Unsupported(String),
    /// The box would need more than [`MAX_TILES`] tiles.
    TooManyTiles(usize),
    /// The fetch failed or timed out.
    Network(String),
    /// A tile is not a JPEG that can be read.
    BadTile(Tile),
    /// The box or the grid size is empty.
    BadRequest,
}

impl fmt::Display for FetchError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FetchError::Unsupported(name) => write!(
                f,
                "imagery {name:?} is not offered: only eox-cloudless-2016 (CC BY 4.0); later EOxCloudless years are CC BY-NC-SA 4.0 and need a paid licence for commercial use"
            ),
            FetchError::TooManyTiles(n) => write!(
                f,
                "the box needs {n} tiles, more than {MAX_TILES}: use a smaller box or fewer pixels"
            ),
            FetchError::Network(why) => write!(f, "the imagery fetch failed: {why}"),
            FetchError::BadTile(t) => {
                write!(f, "tile {}/{}/{} is not a readable JPEG", t.z, t.x, t.y)
            }
            FetchError::BadRequest => f.write_str("the box or the grid size is empty"),
        }
    }
}

impl std::error::Error for FetchError {}

/// A tile of the Web Mercator grid: zoom, column, row (row 0 at the north).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Tile {
    pub z: u8,
    pub x: u32,
    pub y: u32,
}

/// Where tiles come from: the network, a cache, or a test.
pub trait TileFetch {
    /// The JPEG bytes of one tile.
    fn fetch(&self, source: Source, tile: Tile) -> Result<Vec<u8>, FetchError>;
}

/// Tiles from the network.
pub struct Http {
    agent: ureq::Agent,
}

impl Http {
    /// A fetcher whose every request gives up after `timeout`.
    pub fn new(timeout: Duration) -> Http {
        let config = ureq::Agent::config_builder()
            .timeout_global(Some(timeout))
            .user_agent(concat!("otd-imagery/", env!("CARGO_PKG_VERSION")))
            .build();
        Http {
            agent: config.into(),
        }
    }
}

impl TileFetch for Http {
    fn fetch(&self, source: Source, tile: Tile) -> Result<Vec<u8>, FetchError> {
        let mut response = self
            .agent
            .get(&source.tile_url(tile))
            .call()
            .map_err(|e| FetchError::Network(e.to_string()))?;
        response
            .body_mut()
            .with_config()
            .limit(4 * 1024 * 1024)
            .read_to_vec()
            .map_err(|e| FetchError::Network(e.to_string()))
    }
}

/// A disk cache in front of another fetcher: `<dir>/<source>/<z>/<x>/<y>.jpg`. A tile is fetched
/// once. A cache that cannot be written is not an error: the tile is used and fetched again next time.
pub struct Cached<F> {
    pub dir: PathBuf,
    pub inner: F,
}

impl<F: TileFetch> TileFetch for Cached<F> {
    fn fetch(&self, source: Source, tile: Tile) -> Result<Vec<u8>, FetchError> {
        let path = self
            .dir
            .join(source.name())
            .join(tile.z.to_string())
            .join(tile.x.to_string())
            .join(format!("{}.jpg", tile.y));
        if let Ok(bytes) = std::fs::read(&path) {
            return Ok(bytes);
        }
        let bytes = self.inner.fetch(source, tile)?;
        if let Some(parent) = path.parent()
            && std::fs::create_dir_all(parent).is_ok()
        {
            let _ = std::fs::write(&path, &bytes);
        }
        Ok(bytes)
    }
}

/// What to fetch: a box in model coordinates, the grid size, and the map to longitude and latitude.
pub struct Request<'a> {
    /// `[xmin, ymin, xmax, ymax]` in the model coordinates of the drawing, in metres.
    pub bbox_m: [f64; 4],
    pub cols: usize,
    pub rows: usize,
    /// A model point to `[longitude, latitude]` in degrees (WGS84).
    pub to_lon_lat: &'a dyn Fn([f64; 2]) -> [f64; 2],
}

/// The imagery of a request: the grid, and what the sheet must print.
#[derive(Debug, Clone, PartialEq)]
pub struct Imagery {
    pub grid: Grid<[u8; 3]>,
    pub source: Source,
    /// The zoom the tiles were taken at.
    pub zoom: u8,
}

impl Imagery {
    /// The attribution to print, in full, on every sheet that shows it.
    pub fn attribution(&self) -> &'static str {
        self.source.attribution()
    }

    /// The image note: the source and its year, as a river may have moved since.
    pub fn note(&self) -> String {
        format!(
            "Imagery: EOxCloudless {} (Sentinel-2, 10 m). The river may have moved since.",
            self.source.year()
        )
    }
}

/// The Web Mercator pixel of a longitude and latitude at a zoom, in pixels from the north-west.
fn mercator_px(lon_lat: [f64; 2], z: u8) -> [f64; 2] {
    let n = f64::from(TILE_PX) * libm::exp2(f64::from(z));
    let lat = lon_lat[1].clamp(-85.051_128_78, 85.051_128_78).to_radians();
    let x = (lon_lat[0] + 180.0) / 360.0 * n;
    let y =
        (1.0 - libm::log(libm::tan(lat) + 1.0 / libm::cos(lat)) / std::f64::consts::PI) / 2.0 * n;
    [x, y]
}

/// The zoom whose pixels are no larger than the cells of the grid at the latitude of the box.
fn zoom_for(cell_m: f64, lat_deg: f64) -> u8 {
    let ground = |z: u8| {
        2.0 * std::f64::consts::PI * 6_378_137.0 * libm::cos(lat_deg.to_radians())
            / (f64::from(TILE_PX) * libm::exp2(f64::from(z)))
    };
    (0..=MAX_ZOOM)
        .find(|&z| ground(z) <= cell_m)
        .unwrap_or(MAX_ZOOM)
}

/// Fetch the imagery of a request, tile by tile through `tiles`, and sample it into a grid by nearest
/// pixel. Any failed tile fails the whole request, so a sheet never shows a part of an image.
pub fn fetch(
    source: Source,
    tiles: &dyn TileFetch,
    request: &Request,
) -> Result<Imagery, FetchError> {
    let [x0, y0, x1, y1] = request.bbox_m;
    if request.cols == 0 || request.rows == 0 || !(x1 > x0 && y1 > y0) {
        return Err(FetchError::BadRequest);
    }
    let (sx, sy) = (
        (x1 - x0) / request.cols as f64,
        (y1 - y0) / request.rows as f64,
    );
    let centre = (request.to_lon_lat)([(x0 + x1) / 2.0, (y0 + y1) / 2.0]);
    let zoom = zoom_for(sx.min(sy), centre[1]);
    let max = TILE_PX * (1u32 << zoom) - 1;
    // Count the tiles under the box (its corners and edge midpoints) before any per-cell work, so a
    // request that is too large is refused at once.
    let mut lo = [f64::INFINITY; 2];
    let mut hi = [f64::NEG_INFINITY; 2];
    for x in [x0, (x0 + x1) / 2.0, x1] {
        for y in [y0, (y0 + y1) / 2.0, y1] {
            let p = mercator_px((request.to_lon_lat)([x, y]), zoom);
            for i in 0..2 {
                lo[i] = lo[i].min(p[i]);
                hi[i] = hi[i].max(p[i]);
            }
        }
    }
    let span = |i: usize| {
        let t = |v: f64| (v.clamp(0.0, f64::from(max)) / f64::from(TILE_PX)) as usize;
        t(hi[i]) - t(lo[i]) + 1
    };
    if span(0) * span(1) > MAX_TILES {
        return Err(FetchError::TooManyTiles(span(0) * span(1)));
    }
    if request.cols.saturating_mul(request.rows) > MAX_TILES * (TILE_PX * TILE_PX) as usize {
        return Err(FetchError::BadRequest); // more cells than the tiles hold pixels
    }
    let mut cells_px = Vec::with_capacity(request.cols * request.rows);
    for r in 0..request.rows {
        let y = y1 - (r as f64 + 0.5) * sy;
        for c in 0..request.cols {
            let x = x0 + (c as f64 + 0.5) * sx;
            let [px, py] = mercator_px((request.to_lon_lat)([x, y]), zoom);
            cells_px.push([
                (libm::floor(px).max(0.0) as u32).min(max),
                (libm::floor(py).max(0.0) as u32).min(max),
            ]);
        }
    }
    let mut needed: BTreeMap<Tile, Option<Vec<u8>>> = BTreeMap::new();
    for [px, py] in &cells_px {
        needed.insert(
            Tile {
                z: zoom,
                x: px / TILE_PX,
                y: py / TILE_PX,
            },
            None,
        );
    }
    if needed.len() > MAX_TILES {
        return Err(FetchError::TooManyTiles(needed.len()));
    }
    for (tile, pixels) in needed.iter_mut() {
        let bytes = tiles.fetch(source, *tile)?;
        *pixels = Some(decode_rgb(&bytes).ok_or(FetchError::BadTile(*tile))?);
    }
    let cells = cells_px
        .iter()
        .map(|[px, py]| {
            let tile = Tile {
                z: zoom,
                x: px / TILE_PX,
                y: py / TILE_PX,
            };
            let pixels = needed[&tile]
                .as_ref()
                .expect("every needed tile is fetched");
            let i = ((py % TILE_PX) * TILE_PX + px % TILE_PX) as usize * 3;
            [pixels[i], pixels[i + 1], pixels[i + 2]]
        })
        .collect();
    let grid = Grid::new(request.cols, request.rows, request.bbox_m, cells)
        .map_err(|_| FetchError::BadRequest)?;
    Ok(Imagery { grid, source, zoom })
}

/// The RGB pixels of a 256 x 256 tile, or `None`.
fn decode_rgb(bytes: &[u8]) -> Option<Vec<u8>> {
    let mut decoder = jpeg_decoder::Decoder::new(bytes);
    let pixels = decoder.decode().ok()?;
    let info = decoder.info()?;
    if u32::from(info.width) != TILE_PX || u32::from(info.height) != TILE_PX {
        return None;
    }
    match info.pixel_format {
        jpeg_decoder::PixelFormat::RGB24 => Some(pixels),
        jpeg_decoder::PixelFormat::L8 => Some(pixels.iter().flat_map(|&g| [g, g, g]).collect()),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mercator_pixels_of_known_points() {
        // the origin of the map is the centre of the world at every zoom
        assert_eq!(mercator_px([0.0, 0.0], 0), [128.0, 128.0]);
        let [x, y] = mercator_px([-180.0, 85.051_128_78], 1);
        assert!(x.abs() < 1e-9 && y.abs() < 1e-6, "{x} {y}");
    }

    #[test]
    fn the_zoom_follows_the_cell_size() {
        // at the equator zoom z has 156543.03 / 2^z metres a pixel: 9.55 m at 14, 4.78 m at 15
        assert_eq!(zoom_for(10.0, 0.0), 14);
        assert_eq!(zoom_for(5.0, 0.0), 15);
        assert_eq!(zoom_for(1.0, 0.0), MAX_ZOOM);
        assert_eq!(zoom_for(200_000.0, 0.0), 0);
    }

    #[test]
    fn the_tile_url_is_the_2016_layer() {
        assert_eq!(
            Source::EoxCloudless2016.tile_url(Tile {
                z: 14,
                x: 8000,
                y: 7900
            }),
            "https://tiles.maps.eox.at/wmts/1.0.0/s2cloudless_3857/default/g/14/7900/8000.jpg"
        );
    }
}

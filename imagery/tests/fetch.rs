//! Fetched imagery with synthetic tiles: no real EOX tile is in this repository, and no test uses the
//! network.
//!
//! Label: property

use otd::raster::{Grid, jpeg_rgb};
use otd_imagery::{
    ATTRIBUTION, Cached, FetchError, Request, Source, TILE_PX, Tile, TileFetch, fetch,
};
use std::cell::RefCell;

/// A synthetic tile: red on the west half, blue on the east half. JPEG blurs the edge a little, so
/// the tests sample away from it.
fn synthetic_tile() -> Vec<u8> {
    let n = TILE_PX as usize;
    let cells = (0..n * n)
        .map(|i| {
            if i % n < n / 2 {
                [220, 20, 20]
            } else {
                [20, 20, 220]
            }
        })
        .collect();
    jpeg_rgb(&Grid::new(n, n, [0.0, 0.0, 1.0, 1.0], cells).unwrap(), 90).unwrap()
}

/// Tiles from memory, with a log of what was asked.
struct Fake {
    tile: Vec<u8>,
    asked: RefCell<Vec<Tile>>,
    fail: bool,
}

impl Fake {
    fn new() -> Fake {
        Fake {
            tile: synthetic_tile(),
            asked: RefCell::new(vec![]),
            fail: false,
        }
    }
}

impl TileFetch for Fake {
    fn fetch(&self, source: Source, tile: Tile) -> Result<Vec<u8>, FetchError> {
        assert_eq!(source, Source::EoxCloudless2016);
        self.asked.borrow_mut().push(tile);
        if self.fail {
            Err(FetchError::Network("timed out".into()))
        } else {
            Ok(self.tile.clone())
        }
    }
}

/// Model metres near the equator to degrees: good enough for a test of the sampling.
fn lon_lat(p: [f64; 2]) -> [f64; 2] {
    [p[0] / 111_320.0, p[1] / 110_574.0]
}

fn request(bbox_m: [f64; 4], cols: usize, rows: usize) -> Request<'static> {
    Request {
        bbox_m,
        cols,
        rows,
        to_lon_lat: &lon_lat,
    }
}

#[test]
fn no_year_other_than_2016_can_be_asked_for() {
    assert_eq!(
        Source::parse("eox-cloudless-2016"),
        Ok(Source::EoxCloudless2016)
    );
    for name in [
        "eox-cloudless-2017",
        "eox-cloudless-2018",
        "eox-cloudless-2020",
        "eox-cloudless-2024",
        "eox-cloudless",
        "s2cloudless-2018",
        "s2cloudless_3857",
        "EOX-CLOUDLESS-2016",
    ] {
        let err = Source::parse(name).expect_err(name);
        assert!(matches!(err, FetchError::Unsupported(_)), "{name}");
        assert!(err.to_string().contains("CC BY-NC-SA 4.0"), "{err}");
    }
    assert_eq!(Source::EoxCloudless2016.year(), 2016);
}

#[test]
fn the_attribution_is_word_for_word_and_the_note_states_the_year() {
    assert_eq!(
        ATTRIBUTION,
        "EOxCloudless https://cloudless.eox.at by EOX IT Services GmbH (Contains modified Copernicus Sentinel data 2016)"
    );
    let fake = Fake::new();
    let img = fetch(
        Source::EoxCloudless2016,
        &fake,
        &request([0.0, 0.0, 200.0, 100.0], 20, 10),
    )
    .unwrap();
    assert_eq!(img.attribution(), ATTRIBUTION);
    assert!(img.note().contains("2016"), "{}", img.note());
}

#[test]
fn the_grid_is_sampled_from_the_tiles_by_nearest_pixel() {
    let fake = Fake::new();
    // 10 m cells: zoom 14. The box straddles the meridian, so its west part falls on the east half
    // of a tile west of the meridian (blue) and its east part on the west half of the next (red).
    let img = fetch(
        Source::EoxCloudless2016,
        &fake,
        &request([-400.0, 0.0, 400.0, 100.0], 80, 10),
    )
    .unwrap();
    assert_eq!(img.zoom, 14);
    assert_eq!((img.grid.cols, img.grid.rows), (80, 10));
    assert_eq!(img.grid.bbox_m, [-400.0, 0.0, 400.0, 100.0]);
    let west = img.grid.at(5, 5);
    let east = img.grid.at(74, 5);
    assert!(west[2] > 150 && west[0] < 80, "west {west:?}");
    assert!(east[0] > 150 && east[2] < 80, "east {east:?}");
    let mut asked = fake.asked.borrow().clone();
    asked.sort();
    asked.dedup();
    assert_eq!(asked.len(), fake.asked.borrow().len(), "each tile once");
    assert!(asked.len() >= 2);
}

#[test]
fn a_failed_fetch_is_an_error_value_not_a_panic() {
    let fake = Fake {
        fail: true,
        ..Fake::new()
    };
    let err = fetch(
        Source::EoxCloudless2016,
        &fake,
        &request([0.0, 0.0, 200.0, 100.0], 20, 10),
    )
    .unwrap_err();
    assert_eq!(err, FetchError::Network("timed out".into()));
    let bad = Fake {
        tile: b"not a jpeg".to_vec(),
        ..Fake::new()
    };
    assert!(matches!(
        fetch(
            Source::EoxCloudless2016,
            &bad,
            &request([0.0, 0.0, 200.0, 100.0], 20, 10)
        ),
        Err(FetchError::BadTile(_))
    ));
    assert_eq!(
        fetch(
            Source::EoxCloudless2016,
            &Fake::new(),
            &request([0.0, 0.0, 0.0, 100.0], 20, 10)
        ),
        Err(FetchError::BadRequest)
    );
}

#[test]
fn a_box_of_too_many_tiles_is_refused_before_any_fetch() {
    let fake = Fake::new();
    // 400 km at 10 m cells would be about 1700 tiles at zoom 14
    let err = fetch(
        Source::EoxCloudless2016,
        &fake,
        &request([0.0, 0.0, 400_000.0, 400_000.0], 40_000, 40_000),
    );
    assert!(matches!(err, Err(FetchError::TooManyTiles(n)) if n > 400));
    assert!(fake.asked.borrow().is_empty());
}

#[test]
fn the_cache_fetches_a_tile_once() {
    let dir = std::env::temp_dir().join(format!("otd-imagery-test-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let cached = Cached {
        dir: dir.clone(),
        inner: Fake::new(),
    };
    let req = request([0.0, 0.0, 200.0, 100.0], 20, 10);
    let a = fetch(Source::EoxCloudless2016, &cached, &req).unwrap();
    let first = cached.inner.asked.borrow().len();
    let b = fetch(Source::EoxCloudless2016, &cached, &req).unwrap();
    assert_eq!(a, b);
    assert_eq!(
        cached.inner.asked.borrow().len(),
        first,
        "the second run is all from the cache"
    );
    let t = cached.inner.asked.borrow()[0];
    assert!(
        dir.join(format!("eox-cloudless-2016/{}/{}/{}.jpg", t.z, t.x, t.y))
            .is_file()
    );
    let _ = std::fs::remove_dir_all(&dir);
}

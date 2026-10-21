//! The raster module: clip, resample, shaded relief and JPEG, and a backdrop placed through a
//! drawing document (clause 9.10).
//!
//! Label: sourced - hand calculation (the shades of a flat and a 45-degree plane, below)
//! Label: snapshot (the SHA-256 of the JPEG of a synthetic DEM; it pins the bytes on every platform)
//! Label: property (the rest)

use otd::image::{jpeg_info, sha256_hex};
use otd::raster::{
    Backdrop, Grid, JPEG_QUALITY, Light, RasterError, WithImages, jpeg_grey, shaded_relief,
};
use otd::render::{pdf::to_pdf, svg::SvgOptions, svg::to_svg};
use otd::{NoFiles, RunResult, run_value};
use serde_json::json;

/// A DEM of `n x n` cells of 10 m over `[0, 0, 10n, 10n]`, heights from a function of the cell centre.
fn dem(n: usize, height: impl Fn(f64, f64) -> f64) -> Grid<f64> {
    let size = 10.0 * n as f64;
    let cells = (0..n)
        .flat_map(|r| (0..n).map(move |c| (c, r)))
        .map(|(c, r)| height(10.0 * c as f64 + 5.0, size - 10.0 * r as f64 - 5.0))
        .collect();
    Grid::new(n, n, [0.0, 0.0, size, size], cells).unwrap()
}

#[test]
fn a_flat_dem_has_the_shade_of_the_light_altitude() {
    // 255 x sin 45 = 180.3
    let g = shaded_relief(&dem(4, |_, _| 7.0), &Light::default());
    assert!(g.cells.iter().all(|&v| v == 180), "{:?}", g.cells);
}

#[test]
fn a_slope_toward_the_light_is_lighter_and_away_is_darker() {
    // z = x: dz/dx = 1. n = (-1, 0, 1)/sqrt 2; light from 315 deg, 45 deg up: l = (-0.5, 0.5, 0.7071).
    // n.l = (0.5 + 0.7071)/1.4142 = 0.8536 -> 217.7 -> 218. The ground faces west, toward the light.
    let toward = shaded_relief(&dem(5, |x, _| x), &Light::default());
    // z = -x faces east: n = (1, 0, 1)/sqrt 2; n.l = (-0.5 + 0.7071)/1.4142 = 0.1464 -> 37.3 -> 37.
    let away = shaded_relief(&dem(5, |x, _| -x), &Light::default());
    assert_eq!(toward.at(2, 2), 218);
    assert_eq!(away.at(2, 2), 37);
    // the vertical scale steepens the slope: z = 2x with z_factor 1 equals z = x with z_factor 2
    let doubled = Light {
        z_factor: 2.0,
        ..Light::default()
    };
    assert_eq!(
        shaded_relief(&dem(5, |x, _| 2.0 * x), &Light::default()),
        shaded_relief(&dem(5, |x, _| x), &doubled)
    );
}

#[test]
fn a_cell_with_no_height_is_white() {
    let mut g = dem(3, |x, y| x + y);
    g.cells[4] = f64::NAN;
    let s = shaded_relief(&g, &Light::default());
    assert_eq!(s.at(1, 1), 255);
    assert!(s.cells.iter().enumerate().all(|(i, &v)| i == 4 || v != 255));
}

#[test]
fn a_synthetic_dem_gives_the_same_jpeg_bytes_on_every_platform() {
    // a hill: every slope and aspect, in 64 x 48 cells
    let cells = (0..48)
        .flat_map(|r| (0..64).map(move |c| (c, r)))
        .map(|(c, r)| {
            let (x, y) = (c as f64 - 32.0, r as f64 - 24.0);
            40.0 * libm::exp(-(x * x + y * y) / 300.0) + 0.3 * x
        })
        .collect();
    let g = Grid::new(64, 48, [500.0, 1000.0, 1140.0, 1480.0], cells).unwrap();
    let jpeg = jpeg_grey(&shaded_relief(&g, &Light::default()), JPEG_QUALITY).unwrap();
    let info = jpeg_info(&jpeg).expect("a JPEG of clause 9.10.1");
    assert_eq!(
        (info.width_px, info.height_px, info.components),
        (64, 48, 1)
    );
    assert_eq!(
        sha256_hex(&jpeg),
        "1005773965172d6a4c55dd367418094b4c2f3aa5fd5ba50b5431292a3b1a9991",
        "the shaded relief or the JPEG moved"
    );
}

#[test]
fn clip_keeps_whole_cells_inside_the_window() {
    let g = dem(10, |x, y| x * 1000.0 + y); // cells of 10 m over [0, 0, 100, 100]
    let c = g.clip([15.0, 20.0, 62.0, 80.0]).unwrap();
    assert_eq!(c.bbox_m, [20.0, 20.0, 60.0, 80.0]);
    assert_eq!((c.cols, c.rows), (4, 6));
    assert_eq!(c.at(0, 0), g.at(2, 2), "top-left cell of the clip");
    // an edge on a cell edge, give or take a rounding, keeps that cell
    let e = g.clip([10.0 - 1e-9, 0.0, 30.0 + 1e-9, 100.0]).unwrap();
    assert_eq!(e.bbox_m, [10.0, 0.0, 30.0, 100.0]);
    assert_eq!(g.clip([11.0, 11.0, 19.0, 19.0]), Err(RasterError::NoCells));
}

#[test]
fn resample_covers_a_box_exactly_by_nearest_cell() {
    let g = dem(10, |x, y| x * 1000.0 + y);
    let r = g.resample([15.0, 20.0, 62.0, 80.0], 47, 60, -1.0).unwrap();
    assert_eq!(r.bbox_m, [15.0, 20.0, 62.0, 80.0]);
    assert_eq!(r.at(0, 0), g.at(1, 2)); // centre (15.5, 79.5) is in the cell of x 10-20, y 70-80
    let out = g.resample([-50.0, 0.0, 50.0, 100.0], 10, 10, -1.0).unwrap();
    assert_eq!(out.at(0, 0), -1.0);
    assert_eq!(out.at(9, 0), g.at(4, 0));
}

#[test]
fn a_bad_grid_or_a_large_one_is_an_error_not_a_panic() {
    assert_eq!(
        Grid::new(2, 2, [0.0, 0.0, 1.0, 1.0], vec![0u8; 3]),
        Err(RasterError::BadGrid)
    );
    assert_eq!(
        Grid::new(1, 1, [0.0, 0.0, 0.0, 1.0], vec![0u8]),
        Err(RasterError::BadGrid)
    );
    let wide = Grid::new(70000, 1, [0.0, 0.0, 1.0, 1.0], vec![0u8; 70000]).unwrap();
    assert_eq!(jpeg_grey(&wide, 85), Err(RasterError::TooLarge));
}

#[test]
fn a_relief_backdrop_runs_through_a_document_and_every_output() {
    let g = dem(40, |x, y| 0.2 * x + 30.0 * libm::sin(y / 60.0));
    let window = [50.0, 60.0, 350.0, 340.0];
    let backdrop = Backdrop::relief(
        "relief.jpg",
        &g.clip(window).unwrap(),
        &Light::default(),
        JPEG_QUALITY,
    )
    .unwrap();
    let doc = json!({
        "standard_version": "1.1",
        "sheet": {"size": "A3", "orientation": "landscape", "print_profile": "colour"},
        "layers": [
            {"name": "RELIEF", "role": "thin", "line_type": "continuous", "aci": 30},
            {"name": "LINE", "role": "thick", "line_type": "continuous", "aci": 31, "print_colour": "#C00000"}
        ],
        "viewports": [{
            "id": "plan",
            "frame": {"x_mm": 20, "y_mm": 10, "width_mm": 390, "height_mm": 277},
            "window": {"xmin_m": window[0], "ymin_m": window[1], "xmax_m": window[2], "ymax_m": window[3]},
            "scale": "fit",
            "entities": [
                backdrop.entity("RELIEF", 0.9, Some("relief")),
                {"type": "line", "layer": "LINE", "from_m": [60, 100], "to_m": [340, 300]}
            ],
            "scale_bar": {},
            "north": {}
        }]
    });
    let files = WithImages::new(&NoFiles, std::slice::from_ref(&backdrop));
    let RunResult::Ok(norm) = run_value(&doc, &files) else {
        panic!("{:?}", run_value(&doc, &files).to_value())
    };
    let image = norm
        .entities
        .iter()
        .find(|e| e.type_name() == "image")
        .expect("the image");
    let v = image.to_value();
    assert_eq!(v["sha256"], backdrop.sha256());
    assert_eq!(v["size_px"], json!([30, 28]));
    assert_eq!(v["opacity"], 0.9);
    assert_eq!(norm.images[&backdrop.sha256()], backdrop.jpeg);
    assert!(to_svg(&norm, &SvgOptions::default()).contains("data:image/jpeg;base64,"));
    assert!(to_pdf(&norm).windows(9).any(|w| w == b"DCTDecode"));
    // the same document without the bytes is error 9.10.1
    assert!(matches!(run_value(&doc, &NoFiles), RunResult::Errors(e) if e.contains("9.10.1")));
}

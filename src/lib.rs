//! The Drawing Format Standard in Rust.
//!
//! A drawing document in, normalised content out (clauses 4 to 18). The core takes text and values
//! and returns values. It opens no file, reads no clock and uses no network: a document that names
//! a file is given a [`pipeline::Files`] to read it through.

/// A label the drawing writes, with its Message Id. A caller's text tools read the id and the English
/// from the source. Drawings stay English for now (clause 11.6.2 allows printable ASCII only), so
/// this gives the English alone.
macro_rules! label {
    ($id:literal, $english:literal $(,)?) => {
        $english
    };
    ($id:literal, $english:literal $(, $name:ident = $arg:expr)+ $(,)?) => {
        format!($english $(, $name = $arg)+)
    };
}

pub mod conformance;
pub mod data;
pub mod dimensions;
pub mod entities;
pub mod furniture;
pub mod hatch;
pub mod image;
pub mod json;
pub mod numbers;
pub mod pipeline;
pub mod raster;
pub(crate) mod read;
pub mod render;
pub mod rules;
pub mod schema;
pub mod symbols;

pub use pipeline::{Files, NoFiles, Normalised, RunResult, run_text, run_value};

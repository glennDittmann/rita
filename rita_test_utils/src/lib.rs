//! utils for rita tests and rita_lab
#![forbid(unsafe_code)]
#![deny(unused)]
#![warn(clippy::all, clippy::missing_const_for_fn)]

use rand::{distr::Uniform, prelude::Distribution};
use rand_distr::Normal;
use std::ops::RangeInclusive;

pub type Vertex2 = [f64; 2];
pub type Vertex3 = [f64; 3];

/// Samples `n` vertices in 2D space from the [Uniform] distribution.
///
/// If no range is specified, the unit-square centered around the origin is used, `[-0.5, 0.5]`.
pub fn sample_vertices_2d(n: usize, range: Option<RangeInclusive<f64>>) -> Vec<Vertex2> {
    let mut rng = rand::rng();
    let range = range.unwrap_or(-0.5..=0.5);
    let uniform = Uniform::try_from(range).expect("Expected range with a greater start then end");

    let mut vertices: Vec<[f64; 2]> = Vec::with_capacity(n);
    for _ in 0..n {
        let x = uniform.sample(&mut rng);
        let y = uniform.sample(&mut rng);
        vertices.push([x, y]);
    }

    vertices
}

/// Samples `n` vertices in 3D space from the [Uniform] distribution.
///
/// If no range is specified, the unit-square centered around the origin is used, `[-0.5, 0.5]`.
pub fn sample_vertices_3d(n: usize, range: Option<RangeInclusive<f64>>) -> Vec<Vertex3> {
    let mut rng = rand::rng();
    let range = range.unwrap_or(-0.5..=0.5);
    let uniform = Uniform::try_from(range).expect("Expected range with a greater start then end");

    let mut vertices: Vec<[f64; 3]> = Vec::with_capacity(n);
    for _ in 0..n {
        let x = uniform.sample(&mut rng);
        let y = uniform.sample(&mut rng);
        let z = uniform.sample(&mut rng);

        vertices.push([x, y, z]);
    }

    vertices
}

/// Samples `n` weights from a [Normal] distribution.
///
/// The default parametrization is `μ = 0.0` and `σ = 0.005`.
///
/// Parameters can be passed as an optional tuple `(μ, σ)`.
pub fn sample_weights(n: usize, params: Option<(f64, f64)>) -> Vec<f64> {
    let mut rng = rand::rng();
    let (mean, std_dev) = params.unwrap_or((0.0, 0.005));
    let normal = Normal::new(mean, std_dev).unwrap();

    let mut weights: Vec<f64> = Vec::with_capacity(n);
    for _ in 0..n {
        let w: f64 = normal.sample(&mut rng);
        weights.push(w);
    }

    weights
}

//! Geometric predicates abstraction.
//!
//! With feature `geogram` (default): uses [geogram_predicates] (FFI to C++ geogram) — supports
//! weighted 2D/3D (power circle/sphere via `orient_*lifted_SOS`).
//!
//! With feature `wasm`: uses pure-Rust [robust] — unweighted only; weighted APIs are unavailable.

#![allow(dead_code)]
#![allow(non_snake_case)] // match geogram_predicates API (in_sphere_3d_SOS, orient_*lifted_SOS)

use crate::utils::types::{Vertex2, Vertex3};

// Exactly one of geogram or wasm must be enabled.
#[cfg(not(any(feature = "geogram", feature = "wasm")))]
compile_error!(
    "Exactly one of features 'geogram' or 'wasm' must be enabled. Use default (geogram) or --no-default-features --features 'std,wasm' for WASM."
);

#[cfg(all(feature = "geogram", feature = "wasm"))]
compile_error!(
    "Features 'geogram' and 'wasm' are mutually exclusive. For WASM use --no-default-features --features 'std,wasm'."
);

/// Normalize predicate result to sign: -1.0, 0.0, or 1.0 so that `==` compares signs.
#[inline]
fn sign_f64(x: f64) -> f64 {
    if x > 0.0 {
        1.0
    } else if x < 0.0 {
        -1.0
    } else {
        0.0
    }
}

#[cfg(all(feature = "geogram", not(feature = "wasm")))]
mod imp {
    use super::*;
    use geogram_predicates as gp;

    #[inline]
    pub fn orient_2d(a: &Vertex2, b: &Vertex2, c: &Vertex2) -> f64 {
        let r = gp::orient_2d(a, b, c);
        if r > 0i16 {
            1.0
        } else if r < 0i16 {
            -1.0
        } else {
            0.0
        }
    }

    #[inline]
    pub fn orient_3d(a: &Vertex3, b: &Vertex3, c: &Vertex3, d: &Vertex3) -> f64 {
        let r = gp::orient_3d(a, b, c, d);
        if r > 0i16 {
            1.0
        } else if r < 0i16 {
            -1.0
        } else {
            0.0
        }
    }

    #[inline]
    pub fn in_sphere_3d_SOS(
        a: &Vertex3,
        b: &Vertex3,
        c: &Vertex3,
        d: &Vertex3,
        p: &Vertex3,
    ) -> f64 {
        let r = gp::in_sphere_3d_SOS(a, b, c, d, p);
        if r > 0i16 {
            1.0
        } else if r < 0i16 {
            -1.0
        } else {
            0.0
        }
    }

    #[inline]
    pub fn orient_2dlifted_SOS(
        a: &Vertex2,
        b: &Vertex2,
        c: &Vertex2,
        p: &Vertex2,
        h_a: f64,
        h_b: f64,
        h_c: f64,
        h_p: f64,
    ) -> f64 {
        let r = gp::orient_2dlifted_SOS(a, b, c, p, h_a, h_b, h_c, h_p);
        if r > 0i16 {
            1.0
        } else if r < 0i16 {
            -1.0
        } else {
            0.0
        }
    }

    #[inline]
    pub fn orient_3dlifted_SOS(
        a: &Vertex3,
        b: &Vertex3,
        c: &Vertex3,
        d: &Vertex3,
        p: &Vertex3,
        h_a: f64,
        h_b: f64,
        h_c: f64,
        h_d: f64,
        h_p: f64,
    ) -> f64 {
        let r = gp::orient_3dlifted_SOS(a, b, c, d, p, h_a, h_b, h_c, h_d, h_p);
        if r > 0i16 {
            1.0
        } else if r < 0i16 {
            -1.0
        } else {
            0.0
        }
    }
}

#[cfg(all(feature = "wasm", not(feature = "geogram")))]
mod imp {
    use super::*;
    use robust::{Coord, Coord3D, incircle, insphere, orient2d, orient3d};

    #[inline]
    fn coord2(p: &Vertex2) -> Coord<f64> {
        Coord { x: p[0], y: p[1] }
    }

    #[inline]
    fn coord3(p: &Vertex3) -> Coord3D<f64> {
        Coord3D {
            x: p[0],
            y: p[1],
            z: p[2],
        }
    }

    #[inline]
    pub fn orient_2d(a: &Vertex2, b: &Vertex2, c: &Vertex2) -> f64 {
        sign_f64(orient2d(coord2(a), coord2(b), coord2(c)))
    }

    #[inline]
    pub fn orient_3d(a: &Vertex3, b: &Vertex3, c: &Vertex3, d: &Vertex3) -> f64 {
        sign_f64(orient3d(coord3(a), coord3(b), coord3(c), coord3(d)))
    }

    /// Unweighted incircle (power circle with all heights zero). Used when `wasm` feature is on;
    /// weights are not allowed, so this is equivalent to orient_2dlifted_SOS with all h = 0.
    #[inline]
    pub fn orient_2dlifted_SOS(
        a: &Vertex2,
        b: &Vertex2,
        c: &Vertex2,
        p: &Vertex2,
        _h_a: f64,
        _h_b: f64,
        _h_c: f64,
        _h_p: f64,
    ) -> f64 {
        sign_f64(incircle(coord2(a), coord2(b), coord2(c), coord2(p)))
    }

    /// Unweighted insphere (same as in_sphere_3d_SOS). Used when `wasm` feature is on.
    #[inline]
    pub fn in_sphere_3d_SOS(
        a: &Vertex3,
        b: &Vertex3,
        c: &Vertex3,
        d: &Vertex3,
        p: &Vertex3,
    ) -> f64 {
        sign_f64(insphere(
            coord3(a),
            coord3(b),
            coord3(c),
            coord3(d),
            coord3(p),
        ))
    }

    /// Unweighted insphere (power sphere with all heights zero). Used when `wasm` feature is on.
    #[inline]
    pub fn orient_3dlifted_SOS(
        a: &Vertex3,
        b: &Vertex3,
        c: &Vertex3,
        d: &Vertex3,
        p: &Vertex3,
        _h_a: f64,
        _h_b: f64,
        _h_c: f64,
        _h_d: f64,
        _h_p: f64,
    ) -> f64 {
        sign_f64(insphere(
            coord3(a),
            coord3(b),
            coord3(c),
            coord3(d),
            coord3(p),
        ))
    }
}

// Re-export so call sites can use crate::predicates::orient_2d etc.
pub use imp::{in_sphere_3d_SOS, orient_2d, orient_2dlifted_SOS, orient_3d, orient_3dlifted_SOS};

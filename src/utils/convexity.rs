use std::cmp;

use geogram_predicates as gp;

use super::types::Vertex2;

/// Checks if ang(v1--v0, v1--v2) is convex, flat, or concave
pub fn is_convex(v0: Vertex2, v1: Vertex2, v2: Vertex2) -> bool {
    // true <-> used to return 1
    let sign = gp::orient_2d(&v0, &v1, &v2);

    match sign.cmp(&0) {
        cmp::Ordering::Greater => true,
        cmp::Ordering::Less => false,
        cmp::Ordering::Equal => {
            let v1_v0 = [v1[0] - v0[0], v1[1] - v0[1]];
            let v1_v2 = [v1[0] - v2[0], v1[1] - v2[1]];
            let dot_prod = v1_v0[0] * v1_v2[0] + v1_v0[1] * v1_v2[1];

            dot_prod > 0.
        }
    }
}

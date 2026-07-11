use super::*;
use crate::triangulation;
use rita_test_utils::sample_vertices_2d;
#[cfg(not(feature = "wasm"))]
use rita_test_utils::sample_weights;

fn verify_triangulation(triangulation: &Triangulation) {
    let regularity = triangulation.par_is_regular(false);
    let sound = triangulation.is_sound().unwrap();
    assert_eq!(regularity, 1.0);
    assert!(sound);
}

const NUM_VERTICES_LIST: [usize; 7] = [3, 5, 10, 50, 100, 500, 1000];

const EXAMPLE_VERTICES: [[f64; 2]; 10] = [
    [0.0, 0.0],
    [-0.5, 1.0],
    [0.0, 2.5],
    [2.0, 3.0],
    [4.0, 2.5],
    [5.0, 1.5],
    [4.5, 0.5],
    [2.5, -0.5],
    [1.5, 1.5],
    [3.0, 1.0],
];
#[cfg(not(feature = "wasm"))]
const EXAMPLE_WEIGHTS: [f64; 10] = [
    0.681, 0.579, 0.5625, 0.86225, 10.0, 0.472, 0.5865, 0.59625, 0.51225, 7.0,
];

fn run_delaunay_2d_test() {
    for n in NUM_VERTICES_LIST {
        let vertices = sample_vertices_2d(n, None);

        let mut triangulation = Triangulation::new(None);
        let result = triangulation.insert_vertices(&vertices, None, true);

        assert!(
            result.is_ok(),
            "insert_vertices failed: {}",
            result.unwrap_err()
        );

        verify_triangulation(&triangulation);
    }
}

#[test]
fn test_get_tris() {
    // Test unweighted case (runs with both geogram and wasm/robust)
    let mut triangulation = Triangulation::new(None);
    triangulation
        .insert_vertices(&EXAMPLE_VERTICES, None, true)
        .unwrap();

    let tris = triangulation.tris();
    let num_tris = tris.len();

    assert!(tris.len() == 10, "Expected 10 triangles, got {num_tris}");

    // Test weighted case (geogram only; wasm rejects weights)
    #[cfg(not(feature = "wasm"))]
    {
        let mut triangulation = Triangulation::new(None);
        triangulation
            .insert_vertices(&EXAMPLE_VERTICES, Some(EXAMPLE_WEIGHTS.to_vec()), true)
            .unwrap();

        let tris = triangulation.tris();
        let num_tris = tris.len();

        assert!(tris.len() == 8, "Expected 8 triangles, got {num_tris}");
    }
}

#[test]
fn test_delaunay_2d() {
    run_delaunay_2d_test();
}

/// Same as `test_delaunay_2d` but only compiled with `wasm` feature; verifies robust predicates.
#[cfg(feature = "wasm")]
#[test]
fn test_delaunay_2d_wasm() {
    println!("Running test_delaunay_2d_wasm");
    run_delaunay_2d_test();
}

#[cfg(not(feature = "wasm"))]
#[test]
fn test_weighted_delaunay_2d() {
    for n in NUM_VERTICES_LIST {
        let vertices = sample_vertices_2d(n, None);
        let weights = sample_weights(n, None);

        let mut triangulation = Triangulation::new(None);
        let result = triangulation.insert_vertices(&vertices, Some(weights), true);

        assert!(
            result.is_ok(),
            "insert_vertices failed: {}",
            result.unwrap_err()
        );

        verify_triangulation(&triangulation);

        assert!(
            triangulation.num_used_vertices()
                + triangulation.num_redundant_vertices()
                + triangulation.num_ignored_vertices()
                == n
        );
    }
}

/// Epsilon power circle is not supported in wasm (robust predicates are unweighted).
#[cfg(not(feature = "wasm"))]
#[test]
fn test_eps_delaunay_2d() {
    for n in NUM_VERTICES_LIST {
        let vertices = sample_vertices_2d(n, None);

        let mut triangulation = Triangulation::new(Some(1.0 / n as f64));
        let result = triangulation.insert_vertices(&vertices, None, true);

        assert!(
            result.is_ok(),
            "insert_vertices failed: {}",
            result.unwrap_err()
        );

        verify_triangulation(&triangulation);

        assert!(
            triangulation.num_used_vertices()
                + triangulation.num_redundant_vertices()
                + triangulation.num_ignored_vertices()
                == n
        );
    }
}

#[cfg(not(feature = "wasm"))]
#[test]
fn test_eps_weighted_delaunay_2d() {
    for n in NUM_VERTICES_LIST {
        let vertices = sample_vertices_2d(n, None);
        let weights = sample_weights(n, None);

        let mut triangulation = Triangulation::new(Some(1.0 / n as f64));
        let result = triangulation.insert_vertices(&vertices, Some(weights), true);

        assert!(
            result.is_ok(),
            "insert_vertices failed: {}",
            result.unwrap_err()
        );

        verify_triangulation(&triangulation);

        assert!(
            triangulation.num_used_vertices()
                + triangulation.num_redundant_vertices()
                + triangulation.num_ignored_vertices()
                == n
        );
    }
}

#[test]
#[ignore]
#[cfg(feature = "timing")]
// only run this test isolated, as test concurenncy can mess up par_iter
fn test_parallel_regularity_2d() {
    let n_vertices = 2000;
    let vertices = sample_vertices_2d(n_vertices, None);

    let mut triangulation = Triangulation::new(None);
    let _ = triangulation.insert_vertices(&vertices, None, true);

    let now = std::time::Instant::now();
    let (_, _eps_regularity) = triangulation.is_regular().unwrap();
    let elapsed = now.elapsed().as_millis();

    let now = std::time::Instant::now();
    let _regular_p = triangulation.par_is_regular(false);
    let elapsed_p = now.elapsed().as_millis();

    assert!(elapsed_p < elapsed)
}

#[test]
fn results_same_2d() {
    let vertices = &[
        [4.9, 31.9],
        [44.2, -0.05],
        [-49.31, 2.4],
        [98.5, -6.9],
        [7.7, 9.1],
        [3.5, 6.1],
        [6.0, -3.46],
        [4.7, 91.5],
        [6.7, 3.6],
        [-3.7, -40.3],
    ];

    assert_eq!(
        triangulation!(vertices).tris(),
        vec![
            [[6.0, -3.46], [3.5, 6.1], [-49.31, 2.4]],
            [[4.7, 91.5], [4.9, 31.9], [44.2, -0.05]],
            [[3.5, 6.1], [7.7, 9.1], [4.9, 31.9]],
            [[3.5, 6.1], [6.0, -3.46], [6.7, 3.6]],
            [[-3.7, -40.3], [98.5, -6.9], [44.2, -0.05]],
            [[3.5, 6.1], [6.7, 3.6], [7.7, 9.1]],
            [[44.2, -0.05], [6.0, -3.46], [-3.7, -40.3]],
            [[-49.31, 2.4], [-3.7, -40.3], [6.0, -3.46]],
            [[-49.31, 2.4], [3.5, 6.1], [4.9, 31.9]],
            [[4.9, 31.9], [7.7, 9.1], [44.2, -0.05]],
            [[4.9, 31.9], [4.7, 91.5], [-49.31, 2.4]],
            [[44.2, -0.05], [98.5, -6.9], [4.7, 91.5]],
            [[7.7, 9.1], [6.7, 3.6], [44.2, -0.05]],
            [[44.2, -0.05], [6.7, 3.6], [6.0, -3.46]]
        ]
    );

    let vertices = &[
        [-0.37122939978339264, 0.3190369464265699],
        [0.44217013845102393, -0.055915696282054284],
        [-0.4931480236200205, -0.16592024114317144],
        [0.4250889854947786, -0.11789966697253218],
        [0.24723377358550735, 0.2100464123915723],
        [0.36490258549176935, 0.1365021615193457],
        [0.3504827256051506, -0.19027659995331642],
        [-0.28683831662024745, 0.4111240123491553],
        [0.37042241707160173, 0.18423333136526698],
        [-0.3855198542371303, -0.44705493099901394],
    ];

    assert_eq!(
        triangulation!(vertices).tris(),
        vec![
            [
                [-0.4931480236200205, -0.16592024114317144],
                [-0.3855198542371303, -0.44705493099901394],
                [0.3504827256051506, -0.19027659995331642]
            ],
            [
                [-0.37122939978339264, 0.3190369464265699],
                [-0.4931480236200205, -0.16592024114317144],
                [0.24723377358550735, 0.2100464123915723]
            ],
            [
                [-0.28683831662024745, 0.4111240123491553],
                [0.24723377358550735, 0.2100464123915723],
                [0.37042241707160173, 0.18423333136526698]
            ],
            [
                [0.24723377358550735, 0.2100464123915723],
                [-0.28683831662024745, 0.4111240123491553],
                [-0.37122939978339264, 0.3190369464265699]
            ],
            [
                [0.3504827256051506, -0.19027659995331642],
                [0.24723377358550735, 0.2100464123915723],
                [-0.4931480236200205, -0.16592024114317144]
            ],
            [
                [0.24723377358550735, 0.2100464123915723],
                [0.36490258549176935, 0.1365021615193457],
                [0.37042241707160173, 0.18423333136526698]
            ],
            [
                [0.37042241707160173, 0.18423333136526698],
                [0.36490258549176935, 0.1365021615193457],
                [0.44217013845102393, -0.055915696282054284]
            ],
            [
                [0.36490258549176935, 0.1365021615193457],
                [0.24723377358550735, 0.2100464123915723],
                [0.3504827256051506, -0.19027659995331642]
            ],
            [
                [0.44217013845102393, -0.055915696282054284],
                [0.36490258549176935, 0.1365021615193457],
                [0.3504827256051506, -0.19027659995331642]
            ],
            [
                [0.3504827256051506, -0.19027659995331642],
                [0.4250889854947786, -0.11789966697253218],
                [0.44217013845102393, -0.055915696282054284]
            ],
        ]
    );
}

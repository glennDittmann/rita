# rita - Randomized Incremental Triangulation Algorithms

[![Crates.io version](https://img.shields.io/crates/v/rita.svg)](https://crates.io/crates/rita)
[![unsafe forbidden](https://img.shields.io/badge/unsafe-forbidden-brightgreen.svg)](https://github.com/glennDittmann/rita/blob/main/src/lib.rs#L12)

An implementation of (randomized) incremental weighted Delaunay triangulations in safe rust.

You can create a two- or three-dimensional Delaunay triangulation, including weighted points, as follows.

## 2D

```rust
let vertices = vec![
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
let weights = vec![0.2, 0.3, 0.55, 0.5, 0.6, 0.4, 0.65, 0.7, 0.85, 0.35]; // or None

let mut triangulation = Triangulation::new(None); // specify epsilon here
let result = triangulation.insert_vertices(&vertices, Some(weights), true);  // last parameter toggles spatial sorting
```

## 3D
```rust
let vertices = vec![
    [0.0, 0.0, 0.0],
    [-0.5, 1.0, 0.5],
    [0.0, 2.5, 2.5],
    [2.0, 3.0, 5.0],
    [4.0, 2.5, 6.5],
    [5.0, 1.5, 6.5],
    [4.5, 0.5, 5.0],
    [2.5, -0.5, 2.0],
    [1.5, 1.5, 3.0],
    [3.0, 1.0, 4.0],
];
let weights = vec![0.2, 0.3, 0.55, 0.5, 0.6, 0.4, 0.65, 0.7, 0.85, 0.35]; // or None

let mut tetrahedralization = Tetrahedralization::new(None); // specify epsilon here
let result = tetrahedralization.insert_vertices(&vertices, Some(weights), true);  // last parameter toggles spatial sorting
```

The eps parameter is used to perform an approximation technique, which leaves out certain vertices based on epsilon in the incremental insertion process.

:warning: **This is a work in progress.** :warning:
The algorithms work, as test coverage indicates.
However, the code is not yet fully optimized and the API is not yet simplified.
There might be duplicate and unnecessarily complex code.

## Robustness
Robustness is achieved through [geogram_predicates](https://github.com/glenndittmann/geogram_predicates), which itself uses [cxx](https://github.com/dtolnay/cxx) to make the geometric predicates from [geogram](https://github.com/BrunoLevy/geogram) available in `rust`.

## Base implementation
There is decent preliminary work done in the rust eco-system by [Bastien Durix](https://scholar.google.fr/citations?user=Crc4sdsAAAAJ&hl=fr) in the crate [simple_delaunay_lib](https://github.com/Ibujah/simple_delaunay_lib).

We forked this and re-fined by adding
  - weighted triangulations for 2D and 3D
  - adding geograms robust predicates
  - adding novel method: eps-Circles

_Theoretical Concepts_
- extended for 2D weighted delaunay triangulations, which includes
  - the flippability check (check for an edge to be flippable and only then and if it is not regular flip it)
  - the 3->1 flip (with weights come possible redundant points, i.e. points not present in the final triangulation, which are achieved by 3->1 flips)
  - the in_power_circler predicate via geogram_predicates (this is the "in_circle test" for weighted points)
- extend for 3D weighted Delaunay triangulations, which includes
  - adding weights to the general data structure
  - use regular "in-circle" predicates in the appropriate places
  - do not insert redundant vertices

_Code structure_
- reused Node struct, for 3D (instead of duplicating)
- split each struct into own files for better readabilit and maintainability
- implement fmt::Display for structs that had print() or println()
- improved overall readabilit, e.g. by refactoring larger functions, documenting or adding comments
- ...

_Conventions_
- improved overall naming conventions
- applied clippy style guide, e.g. exchanging a lot of if else with match
- align naming in code with literature, i.e. flip namings, data structure namings etc.

_Algorithmic Improvements_
- remove unneccary push of 2 extra edges after 2->2 flip
- remove unused match case in should_flip_hedge()
- early out for match case in should_flip_hedge that always returns false i.e. Flip::None
- exchange geo::robust for geogram_predicates which has the same features plus:
  - perurbation of simplicity
  - arithmetic schemes
- add a naive version of _last inserted triangle_, which speeds up location (especially when using spatial sorting)


_WIP_
- add 3D Flipping Algo (there are just a few edge cases to fix. main work is done)
- extend 3D flipping algo for weighted cases

## Acknowledgements
Thanks to Bastien Durix for his prior work on incremental Delaunay triangulations in rust.

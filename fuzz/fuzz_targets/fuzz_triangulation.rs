#![no_main]

use libfuzzer_sys::fuzz_target;
use rita::Triangulation;

fuzz_target!(|data: (Triangulation, [f64; 2])| {
    let (mut triangulation, vertex) = data;

    let _ = triangulation.insert_vertex(vertex, None, None);
    let _ = triangulation.is_regular();

    drop(triangulation);
});

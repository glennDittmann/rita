#![no_main]

use libfuzzer_sys::fuzz_target;
use rita::Tetrahedralization;

fuzz_target!(|data: (Tetrahedralization, [f64; 3])| {
    let (mut tetrahedralization, vertex) = data;

    let _ = tetrahedralization.insert_vertex(vertex, None);
    let _ = tetrahedralization.is_regular();

    drop(tetrahedralization);
});

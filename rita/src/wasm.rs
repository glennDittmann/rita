//! WASM bindings for 2D Delaunay triangulation.
//!
//! Provides a single function `triangulate` that takes flat vertex coordinates and optional
//! epsilon, and returns triangles and vertices in the same shape as vita's TriangulationResult
//! (vec of Triangle3, vec of Vertex3). For 2D, Vertex3 uses `y: 0` and `x,z` for the plane.

use crate::triangulation::Triangulation;
use wasm_bindgen::prelude::*;

/// 2D Delaunay triangulation.
///
/// # Arguments
/// * `vertices` - Flat array of 2D coordinates: [x1, y1, x2, y2, ...]
/// * `epsilon` - Optional epsilon for regularity (pass `null` or omit for `None`). When provided,
///   a positive value can speed up the triangulation.
///
/// # Returns
/// A JavaScript object with:
/// * `triangles` - Array of `{ id, a: { x, y, z }, b, c }` (2D: y = 0, x/z are the plane)
/// * `vertices` - Array of `{ x, y, z }` (2D: y = 0)
#[wasm_bindgen(js_name = triangulate)]
pub fn triangulate_2d(vertices: &[f64], epsilon: Option<f64>) -> Result<JsValue, JsValue> {
    let vertices_2d = parse_vertices_2d(vertices)?;
    if vertices_2d.len() < 3 {
        return Err(JsValue::from_str(
            "At least 3 vertices are required for 2D triangulation",
        ));
    }

    let mut t = Triangulation::new(epsilon);
    t.insert_vertices(&vertices_2d, None, true)
        .map_err(|e| JsValue::from_str(&format!("insert_vertices failed: {}", e)))?;

    let tri_list = t.tris();
    let vert_list = t.vertices();

    let triangles_js = js_sys::Array::new();
    for (i, tri) in tri_list.iter().enumerate() {
        let obj = triangle_to_js(tri, i)?;
        triangles_js.push(&obj);
    }

    let vertices_js = js_sys::Array::new();
    for v in vert_list.iter() {
        let obj = vertex2_to_js(v);
        vertices_js.push(&obj);
    }

    let result = js_sys::Object::new();
    js_sys::Reflect::set(&result, &"triangles".into(), &triangles_js)?;
    js_sys::Reflect::set(&result, &"vertices".into(), &vertices_js)?;
    Ok(result.into())
}

fn parse_vertices_2d(flat: &[f64]) -> Result<Vec<[f64; 2]>, JsValue> {
    if flat.len() % 2 != 0 {
        return Err(JsValue::from_str(
            "Vertices must have even length (pairs of x, y)",
        ));
    }
    Ok(flat.chunks_exact(2).map(|c| [c[0], c[1]]).collect())
}

/// [x, y] -> { x, y: 0, z } (vita-style 2D vertex in Vertex3)
fn vertex2_to_js(v: &[f64; 2]) -> JsValue {
    let obj = js_sys::Object::new();
    js_sys::Reflect::set(&obj, &"x".into(), &v[0].into()).unwrap();
    js_sys::Reflect::set(&obj, &"y".into(), &0.0_f64.into()).unwrap();
    js_sys::Reflect::set(&obj, &"z".into(), &v[1].into()).unwrap();
    obj.into()
}

/// Triangle2 -> { id, a, b, c } with Vertex3 (2D: y = 0)
fn triangle_to_js(tri: &[[f64; 2]; 3], index: usize) -> Result<JsValue, JsValue> {
    let obj = js_sys::Object::new();
    js_sys::Reflect::set(&obj, &"id".into(), &format!("tri_{}", index).into())?;
    js_sys::Reflect::set(&obj, &"a".into(), &vertex2_to_js(&tri[0]))?;
    js_sys::Reflect::set(&obj, &"b".into(), &vertex2_to_js(&tri[1]))?;
    js_sys::Reflect::set(&obj, &"c".into(), &vertex2_to_js(&tri[2]))?;
    Ok(obj.into())
}

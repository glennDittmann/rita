//! WASM bindings for 2D Delaunay triangulation and 3D Delaunay tetrahedralization.
//!
//! - `triangulate`: flat 2D coordinates → triangles and vertices `{ x, y }`
//! - `triangulate3d`: flat 3D coordinates → tetrahedra and vertices `{ x, y, z }`

use crate::tetrahedralization::Tetrahedralization;
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
/// * `triangles` - Array of `{ id, a: { x, y }, b, c }`
/// * `vertices` - Array of `{ x, y }`
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

/// [x, y] -> { x, y } (2D vertex, same dimension as input)
fn vertex2_to_js(v: &[f64; 2]) -> JsValue {
    let obj = js_sys::Object::new();
    js_sys::Reflect::set(&obj, &"x".into(), &v[0].into()).unwrap();
    js_sys::Reflect::set(&obj, &"y".into(), &v[1].into()).unwrap();
    obj.into()
}

/// Triangle2 -> { id, a, b, c } with each corner as { x, y }
fn triangle_to_js(tri: &[[f64; 2]; 3], index: usize) -> Result<JsValue, JsValue> {
    let obj = js_sys::Object::new();
    js_sys::Reflect::set(&obj, &"id".into(), &format!("tri_{}", index).into())?;
    js_sys::Reflect::set(&obj, &"a".into(), &vertex2_to_js(&tri[0]))?;
    js_sys::Reflect::set(&obj, &"b".into(), &vertex2_to_js(&tri[1]))?;
    js_sys::Reflect::set(&obj, &"c".into(), &vertex2_to_js(&tri[2]))?;
    Ok(obj.into())
}

/// 3D Delaunay tetrahedralization.
///
/// # Arguments
/// * `vertices` - Flat array of 3D coordinates: [x1, y1, z1, x2, y2, z2, ...]
/// * `epsilon` - Optional epsilon for regularity (pass `null` or omit for `None`). When provided,
///   a positive value can speed up the tetrahedralization.
///
/// # Returns
/// A JavaScript object with:
/// * `tetrahedra` - Array of `{ id, a: { x, y, z }, b, c, d }`
/// * `vertices` - Array of `{ x, y, z }`
#[wasm_bindgen(js_name = triangulate3d)]
pub fn triangulate_3d(vertices: &[f64], epsilon: Option<f64>) -> Result<JsValue, JsValue> {
    let vertices_3d = parse_vertices_3d(vertices)?;
    if vertices_3d.len() < 4 {
        return Err(JsValue::from_str(
            "At least 4 vertices are required for 3D tetrahedralization",
        ));
    }

    let mut t = Tetrahedralization::new(epsilon);
    t.insert_vertices(&vertices_3d, None, true)
        .map_err(|e| JsValue::from_str(&format!("insert_vertices failed: {}", e)))?;

    let tet_list = t.tets();
    let vert_list = t.vertices();

    let tetrahedra_js = js_sys::Array::new();
    for (i, tet) in tet_list.iter().enumerate() {
        let obj = tetrahedron_to_js(tet, i)?;
        tetrahedra_js.push(&obj);
    }

    let vertices_js = js_sys::Array::new();
    for v in vert_list.iter() {
        let obj = vertex3_to_js(v);
        vertices_js.push(&obj);
    }

    let result = js_sys::Object::new();
    js_sys::Reflect::set(&result, &"tetrahedra".into(), &tetrahedra_js)?;
    js_sys::Reflect::set(&result, &"vertices".into(), &vertices_js)?;
    Ok(result.into())
}

fn parse_vertices_3d(flat: &[f64]) -> Result<Vec<[f64; 3]>, JsValue> {
    if flat.len() % 3 != 0 {
        return Err(JsValue::from_str(
            "Vertices must have length divisible by 3 (triples of x, y, z)",
        ));
    }
    Ok(flat.chunks_exact(3).map(|c| [c[0], c[1], c[2]]).collect())
}

/// [x, y, z] -> { x, y, z } (3D vertex)
fn vertex3_to_js(v: &[f64; 3]) -> JsValue {
    let obj = js_sys::Object::new();
    js_sys::Reflect::set(&obj, &"x".into(), &v[0].into()).unwrap();
    js_sys::Reflect::set(&obj, &"y".into(), &v[1].into()).unwrap();
    js_sys::Reflect::set(&obj, &"z".into(), &v[2].into()).unwrap();
    obj.into()
}

/// Tetrahedron3 -> { id, a, b, c, d } with each corner as { x, y, z }
fn tetrahedron_to_js(tet: &[[f64; 3]; 4], index: usize) -> Result<JsValue, JsValue> {
    let obj = js_sys::Object::new();
    js_sys::Reflect::set(&obj, &"id".into(), &format!("tet_{}", index).into())?;
    js_sys::Reflect::set(&obj, &"a".into(), &vertex3_to_js(&tet[0]))?;
    js_sys::Reflect::set(&obj, &"b".into(), &vertex3_to_js(&tet[1]))?;
    js_sys::Reflect::set(&obj, &"c".into(), &vertex3_to_js(&tet[2]))?;
    js_sys::Reflect::set(&obj, &"d".into(), &vertex3_to_js(&tet[3]))?;
    Ok(obj.into())
}

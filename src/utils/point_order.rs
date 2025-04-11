use super::types::{Vertex2, Vertex3};

/// Sorts vertices along 2D Hilbert curve
pub fn sort_along_hilbert_curve_2d(vertices: &[Vertex2], indices_to_add: &[usize]) -> Vec<usize> {
    let mut curve_order = Vec::new();

    let (v_min, v_max) = find_min_max_2d(vertices, indices_to_add);

    let mut to_subdiv = Vec::new();
    let indices: Vec<usize> = indices_to_add.to_vec();
    to_subdiv.push((0, v_min, v_max, indices));

    while let Some((rot, pt_min, pt_max, indices_to_add)) = to_subdiv.pop() {
        match indices_to_add.len().cmp(&1) {
            std::cmp::Ordering::Greater => {
                let sep_x = (pt_min[0] + pt_max[0]) / 2.0;
                let sep_y = (pt_min[1] + pt_max[1]) / 2.0;

                let mut ind_a = Vec::new();
                let mut ind_b = Vec::new();
                let mut ind_c = Vec::new();
                let mut ind_d = Vec::new();

                for ind in indices_to_add {
                    let vert = vertices[ind];
                    if vert[0] < sep_x {
                        if vert[1] < sep_y {
                            ind_a.push(ind);
                        } else {
                            ind_b.push(ind);
                        }
                    } else if vert[1] < sep_y {
                        ind_d.push(ind);
                    } else {
                        ind_c.push(ind);
                    }
                }

                let pt_a_min = pt_min;
                let pt_a_max = [sep_x, sep_y];

                let pt_b_min = [pt_min[0], sep_y];
                let pt_b_max = [sep_x, pt_max[1]];

                let pt_c_min = [sep_x, sep_y];
                let pt_c_max = pt_max;

                let pt_d_min = [sep_x, pt_min[1]];
                let pt_d_max = [pt_max[0], sep_y];

                if rot == 0 {
                    to_subdiv.push((3, pt_a_min, pt_a_max, ind_a));
                    to_subdiv.push((0, pt_b_min, pt_b_max, ind_b));
                    to_subdiv.push((0, pt_c_min, pt_c_max, ind_c));
                    to_subdiv.push((7, pt_d_min, pt_d_max, ind_d));
                } else if rot == 1 {
                    to_subdiv.push((6, pt_d_min, pt_d_max, ind_d));
                    to_subdiv.push((1, pt_c_min, pt_c_max, ind_c));
                    to_subdiv.push((1, pt_b_min, pt_b_max, ind_b));
                    to_subdiv.push((2, pt_a_min, pt_a_max, ind_a));
                } else if rot == 2 {
                    to_subdiv.push((5, pt_b_min, pt_b_max, ind_b));
                    to_subdiv.push((2, pt_c_min, pt_c_max, ind_c));
                    to_subdiv.push((2, pt_d_min, pt_d_max, ind_d));
                    to_subdiv.push((1, pt_a_min, pt_a_max, ind_a));
                } else if rot == 3 {
                    to_subdiv.push((0, pt_a_min, pt_a_max, ind_a));
                    to_subdiv.push((3, pt_d_min, pt_d_max, ind_d));
                    to_subdiv.push((3, pt_c_min, pt_c_max, ind_c));
                    to_subdiv.push((4, pt_b_min, pt_b_max, ind_b));
                } else if rot == 4 {
                    to_subdiv.push((7, pt_c_min, pt_c_max, ind_c));
                    to_subdiv.push((4, pt_d_min, pt_d_max, ind_d));
                    to_subdiv.push((4, pt_a_min, pt_a_max, ind_a));
                    to_subdiv.push((3, pt_b_min, pt_b_max, ind_b));
                } else if rot == 5 {
                    to_subdiv.push((2, pt_b_min, pt_b_max, ind_b));
                    to_subdiv.push((5, pt_a_min, pt_a_max, ind_a));
                    to_subdiv.push((5, pt_d_min, pt_d_max, ind_d));
                    to_subdiv.push((6, pt_c_min, pt_c_max, ind_c));
                } else if rot == 6 {
                    to_subdiv.push((1, pt_d_min, pt_d_max, ind_d));
                    to_subdiv.push((6, pt_a_min, pt_a_max, ind_a));
                    to_subdiv.push((6, pt_b_min, pt_b_max, ind_b));
                    to_subdiv.push((5, pt_c_min, pt_c_max, ind_c));
                } else if rot == 7 {
                    to_subdiv.push((4, pt_c_min, pt_c_max, ind_c));
                    to_subdiv.push((7, pt_b_min, pt_b_max, ind_b));
                    to_subdiv.push((7, pt_a_min, pt_a_max, ind_a));
                    to_subdiv.push((0, pt_d_min, pt_d_max, ind_d));
                }
            }
            std::cmp::Ordering::Equal => curve_order.push(indices_to_add[0]),
            _ => (),
        }
    }

    curve_order
}

// Finds the minimum and maximum x and y values of the vertices
fn find_min_max_2d(vertices: &[Vertex2], indices_to_add: &[usize]) -> (Vertex2, Vertex2) {
    let mut v_min = vertices[indices_to_add[0]];
    let mut v_max = vertices[indices_to_add[0]];

    for &ind in indices_to_add {
        let vertex = vertices[ind];
        if v_min[0] > vertex[0] {
            v_min[0] = vertex[0];
        }
        if v_min[1] > vertex[1] {
            v_min[1] = vertex[1];
        }
        if v_max[0] < vertex[0] {
            v_max[0] = vertex[0];
        }
        if v_max[1] < vertex[1] {
            v_max[1] = vertex[1];
        }
    }
    (v_min, v_max)
}

/// Sorts vertices along 3D Hilbert curve
pub fn sort_along_hilbert_curve_3d(vertices: &[Vertex3], indices_to_add: &[usize]) -> Vec<usize> {
    let mut curve_order = Vec::new();

    let mut pt_min = vertices[indices_to_add[0]];
    let mut pt_max = vertices[indices_to_add[0]];

    for &ind in indices_to_add {
        if pt_min[0] > vertices[ind][0] {
            pt_min[0] = vertices[ind][0];
        }
        if pt_min[1] > vertices[ind][1] {
            pt_min[1] = vertices[ind][1];
        }
        if pt_min[2] > vertices[ind][2] {
            pt_min[2] = vertices[ind][2];
        }
        if pt_max[0] < vertices[ind][0] {
            pt_max[0] = vertices[ind][0];
        }
        if pt_max[1] < vertices[ind][1] {
            pt_max[1] = vertices[ind][1];
        }
        if pt_max[2] < vertices[ind][2] {
            pt_max[2] = vertices[ind][2];
        }
    }

    let mut to_subdiv = Vec::new();
    let indices: Vec<usize> = indices_to_add.to_vec();
    to_subdiv.push(([0, 0, 0], 0, pt_min, pt_max, indices));

    while let Some((start, dir, pt_min, pt_max, indices_to_add)) = to_subdiv.pop() {
        match indices_to_add.len().cmp(&1) {
            std::cmp::Ordering::Greater => {
                let sep_x = (pt_min[0] + pt_max[0]) / 2.0;
                let sep_y = (pt_min[1] + pt_max[1]) / 2.0;
                let sep_z = (pt_min[2] + pt_max[2]) / 2.0;

                let mut sep_ind = [
                    [[Vec::new(), Vec::new()], [Vec::new(), Vec::new()]],
                    [[Vec::new(), Vec::new()], [Vec::new(), Vec::new()]],
                ];

                for &ind in indices_to_add.iter() {
                    let vert = vertices[ind];
                    let xind = if vert[0] < sep_x { 0 } else { 1 } as usize;
                    let yind = if vert[1] < sep_y { 0 } else { 1 } as usize;
                    let zind = if vert[2] < sep_z { 0 } else { 1 } as usize;
                    sep_ind[xind][yind][zind].push(ind);
                }

                let pt_x = [pt_min[0], sep_x, pt_max[0]];
                let pt_y = [pt_min[1], sep_y, pt_max[1]];
                let pt_z = [pt_min[2], sep_z, pt_max[2]];

                let (next_modif, dir) = match (dir, start[dir]) {
                    (0, 0) => Some(([1, 2, 1, 0, 1, 2, 1, 0], [1, 2, 2, 0, 0, 2, 2, 1])),
                    (0, 1) => Some(([2, 1, 2, 0, 2, 1, 2, 0], [2, 1, 1, 0, 0, 1, 1, 2])),
                    (1, 0) => Some(([2, 0, 2, 1, 2, 0, 2, 1], [2, 0, 0, 1, 1, 0, 0, 2])),
                    (1, 1) => Some(([0, 2, 0, 1, 0, 2, 0, 1], [0, 2, 2, 1, 1, 2, 2, 0])),
                    (2, 0) => Some(([0, 1, 0, 2, 0, 1, 0, 2], [0, 1, 1, 2, 2, 1, 1, 0])),
                    (2, 1) => Some(([1, 0, 1, 2, 1, 0, 1, 2], [1, 0, 0, 2, 2, 0, 0, 1])),
                    (_, _) => None,
                }
                .unwrap();

                let mut sep_subind = start;
                let mut start_ind = start;
                for i in 0..8 {
                    let mut vec_inds = Vec::new();
                    vec_inds.append(&mut sep_ind[sep_subind[0]][sep_subind[1]][sep_subind[2]]);
                    to_subdiv.push((
                        start_ind,
                        dir[i],
                        [
                            pt_x[sep_subind[0]],
                            pt_y[sep_subind[1]],
                            pt_z[sep_subind[2]],
                        ],
                        [
                            pt_x[sep_subind[0] + 1],
                            pt_y[sep_subind[1] + 1],
                            pt_z[sep_subind[2] + 1],
                        ],
                        vec_inds,
                    ));
                    sep_subind[next_modif[i]] = 1 - sep_subind[next_modif[i]];
                    start_ind[next_modif[i]] = 1 - start_ind[next_modif[i]];
                    start_ind[dir[i]] = 1 - start_ind[dir[i]];
                }
            }
            std::cmp::Ordering::Equal => curve_order.push(indices_to_add[0]),
            _ => (),
        }
    }

    curve_order
}

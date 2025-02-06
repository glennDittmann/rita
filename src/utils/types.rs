// Type aliases for data values.
pub type Vertex2 = [f64; 2];
pub type Vertex3 = [f64; 3];
pub type Edge2 = [Vertex2; 2];
pub type Triangle2 = [Vertex2; 3];
pub type Triangle3 = [Vertex3; 3];
pub type Tetrahedron3 = [Vertex3; 4];

// Type aliases for data indices.
pub type VertexIdx = usize;

// Type aliases for data structure indices.
// This is to know, when a function accepts or returns a usize, what it is for.
pub type HedgeIteratorIdx = usize;
pub type TriIteratorIdx = usize;
pub type TetIteratorIdx = usize;

use super::types::Vertex2;

// TODO: remove these debug helpers
pub struct DisplayWrapper(pub Vertex2);

impl std::fmt::Display for DisplayWrapper {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "[{:.2}, {:.2}]", self.0[0], self.0[1])
    }
}

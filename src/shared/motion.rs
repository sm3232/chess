#[derive(Copy)]
pub struct Motion {
    pub from: usize,
    pub to: usize
}
impl Clone for Motion {
    fn clone(&self) -> Self { Self { to: self.to, from: self.from } }
}

impl std::fmt::Debug for Motion {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        return write!(f, "Motion from {}, to {}", self.from, self.to);
    }
}

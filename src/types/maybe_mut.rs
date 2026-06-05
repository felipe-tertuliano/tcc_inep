pub enum MaybeMut<'a, T> {
    Immutable(&'a T),
    Mutable(&'a mut T),
}

impl<'a, T> MaybeMut<'a, T> {
    pub fn get(&self) -> &T {
        match self {
            MaybeMut::Immutable(r) => r,
            MaybeMut::Mutable(r) => r,
        }
    }

    pub fn get_mut(&mut self) -> Option<&mut T> {
        match self {
            MaybeMut::Mutable(r) => Some(r),
            _ => None,
        }
    }
}

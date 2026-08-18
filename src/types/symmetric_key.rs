use std::hash::Hash;

#[derive(Debug, Clone, Copy)]
pub struct SymmetricKey<T>(pub T, pub T);

impl<T: Eq> PartialEq for SymmetricKey<T> {
    fn eq(&self, other: &Self) -> bool {
        (self.0 == other.0 && self.1 == other.1) || (self.0 == other.1 && self.1 == other.0)
    }
}

impl<T: Eq> Eq for SymmetricKey<T> {}

impl<T: Hash + Ord> Hash for SymmetricKey<T> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        let (min, max) = if self.0 < self.1 {
            (&self.0, &self.1)
        } else {
            (&self.1, &self.0)
        };
        min.hash(state);
        max.hash(state);
    }
}

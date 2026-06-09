pub enum UniRef<'a, T> {
    Mut(&'a mut T),
    Ref(&'a T),
    Loc(T),
    Int,
}

impl<'a, T> UniRef<'a, T> {
    pub fn get_mut(&mut self) -> Option<&mut T> {
        match self {
            UniRef::Mut(r) => Some(r),
            UniRef::Loc(r) => Some(r),
            _ => None,
        }
    }

    pub fn get_ref(&self) -> Option<&T> {
        match self {
            UniRef::Mut(r) => Some(r),
            UniRef::Ref(r) => Some(r),
            UniRef::Loc(r) => Some(r),
            _ => None,
        }
    }
}

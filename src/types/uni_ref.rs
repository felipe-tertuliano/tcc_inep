#[derive(Debug)]
pub enum UniRef<'a, T> {
    Mut(&'a mut T),
    Ref(&'a T),
    Loc(T),
    Int,
}

impl<'a, T: Clone> Clone for UniRef<'a, T> {
    fn clone(&self) -> Self {
        match self {
            Self::Mut(_) => panic!("cannot clone a UniRef::Mut"),
            Self::Ref(r) => Self::Ref(r.clone()),
            Self::Loc(r) => Self::Loc(r.clone()),
            Self::Int => Self::Int,
        }
    }
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

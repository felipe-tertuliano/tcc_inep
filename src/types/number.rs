pub enum Number {
    Int(i64),
    Unsigned(u64),
    Float(f64),
}

impl Number {
    pub fn int() -> Self {
        Number::Int(0)
    }

    pub fn unsigned() -> Self {
        Number::Unsigned(0)
    }

    pub fn float() -> Self {
        Number::Float(0.0)
    }

    pub fn op<FI, FU, FF>(&self, i_op: FI, u_op: FU, f_op: FF) -> Self
    where
        FI: FnOnce(&i64) -> Self,
        FU: FnOnce(&u64) -> Self,
        FF: FnOnce(&f64) -> Self,
    {
        match self {
            Self::Int(num) => i_op(num),
            Self::Unsigned(num) => u_op(num),
            Self::Float(num) => f_op(num),
        }
    }
}

impl Clone for Number {
    fn clone(&self) -> Self {
        match self {
            Self::Int(num) => Self::Int(*num),
            Self::Unsigned(num) => Self::Unsigned(*num),
            Self::Float(num) => Self::Float(*num),
        }
    }
}

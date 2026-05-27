use std::marker::PhantomData;

pub struct DataItem<'a> {
    _marker: PhantomData<&'a ()>,
}

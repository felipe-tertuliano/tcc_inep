use std::{collections::HashMap, marker::PhantomData};

pub struct DataItem<'a> {
    _marker: PhantomData<&'a ()>,
    _hash: HashMap<String, String>,
}

impl<'a> DataItem<'a> {
    pub fn new(hash: HashMap<String, String>) -> Self {
        Self { _marker: PhantomData, _hash: hash }
    }

    // pub fn get<T: FromStr>(&self, name: &str) -> Option<T> {
    //     let value = self._hash.get(name);
    // }
}

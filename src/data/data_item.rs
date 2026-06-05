use super::DataHeader;
use crate::types::MaybeMut;
use std::str::FromStr;

pub struct DataItem<'a> {
    _header: MaybeMut<'a, DataHeader>,
    _value: Vec<String>,
}

impl<'a> DataItem<'a> {
    pub fn new(header: MaybeMut<'a, DataHeader>, value: Vec<String>) -> Self {
        Self {
            _header: header,
            _value: value,
        }
    }

    pub fn get<T: FromStr>(&self, name: &str) -> Option<T> {
        self._header
            .get()
            .get(name)
            .and_then(|i| self._value[*i].parse::<T>().ok())
    }
}

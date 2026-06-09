use super::DataHeader;
use crate::types::UniRef;
use std::{fmt::Display, str::FromStr};

pub struct DataItem<'a> {
    _header: UniRef<'a, DataHeader>,
    _value: Vec<String>,
}

impl<'a> DataItem<'a> {
    pub fn new(header: UniRef<'a, DataHeader>, value: Vec<String>) -> Self {
        Self {
            _header: match header {
                UniRef::Mut(h) => UniRef::Ref(h),
                UniRef::Ref(h) => UniRef::Ref(h),
                UniRef::Loc(h) => UniRef::Loc(h),
                UniRef::Int => UniRef::Loc(DataHeader::new()),
            },
            _value: value,
        }
    }

    pub fn set<T: Display>(&mut self, name: &str, value: T) -> Option<T> {
        if let Some(h) = self._header.get_mut() {
            let pos;
            if let Some(v) = h.get(name) {
                pos = *v;
            } else {
                pos = h.iter()
                    .fold(0, |acc, (_, v)| if *v < acc { acc } else { *v }) + 1;
                h.insert(name.to_string(), pos);
            }
            while self._value.len() - 1 < pos {
                self._value.push(String::new());
            }
            self._value[pos] = value.to_string();
            Some(value)
        } else {
            None
        }
    }

    pub fn get<T: FromStr>(&self, name: &str) -> Option<T> {
        self._header
            .get_ref()
            .and_then(|h| h.get(name).and_then(|i| self._value[*i].parse::<T>().ok()))
    }

    pub fn get_header(&self) -> Option<&DataHeader> {
        self._header.get_ref()
    }
}

impl Display for DataItem<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self._value.join(";"))
    }
}

impl From<DataItem<'_>> for String {
    fn from(val: DataItem) -> Self {
        val.to_string()
    }
}

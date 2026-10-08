use super::DataHeader;
use crate::types::UniRef;
use std::{collections::HashMap, fmt::Display, str::FromStr};

#[derive(Clone, Debug)]
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

    pub fn set<S, T>(&mut self, name: S, value: T) -> Option<T>
    where 
        T: Display,
        S: AsRef<str> + ToString
    {
        if let Some(h) = self._header.get_mut() {
            let pos;
            if let Some(v) = h.get(&name.to_string()) {
                pos = *v;
            } else {
                pos = if h.is_empty() {
                    0
                } else {
                    h.iter()
                        .fold(0, |acc, (_, v)| if *v < acc { acc } else { *v })
                        + 1
                };
                h.insert(name.to_string(), pos);
            }
            if self._value.is_empty() {
                self._value.push(String::new());
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

    pub fn get<S, T>(&self, name: S) -> Option<T>
    where 
        T: FromStr,
        S: AsRef<str> + ToString
    {
        self._header
            .get_ref()
            .and_then(|h| h.get(&name.to_string()).and_then(|i| self._value[*i].parse::<T>().ok()))
    }

    pub fn get_header(&self) -> Option<&DataHeader> {
        self._header.get_ref()
    }

    pub fn get_value(&self) -> &Vec<String> {
        &self._value
    }

    pub fn to_vec(&self) -> Option<Vec<(String, String)>> {
        self.get_header()
            .map(|h| h.iter().map(|(k, v)| (v, k)).collect::<HashMap<_, _>>())
            .map(|header| {
                self._value
                    .iter()
                    .enumerate()
                    .map(|(i, v)| (header.get(&i).unwrap().to_owned().clone(), v.to_owned()))
                    .collect::<Vec<_>>()
            })
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

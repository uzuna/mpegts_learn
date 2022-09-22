//! BER encoding parser

use std::{borrow::Cow, fmt::Debug, marker::PhantomData};

#[cfg(feature = "uasdms")]
pub mod uasdms;

/// KLVパース時に発生するエラーについて
#[derive(Debug)]
pub enum ParseError {
    // 定義にないIDの場合
    UndefinedID(u8),
    // KLV形式を満たさない場合
    LessLength,
    // キーに対応する長さがあるため、それを満たさない場合のエラー
    UnexpectLength(usize),
    // 渡された値が不正値などでパースできない時に返す
    // 'aだとparse()の戻りでライフタイムが足りなくなるので'staticとする
    ValueError(Cow<'static, str>),
}

pub struct KLVRaw<'buf>(&'buf [u8]);

impl<'buf> KLVRaw<'buf> {
    pub fn from_bytes(buf: &'buf [u8]) -> Self {
        Self(buf)
    }
    pub fn key(&self) -> u8 {
        self.0[0]
    }
    #[inline]
    fn len(&self) -> usize {
        self.0[1] as usize
    }
    pub fn value(&self) -> &'buf [u8] {
        &self.0[2..2 + self.len()]
    }
}

impl<'buf> Debug for KLVRaw<'buf> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "KLVRaw key: {}, len: {} value {:?}",
            self.key(),
            self.len(),
            self.value()
        )
    }
}

pub struct KLVRawReader<'buf> {
    buf: &'buf [u8],
    current: usize,
}

impl<'buf> KLVRawReader<'buf> {
    pub fn from_bytes(buf: &'buf [u8]) -> Self {
        Self { buf, current: 0 }
    }
}

impl<'buf> Iterator for KLVRawReader<'buf> {
    type Item = KLVRaw<'buf>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.current >= self.buf.len() {
            return None;
        }
        let current = self.current;
        let len = self.buf[current + 1] as usize;
        self.current = current + 2 + len;
        Some(KLVRaw(&self.buf[current..self.current]))
    }
}

pub trait DataSet {
    type Item;
    fn from_byte(b: u8) -> Option<Self>
    where
        Self: std::marker::Sized;
    fn value(&self, v: &[u8]) -> Result<Self::Item, ParseError>;
    fn expect_length(&self, _len: usize) -> bool {
        true
    }
}

pub struct KLV<'buf, K> {
    buf: &'buf [u8],
    _phantom: PhantomData<K>,
}

impl<'buf, K: DataSet> KLV<'buf, K> {
    pub fn from_bytes(buf: &'buf [u8]) -> Self {
        Self {
            buf,
            _phantom: PhantomData,
        }
    }
    pub fn try_from_bytes(buf: &'buf [u8]) -> Result<Self, ParseError> {
        if buf.len() < 3 || buf.len() < buf[1] as usize {
            Err(ParseError::LessLength)
        } else {
            Ok(Self::from_bytes(buf))
        }
    }
    pub fn key(&self) -> Result<K, ParseError> {
        if let Some(key) = K::from_byte(self.buf[0]) {
            Ok(key)
        } else {
            Err(ParseError::UndefinedID(self.buf[0]))
        }
    }
    #[inline]
    pub fn len(&self) -> usize {
        self.buf[1] as usize
    }
    pub fn is_empty(&self) -> bool {
        self.buf.is_empty()
    }
    #[inline]
    pub fn value(&self) -> &'buf [u8] {
        &self.buf[2..2 + self.len()]
    }
    pub fn parse(&self) -> Result<K::Item, ParseError> {
        match self.key() {
            Ok(key) => {
                if !key.expect_length(self.len()) {
                    Err(ParseError::UnexpectLength(self.len()))
                } else {
                    key.value(self.value())
                }
            }
            Err(x) => Err(x),
        }
    }
}

pub struct KLVReader<'buf, K> {
    buf: &'buf [u8],
    current: usize,
    _phantom: PhantomData<K>,
}

impl<'buf, K> KLVReader<'buf, K> {
    pub fn from_bytes(buf: &'buf [u8]) -> Self {
        Self {
            buf,
            current: 0,
            _phantom: PhantomData,
        }
    }
}

impl<'buf, K: DataSet> Iterator for KLVReader<'buf, K> {
    type Item = KLV<'buf, K>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.current >= self.buf.len() {
            return None;
        }
        let current = self.current;
        let len = self.buf[current + 1] as usize;
        self.current = current + 2 + len;
        Some(KLV::from_bytes(&self.buf[current..self.current]))
    }
}

#[cfg(test)]
mod tests {
    use crate::KLVReader;

    use super::{DataSet, KLVRawReader, ParseError};

    #[test]
    fn test_iterator() {
        let expects: Vec<(u8, usize)> = vec![(1, 1), (2, 4), (3, 2)];
        let buf = vec![1, 1, 0, 2, 4, 1, 2, 3, 4, 3, 2, 1, 2];
        let r = KLVRawReader::from_bytes(&buf);
        for (i, v) in r.enumerate() {
            assert_eq!(expects[i].0, v.key());
            assert_eq!(expects[i].1, v.len());
            // println!("{:?}", v);
        }
    }

    #[derive(Debug, PartialEq, Eq)]
    enum DummyValue {
        U8(u8),
    }

    #[derive(Debug, PartialEq, Eq)]
    enum DummyDataset {
        One,
        Two,
    }
    impl DataSet for DummyDataset {
        type Item = DummyValue;
        fn from_byte(b: u8) -> Option<Self> {
            match b {
                1 => Some(Self::One),
                2 => Some(Self::Two),
                _ => None,
            }
        }
        fn value(&self, v: &[u8]) -> Result<Self::Item, ParseError> {
            let v = match self {
                DummyDataset::One => DummyValue::U8(v[0]),
                DummyDataset::Two => DummyValue::U8(v[0]),
            };
            Ok(v)
        }
        fn expect_length(&self, len: usize) -> bool {
            match self {
                DummyDataset::One => len == 1,
                DummyDataset::Two => len == 2,
            }
        }
    }

    #[test]
    fn test_klv() {
        let expects = vec![
            (DummyDataset::One, DummyValue::U8(0)),
            (DummyDataset::Two, DummyValue::U8(13)),
        ];
        let buf = vec![1, 1, 0, 2, 2, 13, 45];
        let r = KLVReader::<DummyDataset>::from_bytes(&buf);

        for (i, v) in r.enumerate() {
            assert_eq!(expects[i].0, v.key().unwrap());
            assert_eq!(expects[i].1, v.parse().unwrap());
        }
    }
}

use std::{fmt::Debug, marker::PhantomData};

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

trait DataSet {
    type Item;
    fn from_byte(b: u8) -> Option<Self>
    where
        Self: std::marker::Sized;
    fn value(&self, v: &[u8]) -> Self::Item;
}

struct KLV<'buf, K> {
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
    pub fn key(&self) -> Option<K> {
        K::from_byte(self.buf[0])
    }
    #[inline]
    fn len(&self) -> usize {
        self.buf[1] as usize
    }
    pub fn value(&self) -> Option<K::Item> {
        self.key()
            .map(|key| key.value(&self.buf[2..2 + self.len()]))
    }
}

#[cfg(test)]
mod tests {
    use super::{DataSet, KLVRawReader, KLV};

    #[test]
    fn test_iterator() {
        let buf = vec![1, 1, 0, 2, 4, 1, 2, 3, 4, 3, 2, 1, 2];
        let r = KLVRawReader::from_bytes(&buf);
        for v in r {
            println!("{:?}", v);
        }
    }

    #[derive(Debug)]
    enum DummyValue {
        U8(u8),
    }

    #[derive(Debug)]
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
        fn value(&self, v: &[u8]) -> Self::Item {
            match self {
                DummyDataset::One => DummyValue::U8(v[0]),
                DummyDataset::Two => DummyValue::U8(v[0]),
            }
        }
    }

    #[test]
    fn test_klv() {
        let buf = vec![1, 1, 0];
        let v = KLV::<DummyDataset>::from_bytes(&buf);

        println!("debug {:?} {:?}", v.key(), v.value());
    }
}

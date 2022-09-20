use std::fmt::Debug;

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

// pub struct KLV<'buf, T> {
//     buf: &'buf [u8],
//     _phantom: PhantomData<T>
// }

#[cfg(test)]
mod tests {
    use super::KLVRawReader;

    #[test]
    fn test_iterator() {
        let buf = vec![1, 1, 0, 2, 4, 1, 2, 3, 4, 3, 2, 1, 2];
        let r = KLVRawReader::from_bytes(&buf);
        for v in r {
            println!("{:?}", v);
        }
    }
}

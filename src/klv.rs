pub struct KLV<'buf>(&'buf [u8]);

impl<'buf> KLV<'buf> {
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

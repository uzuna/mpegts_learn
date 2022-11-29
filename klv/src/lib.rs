//! BER encoding parser

use std::{borrow::Cow, fmt::Debug, io::Write, marker::PhantomData};

use byteorder::ByteOrder;
use value::Value;

#[cfg(feature = "testserde")]
pub mod nk;
#[cfg(feature = "uasdls")]
pub mod uasdls;
#[cfg(feature = "value")]
pub mod value;

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

pub struct KLVGlobal<'buf>(&'buf [u8]);

impl<'buf> KLVGlobal<'buf> {
    const KEY_LENGHT: usize = 16;
    const MINIMUM_LENGHT: usize = 18;
    pub fn try_from_bytes(buf: &'buf [u8]) -> Result<Self, ParseError> {
        if buf.len() < Self::MINIMUM_LENGHT {
            Err(ParseError::LessLength)
        } else {
            Ok(Self::from_bytes(buf))
        }
    }
    pub fn from_bytes(buf: &'buf [u8]) -> Self {
        Self(buf)
    }

    #[inline]
    pub fn key(&self) -> &[u8] {
        &self.0[..Self::KEY_LENGHT]
    }

    pub fn key_is(&self, key: &[u8]) -> bool {
        key == self.key()
    }
    // return start to end
    fn content_range(&self) -> Option<(usize, usize)> {
        use byteorder::BigEndian;
        match LengthOctet::from_u8(self.0[16]) {
            LengthOctet::Short(x) => Some((17, 17 + x as usize)),
            LengthOctet::Long(x) => match x {
                1 => Some((18, 18 + self.0[17] as usize)),
                2 => Some((19, 19 + BigEndian::read_u16(&self.0[17..19]) as usize)),
                4 => Some((21, 21 + BigEndian::read_u32(&self.0[17..21]) as usize)),
                _ => None,
            },
            LengthOctet::Indefinite | LengthOctet::Reserved => None,
        }
    }
    pub fn content(&self) -> &'buf [u8] {
        if let Some((start, end)) = self.content_range() {
            &self.0[start..end]
        } else {
            &self.0[18..]
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
enum LengthOctet {
    Short(u8),
    Indefinite,
    Long(u8),
    Reserved,
}

impl LengthOctet {
    const FIRST_BIT: u8 = 0b1000_0000;
    const BIT_MASK: u8 = 0b0111_1111;
    fn from_u8(b: u8) -> Self {
        if b & Self::FIRST_BIT != Self::FIRST_BIT {
            Self::Short(b & Self::BIT_MASK)
        } else if b == 255 {
            Self::Reserved
        } else if b == 128 {
            Self::Indefinite
        } else {
            Self::Long(b & Self::BIT_MASK)
        }
    }

    pub fn encode_len(size: usize) -> usize {
        if size <= 127 {
            1
        } else if size <= u8::MAX as usize {
            2
        } else if size <= u16::MAX as usize {
            3
        } else {
            5
        }
    }

    pub fn length_to_buf(buf: &mut dyn std::io::Write, size: usize) -> std::io::Result<usize> {
        use byteorder::BigEndian;
        if size <= 127 {
            buf.write(&[size as u8])
        } else if size <= u8::MAX as usize {
            buf.write(&[0b1000_0001, size as u8])
        } else if size <= u16::MAX as usize {
            let mut r = [0b1000_0010, 0, 0];
            BigEndian::write_u16(&mut r[1..], size as u16);
            buf.write(&r)
        } else {
            let mut r = [0b1000_0100, 0, 0, 0, 0];
            BigEndian::write_u32(&mut r[1..], size as u32);
            buf.write(&r)
        }
    }
}

#[cfg(feature = "value")]
pub fn encode<K: DataSet>(
    mut buf: &mut [u8],
    records: &[(K, Value)],
) -> Result<usize, std::io::Error> {
    let mut size = 0;
    size += buf.write(K::key())?;
    let content_len = contents_len(records);
    size += LengthOctet::length_to_buf(&mut buf, content_len)?;
    size += content_len;
    for (key, value) in records {
        let _size = buf.write(&[key.as_byte(), value.len() as u8])?;
        value.to_bytes(&mut buf)?;
    }
    Ok(size)
}

#[cfg(feature = "value")]
pub fn encode_len<K>(records: &[(K, Value)]) -> usize {
    let mut contents_len = contents_len(records);
    contents_len += 16; // HEADER
    contents_len + LengthOctet::encode_len(contents_len) // length
}

#[cfg(feature = "value")]
fn contents_len<K>(records: &[(K, Value)]) -> usize {
    records
        .iter()
        .fold(0_usize, |size, (_, v)| size + 2 + v.len())
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
    pub fn content(&self) -> &'buf [u8] {
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
            self.content()
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
    fn key() -> &'static [u8];
    fn from_byte(b: u8) -> Option<Self>
    where
        Self: TryFrom<u8>,
    {
        if let Ok(x) = Self::try_from(b) {
            Some(x)
        } else {
            None
        }
    }
    fn as_byte(&self) -> u8;
    fn value(&self, v: &[u8]) -> Result<Self::Item, ParseError>;
    fn expect_length(&self, _len: usize) -> bool {
        true
    }
}

/// expect short encoding format.
/// key and length is 1byte
pub struct KLV<'buf, K> {
    buf: &'buf [u8],
    _phantom: PhantomData<K>,
}

impl<'buf, K: DataSet + TryFrom<u8>> KLV<'buf, K> {
    const MINIMUM_LENGHT: usize = 3;
    pub fn from_bytes(buf: &'buf [u8]) -> Self {
        Self {
            buf,
            _phantom: PhantomData,
        }
    }
    pub fn try_from_bytes(buf: &'buf [u8]) -> Result<Self, ParseError> {
        if buf.len() < Self::MINIMUM_LENGHT || buf.len() < buf[1] as usize {
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
    pub fn content(&self) -> &'buf [u8] {
        &self.buf[2..2 + self.len()]
    }
    pub fn parse(&self) -> Result<K::Item, ParseError> {
        match self.key() {
            Ok(key) => {
                if !key.expect_length(self.len()) {
                    Err(ParseError::UnexpectLength(self.len()))
                } else {
                    key.value(self.content())
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

impl<'buf, K: DataSet + TryFrom<u8>> Iterator for KLVReader<'buf, K> {
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

    use byteorder::ByteOrder;

    use super::{DataSet, KLVRawReader, ParseError};
    use crate::{encode, encode_len, value::Value, KLVGlobal, KLVReader, LengthOctet};

    #[test]
    fn test_length_octets() {
        use LengthOctet::*;
        let td = [
            (0, Short(0)),
            (0b0000_0001, Short(1)),
            (0b0111_1111, Short(127)),
            (0b1000_0000, Indefinite),
            (0b1000_0001, Long(1)),
            (0b1000_0010, Long(2)),
            (0b1111_1111, Reserved),
        ];

        for (b, expect) in td {
            let lo = LengthOctet::from_u8(b);
            assert_eq!(lo, expect);
        }
    }

    #[test]
    fn test_klb_global_range() {
        // (dummy content length, range)
        let td = [
            // SHORT
            (1_usize, (17_usize, 18_usize)),
            (10, (17, 17 + 10)),
            (127, (17, 17 + 127)),
            // LONG(1)
            (128, (18, 18 + 128)),
            (255, (18, 18 + 255)),
            // LONG(2)
            (256, (19, 19 + 256)),
            (65535, (19, 19 + 65535)),
            (255, (18, 18 + 255)),
            // LONG(4)
            (65536, (21, 21 + 65536)),
        ];

        for (size, expect) in td {
            let mut buf = vec![0; 16];
            LengthOctet::length_to_buf(&mut buf, size).unwrap();
            buf.extend_from_slice(&vec![0xff; size]);
            let lo = KLVGlobal::from_bytes(&buf);
            assert_eq!(lo.content_range().unwrap(), expect);
        }
    }

    #[test]
    fn test_iterator() {
        let expects: Vec<(u8, usize)> = vec![(1, 1), (2, 4), (3, 2)];
        let buf = vec![1, 1, 0, 2, 4, 1, 2, 3, 4, 3, 2, 1, 2];
        let r = KLVRawReader::from_bytes(&buf);
        for (i, v) in r.enumerate() {
            assert_eq!(expects[i].0, v.key());
            assert_eq!(expects[i].1, v.len());
        }
    }

    #[repr(u8)]
    #[derive(Debug, PartialEq, Eq, Clone, Copy)]
    enum DummyDataset {
        One = 1,
        Two = 2,
    }
    impl DummyDataset {
        const KEY: &str = "dummydataset0000";
    }

    impl TryFrom<u8> for DummyDataset {
        type Error = ();

        fn try_from(value: u8) -> Result<Self, Self::Error> {
            match value {
                1 => Ok(Self::One),
                2 => Ok(Self::Two),
                _ => Err(()),
            }
        }
    }

    impl DataSet for DummyDataset {
        type Item = Value;

        fn value(&self, v: &[u8]) -> Result<Self::Item, ParseError> {
            let v = match self {
                DummyDataset::One => Self::Item::U8(v[0]),
                DummyDataset::Two => Self::Item::U16(byteorder::BigEndian::read_u16(v)),
            };
            Ok(v)
        }

        fn expect_length(&self, len: usize) -> bool {
            match self {
                DummyDataset::One => len == 1,
                DummyDataset::Two => len == 2,
            }
        }

        fn as_byte(&self) -> u8 {
            *self as u8
        }

        fn key() -> &'static [u8] {
            Self::KEY.as_bytes()
        }
    }

    #[test]
    fn test_klv() {
        let expects = vec![
            (DummyDataset::One, Value::U8(0)),
            (DummyDataset::Two, Value::U16(13)),
        ];
        let buf = vec![1, 1, 0, 2, 2, 0, 13];
        let r = KLVReader::<DummyDataset>::from_bytes(&buf);

        for (i, v) in r.enumerate() {
            assert_eq!(expects[i].0, v.key().unwrap());
            assert_eq!(expects[i].1, v.parse().unwrap());
        }
    }

    #[test]
    fn test_klv_write() {
        let records = vec![
            (DummyDataset::One, Value::U8(127)),
            (DummyDataset::Two, Value::U16(42)),
        ];
        println!("encode_len {}", encode_len(&records));
        let mut content = vec![0; encode_len(&records)];
        let size = encode(&mut content, &records).unwrap();
        assert_eq!(content.len(), size);

        // decode
        let klvg = KLVGlobal::from_bytes(&content);
        assert_eq!(klvg.content(), &content[17..]);
        let r = KLVReader::<DummyDataset>::from_bytes(&klvg.content());

        for (id, record) in r.enumerate() {
            assert_eq!(record.key().unwrap(), records[id].0);
            assert_eq!(record.parse().unwrap(), records[id].1);
        }
    }
}

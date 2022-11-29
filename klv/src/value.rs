use std::{
    io::Write,
    time::{Duration, SystemTime},
};

use byteorder::{BigEndian, ByteOrder};

use crate::ParseError;

#[derive(Debug, PartialEq, Eq)]
pub enum Value {
    U8(u8),
    U16(u16),
    U32(u32),
    U64(u64),
    I8(i8),
    I16(i16),
    I32(i32),
    I64(i64),
    String(String),
    Timestamp(SystemTime),
    Duration(Duration),
}

impl From<u8> for Value {
    fn from(x: u8) -> Self {
        Value::U8(x)
    }
}

impl Value {
    pub fn as_i8(x: &[u8]) -> Self {
        Self::I8(x[0] as i8)
    }
    pub fn as_i16(x: &[u8]) -> Self {
        Self::I16(BigEndian::read_i16(x))
    }
    pub fn as_i32(x: &[u8]) -> Self {
        Self::I32(BigEndian::read_i32(x))
    }
    pub fn as_i64(x: &[u8]) -> Self {
        Self::I64(BigEndian::read_i64(x))
    }
    pub fn as_u8(x: &[u8]) -> Self {
        Self::U8(x[0])
    }
    pub fn as_u16(x: &[u8]) -> Self {
        Self::U16(BigEndian::read_u16(x))
    }
    pub fn as_u32(x: &[u8]) -> Self {
        Self::U32(BigEndian::read_u32(x))
    }
    pub fn as_u64(x: &[u8]) -> Self {
        Self::U64(BigEndian::read_u64(x))
    }
    pub fn as_string(x: &[u8]) -> Self {
        Self::String(String::from_utf8(x.to_owned()).unwrap())
    }
    pub fn as_timestamp(x: &[u8]) -> Result<Self, ParseError> {
        let micros = BigEndian::read_u64(x);
        match SystemTime::UNIX_EPOCH.checked_add(Duration::from_micros(micros)) {
            Some(ts) => Ok(Self::Timestamp(ts)),
            None => Err(ParseError::ValueError("failed to parse timestamp.".into())),
        }
    }
    pub fn as_duration(x: &[u8]) -> Self {
        let secs = BigEndian::read_u64(x);
        let nanos = BigEndian::read_u32(&x[8..]);
        Self::Duration(Duration::new(secs, nanos))
    }

    pub fn to_bytes<W: Write>(&self, mut buf: W) -> std::io::Result<usize> {
        use byteorder::WriteBytesExt;
        use Value::*;
        match self {
            U8(x) => buf.write(&[*x]),
            U16(x) => buf.write_u16::<BigEndian>(*x).map(|_| 2),
            U32(x) => buf.write_u32::<BigEndian>(*x).map(|_| 4),
            U64(x) => buf.write_u64::<BigEndian>(*x).map(|_| 8),
            I8(x) => buf.write(&[*x as u8]),
            I16(x) => buf.write_i16::<BigEndian>(*x).map(|_| 2),
            I32(x) => buf.write_i32::<BigEndian>(*x).map(|_| 4),
            I64(x) => buf.write_i64::<BigEndian>(*x).map(|_| 8),
            String(s) => buf.write(s.as_bytes()),
            Timestamp(x) => {
                let micros = x
                    .duration_since(SystemTime::UNIX_EPOCH)
                    .unwrap()
                    .as_micros();
                buf.write_u64::<BigEndian>(micros as u64).map(|_| 8)
            }
            Duration(x) => {
                buf.write_u64::<BigEndian>(x.as_secs())?;
                buf.write_u32::<BigEndian>(x.subsec_nanos()).map(|_| 12)
            }
        }
    }

    #[allow(clippy::len_without_is_empty)]
    pub fn len(&self) -> usize {
        match self {
            Value::U8(_) | Value::I8(_) => 1,
            Value::U16(_) | Value::I16(_) => 2,
            Value::U32(_) | Value::I32(_) => 4,
            Value::U64(_) | Value::I64(_) => 8,
            Value::String(x) => x.len(),
            Value::Timestamp(_) => 8,
            Value::Duration(_) => 12,
        }
    }
}

#[cfg(test)]
mod tests {
    use std::time::{Duration, SystemTime};

    use super::Value;

    #[test]
    fn test_value_encode_decode() {
        let td = [
            Value::U8(0),
            Value::U8(255),
            Value::U16(256),
            Value::U32(192),
            Value::U64(u64::MAX),
            Value::I8(-127),
            Value::I16(-127),
            Value::I32(-192),
            Value::I64(i64::MIN),
            Value::String("EON_$JK)~DFKSDF".to_owned()),
            Value::Timestamp(SystemTime::now()),
            Value::Duration(Duration::new(1234, 5678)),
        ];
        for x in td {
            let mut buf = vec![];
            let size = x.to_bytes(&mut buf).unwrap();
            assert_eq!(buf.len(), size, "value {:?} {:?} ", x, buf);

            match x {
                Value::U8(x) => {
                    assert_eq!(Value::U8(x), Value::from(buf[0]));
                }
                Value::U16(x) => {
                    assert_eq!(Value::U16(x), Value::as_u16(&buf));
                }
                Value::U32(x) => {
                    assert_eq!(Value::U32(x), Value::as_u32(&buf));
                }
                Value::U64(x) => {
                    assert_eq!(Value::U64(x), Value::as_u64(&buf));
                }
                Value::I8(x) => {
                    assert_eq!(Value::I8(x), Value::as_i8(&buf));
                }
                Value::I16(x) => {
                    assert_eq!(Value::I16(x), Value::as_i16(&buf));
                }
                Value::I32(x) => {
                    assert_eq!(Value::I32(x), Value::as_i32(&buf));
                }
                Value::I64(x) => {
                    assert_eq!(Value::I64(x), Value::as_i64(&buf));
                }
                Value::String(x) => {
                    assert_eq!(Value::String(x), Value::as_string(&buf));
                }
                Value::Timestamp(x) => {
                    if let Value::Timestamp(y) = Value::as_timestamp(&buf).unwrap() {
                        assert_eq!(
                            x.duration_since(SystemTime::UNIX_EPOCH)
                                .unwrap()
                                .as_micros(),
                            y.duration_since(SystemTime::UNIX_EPOCH)
                                .unwrap()
                                .as_micros()
                        );
                    }
                }
                Value::Duration(x) => {
                    assert_eq!(Value::Duration(x), Value::as_duration(&buf));
                }
            }
        }
    }
}

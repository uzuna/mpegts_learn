use std::{
    io::Cursor,
    time::{Duration, SystemTime},
};

use byteorder::{BigEndian, ReadBytesExt};

use crate::klv::KLVRaw;

#[derive(Debug)]
enum Value {
    Timestamp(SystemTime),
}

#[derive(Debug)]
enum ParseError {
    UndefinedID(u8),
    LessLength,
}

#[derive(Debug)]
enum UASDataset {
    Timestamp,
}

impl UASDataset {
    fn from_id(klv: &KLVRaw) -> Result<UASDataset, ParseError> {
        match klv.key() {
            2 => Ok(UASDataset::Timestamp),
            x => Err(ParseError::UndefinedID(x)),
        }
    }
    fn parse(&self, klv: &KLVRaw) -> Result<Value, ParseError> {
        match self {
            UASDataset::Timestamp => {
                let mut rdr = Cursor::new(klv.value());
                let micros = rdr.read_u64::<BigEndian>().unwrap();
                let ts = SystemTime::UNIX_EPOCH
                    .checked_add(Duration::from_micros(micros))
                    .unwrap();
                Ok(Value::Timestamp(ts))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{KLVRaw, UASDataset, Value};
    use chrono::{DateTime, Utc};

    #[test]
    fn test_buffer() {
        let buf = vec![1, 4, 1, 2, 0, 0];

        let klv = KLVRaw::from_bytes(&buf);
        assert_eq!(klv.key(), 1);
        assert_eq!(klv.value(), &buf[2..])
    }

    #[test]
    fn test_uas_datalink_ls() {
        let buf = vec![2, 8, 0, 0x4, 0x6c, 0x8e, 0x20, 0x03, 0x83, 0x85];

        let klv = KLVRaw::from_bytes(&buf);
        let ls = UASDataset::from_id(&klv).unwrap();
        let value = ls.parse(&klv).unwrap();

        match value {
            Value::Timestamp(ts) => {
                let datetime: DateTime<Utc> = ts.into();
                assert_eq!(
                    DateTime::parse_from_rfc3339("2009-06-17T16:53:05.099653+00:00").unwrap(),
                    datetime
                );
            }
        }
    }
}

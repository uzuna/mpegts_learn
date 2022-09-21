use std::{
    io::Cursor,
    time::{Duration, SystemTime},
};

use byteorder::{BigEndian, ReadBytesExt};

use crate::klv::{DataSet, ParseError};

#[derive(Debug, PartialEq)]
enum Value {
    Timestamp(SystemTime),
    U8(u8),
    U16(u16),
    I16(i16),
}

#[derive(Debug, PartialEq, Eq)]
enum UASDataset {
    Timestamp,
    LSVersionNumber,
    // Relative between longitudinal axis and True North measured in the horizontal plane.
    // Map 0..(2^16-1) to 0..360.
    // Resolution: ~5.5 milli degrees.
    PlatformHeadingAngle,
    // Angle between longitudinal axis and horizontal plane.
    // Positive angles above horizontal plane.
    // Map -(2^15-1)..(2^15-1) to +/-20.
    // Use -(2^15) as "out of range" indicator. -(2^15) = 0x8000.
    // Resolution: ~610 micro degrees.
    PlatformPitchAngle,
    // Angle between transverse axis and transvers-longitudinal plane.
    // Positive angles for lowered right wing.
    // Map (-2^15-1)..(2^15-1) to +/-50.
    // Use -(2^15) as "out of range" indicator. -(2^15) = 0x8000.
    // Res: ~1525 micro deg.
    PlatformRollAngle,
}

impl DataSet for UASDataset {
    type Item = Value;

    fn from_byte(b: u8) -> Option<Self>
    where
        Self: std::marker::Sized,
    {
        match b {
            2 => Some(UASDataset::Timestamp),
            5 => Some(UASDataset::PlatformHeadingAngle),
            6 => Some(UASDataset::PlatformPitchAngle),
            7 => Some(UASDataset::PlatformRollAngle),
            65 => Some(UASDataset::LSVersionNumber),
            _ => None,
        }
    }

    fn value(&self, v: &[u8]) -> Result<Self::Item, ParseError> {
        match self {
            UASDataset::Timestamp => {
                let mut rdr = Cursor::new(v);
                let micros = rdr.read_u64::<BigEndian>().unwrap();
                match SystemTime::UNIX_EPOCH.checked_add(Duration::from_micros(micros)) {
                    Some(ts) => Ok(Value::Timestamp(ts)),
                    None => Err(ParseError::ValueError("failed to parse timestamp.".into())),
                }
            }
            // TODO Change value to Degrees
            UASDataset::PlatformHeadingAngle => {
                let mut rdr = Cursor::new(v);
                let angle = rdr.read_u16::<BigEndian>().unwrap();
                Ok(Value::U16(angle))
            }
            UASDataset::PlatformPitchAngle | UASDataset::PlatformRollAngle => {
                let mut rdr = Cursor::new(v);
                let angle = rdr.read_i16::<BigEndian>().unwrap();
                Ok(Value::I16(angle))
            }
            UASDataset::LSVersionNumber => Ok(Value::U8(v[0])),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::klv::KLVReader;

    use super::{UASDataset, Value};
    use chrono::{DateTime, Utc};

    #[test]
    fn test_uas_datalink_ls() {
        #[rustfmt::skip]
        let buf = vec![
            2, 8, 0, 0x4, 0x6c, 0x8e, 0x20, 0x03, 0x83, 0x85,
            65, 1, 1,
            5, 2, 0x3d, 0x3b,
            6, 2, 0x15, 0x80,
            7, 2, 0x01, 0x52,
            ];

        let klv = KLVReader::<UASDataset>::from_bytes(&buf);

        for x in klv {
            let key = x.key().unwrap();
            match (key, x.parse()) {
                (UASDataset::Timestamp, Ok(Value::Timestamp(ts))) => {
                    let datetime: DateTime<Utc> = ts.into();
                    assert_eq!(
                        DateTime::parse_from_rfc3339("2009-06-17T16:53:05.099653+00:00").unwrap(),
                        datetime
                    );
                }
                (UASDataset::LSVersionNumber, Ok(Value::U8(version))) => {
                    assert_eq!(version, 1);
                }
                (UASDataset::PlatformHeadingAngle, Ok(Value::U16(angle))) => {
                    assert_eq!(angle, 15675);
                }
                (k, v) => {
                    println!("debug {:?} {:?}", k, v)
                }
            }
        }
    }
}

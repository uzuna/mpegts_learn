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
    I32(i32),
    String(String),
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
    ImageSourceSensor,
    ImageCoordinateSensor,
    SensorLatitude,
    SensorLongtude,
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
            11 => Some(UASDataset::ImageSourceSensor),
            12 => Some(UASDataset::ImageCoordinateSensor),
            13 => Some(UASDataset::SensorLatitude),
            14 => Some(UASDataset::SensorLongtude),
            65 => Some(UASDataset::LSVersionNumber),
            _ => None,
        }
    }

    fn value(&self, v: &[u8]) -> Result<Self::Item, ParseError> {
        use UASDataset::*;
        match self {
            Timestamp => {
                let mut rdr = Cursor::new(v);
                let micros = rdr.read_u64::<BigEndian>().unwrap();
                match SystemTime::UNIX_EPOCH.checked_add(Duration::from_micros(micros)) {
                    Some(ts) => Ok(Value::Timestamp(ts)),
                    None => Err(ParseError::ValueError("failed to parse timestamp.".into())),
                }
            }
            // TODO Change value to Degrees
            PlatformHeadingAngle => {
                let mut rdr = Cursor::new(v);
                let angle = rdr.read_u16::<BigEndian>().unwrap();
                Ok(Value::U16(angle))
            }
            PlatformPitchAngle | PlatformRollAngle => {
                let mut rdr = Cursor::new(v);
                let angle = rdr.read_i16::<BigEndian>().unwrap();
                Ok(Value::I16(angle))
            }
            SensorLatitude | SensorLongtude => {
                let mut rdr = Cursor::new(v);
                let angle = rdr.read_i32::<BigEndian>().unwrap();
                Ok(Value::I32(angle))
            }
            ImageSourceSensor | ImageCoordinateSensor => {
                Ok(Value::String(String::from_utf8(v.to_owned()).unwrap()))
            }
            LSVersionNumber => Ok(Value::U8(v[0])),
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
            11, 3, 0x45, 0x4f, 0x4e,
            12, 14, 0x47, 0x65, 0x6f, 0x64, 0x65, 0x74, 0x69, 0x63, 0x20, 0x57, 0x47, 0x53, 0x38, 0x34,
            13, 4, 0x4d, 0xc4, 0xdc, 0xbb,
            14, 4, 0xb1, 0xa8, 0x6c, 0xfe,
            ];

        let klv = KLVReader::<UASDataset>::from_bytes(&buf);

        for x in klv {
            let key = x.key();
            if key.is_err() {
                println!("Error {:?}", key);
                continue;
            }
            let key = key.unwrap();
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
                (UASDataset::SensorLatitude, Ok(Value::I32(degrees))) => {
                    assert_eq!(degrees, 1304747195);
                }
                (UASDataset::ImageSourceSensor, Ok(Value::String(name))) => {
                    assert_eq!(&name, "EON");
                }
                (UASDataset::ImageCoordinateSensor, Ok(Value::String(name))) => {
                    assert_eq!(&name, "Geodetic WGS84");
                }
                (k, v) => {
                    println!("debug {:?} {:?}", k, v)
                }
            }
        }
    }
}

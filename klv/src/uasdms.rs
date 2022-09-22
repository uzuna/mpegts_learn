use std::{
    io::{self, Cursor},
    time::{Duration, SystemTime},
};

use byteorder::{BigEndian, ByteOrder, ReadBytesExt};

use crate::{DataSet, ParseError};

pub const LS_UNIVERSAL_KEY0601_8_10: [u8; 16] = [
    0x06, 0x0e, 0x2b, 0x34, 0x02, 0x0b, 0x01, 0x01, 0x0e, 0x01, 0x03, 0x01, 0x01, 0x00, 0x00, 0x00,
];

#[derive(Debug, PartialEq, Eq)]
pub enum Value {
    Timestamp(SystemTime),
    U8(u8),
    U16(u16),
    U32(u32),
    I16(i16),
    I32(i32),
    String(String),
}

impl From<u8> for Value {
    fn from(x: u8) -> Self {
        Value::U8(x)
    }
}

impl Value {
    fn to_i16(x: &[u8]) -> Self {
        Value::I16(BigEndian::read_i16(x))
    }
    fn to_i32(x: &[u8]) -> Self {
        Value::I32(BigEndian::read_i32(x))
    }
    fn to_string(x: &[u8]) -> Self {
        Value::String(String::from_utf8(x.to_owned()).unwrap())
    }
    fn to_timestamp(x: &[u8]) -> Result<Self, ParseError> {
        let micros = BigEndian::read_u64(x);
        match SystemTime::UNIX_EPOCH.checked_add(Duration::from_micros(micros)) {
            Some(ts) => Ok(Value::Timestamp(ts)),
            None => Err(ParseError::ValueError("failed to parse timestamp.".into())),
        }
    }
    fn to_u16(x: &[u8]) -> Self {
        Value::U16(BigEndian::read_u16(x))
    }
    fn to_u32(x: &[u8]) -> Self {
        Value::U32(BigEndian::read_u32(x))
    }

    fn write(&self, mut buf: &mut [u8]) -> io::Result<usize> {
        use std::io::Write;
        use Value::*;
        match self {
            Timestamp(x) => {
                let micros = x
                    .duration_since(SystemTime::UNIX_EPOCH)
                    .unwrap()
                    .as_micros();
                BigEndian::write_u64(buf, micros as u64);
                Ok(8)
            }
            U8(x) => buf.write(&[*x]),
            U16(x) => {
                BigEndian::write_u16(buf, *x);
                Ok(2)
            }
            U32(x) => {
                BigEndian::write_u32(buf, *x);
                Ok(4)
            }
            I16(x) => {
                BigEndian::write_i16(buf, *x);
                Ok(2)
            }
            I32(x) => {
                BigEndian::write_i32(buf, *x);
                Ok(4)
            }
            String(s) => buf.write(s.as_bytes()),
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum UASDataset {
    Checksum,
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
    SensorTrueAltitude,
    SensorHorizontalFOV,
    SensorVerticalFOV,
    SensorRelativeAzimuthAngle,
    SensorRelativeElevationAngle,
    SensorRelativeRollAngle,
    SlantRange,
    // ST 0601.8の仕様書ではではu16だがテストデータでは4バイトだったのでu32とする
    TargetWidth,
    FrameCenterLatitude,
    FrameCenterLongitude,
    FrameCenterElevation,
    TargetLocationLatitude,
    TargetLocationLongitude,
    TargetLocationElevation,
    // Meters/Second
    PlatformGroundSpeed,
    GroundRange,
}

impl DataSet for UASDataset {
    type Item = Value;

    fn from_byte(b: u8) -> Option<Self>
    where
        Self: std::marker::Sized,
    {
        use UASDataset::*;
        match b {
            1 => Some(Checksum),
            2 => Some(Timestamp),
            5 => Some(PlatformHeadingAngle),
            6 => Some(PlatformPitchAngle),
            7 => Some(PlatformRollAngle),
            11 => Some(ImageSourceSensor),
            12 => Some(ImageCoordinateSensor),
            13 => Some(SensorLatitude),
            14 => Some(SensorLongtude),
            15 => Some(SensorTrueAltitude),
            16 => Some(SensorHorizontalFOV),
            17 => Some(SensorVerticalFOV),
            18 => Some(SensorRelativeAzimuthAngle),
            19 => Some(SensorRelativeElevationAngle),
            20 => Some(SensorRelativeRollAngle),
            21 => Some(SlantRange),
            22 => Some(TargetWidth),
            23 => Some(FrameCenterLatitude),
            24 => Some(FrameCenterLongitude),
            25 => Some(FrameCenterElevation),
            40 => Some(TargetLocationLatitude),
            41 => Some(TargetLocationLongitude),
            42 => Some(TargetLocationElevation),
            56 => Some(PlatformGroundSpeed),
            57 => Some(GroundRange),
            65 => Some(LSVersionNumber),
            _ => None,
        }
    }

    fn value(&self, v: &[u8]) -> Result<Self::Item, ParseError> {
        use UASDataset::*;
        match self {
            Timestamp => Value::to_timestamp(v),
            PlatformGroundSpeed | LSVersionNumber => Ok(Value::from(v[0])),
            Checksum
            | PlatformHeadingAngle
            | SensorTrueAltitude
            | SensorHorizontalFOV
            | SensorVerticalFOV
            | FrameCenterElevation
            | TargetLocationElevation => Ok(Value::to_u16(v)),
            PlatformPitchAngle | PlatformRollAngle => Ok(Value::to_i16(v)),
            SensorLatitude
            | SensorLongtude
            | SensorRelativeElevationAngle
            | SensorRelativeRollAngle
            | FrameCenterLatitude
            | FrameCenterLongitude
            | TargetLocationLatitude
            | TargetLocationLongitude => Ok(Value::to_i32(v)),
            SensorRelativeAzimuthAngle | SlantRange | TargetWidth | GroundRange => {
                Ok(Value::to_u32(v))
            }
            ImageSourceSensor | ImageCoordinateSensor => Ok(Value::to_string(v)),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::time::SystemTime;

    use crate::KLVReader;

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
            15, 2, 0x1f, 0x4a,
            16, 2, 0x00, 0x85,
            17, 2, 0x00, 0x4b,
            18, 4, 0x20, 0xc8, 0xd2, 0x7d,
            19, 4, 0xfc, 0xdd, 0x02, 0xd8,
            20, 4, 0xfe, 0xb8, 0xcb, 0x61,
            21, 4, 0x00, 0x8f, 0x3e, 0x61,
            22, 4, 0x00, 0x00, 0x01, 0xc9,
            23, 4, 0x4d, 0xdd, 0x8c, 0x2a,
            24, 4, 0xb1, 0xbe, 0x9e, 0xf4,
            25, 2, 0x0b, 0x85,
            40, 4, 0x4d, 0xdd, 0x8c, 0x2a,
            41, 4, 0xb1, 0xbe, 0x9e, 0xf4,
            42, 2, 0x0b, 0x85,
            56, 1, 0x2e,
            57, 4, 0x00, 0x8d, 0xd4, 0x29,
            1, 2, 0x1c, 0x5f,
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

    #[test]
    fn test_value_encode_decode() {
        let td = [
            Value::U8(0),
            Value::U8(255),
            Value::U16(256),
            Value::U32(192),
            Value::I16(-127),
            Value::I32(-192),
            Value::String("EON_$JK)~DFKSDF".to_owned()),
            Value::Timestamp(SystemTime::now()),
        ];
        for x in td {
            let mut buf = vec![];
            let size = x.write(&mut buf).unwrap();
            assert_eq!(buf.len(), size);

            match x {
                Value::Timestamp(x) => {
                    assert_eq!(Value::Timestamp(x), Value::to_timestamp(&buf).unwrap());
                }
                Value::U8(x) => {
                    assert_eq!(Value::U8(x), Value::from(buf[0]));
                }
                Value::U16(x) => {
                    assert_eq!(Value::U16(x), Value::to_u16(&buf));
                }
                Value::U32(x) => {
                    assert_eq!(Value::U32(x), Value::to_u32(&buf));
                }
                Value::I16(x) => {
                    assert_eq!(Value::I16(x), Value::to_i16(&buf));
                }
                Value::I32(x) => {
                    assert_eq!(Value::I32(x), Value::to_i32(&buf));
                }
                Value::String(x) => {
                    assert_eq!(Value::String(x), Value::to_string(&buf));
                }
            }
        }
    }
}

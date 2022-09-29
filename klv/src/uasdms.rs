use std::{
    io::Write,
    time::{Duration, SystemTime},
};

use byteorder::{BigEndian, ByteOrder};

use crate::{DataSet, LengthOctet, ParseError};

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
    fn as_i16(x: &[u8]) -> Self {
        Value::I16(BigEndian::read_i16(x))
    }
    fn as_i32(x: &[u8]) -> Self {
        Value::I32(BigEndian::read_i32(x))
    }
    fn as_string(x: &[u8]) -> Self {
        Value::String(String::from_utf8(x.to_owned()).unwrap())
    }
    fn as_timestamp(x: &[u8]) -> Result<Self, ParseError> {
        let micros = BigEndian::read_u64(x);
        match SystemTime::UNIX_EPOCH.checked_add(Duration::from_micros(micros)) {
            Some(ts) => Ok(Value::Timestamp(ts)),
            None => Err(ParseError::ValueError("failed to parse timestamp.".into())),
        }
    }
    fn as_u16(x: &[u8]) -> Self {
        Value::U16(BigEndian::read_u16(x))
    }
    fn as_u32(x: &[u8]) -> Self {
        Value::U32(BigEndian::read_u32(x))
    }

    fn to_bytes<W: Write>(&self, mut buf: W) -> std::io::Result<usize> {
        use Value::*;
        match self {
            Timestamp(x) => {
                let mut slice = [0; 8];
                let micros = x
                    .duration_since(SystemTime::UNIX_EPOCH)
                    .unwrap()
                    .as_micros();
                BigEndian::write_u64(&mut slice, micros as u64);
                buf.write(&slice[..])
            }
            U8(x) => buf.write(&[*x]),
            U16(x) => {
                let mut slice = [0; 2];
                BigEndian::write_u16(&mut slice, *x);
                buf.write(&slice[..])
            }
            U32(x) => {
                let mut slice = [0; 4];
                BigEndian::write_u32(&mut slice, *x);
                buf.write(&slice[..])
            }
            I16(x) => {
                let mut slice = [0; 2];
                BigEndian::write_i16(&mut slice, *x);
                buf.write(&slice[..])
            }
            I32(x) => {
                let mut slice = [0; 4];
                BigEndian::write_i32(&mut slice, *x);
                buf.write(&slice[..])
            }
            String(s) => buf.write(s.as_bytes()),
        }
    }

    #[allow(clippy::len_without_is_empty)]
    pub fn len(&self) -> usize {
        match self {
            Value::Timestamp(_) => 8,
            Value::U8(_) => 1,
            Value::U16(_) => 2,
            Value::U32(_) => 4,
            Value::I16(_) => 2,
            Value::I32(_) => 4,
            Value::String(x) => x.len(),
        }
    }
}

#[repr(u8)]
#[derive(Debug, PartialEq, Eq)]
pub enum UASDataset {
    Checksum = 1,
    Timestamp = 2,
    // Relative between longitudinal axis and True North measured in the horizontal plane.
    // Map 0..(2^16-1) to 0..360.
    // Resolution: ~5.5 milli degrees.
    PlatformHeadingAngle = 5,
    // Angle between longitudinal axis and horizontal plane.
    // Positive angles above horizontal plane.
    // Map -(2^15-1)..(2^15-1) to +/-20.
    // Use -(2^15) as "out of range" indicator. -(2^15) = 0x8000.
    // Resolution: ~610 micro degrees.
    PlatformPitchAngle = 6,
    // Angle between transverse axis and transvers-longitudinal plane.
    // Positive angles for lowered right wing.
    // Map (-2^15-1)..(2^15-1) to +/-50.
    // Use -(2^15) as "out of range" indicator. -(2^15) = 0x8000.
    // Res: ~1525 micro deg.
    PlatformRollAngle = 7,
    ImageSourceSensor = 11,
    ImageCoordinateSensor = 12,
    SensorLatitude = 13,
    SensorLongtude = 14,
    SensorTrueAltitude = 15,
    SensorHorizontalFOV = 16,
    SensorVerticalFOV = 17,
    SensorRelativeAzimuthAngle = 18,
    SensorRelativeElevationAngle = 19,
    SensorRelativeRollAngle = 20,
    SlantRange = 21,
    // ST 0601.8の仕様書ではではu16だがテストデータでは4バイトだったのでu32とする
    TargetWidth = 22,
    FrameCenterLatitude = 23,
    FrameCenterLongitude = 24,
    FrameCenterElevation = 25,
    TargetLocationLatitude = 40,
    TargetLocationLongitude = 41,
    TargetLocationElevation = 42,
    // Meters/Second
    PlatformGroundSpeed = 56,
    GroundRange = 57,
    LSVersionNumber = 65,
}
impl TryFrom<u8> for UASDataset {
    type Error = ();

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        use UASDataset::*;
        match value {
            x if x == Checksum as u8 => Ok(Checksum),
            x if x == Timestamp as u8 => Ok(Timestamp),
            x if x == PlatformHeadingAngle as u8 => Ok(PlatformHeadingAngle),
            x if x == PlatformPitchAngle as u8 => Ok(PlatformPitchAngle),
            x if x == PlatformRollAngle as u8 => Ok(PlatformRollAngle),
            x if x >= ImageSourceSensor as u8 && x <= FrameCenterElevation as u8 => {
                Ok(unsafe { std::mem::transmute(x) })
            }
            x if x >= TargetLocationLatitude as u8 && x <= TargetLocationElevation as u8 => {
                Ok(unsafe { std::mem::transmute(x) })
            }
            x if x == PlatformGroundSpeed as u8 => Ok(PlatformGroundSpeed),
            x if x == GroundRange as u8 => Ok(GroundRange),
            x if x == LSVersionNumber as u8 => Ok(LSVersionNumber),
            _ => Err(()),
        }
    }
}

impl DataSet for UASDataset {
    type Item = Value;

    fn from_byte(b: u8) -> Option<Self>
    where
        Self: std::marker::Sized,
    {
        if let Ok(x) = UASDataset::try_from(b) {
            Some(x)
        } else {
            None
        }
    }

    fn value(&self, v: &[u8]) -> Result<Self::Item, ParseError> {
        use UASDataset::*;
        match self {
            Timestamp => Value::as_timestamp(v),
            PlatformGroundSpeed | LSVersionNumber | Checksum => Ok(Value::from(v[0])),
            PlatformHeadingAngle
            | SensorTrueAltitude
            | SensorHorizontalFOV
            | SensorVerticalFOV
            | FrameCenterElevation
            | TargetLocationElevation => Ok(Value::as_u16(v)),
            PlatformPitchAngle | PlatformRollAngle => Ok(Value::as_i16(v)),
            SensorLatitude
            | SensorLongtude
            | SensorRelativeElevationAngle
            | SensorRelativeRollAngle
            | FrameCenterLatitude
            | FrameCenterLongitude
            | TargetLocationLatitude
            | TargetLocationLongitude => Ok(Value::as_i32(v)),
            SensorRelativeAzimuthAngle | SlantRange | TargetWidth | GroundRange => {
                Ok(Value::as_u32(v))
            }
            ImageSourceSensor | ImageCoordinateSensor => Ok(Value::as_string(v)),
        }
    }
}

pub fn encode(
    mut buf: &mut [u8],
    records: &[(UASDataset, Value)],
) -> Result<usize, std::io::Error> {
    let mut size = 0;
    size += buf.write(&LS_UNIVERSAL_KEY0601_8_10)?;
    let content_len = contents_len(records);
    size += LengthOctet::length_to_buf(&mut buf, content_len)?;
    size += content_len;
    for (key, value) in records {
        let _ = buf.write(&[*key as u8, value.len() as u8])?;
        value.to_bytes(&mut buf)?;
    }
    Ok(size)
}
fn contents_len(records: &[(UASDataset, Value)]) -> usize {
    records
        .iter()
        .fold(0_usize, |size, (_, v)| size + 2 + v.len())
}
pub fn encode_len(records: &[(UASDataset, Value)]) -> usize {
    let mut contents_len = contents_len(records);
    contents_len += 16; // HEADER
    contents_len + LengthOctet::encode_len(contents_len) // length
}

#[cfg(test)]
mod tests {
    use std::time::SystemTime;

    use crate::{
        uasdms::{encode_len, LS_UNIVERSAL_KEY0601_8_10},
        KLVGlobal, KLVReader,
    };

    use super::{encode, UASDataset, Value};
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
            1, 2, 0x1c, 0x5f
            ];

        let klv = KLVReader::<UASDataset>::from_bytes(&buf);

        for x in klv {
            let key = x.key();
            if key.is_err() {
                println!("Error {:?}", key);
                continue;
            }
            let key = key.unwrap();
            println!("key {:?} {:?}", key, x.content());
            println!("value {:?}", x.parse());
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
                    println!("without assert test case {:?} {:?}", k, v)
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
            let size = x.to_bytes(&mut buf).unwrap();
            assert_eq!(buf.len(), size, "value {:?} {:?} ", x, buf);

            match x {
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
                Value::U8(x) => {
                    assert_eq!(Value::U8(x), Value::from(buf[0]));
                }
                Value::U16(x) => {
                    assert_eq!(Value::U16(x), Value::as_u16(&buf));
                }
                Value::U32(x) => {
                    assert_eq!(Value::U32(x), Value::as_u32(&buf));
                }
                Value::I16(x) => {
                    assert_eq!(Value::I16(x), Value::as_i16(&buf));
                }
                Value::I32(x) => {
                    assert_eq!(Value::I32(x), Value::as_i32(&buf));
                }
                Value::String(x) => {
                    assert_eq!(Value::String(x), Value::as_string(&buf));
                }
            }
        }
    }

    #[test]
    fn test_encode() {
        let records = [
            (UASDataset::Timestamp, Value::Timestamp(SystemTime::now())),
            (
                UASDataset::ImageSourceSensor,
                Value::String("TESTS".to_string()),
            ),
            (UASDataset::TargetLocationLatitude, Value::I32(1234)),
        ];
        let mut buf = vec![0; 100];
        let write_size = encode(&mut buf, &records).unwrap();
        let encode_size = encode_len(&records);
        assert_eq!(encode_size, write_size);

        if let Ok(klvg) = KLVGlobal::try_from_bytes(&buf) {
            if klvg.key_is(&LS_UNIVERSAL_KEY0601_8_10) {
                let r = KLVReader::<UASDataset>::from_bytes(klvg.content());
                for x in r {
                    let key = x.key().unwrap();
                    assert!(
                        key == UASDataset::Timestamp
                            || key == UASDataset::ImageSourceSensor
                            || key == UASDataset::TargetLocationLatitude
                    );
                }
            } else {
                println!("unknown key {:?}", &buf[..16]);
            }
        } else {
            println!("unknown data {:?}", &buf);
        }
    }
}

//! MISB Standard 0601
//! the Unmanned Air System (UAS) Datalink Local Set (LS)
//! reference: MISB ST 0601.8

use crate::{value::Value, DataSet, ParseError};

#[repr(u8)]
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
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
impl UASDataset {
    const KEY: [u8; 16] = [
        0x06, 0x0e, 0x2b, 0x34, 0x02, 0x0b, 0x01, 0x01, 0x0e, 0x01, 0x03, 0x01, 0x01, 0x00, 0x00,
        0x00,
    ];
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

    fn key() -> &'static [u8] {
        &Self::KEY
    }

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

    fn as_byte(&self) -> u8 {
        *self as u8
    }
}

#[cfg(test)]
mod tests {
    use std::time::SystemTime;

    use crate::{encode, encode_len, KLVGlobal, KLVReader};

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
    fn test_encode() {
        let records = [
            (UASDataset::Timestamp, Value::Timestamp(SystemTime::now())),
            (
                UASDataset::ImageSourceSensor,
                Value::String("asdasdasd".to_string()),
            ),
            (UASDataset::TargetLocationLatitude, Value::I32(1234)),
        ];
        let encode_size = encode_len(&records);
        let mut buf = vec![0_u8; encode_size];
        let write_size = encode(&mut buf, &records).unwrap();
        assert_eq!(encode_size, write_size);

        if let Ok(klvg) = KLVGlobal::try_from_bytes(&buf) {
            if klvg.key_is(&UASDataset::KEY) {
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

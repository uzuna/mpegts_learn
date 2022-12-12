use std::collections::BTreeSet;

use byteorder::{BigEndian, WriteBytesExt};
use serde::{ser, Serialize};

use crate::{
    error::{Error, Result},
    LengthOctet,
};

pub struct Serializer {
    // This string starts empty and JSON is appended as values are serialized.
    universal_key: Vec<u8>,
    output: Vec<u8>,
    keys: BTreeSet<u8>,
}

impl Serializer {
    fn concat(self) -> Vec<u8> {
        let Self {
            universal_key: mut key,
            output,
            ..
        } = self;
        LengthOctet::length_to_buf(&mut key, output.len()).unwrap();
        key.extend_from_slice(&output);
        key
    }
    // TODO常にチェックサムを埋め込み、データ破損に対してロバストにする
    #[allow(dead_code)]
    fn checksum(buf: &[u8]) -> u32 {
        buf.iter().fold(0, |a, x| a + *x as u32)
    }
}

pub fn to_bytes<T>(value: &T) -> Result<Vec<u8>>
where
    T: Serialize,
{
    let mut serializer = Serializer {
        universal_key: vec![],
        output: vec![],
        keys: BTreeSet::new(),
    };
    value.serialize(&mut serializer)?;
    // ここでKeyを合成するのが良さそう
    Ok(serializer.concat())
}

impl<'a> ser::Serializer for &'a mut Serializer {
    // io::Writeを想定するのが良い?
    type Ok = ();

    type Error = Error;

    // シリアライズ中に異なる状態を示す方がある場合に使う
    type SerializeSeq = Self;
    type SerializeTuple = Self;
    type SerializeTupleStruct = Self;
    type SerializeTupleVariant = Self;
    type SerializeMap = Self;
    type SerializeStruct = Self;
    type SerializeStructVariant = Self;

    fn serialize_bool(self, v: bool) -> Result<Self::Ok> {
        LengthOctet::length_to_buf(&mut self.output, 1).map_err(Error::IO)?;
        self.output.push(v as u8);
        Ok(())
    }

    fn serialize_i8(self, v: i8) -> Result<Self::Ok> {
        LengthOctet::length_to_buf(&mut self.output, 1).map_err(Error::IO)?;
        self.output.push(v as u8);
        Ok(())
    }

    fn serialize_i16(self, v: i16) -> Result<Self::Ok> {
        LengthOctet::length_to_buf(&mut self.output, 2).map_err(Error::IO)?;
        self.output
            .write_i16::<BigEndian>(v)
            .map_err(|e| Error::Encode(format!("encodind error i16 {v} to byte. {e}")))?;
        Ok(())
    }

    fn serialize_i32(self, v: i32) -> Result<Self::Ok> {
        LengthOctet::length_to_buf(&mut self.output, 4).map_err(Error::IO)?;
        self.output
            .write_i32::<BigEndian>(v)
            .map_err(|e| Error::Encode(format!("encodind error i32 {v} to byte. {e}")))?;
        Ok(())
    }

    fn serialize_i64(self, v: i64) -> Result<Self::Ok> {
        LengthOctet::length_to_buf(&mut self.output, 8).map_err(Error::IO)?;
        self.output
            .write_i64::<BigEndian>(v)
            .map_err(|e| Error::Encode(format!("encodind error i64 {v} to byte. {e}")))?;
        Ok(())
    }

    fn serialize_u8(self, v: u8) -> Result<Self::Ok> {
        LengthOctet::length_to_buf(&mut self.output, 1).map_err(Error::IO)?;
        self.output.push(v);
        Ok(())
    }

    fn serialize_u16(self, v: u16) -> Result<Self::Ok> {
        LengthOctet::length_to_buf(&mut self.output, 2).map_err(Error::IO)?;
        self.output
            .write_u16::<BigEndian>(v)
            .map_err(|e| Error::Encode(format!("encodind error u16 {v} to byte. {e}")))?;
        Ok(())
    }

    fn serialize_u32(self, v: u32) -> Result<Self::Ok> {
        LengthOctet::length_to_buf(&mut self.output, 4).map_err(Error::IO)?;
        self.output
            .write_u32::<BigEndian>(v)
            .map_err(|e| Error::Encode(format!("encodind error u32 {v} to byte. {e}")))?;
        Ok(())
    }

    fn serialize_u64(self, v: u64) -> Result<Self::Ok> {
        LengthOctet::length_to_buf(&mut self.output, 8).map_err(Error::IO)?;
        self.output
            .write_u64::<BigEndian>(v)
            .map_err(|e| Error::Encode(format!("encodind error u64 {v} to byte. {e}")))?;
        Ok(())
    }

    fn serialize_f32(self, v: f32) -> Result<Self::Ok> {
        LengthOctet::length_to_buf(&mut self.output, 4).map_err(Error::IO)?;
        self.output
            .write_f32::<BigEndian>(v)
            .map_err(|e| Error::Encode(format!("encodind error f32 {v} to byte. {e}")))?;
        Ok(())
    }

    fn serialize_f64(self, v: f64) -> Result<Self::Ok> {
        LengthOctet::length_to_buf(&mut self.output, 8).map_err(Error::IO)?;
        self.output
            .write_f64::<BigEndian>(v)
            .map_err(|e| Error::Encode(format!("encodind error f32 {v} to byte. {e}")))?;
        Ok(())
    }

    fn serialize_char(self, v: char) -> Result<Self::Ok> {
        self.serialize_u32(v as u32)
    }

    fn serialize_str(self, v: &str) -> Result<Self::Ok> {
        let encoded = v.as_bytes();
        LengthOctet::length_to_buf(&mut self.output, encoded.len()).map_err(Error::IO)?;
        self.output.extend_from_slice(encoded);
        Ok(())
    }

    fn serialize_bytes(self, _v: &[u8]) -> Result<Self::Ok> {
        todo!()
    }

    fn serialize_none(self) -> Result<Self::Ok> {
        todo!()
    }

    fn serialize_some<T: ?Sized>(self, _value: &T) -> Result<Self::Ok>
    where
        T: Serialize,
    {
        todo!()
    }

    fn serialize_unit(self) -> Result<Self::Ok> {
        todo!()
    }

    fn serialize_unit_struct(self, _name: &'static str) -> Result<Self::Ok> {
        todo!()
    }

    fn serialize_unit_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        _variant: &'static str,
    ) -> Result<Self::Ok> {
        todo!()
    }

    fn serialize_newtype_struct<T: ?Sized>(
        self,
        _name: &'static str,
        _value: &T,
    ) -> Result<Self::Ok>
    where
        T: Serialize,
    {
        todo!()
    }

    fn serialize_newtype_variant<T: ?Sized>(
        self,
        _name: &'static str,
        _variant_index: u32,
        _variant: &'static str,
        _value: &T,
    ) -> Result<Self::Ok>
    where
        T: Serialize,
    {
        todo!()
    }

    fn serialize_seq(self, _len: Option<usize>) -> Result<Self::SerializeSeq> {
        todo!()
    }

    fn serialize_tuple(self, _len: usize) -> Result<Self::SerializeTuple> {
        todo!()
    }

    fn serialize_tuple_struct(
        self,
        _name: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeTupleStruct> {
        todo!()
    }

    fn serialize_tuple_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        _variant: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeTupleVariant> {
        todo!()
    }

    fn serialize_map(self, _len: Option<usize>) -> Result<Self::SerializeMap> {
        Ok(self)
    }

    fn serialize_struct(self, name: &'static str, len: usize) -> Result<Self::SerializeStruct> {
        // 名前の長さを制限することでSerializeのエラーを出せる(実行時)
        println!("serialize_struct: {name} {}", name.len());
        if name.len() != 16 {
            return Err(Error::Key(format!(
                "Prease set struct universal Key for {}",
                name
            )));
        }
        self.universal_key.extend_from_slice(name.as_bytes());
        // lenは構造体のfield数であるため実際の長さがわからない
        // KLV形式においてはヘッダをあとづけするしかない
        self.serialize_map(Some(len))
    }

    fn serialize_struct_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        _variant: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeStructVariant> {
        todo!()
    }
}

impl<'a> ser::SerializeSeq for &'a mut Serializer {
    // Must match the `Ok` type of the serializer.
    type Ok = ();
    // Must match the `Error` type of the serializer.
    type Error = Error;

    // Serialize a single element of the sequence.
    fn serialize_element<T>(&mut self, value: &T) -> Result<()>
    where
        T: ?Sized + Serialize,
    {
        value.serialize(&mut **self)
    }

    // Close the sequence.
    fn end(self) -> Result<()> {
        Ok(())
    }
}

impl<'a> ser::SerializeTuple for &'a mut Serializer {
    type Ok = ();
    type Error = Error;

    fn serialize_element<T>(&mut self, value: &T) -> Result<()>
    where
        T: ?Sized + Serialize,
    {
        value.serialize(&mut **self)
    }

    fn end(self) -> Result<()> {
        Ok(())
    }
}

impl<'a> ser::SerializeTupleStruct for &'a mut Serializer {
    type Ok = ();
    type Error = Error;

    fn serialize_field<T>(&mut self, value: &T) -> Result<()>
    where
        T: ?Sized + Serialize,
    {
        value.serialize(&mut **self)
    }

    fn end(self) -> Result<()> {
        Ok(())
    }
}

impl<'a> ser::SerializeTupleVariant for &'a mut Serializer {
    type Ok = ();
    type Error = Error;

    fn serialize_field<T>(&mut self, value: &T) -> Result<()>
    where
        T: ?Sized + Serialize,
    {
        value.serialize(&mut **self)
    }

    fn end(self) -> Result<()> {
        Ok(())
    }
}

impl<'a> ser::SerializeMap for &'a mut Serializer {
    type Ok = ();
    type Error = Error;

    fn serialize_key<T>(&mut self, key: &T) -> Result<()>
    where
        T: ?Sized + Serialize,
    {
        key.serialize(&mut **self)
    }

    fn serialize_value<T>(&mut self, value: &T) -> Result<()>
    where
        T: ?Sized + Serialize,
    {
        value.serialize(&mut **self)
    }

    fn end(self) -> Result<()> {
        Ok(())
    }
}

impl<'a> ser::SerializeStruct for &'a mut Serializer {
    type Ok = ();
    type Error = Error;

    fn serialize_field<T>(&mut self, key: &'static str, value: &T) -> Result<()>
    where
        T: ?Sized + Serialize,
    {
        let key = key
            .parse::<u8>()
            .map_err(|e| Error::Key(format!("failed t kparse key str to u8 {} {}", key, e)))?;
        if !self.keys.insert(key) {
            return Err(Error::Key(format!("already use field {}", key)));
        }
        self.output.push(key);
        value.serialize(&mut **self)
    }

    fn end(self) -> Result<()> {
        Ok(())
    }
}

impl<'a> ser::SerializeStructVariant for &'a mut Serializer {
    type Ok = ();
    type Error = Error;

    fn serialize_field<T>(&mut self, key: &'static str, value: &T) -> Result<()>
    where
        T: ?Sized + Serialize,
    {
        key.serialize(&mut **self)?;
        value.serialize(&mut **self)
    }

    fn end(self) -> Result<()> {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use serde::{Deserialize, Serialize};

    use crate::de::from_bytes;
    use crate::error::Error;
    use crate::se::to_bytes;

    /// シリアライズ、デシリアライズで対称性のある構造体
    #[test]
    fn test_serialize_symmetry_numbers() {
        #[derive(Debug, Serialize, Deserialize, PartialEq)]
        // こうすると指定しやすいけどASCII文字以外が使えないのが難点
        #[serde(rename = "TESTDATA00000000")]
        struct Test {
            #[serde(rename = "128")]
            x: bool,
            #[serde(rename = "10")]
            u8: u8,
            #[serde(rename = "11")]
            u16: u16,
            #[serde(rename = "12")]
            u32: u32,
            #[serde(rename = "13")]
            u64: u64,
            #[serde(rename = "15")]
            i8: i8,
            #[serde(rename = "16")]
            i16: i16,
            #[serde(rename = "17")]
            i32: i32,
            #[serde(rename = "18")]
            i64: i64,
            #[serde(rename = "20")]
            f32: f32,
            #[serde(rename = "21")]
            f64: f64,
        }

        let t = Test {
            x: true,
            u8: 8,
            u16: 16,
            u32: 32,
            u64: 64,
            i8: -8,
            i16: -16,
            i32: -32,
            i64: -64,
            f32: 0.1,
            f64: -123.45,
        };
        let s = to_bytes(&t).unwrap();
        let x = from_bytes::<Test>(&s).unwrap();
        assert_eq!(t, x);
    }

    #[test]
    fn test_serialize_error_by_key() {
        #[derive(Debug, Serialize, Deserialize, PartialEq)]
        #[serde(rename = "TESTDATA00000000")]
        struct TestKeyRangeOutFromU8 {
            #[serde(rename = "-1")]
            x: bool,
        }

        let t = TestKeyRangeOutFromU8 { x: true };
        let res = to_bytes(&t);
        match res {
            Err(Error::Key(_)) => {}
            _ => unreachable!(),
        }

        #[derive(Debug, Serialize, Deserialize, PartialEq)]
        #[serde(rename = "TESTDATA00000000")]
        struct TestForgetRename {
            bbb: bool,
        }
        let t = TestForgetRename { bbb: true };
        let res = to_bytes(&t);
        match res {
            Err(Error::Key(_)) => {}
            _ => unreachable!(),
        }

        #[derive(Debug, Serialize, Deserialize, PartialEq)]
        #[serde(rename = "TESTDATA00000000")]
        struct TestSameName {
            #[serde(rename = "10")]
            bbb: bool,
            #[serde(rename = "10")]
            u8: u8,
        }
        let t = TestSameName { bbb: true, u8: 128 };
        let res = to_bytes(&t);
        match res {
            Err(Error::Key(_)) => {}
            _ => unreachable!(),
        }

        #[derive(Debug, Serialize, Deserialize, PartialEq)]
        struct TestNoUniversalKey {
            #[serde(rename = "10")]
            bbb: bool,
        }
        let t = TestNoUniversalKey { bbb: true };
        let res = to_bytes(&t);
        match res {
            Err(Error::Key(_)) => {}
            _ => unreachable!(),
        }

        //
        // Check same field struct other UniversalKey
        //
        #[derive(Debug, Serialize, Deserialize, PartialEq)]
        #[serde(rename = "TESTDATA00000000")]
        struct TestRef {
            #[serde(rename = "10")]
            bbb: bool,
        }
        #[derive(Debug, Serialize, Deserialize, PartialEq)]
        #[serde(rename = "TESTDATA00000001")]
        struct TestTargetOtherUniversalKey {
            #[serde(rename = "10")]
            bbb: bool,
        }
        let t = TestRef { bbb: true };
        let reference = to_bytes(&t).unwrap();

        let res = from_bytes::<TestTargetOtherUniversalKey>(&reference);
        match res {
            Err(Error::Key(_)) => {}
            _ => unreachable!(),
        }
    }

    #[test]
    fn test_serialize_str() {
        #[derive(Debug, Serialize, Deserialize, PartialEq)]
        #[serde(rename = "TESTDATA00000000")]
        struct TestStr<'a> {
            #[serde(rename = "30")]
            str: &'a str,
        }
        let t = TestStr {
            str: "this is str\09joi4t@",
        };
        let s = to_bytes(&t).unwrap();
        let x = from_bytes::<TestStr>(&s).unwrap();
        assert_eq!(t, x);
    }

    #[test]
    fn test_serialize_char() {
        #[derive(Debug, Serialize, Deserialize, PartialEq)]
        #[serde(rename = "TESTDATA00000000")]
        struct TestChar {
            #[serde(rename = "30")]
            char8: char,
            #[serde(rename = "31")]
            char16: char,
            #[serde(rename = "32")]
            char32: char,
        }
        let t = TestChar {
            char8: '\n',
            char16: std::char::from_u32(257).unwrap(),
            char32: std::char::from_u32(u16::MAX as u32 + 1).unwrap(),
        };
        let s = to_bytes(&t).unwrap();
        let x = from_bytes::<TestChar>(&s).unwrap();
        assert_eq!(t, x);
    }

    /// デシリアライズ時に欠損や過剰なデータなどの非対称性があるデータ
    #[test]
    #[ignore]
    fn test_serialize_asymmetry() {
        todo!()
    }
}

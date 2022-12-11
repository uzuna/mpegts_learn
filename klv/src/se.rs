use serde::{ser, Serialize};

use crate::error::{Error, Result};

pub struct Serializer {
    // This string starts empty and JSON is appended as values are serialized.
    key: Vec<u8>,
    output: Vec<u8>,
}

pub fn to_string<T>(value: &T) -> Result<Vec<u8>>
where
    T: Serialize,
{
    let mut serializer = Serializer {
        key: vec![],
        output: vec![],
    };
    value.serialize(&mut serializer)?;
    // ここでKeyを合成するのが良さそう
    Ok(serializer.output)
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
        self.output.push(v as u8);
        Ok(())
    }

    fn serialize_i8(self, _v: i8) -> Result<Self::Ok> {
        todo!()
    }

    fn serialize_i16(self, _v: i16) -> Result<Self::Ok> {
        todo!()
    }

    fn serialize_i32(self, _v: i32) -> Result<Self::Ok> {
        todo!()
    }

    fn serialize_i64(self, _v: i64) -> Result<Self::Ok> {
        todo!()
    }

    fn serialize_u8(self, _v: u8) -> Result<Self::Ok> {
        todo!()
    }

    fn serialize_u16(self, _v: u16) -> Result<Self::Ok> {
        todo!()
    }

    fn serialize_u32(self, _v: u32) -> Result<Self::Ok> {
        todo!()
    }

    fn serialize_u64(self, _v: u64) -> Result<Self::Ok> {
        todo!()
    }

    fn serialize_f32(self, _v: f32) -> Result<Self::Ok> {
        todo!()
    }

    fn serialize_f64(self, _v: f64) -> Result<Self::Ok> {
        todo!()
    }

    fn serialize_char(self, _v: char) -> Result<Self::Ok> {
        todo!()
    }

    fn serialize_str(self, v: &str) -> Result<Self::Ok> {
        self.output.extend_from_slice(v.as_bytes());
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

    fn serialize_newtype_struct<T: ?Sized>(self, _name: &'static str, _value: &T) -> Result<Self::Ok>
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
        if name.len() != 16 {
            return Err(Error::Message(format!(
                "Prease set struct name 16 char got {}",
                name
            )));
        }
        self.key.extend_from_slice(name.as_bytes());
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
        println!("key {}", key);
        key.serialize(&mut **self)?;
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
    use serde::Serialize;

    use crate::se::to_string;

    #[test]
    fn test_serialize() {
        #[derive(Serialize)]
        #[serde(rename = "TESTDATA00000000")]
        struct Test {
            #[serde(rename = "1")]
            x: bool,
            #[serde(rename = "2")]
            y: bool,
        }

        let t = Test { x: true, y: false };
        println!("{:?}", to_string(&t).unwrap())
    }
}

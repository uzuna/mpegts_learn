use std::ops::{AddAssign, MulAssign, Neg};

use serde::de::{
    self, DeserializeSeed, EnumAccess, IntoDeserializer, MapAccess, SeqAccess, VariantAccess,
    Visitor,
};
use serde::Deserialize;

use crate::error::{Error, Result};

pub struct Deserializer<'de> {
    input: &'de [u8],
}

impl<'de> Deserializer<'de> {
    pub fn from_bytes(input: &'de [u8]) -> Self {
        Deserializer { input }
    }
}

pub fn from_bytes<'a, T>(s: &'a [u8]) -> Result<T>
where
    T: Deserialize<'a>,
{
    let mut deserializer = Deserializer::from_bytes(s);
    let t = T::deserialize(&mut deserializer)?;
    if deserializer.input.is_empty() {
        Ok(t)
    } else {
        Err(Error::TrailingCharacters)
    }
}

impl<'de> Deserializer<'de> {
    fn parse_bool(&mut self) -> Result<bool> {
        let result = self.input[0] != 0;
        self.input = &self.input[1..];
        Ok(result)
    }
    fn parse_string(&mut self) -> Result<&'de str> {
        // 長さに合わせてStringにする
        todo!()
    }
}

impl<'de, 'a> de::Deserializer<'de> for &'a mut Deserializer<'de> {
    type Error = Error;

    // 不明な型をParseする場合
    fn deserialize_any<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        todo!()
    }

    fn deserialize_bool<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        visitor.visit_bool(self.parse_bool()?)
    }

    fn deserialize_i8<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        todo!()
    }

    fn deserialize_i16<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        todo!()
    }

    fn deserialize_i32<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        todo!()
    }

    fn deserialize_i64<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        todo!()
    }

    fn deserialize_u8<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        todo!()
    }

    fn deserialize_u16<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        todo!()
    }

    fn deserialize_u32<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        todo!()
    }

    fn deserialize_u64<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        todo!()
    }

    fn deserialize_f32<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        todo!()
    }

    fn deserialize_f64<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        todo!()
    }

    fn deserialize_str<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        visitor.visit_borrowed_str(self.parse_string()?)
    }

    fn deserialize_string<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        self.deserialize_str(visitor)
    }

    fn deserialize_bytes<V>(self, _visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        todo!()
    }

    fn deserialize_byte_buf<V>(self, _visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        todo!()
    }

    fn deserialize_option<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        todo!()
    }

    fn deserialize_unit<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        todo!()
    }

    fn deserialize_unit_struct<V>(self, _name: &'static str, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        self.deserialize_unit(visitor)
    }

    fn deserialize_newtype_struct<V>(self, _name: &'static str, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        visitor.visit_newtype_struct(self)
    }

    fn deserialize_seq<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        // 数値列はありかも知れない
        unimplemented!()
    }

    fn deserialize_tuple<V>(self, _len: usize, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        self.deserialize_seq(visitor)
    }

    // Tuple structs look just like sequences in JSON.
    fn deserialize_tuple_struct<V>(
        self,
        _name: &'static str,
        _len: usize,
        visitor: V,
    ) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        self.deserialize_seq(visitor)
    }

    fn deserialize_map<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        // 何らかのルールに従うVisitorの実装が必要
        // JSONの場合はCommaSeparatedでコンマ区切り毎にKVを返すVisitorを渡している
        // KLVの場合はKey-Length-Valueが続く構造であるため親側の長さの範囲内でKLVを読んでいく
        visitor.visit_map(KLVVisitor::new(self, 0))
    }

    fn deserialize_enum<V>(
        self,
        _name: &'static str,
        _variants: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        todo!()
    }

    fn deserialize_char<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        todo!()
    }

    fn deserialize_struct<V>(
        self,
        name: &'static str,
        fields: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        // jsonの場合はtoplevelがMapなのでmapに飛ばしている
        // UniversalKeyとContentLengthを取り出してDeseliarizerに処理を移乗する
        todo!()
    }

    fn deserialize_identifier<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        // jsonの場合はdeserialize_strへ飛んでいる
        // Key-Lengthを読み出す関数を作る必要がある
        todo!()
    }

    fn deserialize_ignored_any<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        todo!()
    }
}

struct KLVVisitor<'a, 'de: 'a> {
    de: &'a mut Deserializer<'de>,
    len: usize,
    current: usize,
}

impl<'a, 'de> KLVVisitor<'a, 'de> {
    fn new(de: &'a mut Deserializer<'de>, len: usize) -> Self {
        Self {
            de,
            len,
            current: 0,
        }
    }
}

impl<'de, 'a> MapAccess<'de> for KLVVisitor<'a, 'de> {
    type Error = Error;

    fn next_key_seed<K>(&mut self, seed: K) -> Result<Option<K::Value>>
    where
        K: DeserializeSeed<'de>,
    {
        // Check if there are no more entries.
        if self.current >= self.len {
            return Ok(None);
        }
        // Deserialize a map key.
        // jump to any?
        seed.deserialize(&mut *self.de).map(Some)
    }

    fn next_value_seed<V>(&mut self, seed: V) -> Result<V::Value>
    where
        V: DeserializeSeed<'de>,
    {
        if self.current >= self.len {
            return Err(Error::ExpectedMapEnd);
        }
        // Deserialize a map value.
        seed.deserialize(&mut *self.de)
    }
}

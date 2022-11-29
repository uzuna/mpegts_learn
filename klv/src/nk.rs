//! シリアライズデシリアライズ実装テスト
//!
//! ほとんどのユースケースにおいては型からKLVを生成したいのでderiveマクロでstructに指定できるのが望ましい
//! 未知のKLVを取り出す際にはプリミティブにマップする実装があるとよい
//! 上記を両立できる実装を探る
use byteorder::{BigEndian, ByteOrder};

#[repr(u8)]
#[derive(Debug, Clone, Copy)]
enum Key {
    KA,
    KB,
}

impl Key {
    fn id(&self) -> u8 {
        *self as u8
    }
}

#[derive(Debug)]
enum Value {
    VA(u8),
    VB(u16),
}

impl Value {
    fn size(&self) -> usize {
        match self {
            Value::VA(_) => 1,
            Value::VB(_) => 2,
        }
    }
    fn parse(&self, buf: &[u8]) -> Self {
        match self {
            Value::VA(_) => Value::VA(buf[0]),
            Value::VB(_) => Value::VB(BigEndian::read_u16(buf)),
        }
    }
}

#[derive(Debug)]
struct Template {
    k: Key,
    v: Value,
}

impl Template {
    fn new(k: Key, v: Value) -> Self {
        Self { k, v }
    }

    fn parse(&self, buf: &[u8]) -> Option<(Value, usize)> {
        if self.k.id() != buf[0] {
            return None;
        }
        let len = buf[1] as usize;
        if self.v.size() == len {
            Some((self.v.parse(&buf[2..]), 2 + len))
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{Key, Template, Value};

    #[test]
    fn test_type_kl() {
        let temp: Vec<Template> = vec![
            Template::new(Key::KA, Value::VA(0)),
            Template::new(Key::KB, Value::VB(0)),
        ];

        let buf = vec![0_u8; 24];
        let cursor = 0;
        for x in temp {
            if let Some(v) = x.parse(&buf[cursor..cursor + 2]) {
                println!("v {:?}", v);
            }
        }
    }
}

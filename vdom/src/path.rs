use std::rc::Rc;
use std::{fmt, hash, mem};

#[derive(Clone, Eq, Debug)]
pub enum Key {
    U64(u64),
    I64(i64),
    Str(&'static str),
    String(Rc<String>),
    Bytes(Rc<Vec<u8>>),
}

impl PartialEq for Key {
    fn eq(&self, other: &Key) -> bool {
        match (self, other) {
            (Key::U64(a), Key::U64(b)) => a == b,
            (Key::I64(a), Key::I64(b)) => a == b,
            (Key::String(a), Key::String(b)) => a.as_ref() == b.as_ref(),
            (Key::String(a), Key::Str(b)) => a.as_ref() == b,
            (Key::Str(a), Key::String(b)) => a == b.as_ref(),
            (Key::Str(a), Key::Str(b)) => a == b,
            (Key::Bytes(a), Key::Bytes(b)) => a == b,
            _ => false,
        }
    }
}

impl hash::Hash for Key {
    fn hash<H: hash::Hasher>(&self, state: &mut H) {
        match self {
            Key::U64(v) => {
                state.write_u8(1);
                v.hash(state);
            }
            Key::I64(v) => {
                state.write_u8(2);
                v.hash(state);
            }
            Key::String(v) => {
                state.write_u8(3);
                v.hash(state);
            }
            Key::Str(v) => {
                state.write_u8(3);
                v.hash(state);
            }
            Key::Bytes(v) => {
                state.write_u8(4);
                v.hash(state);
            }
        }
    }
}

impl fmt::Display for Key {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Key::U64(n) => write!(f, "u{}", n),
            Key::I64(n) => write!(f, "i{}", n),
            Key::String(s) => write!(f, "s{}", s),
            Key::Str(s) => write!(f, "s{}", s),
            Key::Bytes(bytes) => {
                write!(f, "0x")?;
                for b in bytes.iter() {
                    write!(f, "{:02x}", b)?;
                }
                Ok(())
            }
        }
    }
}

macro_rules! impl_from_int_for_key {
    ($tyu: ty, $tyi: ty) => {
        impl From<$tyu> for Key {
            #[inline]
            fn from(v: $tyu) -> Key {
                Key::U64(v as u64)
            }
        }

        impl From<$tyi> for Key {
            #[inline]
            fn from(v: $tyi) -> Key {
                Key::I64(v as i64)
            }
        }
    };
}

impl_from_int_for_key!(u8, i8);
impl_from_int_for_key!(u16, i16);
impl_from_int_for_key!(u32, i32);
impl_from_int_for_key!(u64, i64);
impl_from_int_for_key!(usize, isize);

impl From<String> for Key {
    #[inline]
    fn from(v: String) -> Key {
        Key::String(Rc::new(v))
    }
}

impl From<Rc<String>> for Key {
    #[inline]
    fn from(v: Rc<String>) -> Key {
        Key::String(v)
    }
}

impl From<&'static str> for Key {
    #[inline]
    fn from(v: &'static str) -> Key {
        Key::Str(v)
    }
}

impl From<Vec<u8>> for Key {
    fn from(v: Vec<u8>) -> Key {
        Key::Bytes(Rc::new(v))
    }
}

impl From<Rc<Vec<u8>>> for Key {
    fn from(v: Rc<Vec<u8>>) -> Key {
        Key::Bytes(v)
    }
}

#[derive(Clone, Eq, PartialEq, Debug)]
pub struct Path<'a> {
    value: Key,
    parent: Option<&'a Path<'a>>,
}

impl<'a> Path<'a> {
    #[inline]
    pub fn new(value: Key) -> Path<'a> {
        Path {
            value,
            parent: None,
        }
    }

    #[inline]
    pub fn push<V: Into<Key>>(&'a self, value: V) -> Path<'a> {
        Path {
            value: value.into(),
            parent: Some(self),
        }
    }

    pub fn replace<V: Into<Key>>(&'a self, value: V) -> Path<'a> {
        Path {
            value: value.into(),
            parent: self.parent,
        }
    }

    #[inline]
    pub fn get(&self) -> &Key {
        &self.value
    }

    #[inline]
    pub fn parent(&'a self) -> Option<&'a Path<'a>> {
        self.parent
    }

    #[inline]
    pub fn iter(&'a self) -> impl Iterator<Item = &'a Key> {
        PathIterator(Some(self))
    }
}

impl<'a> fmt::Display for Path<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for (i, e) in self.iter().enumerate() {
            if 0 < i {
                write!(f, ".")?;
            }
            write!(f, "{}", e)?;
        }
        Ok(())
    }
}

struct PathIterator<'a>(Option<&'a Path<'a>>);

impl<'a> Iterator for PathIterator<'a> {
    type Item = &'a Key;

    #[inline]
    fn next(&mut self) -> Option<&'a Key> {
        let next = self.0.as_ref().and_then(|p| p.parent());
        mem::replace(&mut self.0, next).map(|p| p.get())
    }
}

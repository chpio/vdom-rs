use std::fmt;
use std::iter::{FromIterator, IntoIterator};
use std::mem;
use std::rc::Rc;
use std::borrow::Borrow;
use std::hash::{Hash, Hasher};

#[derive(Debug, Eq, Clone)]
pub enum Key {
    U64(u64),
    I64(i64),
    String(Rc<String>),
    Str(&'static str),
    Bytes(Rc<Vec<u8>>),
}

impl Hash for Key {
    fn hash<H: Hasher>(&self, state: &mut H) {
        match self {
            &Key::U64(ref v) => {
                state.write_u8(1);
                v.hash(state);
            }
            &Key::I64(ref v) => {
                state.write_u8(2);
                v.hash(state);
            }
            &Key::String(ref v) => {
                state.write_u8(3);
                v.hash(state);
            }
            &Key::Str(v) => {
                state.write_u8(3);
                v.hash(state);
            }
            &Key::Bytes(ref v) => {
                state.write_u8(4);
                v.hash(state);
            }
        }
    }
}

impl PartialEq for Key {
    fn eq(&self, other: &Key) -> bool {
        match (self, other) {
            (&Key::U64(ref a), &Key::U64(ref b)) => a == b,
            (&Key::I64(ref a), &Key::I64(ref b)) => a == b,
            (&Key::String(ref a), &Key::String(ref b)) => a.as_str() == b.as_str(),
            (&Key::String(ref a), &Key::Str(b)) => a.as_str() == b,
            (&Key::Str(a), &Key::String(ref b)) => a == b.as_str(),
            (&Key::Str(a), &Key::Str(b)) => a == b,
            (&Key::Bytes(ref a), &Key::Bytes(ref b)) => a == b,
            _ => false,
        }
    }
}



macro_rules! impl_from_int_for_key {
    ($tyu:ty, $tyi:ty) => {
        impl From<$tyu> for Key {
            fn from(v: $tyu) -> Key {
                Key::U64(v as u64)
            }
        }

        impl From<$tyi> for Key {
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
    fn from(v: String) -> Key {
        Key::String(Rc::new(v))
    }
}

impl From<Rc<String>> for Key {
    fn from(v: Rc<String>) -> Key {
        Key::String(v)
    }
}

impl From<&'static str> for Key {
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

impl<'a, T, O> From<&'a T> for Key
where
    T: ToOwned<Owned = O> + ?Sized,
    O: Borrow<T> + Into<Key>,
{
    default fn from(v: &'a T) -> Key {
        v.to_owned().into()
    }
}

impl fmt::Display for Key {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            Key::U64(ref n) => write!(f, "u{}", n),
            Key::I64(ref n) => write!(f, "i{}", n),
            Key::String(ref s) => write!(f, "s{}", s),
            Key::Str(s) => write!(f, "s{}", s),
            Key::Bytes(ref bytes) => {
                write!(f, "0x")?;
                for b in bytes.iter() {
                    write!(f, "{:02x}", b)?;
                }
                Ok(())
            }
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Hash)]
pub enum Ident {
    Key(Key),

    /// A *non keyed* (keyed nodes are not counted into it) index.
    Index(usize),
}

impl fmt::Display for Ident {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            &Ident::Key(ref key) => write!(f, "{}", key),
            &Ident::Index(index) => write!(f, "{}", index),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Hash)]
pub struct Path {
    path: Vec<Ident>,
}

impl Path {
    pub fn new() -> Path {
        Path { path: Vec::new() }
    }

    pub fn iter<'a>(&'a self) -> impl Iterator<Item = &'a Ident> {
        self.path.iter()
    }
}

impl FromIterator<Ident> for Path {
    fn from_iter<T>(iter: T) -> Self
    where
        T: IntoIterator<Item = Ident>,
    {
        Path {
            path: iter.into_iter().collect(),
        }
    }
}

impl fmt::Display for Path {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        for (i, e) in self.path.iter().enumerate() {
            if 0 < i {
                write!(f, ".")?;
            }
            write!(f, "{}", e)?;
        }
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct PathFrame<'a> {
    parent: Option<&'a PathFrame<'a>>,
    ident: Ident,
}

impl<'a> PathFrame<'a> {
    pub fn new() -> PathFrame<'a> {
        PathFrame {
            parent: None,
            ident: Ident::Key("".into()),
        }
    }

    pub fn add_key(&'a self, key: Key) -> PathFrame<'a> {
        PathFrame {
            parent: Some(self),
            ident: Ident::Key(key),
        }
    }

    pub fn add_index(&'a self, index: usize) -> PathFrame<'a> {
        PathFrame {
            parent: Some(self),
            ident: Ident::Index(index),
        }
    }

    pub fn parent(&'a self) -> Option<&'a PathFrame<'a>> {
        self.parent.as_ref().map(|pf| *pf)
    }

    pub fn ident(&self) -> &Ident {
        &self.ident
    }

    pub fn iter(&'a self) -> PathFrameIter<'a> {
        PathFrameIter(Some(self))
    }

    pub fn to_path(&self) -> Path {
        self.iter().map(|ident| ident.clone()).collect()
    }
}

impl<'a> fmt::Display for PathFrame<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        for (i, e) in self.iter().enumerate() {
            if 0 < i {
                write!(f, ".")?;
            }
            write!(f, "{}", e)?;
        }
        Ok(())
    }
}

pub struct PathFrameIter<'a>(Option<&'a PathFrame<'a>>);

impl<'a> Iterator for PathFrameIter<'a> {
    type Item = &'a Ident;

    fn next(&mut self) -> Option<&'a Ident> {
        let next = match self.0 {
            Some(ref pf) => pf.parent(),
            None => None,
        };
        mem::replace(&mut self.0, next).map(|pf| pf.ident())
    }
}

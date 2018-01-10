use std::fmt;
use std::iter::{FromIterator, IntoIterator};
use std::mem;
use std::rc::Rc;
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

#[derive(Debug, Clone)]
pub enum PathRef<'a> {
    Path(&'a Path),
    PathFrame(&'a PathFrame<'a>),
}

impl <'a> PathRef<'a> {
    pub fn parent(&self) -> Option<PathRef<'a>> {
        match self {
            &PathRef::Path(p) => p.parent().map(|p| p.into()),
            &PathRef::PathFrame(pf) => pf.parent(),
        }
    }

    pub fn ident(&self) -> &'a Ident {
        match self {
            &PathRef::Path(p) => p.ident(),
            &PathRef::PathFrame(pf) => pf.ident(),
        }
    }

    pub fn iter(&self) -> PathRefIter<'a> {
        PathRefIter(Some(self.clone()))
    }
}

impl <'a> fmt::Display for PathRef<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        for (i, e) in self.iter().map(|p| p.ident()).enumerate() {
            if 0 < i {
                write!(f, ".")?;
            }
            write!(f, "{}", e)?;
        }
        Ok(())
    }
}

impl <'a> From<&'a Path> for PathRef<'a> {
    fn from(path: &'a Path) -> Self {
        PathRef::Path(path)
    }
}

impl <'a> From<&'a PathFrame<'a>> for PathRef<'a> {
    fn from(path: &'a PathFrame<'a>) -> Self {
        PathRef::PathFrame(path)
    }
}

pub struct PathRefIter<'a>(Option<PathRef<'a>>);

impl<'a> Iterator for PathRefIter<'a> {
    type Item = PathRef<'a>;

    fn next(&mut self) -> Option<PathRef<'a>> {
        let next = self.0.as_ref().and_then(|pf| pf.parent());
        mem::replace(&mut self.0, next)
    }
}

#[derive(Debug, Eq, PartialEq, Hash)]
struct PathIntern {
    parent: Option<Path>,
    ident: Ident,
}

#[derive(Clone, Debug, Eq, PartialEq, Hash)]
pub struct Path(Rc<PathIntern>);

impl Path {
    pub fn new(parent: Option<Path>, ident: Ident) -> Path {
        Path(Rc::new(PathIntern {
            parent: parent,
            ident: ident,
        }))
    }

    pub fn parent(&self) -> Option<&Path> {
        self.0.parent.as_ref()
    }

    pub fn ident(&self) -> &Ident {
        &self.0.ident
    }

    pub fn iter<'a>(&'a self) -> PathIter<'a> {
        PathIter(Some(self))
    }
}

impl fmt::Display for Path {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        Into::<PathRef>::into(self).fmt(f)
    }
}

pub struct PathIter<'a>(Option<&'a Path>);

impl<'a> Iterator for PathIter<'a> {
    type Item = &'a Path;

    fn next(&mut self) -> Option<&'a Path> {
        let next = self.0.as_ref().and_then(|pf| pf.parent());
        mem::replace(&mut self.0, next)
    }
}

impl FromIterator<Ident> for Option<Path> {
    fn from_iter<T>(iter: T) -> Option<Path>
    where
        T: IntoIterator<Item = Ident>,
    {
        let mut parent = None;
        for ident in iter.into_iter() {
            parent = Some(Path::new(parent.take(), ident));
        }
        parent
    }
}

impl <'a> From<&'a PathRef<'a>> for &'a Path {
    fn from(pr: &'a PathRef<'a>) -> &'a Path {
        match pr {
            &PathRef::Path(ref p) => p,
            &PathRef::PathFrame(ref pf) => pf.path(),
        }
    }
}

impl <'a> From<&'a PathRef<'a>> for Path {
    fn from(pr: &'a PathRef<'a>) -> Path {
        Into::<&'a Path>::into(pr).clone()
    }
}

#[derive(Debug, Clone)]
pub struct PathFrame<'a> {
    parent: Option<PathRef<'a>>,
    path: Option<Path>,
    ident: Ident,
}

impl<'a> PathFrame<'a> {
    pub fn new() -> PathFrame<'a> {
        PathFrame {
            parent: None,
            path: None,
            ident: Ident::Key("".into()),
        }
    }

    pub fn add_key(&'a self, key: Key) -> PathFrame<'a> {
        PathFrame {
            parent: Some(self.into()),
            path: None,
            ident: Ident::Key(key),
        }
    }

    pub fn add_index(&'a self, index: usize) -> PathFrame<'a> {
        PathFrame {
            parent: Some(self.into()),
            path: None,
            ident: Ident::Index(index),
        }
    }

    pub fn parent(&'a self) -> Option<PathRef<'a>> {
        self.parent.as_ref().cloned()
    }

    pub fn ident(&self) -> &Ident {
        &self.ident
    }

    pub fn iter(&'a self) -> PathRefIter<'a> {
        Into::<PathRef>::into(self).iter()
    }

    pub fn path(&self) -> &Path {
        if let Some(ref path) = self.path {
            path
        } else {
            let parent_path = self.parent.as_ref().map(|p| p.into());
            let path = Path::new(parent_path, self.ident.clone());
            unsafe {
                let mut_path = &self.path as *const _ as *mut _;
                *mut_path = Some(path);
            }
            self.path.as_ref().unwrap()
        }
    }
}

impl<'a> fmt::Display for PathFrame<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        Into::<PathRef>::into(self).fmt(f)
    }
}

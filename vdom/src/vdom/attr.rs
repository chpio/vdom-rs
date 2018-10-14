#[derive(Clone, Eq, PartialEq)]
pub enum AttrValue {
    True,
    Null,
    Str(&'static str),
    String(String),
}

impl From<bool> for AttrValue {
    fn from(v: bool) -> AttrValue {
        match v {
            true => AttrValue::True,
            false => AttrValue::Null,
        }
    }
}

impl From<Option<()>> for AttrValue {
    fn from(v: Option<()>) -> AttrValue {
        match v {
            Some(()) => AttrValue::True,
            None => AttrValue::Null,
        }
    }
}

impl From<&'static str> for AttrValue {
    fn from(v: &'static str) -> AttrValue {
        AttrValue::Str(v)
    }
}

impl From<Option<&'static str>> for AttrValue {
    fn from(v: Option<&'static str>) -> AttrValue {
        match v {
            Some(s) => AttrValue::Str(s),
            None => AttrValue::Null,
        }
    }
}

impl From<String> for AttrValue {
    fn from(v: String) -> AttrValue {
        AttrValue::String(v)
    }
}

impl From<Option<String>> for AttrValue {
    fn from(v: Option<String>) -> AttrValue {
        match v {
            Some(s) => AttrValue::String(s),
            None => AttrValue::Null,
        }
    }
}

#[derive(Clone, Eq, PartialEq)]
pub enum AttrRefValue<'a> {
    True,
    Null,
    Str(&'a str),
}

impl<'a> From<&'a AttrValue> for AttrRefValue<'a> {
    fn from(v: &'a AttrValue) -> AttrRefValue<'a> {
        match v {
            AttrValue::True => AttrRefValue::True,
            AttrValue::Null => AttrRefValue::Null,
            AttrValue::Str(s) => AttrRefValue::Str(s),
            AttrValue::String(s) => AttrRefValue::Str(s.as_str()),
        }
    }
}

pub trait Attr {
    fn is_value_static(&self) -> bool;
    fn name(&self) -> &str;
    fn value(&self) -> AttrRefValue<'_>;
}

pub struct AttrTrue {
    key: &'static str,
}

impl AttrTrue {
    pub fn new(key: &'static str) -> AttrTrue {
        AttrTrue { key }
    }
}

impl Attr for AttrTrue {
    #[inline]
    fn is_value_static(&self) -> bool {
        true
    }

    #[inline]
    fn name(&self) -> &str {
        self.key
    }

    #[inline]
    fn value(&self) -> AttrRefValue<'_> {
        AttrRefValue::True
    }
}

pub struct AttrStr {
    key: &'static str,
    value: &'static str,
}

impl AttrStr {
    pub fn new(key: &'static str, value: &'static str) -> AttrStr {
        AttrStr { key, value }
    }
}

impl Attr for AttrStr {
    #[inline]
    fn is_value_static(&self) -> bool {
        true
    }

    #[inline]
    fn name(&self) -> &str {
        self.key
    }

    #[inline]
    fn value(&self) -> AttrRefValue<'_> {
        AttrRefValue::Str(self.value)
    }
}

pub struct AttrDyn {
    key: &'static str,
    value: AttrValue,
}

impl AttrDyn {
    pub fn new<V>(key: &'static str, value: V) -> AttrDyn
    where
        V: Into<AttrValue>,
    {
        AttrDyn {
            key,
            value: value.into(),
        }
    }
}

impl Attr for AttrDyn {
    #[inline]
    fn is_value_static(&self) -> bool {
        false
    }

    #[inline]
    fn name(&self) -> &str {
        self.key
    }

    #[inline]
    fn value(&self) -> AttrRefValue<'_> {
        (&self.value).into()
    }
}

pub trait AttrVisitor {
    fn on_attr<A>(&mut self, attr: &A)
    where
        A: Attr;
}

pub trait AttrDiffer {
    fn on_diff<A>(&mut self, curr: &A, ancestor: &A)
    where
        A: Attr;
}

pub trait AttrList {
    fn visit<V>(&self, visitor: &mut V)
    where
        V: AttrVisitor;

    fn diff<D>(&self, ancestor: &Self, differ: &mut D)
    where
        D: AttrDiffer;
}

impl<L1, L2> AttrList for (L1, L2)
where
    L1: AttrList,
    L2: AttrList,
{
    fn visit<V>(&self, visitor: &mut V)
    where
        V: AttrVisitor,
    {
        self.0.visit(visitor);
        self.1.visit(visitor);
    }

    fn diff<D>(&self, ancestor: &Self, differ: &mut D)
    where
        D: AttrDiffer,
    {
        self.0.diff(&ancestor.0, differ);
        self.1.diff(&ancestor.1, differ);
    }
}

pub struct AttrListEntry<A>(A);

impl<A> AttrList for AttrListEntry<A>
where
    A: Attr,
{
    fn visit<V>(&self, visitor: &mut V)
    where
        V: AttrVisitor,
    {
        visitor.on_attr(&self.0);
    }

    fn diff<D>(&self, ancestor: &Self, differ: &mut D)
    where
        D: AttrDiffer,
    {
        debug_assert_eq!(self.0.name(), ancestor.0.name());

        debug_assert!{
            if self.0.is_value_static() {
                self.0.value() == ancestor.0.value()
            } else {
                true
            }
        };

        differ.on_diff(&self.0, &ancestor.0);
    }
}

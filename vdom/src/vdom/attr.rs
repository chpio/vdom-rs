use crate::driver::Driver;

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

pub trait Attr<D>
where
    D: Driver,
{
    fn is_value_static(&self) -> bool;
    fn name(&self) -> &str;
    fn value(&self) -> AttrRefValue<'_>;
    fn driver_store(&mut self) -> &mut D::AttrStore;
}

pub struct AttrTrue<D>
where
    D: Driver,
{
    key: &'static str,
    driver_store: D::AttrStore,
}

impl<D> AttrTrue<D>
where
    D: Driver,
{
    pub fn new(key: &'static str) -> AttrTrue<D> {
        AttrTrue {
            key,
            driver_store: D::new_attr_store(),
        }
    }
}

impl<D> Attr<D> for AttrTrue<D>
where
    D: Driver,
{
    fn is_value_static(&self) -> bool {
        true
    }

    fn name(&self) -> &str {
        self.key
    }

    fn value(&self) -> AttrRefValue<'_> {
        AttrRefValue::True
    }

    fn driver_store(&mut self) -> &mut D::AttrStore {
        &mut self.driver_store
    }
}

pub struct AttrStr<D>
where
    D: Driver,
{
    key: &'static str,
    value: &'static str,
    driver_store: D::AttrStore,
}

impl<D> AttrStr<D>
where
    D: Driver,
{
    pub fn new(key: &'static str, value: &'static str) -> AttrStr<D> {
        AttrStr {
            key,
            value,
            driver_store: D::new_attr_store(),
        }
    }
}

impl<D> Attr<D> for AttrStr<D>
where
    D: Driver,
{
    fn is_value_static(&self) -> bool {
        true
    }

    fn name(&self) -> &str {
        self.key
    }

    fn value(&self) -> AttrRefValue<'_> {
        AttrRefValue::Str(self.value)
    }

    fn driver_store(&mut self) -> &mut D::AttrStore {
        &mut self.driver_store
    }
}

pub struct AttrDyn<D>
where
    D: Driver,
{
    key: &'static str,
    value: AttrValue,
    driver_store: D::AttrStore,
}

impl<D> AttrDyn<D>
where
    D: Driver,
{
    pub fn new<V>(key: &'static str, value: V) -> AttrDyn<D>
    where
        V: Into<AttrValue>,
    {
        AttrDyn {
            key,
            value: value.into(),
            driver_store: D::new_attr_store(),
        }
    }
}

impl<D> Attr<D> for AttrDyn<D>
where
    D: Driver,
{
    fn is_value_static(&self) -> bool {
        false
    }

    fn name(&self) -> &str {
        self.key
    }

    fn value(&self) -> AttrRefValue<'_> {
        (&self.value).into()
    }

    fn driver_store(&mut self) -> &mut D::AttrStore {
        &mut self.driver_store
    }
}

pub trait AttrVisitor<D>
where
    D: Driver,
{
    type Err;

    fn on_attr<A>(&mut self, attr: &mut A) -> Result<(), Self::Err>
    where
        A: Attr<D>;
}

pub trait AttrDiffer<D>
where
    D: Driver,
{
    type Err;

    fn on_diff<A>(&mut self, curr: &mut A, ancestor: &mut A) -> Result<(), Self::Err>
    where
        A: Attr<D>;
}

pub trait AttrList<D>
where
    Self: Sized,
    D: Driver,
{
    fn visit<AV>(&mut self, visitor: &mut AV) -> Result<(), AV::Err>
    where
        AV: AttrVisitor<D>;

    fn diff<AD>(&mut self, ancestor: &mut Self, differ: &mut AD) -> Result<(), AD::Err>
    where
        AD: AttrDiffer<D>;

    fn push<A>(self, attr: A) -> (Self, AttrListEntry<A>)
    where
        A: Attr<D>,
    {
        (self, AttrListEntry(attr))
    }
}

impl<D, L1, L2> AttrList<D> for (L1, L2)
where
    D: Driver,
    L1: AttrList<D>,
    L2: AttrList<D>,
{
    fn visit<AV>(&mut self, visitor: &mut AV) -> Result<(), AV::Err>
    where
        AV: AttrVisitor<D>,
    {
        self.0.visit(visitor)?;
        self.1.visit(visitor)?;
        Ok(())
    }

    fn diff<AD>(&mut self, ancestor: &mut Self, differ: &mut AD) -> Result<(), AD::Err>
    where
        AD: AttrDiffer<D>,
    {
        self.0.diff(&mut ancestor.0, differ)?;
        self.1.diff(&mut ancestor.1, differ)?;
        Ok(())
    }
}

impl<D> AttrList<D> for ()
where
    D: Driver,
{
    fn visit<AV>(&mut self, visitor: &mut AV) -> Result<(), AV::Err>
    where
        AV: AttrVisitor<D>,
    {
        Ok(())
    }

    fn diff<AD>(&mut self, ancestor: &mut Self, differ: &mut AD) -> Result<(), AD::Err>
    where
        AD: AttrDiffer<D>,
    {
        Ok(())
    }
}

pub struct AttrListEntry<A>(pub A);

impl<A, D> AttrList<D> for AttrListEntry<A>
where
    A: Attr<D>,
    D: Driver,
{
    fn visit<AV>(&mut self, visitor: &mut AV) -> Result<(), AV::Err>
    where
        AV: AttrVisitor<D>,
    {
        visitor.on_attr(&mut self.0)
    }

    fn diff<AD>(&mut self, ancestor: &mut Self, differ: &mut AD) -> Result<(), AD::Err>
    where
        AD: AttrDiffer<D>,
    {
        debug_assert_eq!(self.0.name(), ancestor.0.name());

        debug_assert!{
            if self.0.is_value_static() {
                self.0.value() == ancestor.0.value()
            } else {
                true
            }
        };

        differ.on_diff(&mut self.0, &mut ancestor.0)?;
        Ok(())
    }
}

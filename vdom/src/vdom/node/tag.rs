use super::*;

pub trait Tag<D>
where
    D: Driver,
{
    fn is_tag_static(&self) -> bool;

    fn tag(&self) -> &str;

    fn visit_children<NV>(&mut self, visitor: &mut NV) -> Result<(), NV::Err>
    where
        NV: NodeVisitor<D>;

    fn diff_children<ND>(&mut self, ancestor: &mut Self, differ: &mut ND) -> Result<(), ND::Err>
    where
        ND: NodeDiffer<D>;

    fn visit_attrs<NV>(&mut self, visitor: &mut NV) -> Result<(), NV::Err>
    where
        NV: AttrVisitor<D>;

    fn diff_attrs<AD>(&mut self, ancestor: &mut Self, differ: &mut AD) -> Result<(), AD::Err>
    where
        AD: AttrDiffer<D>;

    fn driver_store(&mut self) -> &mut D::TagStore;
}

pub struct TagStatic<D, C, A>
where
    D: Driver,
{
    tag: &'static str,
    children: C,
    attrs: A,
    driver_store: D::TagStore,
}

impl<D, C, A> TagStatic<D, C, A>
where
    D: Driver,
    C: Node<D>,
    A: AttrList<D>,
{
    pub fn new(tag: &'static str, attrs: A, children: C) -> TagStatic<D, C, A> {
        TagStatic {
            tag,
            children,
            attrs,
            driver_store: D::new_tag_store(),
        }
    }
}

impl<D, C, A> Tag<D> for TagStatic<D, C, A>
where
    D: Driver,
    C: Node<D>,
    A: AttrList<D>,
{
    fn is_tag_static(&self) -> bool {
        true
    }

    fn tag(&self) -> &str {
        self.tag
    }

    fn visit_children<NV>(&mut self, visitor: &mut NV) -> Result<(), NV::Err>
    where
        NV: NodeVisitor<D>,
    {
        self.children.visit(&mut 0, visitor)
    }

    fn diff_children<ND>(&mut self, ancestor: &mut Self, differ: &mut ND) -> Result<(), ND::Err>
    where
        ND: NodeDiffer<D>,
    {
        self.children
            .diff(&mut 0, &mut 0, &mut ancestor.children, differ)
    }

    fn visit_attrs<AV>(&mut self, visitor: &mut AV) -> Result<(), AV::Err>
    where
        AV: AttrVisitor<D>,
    {
        self.attrs.visit(visitor)
    }

    fn diff_attrs<AD>(&mut self, ancestor: &mut Self, differ: &mut AD) -> Result<(), AD::Err>
    where
        AD: AttrDiffer<D>,
    {
        self.attrs.diff(&mut ancestor.attrs, differ)
    }

    fn driver_store(&mut self) -> &mut D::TagStore {
        &mut self.driver_store
    }
}

impl<D, C, A> Node<D> for TagStatic<D, C, A>
where
    D: Driver,
    C: Node<D>,
    A: AttrList<D>,
{
    fn visit<NV>(&mut self, index: &mut usize, visitor: &mut NV) -> Result<(), NV::Err>
    where
        NV: NodeVisitor<D>,
    {
        visitor.on_tag(*index, self)?;
        *index += 1;
        Ok(())
    }

    fn diff<ND>(
        &mut self,
        curr_index: &mut usize,
        ancestor_index: &mut usize,
        ancestor: &mut Self,
        differ: &mut ND,
    ) -> Result<(), ND::Err>
    where
        ND: NodeDiffer<D>,
    {
        debug_assert_eq!(self.tag, ancestor.tag);

        differ.on_tag(*curr_index, *ancestor_index, self, ancestor)?;
        *curr_index += 1;
        *ancestor_index += 1;
        Ok(())
    }
}

pub struct TagDyn<D, C, A>
where
    D: Driver,
{
    tag: Cow<'static, str>,
    children: C,
    attrs: A,
    driver_store: D::TagStore,
}

impl<D, C, A> TagDyn<D, C, A>
where
    D: Driver,
    C: Node<D>,
    A: AttrList<D>,
{
    pub fn new<T>(tag: T, attrs: A, children: C) -> TagDyn<D, C, A>
    where
        T: Into<Cow<'static, str>>,
    {
        TagDyn {
            tag: tag.into(),
            children,
            attrs,
            driver_store: D::new_tag_store(),
        }
    }
}

impl<D, C, A> Tag<D> for TagDyn<D, C, A>
where
    D: Driver,
    C: Node<D>,
    A: AttrList<D>,
{
    fn is_tag_static(&self) -> bool {
        false
    }

    fn tag(&self) -> &str {
        self.tag.as_ref()
    }

    fn visit_children<NV>(&mut self, visitor: &mut NV) -> Result<(), NV::Err>
    where
        NV: NodeVisitor<D>,
    {
        self.children.visit(&mut 0, visitor)
    }

    fn diff_children<ND>(&mut self, ancestor: &mut Self, differ: &mut ND) -> Result<(), ND::Err>
    where
        ND: NodeDiffer<D>,
    {
        self.children
            .diff(&mut 0, &mut 0, &mut ancestor.children, differ)
    }

    fn visit_attrs<AV>(&mut self, visitor: &mut AV) -> Result<(), AV::Err>
    where
        AV: AttrVisitor<D>,
    {
        self.attrs.visit(visitor)
    }

    fn diff_attrs<AD>(&mut self, ancestor: &mut Self, differ: &mut AD) -> Result<(), AD::Err>
    where
        AD: AttrDiffer<D>,
    {
        self.attrs.diff(&mut ancestor.attrs, differ)
    }

    fn driver_store(&mut self) -> &mut D::TagStore {
        &mut self.driver_store
    }
}

impl<D, C, A> Node<D> for TagDyn<D, C, A>
where
    D: Driver,
    C: Node<D>,
    A: AttrList<D>,
{
    fn visit<NV>(&mut self, index: &mut usize, visitor: &mut NV) -> Result<(), NV::Err>
    where
        NV: NodeVisitor<D>,
    {
        visitor.on_tag(*index, self)?;
        *index += 1;
        Ok(())
    }

    fn diff<ND>(
        &mut self,
        curr_index: &mut usize,
        ancestor_index: &mut usize,
        ancestor: &mut Self,
        differ: &mut ND,
    ) -> Result<(), ND::Err>
    where
        ND: NodeDiffer<D>,
    {
        differ.on_tag(*curr_index, *ancestor_index, self, ancestor)?;
        *curr_index += 1;
        *ancestor_index += 1;
        Ok(())
    }
}

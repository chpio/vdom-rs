use std::borrow::Cow;

use super::attr::{AttrDiffer, AttrList, AttrVisitor};
use super::path::Path;
use crate::driver::Driver;

pub trait NodeVisitor<D>
where
    D: Driver,
{
    fn on_tag<T>(&mut self, path: &Path<'_>, tag: &mut T)
    where
        T: Tag<D>;

    fn on_text<T>(&mut self, path: &Path<'_>, text: &mut T)
    where
        T: Text<D>;
}

pub trait NodeDiffer<D>
where
    D: Driver,
{
    fn on_node_added<N>(&mut self, path: &Path<'_>, curr: &mut N)
    where
        N: Node<D>;

    fn on_node_removed<N>(&mut self, path: &Path<'_>, ancestor: &mut N)
    where
        N: Node<D>;

    fn on_tag<T>(&mut self, path: &Path<'_>, curr: &mut T, ancestor: &mut T)
    where
        T: Tag<D>;

    fn on_text<T>(&mut self, path: &Path<'_>, curr: &mut T, ancestor: &mut T)
    where
        T: Text<D>;
}

pub trait Node<D>
where
    D: Driver,
{
    fn visit<NV>(&mut self, path: &Path<'_>, visitor: &mut NV)
    where
        NV: NodeVisitor<D>;

    fn diff<ND>(&mut self, path: &Path<'_>, ancestor: &mut Self, differ: &mut ND)
    where
        ND: NodeDiffer<D>;
}

pub trait Tag<D>
where
    D: Driver,
{
    fn is_tag_static(&self) -> bool;

    fn tag(&self) -> &str;

    fn visit_children<NV>(&mut self, path: &Path<'_>, visitor: &mut NV)
    where
        NV: NodeVisitor<D>;

    fn diff_children<ND>(&mut self, path: &Path<'_>, ancestor: &mut Self, differ: &mut ND)
    where
        ND: NodeDiffer<D>;

    fn visit_attr<NV>(&mut self, visitor: &mut NV)
    where
        NV: AttrVisitor<D>;

    fn diff_attr<AD>(&mut self, ancestor: &mut Self, differ: &mut AD)
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
    C: NodeList<D>,
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
    C: NodeList<D>,
    A: AttrList<D>,
{
    #[inline]
    fn is_tag_static(&self) -> bool {
        true
    }

    #[inline]
    fn tag(&self) -> &str {
        self.tag
    }

    #[inline]
    fn visit_children<NV>(&mut self, path: &Path<'_>, visitor: &mut NV)
    where
        NV: NodeVisitor<D>,
    {
        self.children.visit(path, 0, visitor);
    }

    #[inline]
    fn diff_children<ND>(&mut self, path: &Path<'_>, ancestor: &mut Self, differ: &mut ND)
    where
        ND: NodeDiffer<D>,
    {
        self.children.diff(path, 0, &mut ancestor.children, differ);
    }

    #[inline]
    fn visit_attr<AV>(&mut self, visitor: &mut AV)
    where
        AV: AttrVisitor<D>,
    {
        self.attrs.visit(visitor);
    }

    #[inline]
    fn diff_attr<AD>(&mut self, ancestor: &mut Self, differ: &mut AD)
    where
        AD: AttrDiffer<D>,
    {
        self.attrs.diff(&mut ancestor.attrs, differ);
    }

    #[inline]
    fn driver_store(&mut self) -> &mut D::TagStore {
        &mut self.driver_store
    }
}

impl<D, C, A> Node<D> for TagStatic<D, C, A>
where
    D: Driver,
    C: NodeList<D>,
    A: AttrList<D>,
{
    #[inline]
    fn visit<NV>(&mut self, path: &Path<'_>, visitor: &mut NV)
    where
        NV: NodeVisitor<D>,
    {
        visitor.on_tag(path, self);
    }

    #[inline]
    fn diff<ND>(&mut self, path: &Path<'_>, ancestor: &mut Self, differ: &mut ND)
    where
        ND: NodeDiffer<D>,
    {
        debug_assert_eq!(self.tag, ancestor.tag);

        differ.on_tag(path, self, ancestor);
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
    C: NodeList<D>,
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
    C: NodeList<D>,
    A: AttrList<D>,
{
    #[inline]
    fn is_tag_static(&self) -> bool {
        false
    }

    #[inline]
    fn tag(&self) -> &str {
        self.tag.as_ref()
    }

    #[inline]
    fn visit_children<NV>(&mut self, path: &Path<'_>, visitor: &mut NV)
    where
        NV: NodeVisitor<D>,
    {
        self.children.visit(path, 0, visitor);
    }

    #[inline]
    fn diff_children<ND>(&mut self, path: &Path<'_>, ancestor: &mut Self, differ: &mut ND)
    where
        ND: NodeDiffer<D>,
    {
        self.children.diff(path, 0, &mut ancestor.children, differ);
    }

    #[inline]
    fn visit_attr<AV>(&mut self, visitor: &mut AV)
    where
        AV: AttrVisitor<D>,
    {
        self.attrs.visit(visitor);
    }

    #[inline]
    fn diff_attr<AD>(&mut self, ancestor: &mut Self, differ: &mut AD)
    where
        AD: AttrDiffer<D>,
    {
        self.attrs.diff(&mut ancestor.attrs, differ);
    }

    #[inline]
    fn driver_store(&mut self) -> &mut D::TagStore {
        &mut self.driver_store
    }
}

impl<D, C, A> Node<D> for TagDyn<D, C, A>
where
    D: Driver,
    C: NodeList<D>,
    A: AttrList<D>,
{
    #[inline]
    fn visit<NV>(&mut self, path: &Path<'_>, visitor: &mut NV)
    where
        NV: NodeVisitor<D>,
    {
        visitor.on_tag(path, self);
    }

    #[inline]
    fn diff<ND>(&mut self, path: &Path<'_>, ancestor: &mut Self, differ: &mut ND)
    where
        ND: NodeDiffer<D>,
    {
        differ.on_tag(path, self, ancestor);
    }
}

pub trait Text<D>
where
    D: Driver,
{
    fn is_static(&self) -> bool;
    fn get(&self) -> &str;
    fn driver_store(&mut self) -> &mut D::TextStore;
}

pub struct TextStatic<D>
where
    D: Driver,
{
    text: &'static str,
    driver_store: D::TextStore,
}

impl<D> TextStatic<D>
where
    D: Driver,
{
    pub fn new(text: &'static str) -> TextStatic<D> {
        TextStatic {
            text,
            driver_store: D::new_text_store(),
        }
    }
}

impl<D> Text<D> for TextStatic<D>
where
    D: Driver,
{
    #[inline]
    fn is_static(&self) -> bool {
        true
    }

    #[inline]
    fn get(&self) -> &str {
        &self.text
    }

    #[inline]
    fn driver_store(&mut self) -> &mut D::TextStore {
        &mut self.driver_store
    }
}

impl<D> Node<D> for TextStatic<D>
where
    D: Driver,
{
    #[inline]
    fn visit<NV>(&mut self, path: &Path<'_>, visitor: &mut NV)
    where
        NV: NodeVisitor<D>,
    {
        visitor.on_text(path, self);
    }

    #[inline]
    fn diff<ND>(&mut self, path: &Path<'_>, ancestor: &mut Self, differ: &mut ND)
    where
        ND: NodeDiffer<D>,
    {
        debug_assert_eq!(self.text, ancestor.text);
        differ.on_text(path, self, ancestor);
    }
}

pub struct TextDyn<D>
where
    D: Driver,
{
    text: Cow<'static, str>,
    driver_store: D::TextStore,
}

impl<D> TextDyn<D>
where
    D: Driver,
{
    pub fn new<T>(text: T) -> TextDyn<D>
    where
        T: Into<Cow<'static, str>>,
    {
        TextDyn {
            text: text.into(),
            driver_store: D::new_text_store(),
        }
    }
}

impl<D> Text<D> for TextDyn<D>
where
    D: Driver,
{
    #[inline]
    fn is_static(&self) -> bool {
        false
    }

    #[inline]
    fn get(&self) -> &str {
        self.text.as_ref()
    }

    #[inline]
    fn driver_store(&mut self) -> &mut D::TextStore {
        &mut self.driver_store
    }
}

impl<D> Node<D> for TextDyn<D>
where
    D: Driver,
{
    #[inline]
    fn visit<NV>(&mut self, path: &Path<'_>, visitor: &mut NV)
    where
        NV: NodeVisitor<D>,
    {
        visitor.on_text(path, self);
    }

    #[inline]
    fn diff<ND>(&mut self, path: &Path<'_>, ancestor: &mut Self, differ: &mut ND)
    where
        ND: NodeDiffer<D>,
    {
        differ.on_text(path, self, ancestor);
    }
}

pub trait NodeList<D>
where
    Self: Sized,
    D: Driver,
{
    fn visit<NV>(&mut self, path: &Path<'_>, index: usize, visitor: &mut NV) -> usize
    where
        NV: NodeVisitor<D>;

    fn diff<ND>(
        &mut self,
        path: &Path<'_>,
        index: usize,
        ancestor: &mut Self,
        differ: &mut ND,
    ) -> usize
    where
        ND: NodeDiffer<D>;

    fn push<N>(self, node: N) -> (Self, NodeListEntry<N>)
    where
        N: Node<D>,
    {
        (self, NodeListEntry(node))
    }
}

impl<D, L1, L2> NodeList<D> for (L1, L2)
where
    D: Driver,
    L1: NodeList<D>,
    L2: NodeList<D>,
{
    #[inline]
    fn visit<NV>(&mut self, path: &Path<'_>, index: usize, visitor: &mut NV) -> usize
    where
        NV: NodeVisitor<D>,
    {
        let index = self.0.visit(path, index, visitor);
        self.1.visit(path, index, visitor)
    }

    #[inline]
    fn diff<ND>(
        &mut self,
        path: &Path<'_>,
        index: usize,
        ancestor: &mut Self,
        differ: &mut ND,
    ) -> usize
    where
        ND: NodeDiffer<D>,
    {
        let index = self.0.diff(path, index, &mut ancestor.0, differ);
        self.1.diff(path, index, &mut ancestor.1, differ)
    }
}

impl<D> NodeList<D> for ()
where
    D: Driver,
{
    #[inline]
    fn visit<NV>(&mut self, path: &Path<'_>, index: usize, visitor: &mut NV) -> usize
    where
        NV: NodeVisitor<D>,
    {
        index
    }

    #[inline]
    fn diff<ND>(
        &mut self,
        path: &Path<'_>,
        index: usize,
        ancestor: &mut Self,
        differ: &mut ND,
    ) -> usize
    where
        ND: NodeDiffer<D>,
    {
        index
    }
}

pub struct NodeListEntry<N>(pub N);

impl<D, N> NodeList<D> for NodeListEntry<N>
where
    D: Driver,
    N: Node<D>,
{
    #[inline]
    fn visit<NV>(&mut self, path: &Path<'_>, index: usize, visitor: &mut NV) -> usize
    where
        NV: NodeVisitor<D>,
    {
        let path = path.push(index);
        self.0.visit(&path, visitor);
        index + 1
    }

    #[inline]
    fn diff<ND>(
        &mut self,
        path: &Path<'_>,
        index: usize,
        ancestor: &mut Self,
        differ: &mut ND,
    ) -> usize
    where
        ND: NodeDiffer<D>,
    {
        let path = path.push(index);
        self.0.diff(&path, &mut ancestor.0, differ);
        index + 1
    }
}

impl<D, N> Node<D> for Option<N>
where
    D: Driver,
    N: Node<D>,
{
    fn visit<NV>(&mut self, path: &Path<'_>, visitor: &mut NV)
    where
        NV: NodeVisitor<D>,
    {
        if let Some(node) = self {
            node.visit(path, visitor);
        }
    }

    fn diff<ND>(&mut self, path: &Path<'_>, ancestor: &mut Self, differ: &mut ND)
    where
        ND: NodeDiffer<D>,
    {
        match (self, ancestor) {
            (Some(curr), Some(ancestor)) => curr.diff(path, ancestor, differ),
            (Some(curr), None) => differ.on_node_added(path, curr),
            (None, Some(ancestor)) => differ.on_node_removed(path, ancestor),
            (None, None) => {}
        }
    }
}

pub trait IntoNode<D>
where
    D: Driver,
{
    type Node: Node<D>;

    fn into_node(self) -> Self::Node;
}

impl<D> IntoNode<D> for &'static str
where
    D: Driver,
{
    type Node = TextStatic<D>;

    fn into_node(self) -> Self::Node {
        TextStatic::new(self)
    }
}

impl<D> IntoNode<D> for Cow<'static, str>
where
    D: Driver,
{
    type Node = TextDyn<D>;

    fn into_node(self) -> Self::Node {
        TextDyn::new(self)
    }
}

impl<D> IntoNode<D> for String
where
    D: Driver,
{
    type Node = TextDyn<D>;

    fn into_node(self) -> Self::Node {
        TextDyn::new(self)
    }
}

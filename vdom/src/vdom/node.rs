use std::borrow::Cow;

use super::attr::{AttrDiffer, AttrList, AttrVisitor};
use crate::driver::Driver;

pub trait NodeVisitor<D>
where
    D: Driver,
{
    fn on_tag<T>(&mut self, index: usize, tag: &mut T)
    where
        T: Tag<D>;

    fn on_text<T>(&mut self, index: usize, text: &mut T)
    where
        T: Text<D>;
}

pub trait NodeDiffer<D>
where
    D: Driver,
{
    fn on_node_added<N>(&mut self, index: &mut usize, curr: &mut N)
    where
        N: Node<D>;

    fn on_node_removed<N>(&mut self, ancestor_index: &mut usize, ancestor: &mut N)
    where
        N: Node<D>;

    fn on_tag<T>(
        &mut self,
        curr_index: usize,
        ancestor_index: usize,
        curr: &mut T,
        ancestor: &mut T,
    ) where
        T: Tag<D>;

    fn on_text<T>(
        &mut self,
        curr_index: usize,
        ancestor_index: usize,
        curr: &mut T,
        ancestor: &mut T,
    ) where
        T: Text<D>;
}

pub trait Node<D>
where
    D: Driver,
{
    fn visit<NV>(&mut self, index: &mut usize, visitor: &mut NV)
    where
        NV: NodeVisitor<D>;

    fn diff<ND>(
        &mut self,
        curr_index: &mut usize,
        ancestor_index: &mut usize,
        ancestor: &mut Self,
        differ: &mut ND,
    ) where
        ND: NodeDiffer<D>;
}

pub trait Tag<D>
where
    D: Driver,
{
    fn is_tag_static(&self) -> bool;

    fn tag(&self) -> &str;

    fn visit_children<NV>(&mut self, visitor: &mut NV)
    where
        NV: NodeVisitor<D>;

    fn diff_children<ND>(&mut self, ancestor: &mut Self, differ: &mut ND)
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

    fn visit_children<NV>(&mut self, visitor: &mut NV)
    where
        NV: NodeVisitor<D>,
    {
        self.children.visit(&mut 0, visitor);
    }

    fn diff_children<ND>(&mut self, ancestor: &mut Self, differ: &mut ND)
    where
        ND: NodeDiffer<D>,
    {
        self.children
            .diff(&mut 0, &mut 0, &mut ancestor.children, differ);
    }

    fn visit_attr<AV>(&mut self, visitor: &mut AV)
    where
        AV: AttrVisitor<D>,
    {
        self.attrs.visit(visitor);
    }

    fn diff_attr<AD>(&mut self, ancestor: &mut Self, differ: &mut AD)
    where
        AD: AttrDiffer<D>,
    {
        self.attrs.diff(&mut ancestor.attrs, differ);
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
    fn visit<NV>(&mut self, index: &mut usize, visitor: &mut NV)
    where
        NV: NodeVisitor<D>,
    {
        visitor.on_tag(*index, self);
        *index += 1;
    }

    fn diff<ND>(
        &mut self,
        curr_index: &mut usize,
        ancestor_index: &mut usize,
        ancestor: &mut Self,
        differ: &mut ND,
    ) where
        ND: NodeDiffer<D>,
    {
        debug_assert_eq!(self.tag, ancestor.tag);

        differ.on_tag(*curr_index, *ancestor_index, self, ancestor);
        *curr_index += 1;
        *ancestor_index += 1;
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

    fn visit_children<NV>(&mut self, visitor: &mut NV)
    where
        NV: NodeVisitor<D>,
    {
        self.children.visit(&mut 0, visitor);
    }

    fn diff_children<ND>(&mut self, ancestor: &mut Self, differ: &mut ND)
    where
        ND: NodeDiffer<D>,
    {
        self.children
            .diff(&mut 0, &mut 0, &mut ancestor.children, differ);
    }

    fn visit_attr<AV>(&mut self, visitor: &mut AV)
    where
        AV: AttrVisitor<D>,
    {
        self.attrs.visit(visitor);
    }

    fn diff_attr<AD>(&mut self, ancestor: &mut Self, differ: &mut AD)
    where
        AD: AttrDiffer<D>,
    {
        self.attrs.diff(&mut ancestor.attrs, differ);
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
    fn visit<NV>(&mut self, index: &mut usize, visitor: &mut NV)
    where
        NV: NodeVisitor<D>,
    {
        visitor.on_tag(*index, self);
        *index += 1;
    }

    fn diff<ND>(
        &mut self,
        curr_index: &mut usize,
        ancestor_index: &mut usize,
        ancestor: &mut Self,
        differ: &mut ND,
    ) where
        ND: NodeDiffer<D>,
    {
        differ.on_tag(*curr_index, *ancestor_index, self, ancestor);
        *curr_index += 1;
        *ancestor_index += 1;
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
    fn is_static(&self) -> bool {
        true
    }

    fn get(&self) -> &str {
        &self.text
    }

    fn driver_store(&mut self) -> &mut D::TextStore {
        &mut self.driver_store
    }
}

impl<D> Node<D> for TextStatic<D>
where
    D: Driver,
{
    fn visit<NV>(&mut self, index: &mut usize, visitor: &mut NV)
    where
        NV: NodeVisitor<D>,
    {
        visitor.on_text(*index, self);
        *index += 1;
    }

    fn diff<ND>(
        &mut self,
        curr_index: &mut usize,
        ancestor_index: &mut usize,
        ancestor: &mut Self,
        differ: &mut ND,
    ) where
        ND: NodeDiffer<D>,
    {
        debug_assert_eq!(self.text, ancestor.text);
        differ.on_text(*curr_index, *ancestor_index, self, ancestor);
        *curr_index += 1;
        *ancestor_index += 1;
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
    fn is_static(&self) -> bool {
        false
    }

    fn get(&self) -> &str {
        self.text.as_ref()
    }

    fn driver_store(&mut self) -> &mut D::TextStore {
        &mut self.driver_store
    }
}

impl<D> Node<D> for TextDyn<D>
where
    D: Driver,
{
    fn visit<NV>(&mut self, index: &mut usize, visitor: &mut NV)
    where
        NV: NodeVisitor<D>,
    {
        visitor.on_text(*index, self);
        *index += 1;
    }

    fn diff<ND>(
        &mut self,
        curr_index: &mut usize,
        ancestor_index: &mut usize,
        ancestor: &mut Self,
        differ: &mut ND,
    ) where
        ND: NodeDiffer<D>,
    {
        differ.on_text(*curr_index, *ancestor_index, self, ancestor);
        *curr_index += 1;
        *ancestor_index += 1;
    }
}

impl<D, L1, L2> Node<D> for (L1, L2)
where
    D: Driver,
    L1: Node<D>,
    L2: Node<D>,
{
    fn visit<NV>(&mut self, index: &mut usize, visitor: &mut NV)
    where
        NV: NodeVisitor<D>,
    {
        self.0.visit(index, visitor);
        self.1.visit(index, visitor);
    }

    fn diff<ND>(
        &mut self,
        curr_index: &mut usize,
        ancestor_index: &mut usize,
        ancestor: &mut Self,
        differ: &mut ND,
    ) where
        ND: NodeDiffer<D>,
    {
        self.0
            .diff(curr_index, ancestor_index, &mut ancestor.0, differ);
        self.1
            .diff(curr_index, ancestor_index, &mut ancestor.1, differ);
    }
}

impl<D> Node<D> for ()
where
    D: Driver,
{
    fn visit<NV>(&mut self, index: &mut usize, visitor: &mut NV)
    where
        NV: NodeVisitor<D>,
    {
    }

    fn diff<ND>(
        &mut self,
        curr_index: &mut usize,
        ancestor_index: &mut usize,
        ancestor: &mut Self,
        differ: &mut ND,
    ) where
        ND: NodeDiffer<D>,
    {
    }
}

impl<D, N> Node<D> for Option<N>
where
    D: Driver,
    N: Node<D>,
{
    fn visit<NV>(&mut self, index: &mut usize, visitor: &mut NV)
    where
        NV: NodeVisitor<D>,
    {
        if let Some(node) = self {
            node.visit(index, visitor);
        }
    }

    fn diff<ND>(
        &mut self,
        curr_index: &mut usize,
        ancestor_index: &mut usize,
        ancestor: &mut Self,
        differ: &mut ND,
    ) where
        ND: NodeDiffer<D>,
    {
        match (self, ancestor) {
            (Some(curr), Some(ancestor)) => curr.diff(curr_index, ancestor_index, ancestor, differ),
            (Some(curr), None) => differ.on_node_added(curr_index, curr),
            (None, Some(ancestor)) => differ.on_node_removed(ancestor_index, ancestor),
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
    type Node = TextDyn<D>;

    fn into_node(self) -> Self::Node {
        TextDyn::new(self)
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

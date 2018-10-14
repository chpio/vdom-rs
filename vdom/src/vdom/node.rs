use std::borrow::Cow;

use super::attr::{AttrDiffer, AttrList, AttrVisitor};
use super::path::Path;

pub trait NodeVisitor {
    fn on_tag<T>(&mut self, path: &Path<'_>, tag: &T)
    where
        T: Tag;

    fn on_text<T>(&mut self, path: &Path<'_>, text: &T)
    where
        T: Text;
}

pub trait NodeDiffer {
    fn on_tag<T>(&mut self, path: &Path<'_>, curr: &T, ancestor: &T)
    where
        T: Tag;

    fn on_text<T>(&mut self, path: &Path<'_>, curr: &T, ancestor: &T)
    where
        T: Text;
}

pub trait Node {
    fn visit<V>(&self, path: &Path<'_>, visitor: &mut V)
    where
        V: NodeVisitor;

    fn diff<D>(&self, path: &Path<'_>, ancestor: &Self, differ: &mut D)
    where
        D: NodeDiffer;
}

pub trait Tag {
    fn is_tag_static(&self) -> bool;

    fn tag(&self) -> &str;

    fn visit_children<V>(&self, path: &Path<'_>, visitor: &mut V)
    where
        V: NodeVisitor;

    fn diff_children<D>(&self, path: &Path<'_>, ancestor: &Self, differ: &mut D)
    where
        D: NodeDiffer;

    fn visit_attr<V>(&self, visitor: &mut V)
    where
        V: AttrVisitor;

    fn diff_attr<D>(&self, ancestor: &Self, differ: &mut D)
    where
        D: AttrDiffer;
}

pub struct TagStatic<C, A> {
    tag: &'static str,
    children: C,
    attrs: A,
}

impl<C, A> Tag for TagStatic<C, A>
where
    C: NodeList,
    A: AttrList,
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
    fn visit_children<V>(&self, path: &Path<'_>, visitor: &mut V)
    where
        V: NodeVisitor,
    {
        self.children.visit(path, 0, visitor);
    }

    #[inline]
    fn diff_children<D>(&self, path: &Path<'_>, ancestor: &Self, differ: &mut D)
    where
        D: NodeDiffer,
    {
        self.children.diff(path, 0, &ancestor.children, differ);
    }

    #[inline]
    fn visit_attr<V>(&self, visitor: &mut V)
    where
        V: AttrVisitor,
    {
        self.attrs.visit(visitor);
    }

    #[inline]
    fn diff_attr<D>(&self, ancestor: &Self, differ: &mut D)
    where
        D: AttrDiffer,
    {
        self.attrs.diff(&ancestor.attrs, differ);
    }
}

impl<C, A> Node for TagStatic<C, A>
where
    C: NodeList,
    A: AttrList,
{
    #[inline]
    fn visit<V>(&self, path: &Path<'_>, visitor: &mut V)
    where
        V: NodeVisitor,
    {
        visitor.on_tag(path, self);
    }

    #[inline]
    fn diff<D>(&self, path: &Path<'_>, ancestor: &Self, differ: &mut D)
    where
        D: NodeDiffer,
    {
        debug_assert_eq!(self.tag, ancestor.tag);

        differ.on_tag(path, self, ancestor);
    }
}

pub struct TagDyn<C, A> {
    tag: Cow<'static, str>,
    children: C,
    attrs: A,
}

impl<C, A> Tag for TagDyn<C, A>
where
    C: NodeList,
    A: AttrList,
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
    fn visit_children<V>(&self, path: &Path<'_>, visitor: &mut V)
    where
        V: NodeVisitor,
    {
        self.children.visit(path, 0, visitor);
    }

    #[inline]
    fn diff_children<D>(&self, path: &Path<'_>, ancestor: &Self, differ: &mut D)
    where
        D: NodeDiffer,
    {
        self.children.diff(path, 0, &ancestor.children, differ);
    }

    #[inline]
    fn visit_attr<V>(&self, visitor: &mut V)
    where
        V: AttrVisitor,
    {
        self.attrs.visit(visitor);
    }

    #[inline]
    fn diff_attr<D>(&self, ancestor: &Self, differ: &mut D)
    where
        D: AttrDiffer,
    {
        self.attrs.diff(&ancestor.attrs, differ);
    }
}

impl<C, A> Node for TagDyn<C, A>
where
    C: NodeList,
    A: AttrList,
{
    #[inline]
    fn visit<V>(&self, path: &Path<'_>, visitor: &mut V)
    where
        V: NodeVisitor,
    {
        visitor.on_tag(path, self);
    }

    #[inline]
    fn diff<D>(&self, path: &Path<'_>, ancestor: &Self, differ: &mut D)
    where
        D: NodeDiffer,
    {
        differ.on_tag(path, self, ancestor);
    }
}

pub trait Text {
    fn is_static(&self) -> bool;

    fn get(&self) -> &str;
}

pub struct TextStatic(&'static str);

impl Text for TextStatic {
    #[inline]
    fn is_static(&self) -> bool {
        true
    }

    #[inline]
    fn get(&self) -> &str {
        &self.0
    }
}

impl Node for TextStatic {
    #[inline]
    fn visit<V>(&self, path: &Path<'_>, visitor: &mut V)
    where
        V: NodeVisitor,
    {
        visitor.on_text(path, self);
    }

    #[inline]
    fn diff<D>(&self, path: &Path<'_>, ancestor: &Self, differ: &mut D)
    where
        D: NodeDiffer,
    {
        debug_assert_eq!(self.0, ancestor.0);

        differ.on_text(path, self, ancestor);
    }
}

pub struct TextDyn(Cow<'static, str>);

impl Text for TextDyn {
    #[inline]
    fn is_static(&self) -> bool {
        false
    }

    #[inline]
    fn get(&self) -> &str {
        self.0.as_ref()
    }
}

impl Node for TextDyn {
    #[inline]
    fn visit<V>(&self, path: &Path<'_>, visitor: &mut V)
    where
        V: NodeVisitor,
    {
        visitor.on_text(path, self);
    }

    #[inline]
    fn diff<D>(&self, path: &Path<'_>, ancestor: &Self, differ: &mut D)
    where
        D: NodeDiffer,
    {
        differ.on_text(path, self, ancestor);
    }
}

pub trait NodeList {
    fn visit<V>(&self, path: &Path<'_>, index: u64, visitor: &mut V) -> u64
    where
        V: NodeVisitor;

    fn diff<D>(&self, path: &Path<'_>, index: u64, ancestor: &Self, differ: &mut D) -> u64
    where
        D: NodeDiffer;
}

impl<L1, L2> NodeList for (L1, L2)
where
    L1: NodeList,
    L2: NodeList,
{
    #[inline]
    fn visit<V>(&self, path: &Path<'_>, index: u64, visitor: &mut V) -> u64
    where
        V: NodeVisitor,
    {
        let index = self.0.visit(path, index, visitor);
        self.1.visit(path, index, visitor)
    }

    #[inline]
    fn diff<D>(&self, path: &Path<'_>, index: u64, ancestor: &Self, differ: &mut D) -> u64
    where
        D: NodeDiffer,
    {
        let index = self.0.diff(path, index, &ancestor.0, differ);
        self.1.diff(path, index, &ancestor.1, differ)
    }
}

pub struct NodeListEntry<N>(N);

impl<N> NodeList for NodeListEntry<N>
where
    N: Node,
{
    #[inline]
    fn visit<V>(&self, path: &Path<'_>, index: u64, visitor: &mut V) -> u64
    where
        V: NodeVisitor,
    {
        let path = path.push(index);
        self.0.visit(&path, visitor);
        index + 1
    }

    #[inline]
    fn diff<D>(&self, path: &Path<'_>, index: u64, ancestor: &Self, differ: &mut D) -> u64
    where
        D: NodeDiffer,
    {
        let path = path.push(index);
        self.0.diff(&path, &ancestor.0, differ);
        index + 1
    }
}

mod tag;
mod text;

use std::borrow::Cow;

pub use self::tag::*;
pub use self::text::*;
use super::attr::{AttrDiffer, AttrList, AttrVisitor};
use crate::driver::Driver;

pub trait NodeVisitor<D>
where
    D: Driver,
{
    type Err;

    fn on_tag<T>(&mut self, index: usize, tag: &mut T) -> Result<(), Self::Err>
    where
        T: Tag<D>;

    fn on_text<T>(&mut self, index: usize, text: &mut T) -> Result<(), Self::Err>
    where
        T: Text<D>;
}

pub trait NodeDiffer<D>
where
    D: Driver,
{
    type Err;

    fn on_node_added<N>(&mut self, index: &mut usize, curr: &mut N) -> Result<(), Self::Err>
    where
        N: Node<D>;

    fn on_node_removed<N>(
        &mut self,
        ancestor_index: &mut usize,
        ancestor: &mut N,
    ) -> Result<(), Self::Err>
    where
        N: Node<D>;

    fn on_tag<T>(
        &mut self,
        curr_index: usize,
        ancestor_index: usize,
        curr: &mut T,
        ancestor: &mut T,
    ) -> Result<(), Self::Err>
    where
        T: Tag<D>;

    fn on_text<T>(
        &mut self,
        curr_index: usize,
        ancestor_index: usize,
        curr: &mut T,
        ancestor: &mut T,
    ) -> Result<(), Self::Err>
    where
        T: Text<D>;
}

pub trait Node<D>
where
    D: Driver,
{
    fn visit<NV>(&mut self, index: &mut usize, visitor: &mut NV) -> Result<(), NV::Err>
    where
        NV: NodeVisitor<D>;

    fn diff<ND>(
        &mut self,
        curr_index: &mut usize,
        ancestor_index: &mut usize,
        ancestor: &mut Self,
        differ: &mut ND,
    ) -> Result<(), ND::Err>
    where
        ND: NodeDiffer<D>;
}

impl<D, L1, L2> Node<D> for (L1, L2)
where
    D: Driver,
    L1: Node<D>,
    L2: Node<D>,
{
    fn visit<NV>(&mut self, index: &mut usize, visitor: &mut NV) -> Result<(), NV::Err>
    where
        NV: NodeVisitor<D>,
    {
        self.0.visit(index, visitor)?;
        self.1.visit(index, visitor)
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
        self.0
            .diff(curr_index, ancestor_index, &mut ancestor.0, differ)?;
        self.1
            .diff(curr_index, ancestor_index, &mut ancestor.1, differ)
    }
}

impl<D> Node<D> for ()
where
    D: Driver,
{
    fn visit<NV>(&mut self, index: &mut usize, visitor: &mut NV) -> Result<(), NV::Err>
    where
        NV: NodeVisitor<D>,
    {
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
        Ok(())
    }
}

impl<D, N> Node<D> for Option<N>
where
    D: Driver,
    N: Node<D>,
{
    fn visit<NV>(&mut self, index: &mut usize, visitor: &mut NV) -> Result<(), NV::Err>
    where
        NV: NodeVisitor<D>,
    {
        if let Some(node) = self {
            node.visit(index, visitor)
        } else {
            Ok(())
        }
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
        match (self, ancestor) {
            (Some(curr), Some(ancestor)) => curr.diff(curr_index, ancestor_index, ancestor, differ),
            (Some(curr), None) => differ.on_node_added(curr_index, curr),
            (None, Some(ancestor)) => differ.on_node_removed(ancestor_index, ancestor),
            (None, None) => Ok(()),
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

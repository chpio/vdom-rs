use crate::{
    driver::Driver,
    vdom::node::{Node, NodeDiffer, NodeVisitor},
};

use std::{
    cell::{Ref, RefCell, RefMut},
    marker::PhantomData,
    rc::Rc,
};

pub trait Comp<D>
where
    D: Driver,
    Self: Clone + Eq,
{
    type Input: Clone + Eq;
    type Rendered: Node<D>;

    fn new() -> Self;

    fn render(&self, input: &Self::Input, instance: &CompInstance<D, Self>) -> Self::Rendered;
}

enum CompNodeContent<D, C>
where
    D: Driver,
    C: Comp<D>,
{
    Input(Option<C::Input>),
    Snapshot(Snapshot<D, C>),
}

pub struct CompNode<D, C>
where
    D: Driver,
    C: Comp<D>,
{
    content: CompNodeContent<D, C>,
    instance: Option<CompInstance<D, C>>,
    driver_store: D::CompStore,
}

impl<D, C> CompNode<D, C>
where
    D: Driver,
    C: Comp<D>,
{
    pub fn new(input: C::Input) -> CompNode<D, C> {
        CompNode {
            content: CompNodeContent::Input(Some(input)),
            instance: None,
            driver_store: D::new_comp_store(),
        }
    }

    pub fn visit_rendered<NV>(&mut self, index: &mut usize, visitor: &mut NV) -> Result<(), NV::Err>
    where
        NV: NodeVisitor<D>,
    {
        self.render_and_get_snapshot(None)
            .rendered_mut()
            .visit(index, visitor)
    }

    pub fn diff_rendered<ND>(
        &mut self,
        curr_index: &mut usize,
        ancestor_index: &mut usize,
        ancestor: &mut Self,
        differ: &mut ND,
    ) -> Result<(), ND::Err>
    where
        ND: NodeDiffer<D>,
    {
        let snapshot_ancestor = match &ancestor.content {
            CompNodeContent::Snapshot(snapshot) => snapshot,
            _ => panic!("Ancestor `CompNode` not a rendered `Snapshot`"),
        };

        let snapshot = self.render_and_get_snapshot(Some(snapshot_ancestor));

        if !snapshot.ptr_eq(snapshot_ancestor) {
            snapshot.rendered_mut().diff(
                curr_index,
                ancestor_index,
                &mut *snapshot_ancestor.rendered_mut(),
                differ,
            )
        } else {
            Ok(())
        }
    }

    pub fn comp_instance(&self) -> Option<&CompInstance<D, C>> {
        self.instance.as_ref()
    }

    pub fn set_comp_instance(&mut self, instance: CompInstance<D, C>) {
        self.instance = Some(instance);
    }

    pub fn driver_store(&mut self) -> &mut D::CompStore {
        &mut self.driver_store
    }

    fn render_and_get_snapshot(
        &mut self,
        ancestor_snapshot: Option<&Snapshot<D, C>>,
    ) -> &Snapshot<D, C> {
        match &mut self.content {
            content @ CompNodeContent::Input(_) => {
                let input = match content {
                    CompNodeContent::Input(input) => input.take().unwrap(),
                    _ => unreachable!(),
                };
                let instance = match self.instance.as_ref() {
                    Some(i) => i,
                    None => panic!("`CompInstance` is `None`"),
                };

                let render = if let Some(ancestor_snapshot) = ancestor_snapshot {
                    let inner = ancestor_snapshot.inner.try_borrow().unwrap();
                    if inner.input == input && inner.state == *instance.as_ref() {
                        *content = CompNodeContent::Snapshot(ancestor_snapshot.clone());
                        false
                    } else {
                        true
                    }
                } else {
                    true
                };
                if render {
                    *content = CompNodeContent::Snapshot(Snapshot::new(instance, input));
                }

                match content {
                    CompNodeContent::Snapshot(snapshot) => snapshot,
                    _ => unreachable!(),
                }
            }
            CompNodeContent::Snapshot(snapshot) => snapshot,
        }
    }
}

impl<D, C> Node<D> for CompNode<D, C>
where
    D: Driver,
    C: Comp<D>,
{
    fn visit<NV>(&mut self, index: &mut usize, visitor: &mut NV) -> Result<(), NV::Err>
    where
        NV: NodeVisitor<D>,
    {
        visitor.on_comp(index, self)
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
        differ.on_comp(curr_index, ancestor_index, self, ancestor)
    }
}

pub struct CompInstance<D, C>
where
    D: Driver,
    C: Comp<D>,
{
    comp: Rc<RefCell<C>>,
    phantom: PhantomData<D>,
}

impl<D, C> CompInstance<D, C>
where
    D: Driver,
    C: Comp<D>,
{
    pub fn new(comp: C) -> CompInstance<D, C> {
        CompInstance {
            comp: Rc::new(RefCell::new(comp)),
            phantom: PhantomData,
        }
    }

    pub fn as_ref(&self) -> Ref<'_, C> {
        match self.comp.try_borrow() {
            Ok(comp) => comp,
            Err(e) => panic!(e),
        }
    }

    pub fn as_mut(&self) -> RefMut<'_, C> {
        match self.comp.try_borrow_mut() {
            Ok(comp) => comp,
            Err(e) => panic!(e),
        }
    }
}

impl<D, C> Clone for CompInstance<D, C>
where
    D: Driver,
    C: Comp<D>,
{
    fn clone(&self) -> Self {
        CompInstance {
            comp: self.comp.clone(),
            phantom: PhantomData,
        }
    }
}

struct SnapshotInner<D, C>
where
    D: Driver,
    C: Comp<D>,
{
    state: C,
    input: C::Input,
    rendered: Option<C::Rendered>,
}

pub struct Snapshot<D, C>
where
    D: Driver,
    C: Comp<D>,
{
    inner: Rc<RefCell<SnapshotInner<D, C>>>,
}

impl<D, C> Snapshot<D, C>
where
    D: Driver,
    C: Comp<D>,
{
    pub fn new(instance: &CompInstance<D, C>, input: C::Input) -> Snapshot<D, C> {
        let mut inner = Rc::new(RefCell::new(SnapshotInner {
            state: instance.as_ref().clone(),
            input,
            rendered: None,
        }));

        match Rc::get_mut(&mut inner) {
            Some(pinned) => {
                let pinned = pinned.get_mut();
                pinned.rendered = Some(pinned.state.render(&pinned.input, instance));
            }
            None => unreachable!(),
        }

        Snapshot { inner }
    }

    fn map<F, R>(&self, f: F) -> Ref<'_, R>
    where
        F: FnOnce(&SnapshotInner<D, C>) -> &R,
    {
        match self.inner.try_borrow() {
            Ok(inner) => Ref::map(inner, |inner| f(inner)),
            Err(e) => panic!(e),
        }
    }

    fn map_mut<F, R>(&self, f: F) -> RefMut<'_, R>
    where
        F: FnOnce(&mut SnapshotInner<D, C>) -> &mut R,
    {
        match self.inner.try_borrow_mut() {
            Ok(inner) => RefMut::map(inner, |inner| f(inner)),
            Err(e) => panic!(e),
        }
    }

    pub fn state(&self) -> Ref<'_, C> {
        self.map(|inner| &inner.state)
    }

    pub fn input(&self) -> Ref<'_, C::Input> {
        self.map(|inner| &inner.input)
    }

    pub fn rendered(&self) -> Ref<'_, C::Rendered> {
        self.map(|inner| {
            match &inner.rendered {
                Some(rendered) => rendered,
                None => unreachable!(),
            }
        })
    }

    pub fn rendered_mut(&self) -> RefMut<'_, C::Rendered> {
        self.map_mut(|inner| {
            match &mut inner.rendered {
                Some(rendered) => rendered,
                None => unreachable!(),
            }
        })
    }

    pub fn ptr_eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.inner, &other.inner)
    }
}

impl<D, C> Clone for Snapshot<D, C>
where
    D: Driver,
    C: Comp<D>,
{
    fn clone(&self) -> Self {
        Snapshot {
            inner: self.inner.clone(),
        }
    }
}

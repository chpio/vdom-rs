use crate::{
    driver::Driver,
    vdom::node::{Node, NodeDiffer, NodeVisitor},
};

use std::{
    cell::{Ref, RefCell, RefMut},
    marker::PhantomData,
    mem,
    rc::Rc,
};

pub trait Comp<D>
where
    D: Driver,
    Self: Clone + Eq,
{
    type Input: Clone + Eq;
    type Rendered: Node<D>;

    fn new(input: &Self::Input, ctx: &CompCtx<D, Self>) -> Self;

    fn render(&self, input: &Self::Input) -> Self::Rendered;
}

enum CompNodeCompRendered<D, C>
where
    D: Driver,
    C: Comp<D>,
{
    NotRendered,
    Rendered(C, C::Input, C::Rendered),
    Taken,
}

pub struct CompNode<D, C>
where
    D: Driver,
    C: Comp<D>,
{
    input: Option<C::Input>,
    comp_rendered: CompNodeCompRendered<D, C>,
    comp_ctx: Option<CompCtx<D, C>>,
    driver_store: D::CompStore,
}

impl<D, C> CompNode<D, C>
where
    D: Driver,
    C: Comp<D>,
{
    pub fn new(input: C::Input) -> CompNode<D, C> {
        CompNode {
            input: Some(input),
            comp_rendered: CompNodeCompRendered::NotRendered,
            comp_ctx: None,
            driver_store: D::new_comp_store(),
        }
    }

    pub fn comp_ctx(&self) -> Option<&CompCtx<D, C>> {
        self.comp_ctx.as_ref()
    }

    pub fn init_comp_ctx(&mut self) {
        self.comp_ctx = Some(CompCtx::new(self.input.take().unwrap()));
    }

    pub fn set_comp_ctx(&mut self, comp_instance: CompCtx<D, C>) {
        self.comp_ctx = Some(comp_instance);
    }

    pub fn visit_rendered<NV>(&mut self, index: &mut usize, visitor: &mut NV) -> Result<(), NV::Err>
    where
        NV: NodeVisitor<D>,
    {
        use self::CompNodeCompRendered::*;

        let rendered = match &mut self.comp_rendered {
            NotRendered => {
                let instance = self
                    .comp_ctx
                    .as_ref()
                    .expect("CompNode.comp_ctx is None")
                    .instance_mut();
                let rendered = instance.comp.render(&instance.input);
                self.comp_rendered =
                    Rendered(instance.comp.clone(), instance.input.clone(), rendered);
                match &mut self.comp_rendered {
                    Rendered(_, _, rendered) => rendered,
                    _ => unreachable!(),
                }
            }
            Rendered(_, _, rendered) => rendered,
            Taken => panic!("comp_rendered is Taken"),
        };
        rendered.visit(index, visitor)
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
        use self::CompNodeCompRendered::*;

        let (ancestor_comp, ancestor_input, ancestor_rendered) = match &mut ancestor.comp_rendered {
            NotRendered => panic!("ancestor.comp_rendered is NotRendered"),
            Rendered(ancestor_comp, ancestor_input, ancestor_rendered) => {
                (ancestor_comp, ancestor_input, ancestor_rendered)
            }
            Taken => {
                match &self.comp_rendered {
                    NotRendered => panic!("self.comp_rendered is NotRendered"),
                    Rendered(..) => return Ok(()),
                    Taken => panic!("self.comp_rendered is Taken"),
                }
            }
        };
        let rendered = match &mut self.comp_rendered {
            NotRendered => {
                let instance = self
                    .comp_ctx
                    .as_ref()
                    .expect("CompNode.comp_ctx is None")
                    .instance_mut();
                if ancestor_comp == &instance.comp && ancestor_input == &instance.input {
                    self.comp_rendered = mem::replace(&mut ancestor.comp_rendered, Taken);
                    return Ok(());
                } else {
                    let rendered = instance.comp.render(&instance.input);
                    self.comp_rendered =
                        Rendered(instance.comp.clone(), instance.input.clone(), rendered);
                    match &mut self.comp_rendered {
                        Rendered(_, _, rendered) => rendered,
                        _ => unreachable!(),
                    }
                }
            }
            Rendered(_, _, rendered) => rendered,
            Taken => panic!("self.comp_rendered is Taken"),
        };
        rendered.diff(curr_index, ancestor_index, ancestor_rendered, differ)
    }

    pub fn driver_store(&mut self) -> &mut D::CompStore {
        &mut self.driver_store
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
    pub comp: C,
    pub input: C::Input,
    phantom: PhantomData<D>,
}

pub struct CompCtx<D, C>
where
    D: Driver,
    C: Comp<D>,
{
    instance: Rc<RefCell<Option<CompInstance<D, C>>>>,
}

impl<D, C> CompCtx<D, C>
where
    D: Driver,
    C: Comp<D>,
{
    pub fn new(input: C::Input) -> CompCtx<D, C> {
        let ctx = CompCtx {
            instance: Rc::new(RefCell::new(None)),
        };
        let comp = C::new(&input, &ctx);
        ctx.instance.replace(Some(CompInstance {
            comp,
            input,
            phantom: PhantomData,
        }));
        ctx
    }

    pub fn instance(&self) -> Ref<'_, CompInstance<D, C>> {
        Ref::map(self.instance.borrow(), |r| r.as_ref().unwrap())
    }

    pub fn instance_mut(&self) -> RefMut<'_, CompInstance<D, C>> {
        RefMut::map(self.instance.borrow_mut(), |r| r.as_mut().unwrap())
    }
}

impl<D, C> Clone for CompCtx<D, C>
where
    D: Driver,
    C: Comp<D>,
{
    fn clone(&self) -> Self {
        CompCtx {
            instance: self.instance.clone(),
        }
    }
}

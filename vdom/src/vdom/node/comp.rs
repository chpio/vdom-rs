use crate::{
    driver::Driver,
    vdom::node::{Node, NodeDiffer, NodeVisitor},
};

use std::{cell::RefCell, marker::PhantomData, mem, rc::Rc};

pub trait Comp<D>
where
    D: Driver,
    Self: Clone + Eq,
{
    type Input: Eq;
    type Rendered: Node<D>;

    fn new(input: &Self::Input) -> Self;

    fn render(&self, input: &Self::Input) -> Self::Rendered;
}

enum CompNodeCompRendered<D, C>
where
    D: Driver,
    C: Comp<D>,
{
    NotRendered,
    Rendered(C, C::Rendered),
    Taken,
}

pub struct CompNode<D, C>
where
    D: Driver,
    C: Comp<D>,
{
    input: C::Input,
    comp_rendered: CompNodeCompRendered<D, C>,
    comp_instance: Option<CompInstance<D, C>>,
    driver_store: D::CompStore,
}

impl<D, C> CompNode<D, C>
where
    D: Driver,
    C: Comp<D>,
{
    pub fn new(input: C::Input) -> CompNode<D, C> {
        CompNode {
            input: input,
            comp_rendered: CompNodeCompRendered::NotRendered,
            comp_instance: None,
            driver_store: D::new_comp_store(),
        }
    }

    pub fn input(&mut self) -> &C::Input {
        &self.input
    }

    pub fn comp_instance(&self) -> Option<&CompInstance<D, C>> {
        self.comp_instance.as_ref()
    }

    pub fn init_comp_instance(&mut self) {
        self.comp_instance = Some(CompInstance::new(&self.input));
    }

    pub fn set_comp_instance(&mut self, comp_instance: CompInstance<D, C>) {
        self.comp_instance = Some(comp_instance);
    }

    pub fn visit_rendered<NV>(&mut self, index: &mut usize, visitor: &mut NV) -> Result<(), NV::Err>
    where
        NV: NodeVisitor<D>,
    {
        use self::CompNodeCompRendered::*;

        let rendered = match &mut self.comp_rendered {
            NotRendered => {
                let comp = self
                    .comp_instance
                    .as_ref()
                    .expect("wrapper is None")
                    .comp
                    .borrow_mut();
                let rendered = comp.render(&self.input);
                self.comp_rendered = Rendered(comp.clone(), rendered);
                match &mut self.comp_rendered {
                    Rendered(_, rendered) => rendered,
                    _ => unreachable!(),
                }
            }
            Rendered(_, rendered) => rendered,
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

        let (ancestor_comp, ancestor_rendered) = match &mut ancestor.comp_rendered {
            NotRendered => panic!("ancestor.comp_rendered is NotRendered"),
            Rendered(ancestor_comp, ancestor_rendered) => (ancestor_comp, ancestor_rendered),
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
                let comp = self
                    .comp_instance
                    .as_ref()
                    .expect("wrapper is None")
                    .comp
                    .borrow_mut();
                let rendered = comp.render(&self.input);
                if ancestor_comp == &*comp && ancestor.input == self.input {
                    self.comp_rendered = mem::replace(&mut ancestor.comp_rendered, Taken);
                    return Ok(());
                } else {
                    self.comp_rendered = Rendered(comp.clone(), rendered);
                    match &mut self.comp_rendered {
                        Rendered(_, rendered) => rendered,
                        _ => unreachable!(),
                    }
                }
            }
            Rendered(_, rendered) => rendered,
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
    comp: Rc<RefCell<C>>,
    phantom: PhantomData<D>,
}

impl<D, C> CompInstance<D, C>
where
    D: Driver,
    C: Comp<D>,
{
    pub fn new(input: &C::Input) -> CompInstance<D, C> {
        CompInstance {
            comp: Rc::new(RefCell::new(C::new(input))),
            phantom: PhantomData,
        }
    }

    pub fn comp(&self) -> &Rc<RefCell<C>> {
        &self.comp
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

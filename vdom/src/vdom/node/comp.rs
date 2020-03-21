use crate::{
    driver::{Driver, DriverCtx},
    vdom::node::{Node, NodeDiffer, NodeVisitor},
};
use futures::{channel::mpsc, Sink, Stream, StreamExt as _};
use std::{
    cell::{Ref, RefCell, RefMut},
    future::Future,
    hash::{Hash, Hasher},
    marker::PhantomData,
    mem,
    pin::Pin,
    rc::{Rc, Weak},
    task,
};

pub trait Comp<D>
where
    D: Driver,
    Self: Clone + Eq,
{
    type Input: Clone + Eq;
    type Rendered: Node<D>;

    fn new(input: &Self::Input, ctx: CompCtx<D, Self>) -> Self;

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
    comp_ctx: Option<StrongCompCtx<D, C>>,
    driver_store: D::CompStore, // TODO: rename to `D::CompNodeStore`?
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

    pub fn comp_ctx(&self) -> Option<&StrongCompCtx<D, C>> {
        self.comp_ctx.as_ref()
    }

    pub fn init_comp_ctx(&mut self, driver_ctx: DriverCtx<D>) {
        self.comp_ctx = Some(StrongCompCtx::new(driver_ctx, self.input.take().unwrap()));
    }

    pub fn set_comp_ctx(&mut self, comp_instance: StrongCompCtx<D, C>) {
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
    driver_ctx: DriverCtx<D>,
    phantom: PhantomData<D>,
}

pub struct StrongCompCtx<D, C>
where
    D: Driver,
    C: Comp<D>,
{
    instance: Rc<RefCell<Option<CompInstance<D, C>>>>,
}

impl<D, C> StrongCompCtx<D, C>
where
    D: Driver,
    C: Comp<D>,
{
    pub fn new(driver_ctx: DriverCtx<D>, input: C::Input) -> StrongCompCtx<D, C> {
        let ctx = StrongCompCtx {
            instance: Rc::new(RefCell::new(None)),
        };
        let comp = C::new(&input, ctx.downgrade());
        *ctx.instance.borrow_mut() = Some(CompInstance {
            comp,
            input,
            driver_ctx,
            phantom: PhantomData,
        });
        ctx
    }

    pub fn downgrade(&self) -> CompCtx<D, C> {
        CompCtx {
            instance: Rc::downgrade(&self.instance),
        }
    }

    pub fn instance(&self) -> Ref<'_, CompInstance<D, C>> {
        Ref::map(self.instance.borrow(), |r| r.as_ref().unwrap())
    }

    pub fn instance_mut(&self) -> RefMut<'_, CompInstance<D, C>> {
        RefMut::map(self.instance.borrow_mut(), |r| r.as_mut().unwrap())
    }
}

impl<D, C> Clone for StrongCompCtx<D, C>
where
    D: Driver,
    C: Comp<D>,
{
    fn clone(&self) -> Self {
        StrongCompCtx {
            instance: self.instance.clone(),
        }
    }
}

impl<D, C> PartialEq for StrongCompCtx<D, C>
where
    D: Driver,
    C: Comp<D>,
{
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.instance, &other.instance)
    }
}

impl<D, C> Eq for StrongCompCtx<D, C>
where
    D: Driver,
    C: Comp<D>,
{
}

impl<D, C> Hash for StrongCompCtx<D, C>
where
    D: Driver,
    C: Comp<D>,
{
    fn hash<H: Hasher>(&self, state: &mut H) {
        state.write_usize(&*self.instance as *const _ as usize);
    }
}

pub struct CompCtx<D, C>
where
    D: Driver,
    C: Comp<D>,
{
    instance: Weak<RefCell<Option<CompInstance<D, C>>>>,
}

impl<D, C> CompCtx<D, C>
where
    D: Driver,
    C: Comp<D>,
{
    pub fn build_stream<F, T, R>(&self, f: F) -> mpsc::UnboundedSender<T>
    where
        F: FnOnce(mpsc::UnboundedReceiver<T>) -> R,
        R: Future<Output = ()> + 'static,
    {
        let (sender, receiver) = mpsc::unbounded();

        let fut = f(receiver);

        // let id =
        self.with_instance(|instance| {
            instance.driver_ctx.with_mut(|drv| {
                drv.spawn(fut);
            });
            // instance.driver_ctx.next_id()
        })
        .unwrap();

        sender
        // Sender { sender, id }
    }

    pub fn with_instance<F, R>(&self, f: F) -> Option<R>
    where
        F: FnOnce(&CompInstance<D, C>) -> R,
    {
        if let Some(instance) = self.instance.upgrade() {
            let instance = instance.borrow();
            let instance = instance.as_ref().unwrap();
            Some(f(instance))
        } else {
            None
        }
    }

    pub fn with_instance_mut<F, R>(&self, f: F) -> Option<R>
    where
        F: FnOnce(&mut CompInstance<D, C>) -> R,
    {
        if let Some(instance) = self.instance.upgrade() {
            let mut instance = instance.borrow_mut();
            let instance = instance.as_mut().unwrap();
            Some(f(instance))
        } else {
            None
        }
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

// #[derive(Debug)]
// pub struct Sender<T> {
//     sender: mpsc::UnboundedSender<T>,
//     id: u64,
// }

// impl<T> PartialEq for Sender<T> {
//     fn eq(&self, other: &Self) -> bool {
//         self.id == other.id
//     }
// }

// impl<T> Eq for Sender<T> {}

// impl<T> Clone for Sender<T> {
//     fn clone(&self) -> Self {
//         Sender {
//             sender: self.sender.clone(),
//             id: self.id,
//         }
//     }
// }

// pub struct ForwardWith<S, F, R, D, C>
// where
//     S: Stream,
//     F: FnMut(&CompInstance<D, C>) -> &R,
//     R: Sink<S::Item>,
//     D: Driver,
//     C: Comp<D>,
// {
//     stream: S,
//     sink: R,
//     f: F,
//     ctx: CompCtx<D, C>,
// }

// // impl ForwardWith<S, F, R, D, C>
// // where
// //     S: Stream,
// //     F: FnMut(&'_ mut CompInstance<D, C>) -> &'_ mut R,
// //     R: Sink<S::Item>,
// //     D: Driver,
// //     C: Comp<D>,
// // {
// // }

// impl<S, F, R, D, C> Future for ForwardWith<S, F, R, D, C>
// where
//     S: Stream,
//     F: FnMut(&'_ mut CompInstance<D, C>) -> &'_ R,
//     R: Sink<S::Item>,
//     D: Driver,
//     C: Comp<D>,
// {
//     type Output = ();

//     fn poll(self: Pin<&mut Self>, cx: &mut task::Context<'_>) -> task::Poll<Self::Output> {
//         // loop {
//         //     match self.as_mut().stream().poll_next(cx) {
//         //         Poll::Ready(Some(Ok(item))) => ready!(self.as_mut().try_start_send(cx, item))?,
//         //         Poll::Ready(Some(Err(e))) => return Poll::Ready(Err(e)),
//         //         Poll::Ready(None) => {
//         //             ready!(self
//         //                 .as_mut()
//         //                 .sink()
//         //                 .as_pin_mut()
//         //                 .expect(INVALID_POLL)
//         //                 .poll_close(cx))?;
//         //             self.as_mut().sink().set(None);
//         //             return Poll::Ready(Ok(()));
//         //         }
//         //         Poll::Pending => {
//         //             ready!(self
//         //                 .as_mut()
//         //                 .sink()
//         //                 .as_pin_mut()
//         //                 .expect(INVALID_POLL)
//         //                 .poll_flush(cx))?;
//         //             return Poll::Pending;
//         //         }
//         //     }
//         // }

//         unimplemented!()
//     }
// }

// impl<S, F, R, D, C> LifetimeReceiver<D, C> for ForwardWith<S, F, R, D, C>
// where
//     S: Stream,
//     F: FnMut(&'_ mut CompInstance<D, C>) -> & R,
//     R: Sink<S::Item> + Eq,
//     D: Driver,
//     C: Comp<D>,
// {
//     fn on_input_changed(&mut self, instance: &mut CompInstance<D, C>) {
//         let new_sink = self.f(instance);

//         if new_sink != self.sink {
//             self.sink = new_sink;
//         }
//     }
// }

// pub trait StreamExt: Stream {
//     fn forward_with<F, R, D, C>(self, ctx: CompCtx<D, C>, f: F) -> ForwardWith<Self, F, R, D, C>
//     where
//         Self: Sized,
//         F: FnMut(&'_ mut CompInstance<D, C>) -> &'_ mut R,
//         R: Sink<Self::Item>,
//         D: Driver,
//         C: Comp<D>,
//     {
//         unimplemented!()
//     }
// }

// pub trait LifetimeReceiver<D, C>
// where
//     D: Driver,
//     C: Comp<D>,
// {
//     fn on_input_changed(&mut self, _instance: &mut CompInstance<D, C>) {}
// }

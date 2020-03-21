use futures::Future;
use std::{cell::RefCell, marker::PhantomData, rc::Rc};

pub trait Driver /*: LocalSpawn */ {
    type AttrStore;
    type TagStore;
    type TextStore;
    type CompStore;

    fn new_attr_store() -> Self::AttrStore;
    fn new_tag_store() -> Self::TagStore;
    fn new_text_store() -> Self::TextStore;
    fn new_comp_store() -> Self::CompStore;

    fn spawn<F>(&mut self, fut: F)
    where
        F: Future<Output = ()> + 'static;
}

struct DriverInstance<D> {
    id: u64,
    driver: D,
}

pub struct DriverCtx<D> {
    instance: Rc<RefCell<DriverInstance<D>>>,
}

impl<D> DriverCtx<D> {
    pub fn new(driver: D) -> DriverCtx<D> {
        DriverCtx {
            instance: Rc::new(RefCell::new(DriverInstance { id: 0, driver })),
        }
    }

    pub fn next_id(&self) -> u64 {
        let mut instance = self.instance.borrow_mut();
        assert!(instance.id < u64::max_value());
        instance.id += 1;
        instance.id
    }

    pub fn with<F, R>(&self, f: F) -> R
    where
        F: FnOnce(&D) -> R,
    {
        let instance = self.instance.borrow();
        f(&instance.driver)
    }

    pub fn with_mut<F, R>(&self, f: F) -> R
    where
        F: FnOnce(&mut D) -> R,
    {
        let mut instance = self.instance.borrow_mut();
        f(&mut instance.driver)
    }
}

impl<D> Clone for DriverCtx<D> {
    fn clone(&self) -> Self {
        DriverCtx {
            instance: self.instance.clone(),
        }
    }
}

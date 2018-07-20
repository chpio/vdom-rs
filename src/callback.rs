use diff::{Context, Driver};
use path::Path;
use std::fmt;
use std::marker::PhantomData;
use widget::{Widget, WidgetHolderTrait};

// generic W to make rustc happy at `impl InnerTrait for Inner` due to the
// "unused type parameter" error
#[derive(Clone)]
struct Inner<F, W> {
    callback: F,
    pd: PhantomData<W>,
}

trait InnerTrait<I, O> {
    fn call(&mut self, ctx: &mut Context, input: I, widget_path: &Path) -> O;
    fn clone_box(&self) -> Box<InnerTrait<I, O>>;
}

impl<I, O, W, F> InnerTrait<I, O> for Inner<F, W>
where
    W: 'static + Widget,
    F: 'static + FnMut(&mut W, I) -> O + Clone,
{
    fn call(&mut self, ctx: &mut Context, input: I, widget_path: &Path) -> O {
        let widget_holder = ctx
            .widget_holders
            .get_mut(widget_path)
            .unwrap()
            .downcast_mut::<W>()
            .unwrap();
        let res = (self.callback)(&mut widget_holder.curr_widget, input);
        widget_holder.update_is_dirty();
        res
    }

    fn clone_box(&self) -> Box<InnerTrait<I, O>> {
        Box::new(self.clone())
    }
}

pub struct WidgetCallback<I, O> {
    callback: Box<InnerTrait<I, O>>,
    widget_path: Path,
}

impl<I, O> WidgetCallback<I, O> {
    pub fn new<F, W>(widget_path: Path, f: F) -> WidgetCallback<I, O>
    where
        W: 'static + Widget,
        F: 'static + FnMut(&mut W, I) -> O + Clone,
    {
        WidgetCallback {
            callback: Box::new(Inner {
                callback: f,
                pd: PhantomData,
            }),
            widget_path: widget_path,
        }
    }

    pub fn call(&mut self, ctx: &mut Context, input: I) -> O {
        self.callback.call(input, ctx, &self.widget_path)
    }
}

impl<I, O> fmt::Debug for WidgetCallback<I, O> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("WidgetCallback")
            .field("callback", &"{Fn}")
            .field("widget_path", &self.widget_path)
            .finish()
    }
}

impl<I, O> Clone for WidgetCallback<I, O> {
    fn clone(&self) -> WidgetCallback<I, O> {
        WidgetCallback {
            callback: self.callback.clone_box(),
            widget_path: self.widget_path.clone(),
        }
    }
}

impl<I, O> PartialEq for WidgetCallback<I, O> {
    fn eq(&self, _other: &WidgetCallback<I, O>) -> bool {
        true
    }
}

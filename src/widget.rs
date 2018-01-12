use Child;
use ChildBuilder;
use path::Path;
use diff::Differ;

use std::any::TypeId;
use std::fmt::Debug;
use std::mem;

pub trait WidgetDataTrait: Debug {
    fn try_render(&mut self, differ: &mut Differ, path: Path) -> Option<Child>;
    fn remove(&self, differ: &mut Differ, path: &Path);
    fn widget_type_id(&self) -> TypeId;
}

#[derive(Debug)]
pub struct WidgetData<W: Widget> {
    input: Option<W::Input>,
}

impl<W> WidgetData<W>
where
    W: 'static + Widget,
{
    pub fn new(input: W::Input) -> WidgetData<W> {
        WidgetData { input: Some(input) }
    }
}

impl<W> WidgetDataTrait for WidgetData<W>
where
    W: 'static + Widget,
{
    // self is "curr"
    fn try_render(&mut self, differ: &mut Differ, path: Path) -> Option<Child> {
        use std::collections::hash_map::Entry::{Occupied, Vacant};
        match differ.widget_holders.entry(path.clone()) {
            Occupied(mut oe) => {
                let widget_holder = oe.get_mut()
                    .downcast_mut::<W>()
                    .unwrap_or_else(|| panic!("WidgetHolder `{}` has wrong Widget type", path));
                if let Some(input) = self.input.take() {
                    widget_holder.last_input =
                        Some(mem::replace(&mut widget_holder.curr_input, input));
                }
                if widget_holder.should_rerender() {
                    Some(widget_holder.render())
                } else {
                    None
                }
            }
            Vacant(ve) => {
                let widget = W::new();
                let mut widget_holder = WidgetHolder {
                    last_widget: None,
                    curr_widget: widget,
                    last_input: None,
                    curr_input: self.input.take().unwrap(),
                    is_dirty: false,
                };
                let res = Some(widget_holder.render());
                ve.insert(Box::new(widget_holder));
                res
            }
        }
    }

    fn remove(&self, differ: &mut Differ, path: &Path) {
        differ
            .widget_holders
            .remove(path)
            .unwrap_or_else(|| panic!("Widget `{}` is non-existent", path))
            .remove();
    }

    fn widget_type_id(&self) -> TypeId {
        TypeId::of::<W>()
    }
}

pub trait Widget: Debug + Eq + Clone {
    type Input: Debug + Eq;

    fn new() -> Self;
    fn remove(self) {}
    fn should_rerender(
        last: &Self,
        curr: &Self,
        last_input: &Self::Input,
        curr_input: &Self::Input,
    ) -> bool {
        last_input != curr_input || last != curr
    }
    fn render(&self, &Self::Input) -> ChildBuilder<Self>;
}

#[derive(Debug)]
pub struct WidgetHolder<W>
where
    W: 'static + Widget,
{
    pub last_widget: Option<W>,
    pub curr_widget: W,
    pub last_input: Option<W::Input>,
    pub curr_input: W::Input,
    pub is_dirty: bool,
}

pub trait WidgetHolderTrait: 'static {
    fn should_rerender(&self) -> bool;
    fn render(&mut self) -> Child;
    fn remove(self: Box<Self>);
    fn widget_type_id(&self) -> TypeId;
}

impl WidgetHolderTrait {
    #[inline]
    pub fn is<W>(&self) -> bool
    where
        W: 'static + Widget,
    {
        TypeId::of::<W>() == self.widget_type_id()
    }

    pub fn downcast_ref<W>(&self) -> Option<&WidgetHolder<W>>
    where
        W: 'static + Widget,
    {
        if self.is::<W>() {
            unsafe { Some(&*(self as *const _ as *const WidgetHolder<W>)) }
        } else {
            None
        }
    }

    pub fn downcast_mut<W>(&mut self) -> Option<&mut WidgetHolder<W>>
    where
        W: 'static + Widget,
    {
        if self.is::<W>() {
            unsafe { Some(&mut *(self as *mut _ as *mut WidgetHolder<W>)) }
        } else {
            None
        }
    }
}

impl<W> WidgetHolderTrait for WidgetHolder<W>
where
    W: 'static + Widget,
{
    fn should_rerender(&self) -> bool {
        if !self.is_dirty {
            return false;
        }
        if let (&Some(ref last_widget), &Some(ref last_input)) =
            (&self.last_widget, &self.last_input)
        {
            W::should_rerender(last_widget, &self.curr_widget, last_input, &self.curr_input)
        } else {
            true
        }
    }

    fn render(&mut self) -> Child {
        self.is_dirty = false;
        self.last_widget = Some(self.curr_widget.clone());
        self.curr_widget.render(&self.curr_input).into()
    }

    fn remove(self: Box<Self>) {
        self.curr_widget.remove();
    }

    fn widget_type_id(&self) -> TypeId {
        TypeId::of::<W>()
    }
}

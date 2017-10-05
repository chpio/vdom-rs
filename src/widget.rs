use Child;
use path::PathFrame;
use diff::Differ;

use std::any::TypeId;
use std::fmt::Debug;

pub trait WidgetDataTrait: Debug {
    fn render(
        &self,
        differ: &mut Differ,
        pf: &PathFrame,
        last: Option<&WidgetDataTrait>,
    ) -> Option<Child>;
    fn remove(&self, differ: &mut Differ, pf: &PathFrame);
    fn widget_type_id(&self) -> TypeId;
}

#[derive(Debug)]
pub struct WidgetData<W: Widget>(pub W::Input);

impl<W> WidgetDataTrait for WidgetData<W>
where
    W: Widget + 'static,
{
    // self is "curr"
    fn render(
        &self,
        differ: &mut Differ,
        pf: &PathFrame,
        last: Option<&WidgetDataTrait>,
    ) -> Option<Child> {
        if let Some(last) = last {
            assert_eq!(
                self.widget_type_id(),
                last.widget_type_id(),
                "Last input has other widget type than curr input `{}`",
                pf
            );
            let last = unsafe { &*(last as *const _ as *const WidgetData<W>) };
            let widget_holder = differ
                .widget_holders
                .get(&pf.to_path())
                .unwrap_or_else(|| panic!("Widget `{}` is non-existent", pf))
                .downcast_ref::<WidgetHolder<W>>()
                .unwrap_or_else(|| panic!("WidgetHolder `{}` has wrong Widget type", pf));
            if !widget_holder.should_rerender(&last.0, &self.0) {
                return None;
            }
            return Some(widget_holder.curr_widget.render(&self.0));
        }

        let widget = W::new();
        let res = Some(widget.render(&self.0));
        assert!(
            differ
                .widget_holders
                .insert(pf.to_path(), Box::new(WidgetHolder::new(widget)))
                .is_none(),
            "Widget `{}` already inserted",
            pf
        );
        res
    }

    fn remove(&self, differ: &mut Differ, pf: &PathFrame) {
        let widget_holder = differ
            .widget_holders
            .remove(&pf.to_path())
            .unwrap_or_else(|| panic!("Widget `{}` is non-existent", pf))
            .downcast::<WidgetHolder<W>>()
            .ok()
            .unwrap_or_else(|| panic!("WidgetHolder `{}` has wrong Widget type", pf));
        widget_holder.curr_widget.remove();
    }

    fn widget_type_id(&self) -> TypeId {
        TypeId::of::<W>()
    }
}

pub trait Widget: Debug + Eq + Clone {
    type Input: Debug + Eq;

    fn new() -> Self;
    fn remove(self) {}
    fn render(&self, &Self::Input) -> Child;

    fn should_rerender(
        last: &Self,
        curr: &Self,
        last_input: &Self::Input,
        curr_input: &Self::Input,
    ) -> bool {
        last_input != curr_input || last != curr
    }
}

#[derive(Debug)]
pub struct WidgetHolder<W: Widget> {
    pub last_widget: Option<W>,
    pub curr_widget: W,
}

impl<W: Widget> WidgetHolder<W> {
    fn new(curr: W) -> WidgetHolder<W> {
        WidgetHolder {
            last_widget: None,
            curr_widget: curr,
        }
    }

    fn should_rerender(&self, last_input: &W::Input, curr_input: &W::Input) -> bool {
        match &self.last_widget {
            &Some(ref last) => W::should_rerender(last, &self.curr_widget, last_input, curr_input),
            &None => true,
        }
    }
}

use event::{Event, ListenerManager};
use path::{Ident, Path, PathFrame};
use widget::{Widget, WidgetData, WidgetHolderTrait};
use {Child, ChildBuilder, Node};

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::{Rc, Weak};

pub trait Driver {
    fn node_added(ctx_driver: &mut ContextDriver<Self>, pf: &PathFrame, index: usize, ty: &str);
    fn node_removed(ctx_driver: &mut ContextDriver<Self>, pf: &PathFrame);

    fn text_added(ctx_driver: &mut ContextDriver<Self>, pf: &PathFrame, index: usize, curr: &str);
    fn text_changed(ctx_driver: &mut ContextDriver<Self>, pf: &PathFrame, curr: &str);
    fn text_removed(ctx_driver: &mut ContextDriver<Self>, pf: &PathFrame);

    fn node_reordered(ctx_driver: &mut ContextDriver<Self>, pf: &PathFrame, index: usize);

    fn attribute_added(
        ctx_driver: &mut ContextDriver<Self>,
        pf: &PathFrame,
        name: &str,
        value: &str,
    );
    fn attribute_changed(
        ctx_driver: &mut ContextDriver<Self>,
        pf: &PathFrame,
        name: &str,
        value: &str,
    );
    fn attribute_removed(ctx_driver: &mut ContextDriver<Self>, pf: &PathFrame, name: &str);

    fn flush_changes(ctx_driver: &mut ContextDriver<Self>);

    fn schedule_repaint(ctx_driver: &mut ContextDriver<Self>);

    fn register_root_event_listener<E, F>(ctx_driver: &mut ContextDriver<Self>, F)
    where
        F: Fn(&mut ContextDriver<Self>, &Path, E),
        E: Event;

    fn unregister_root_event_listener<E>(ctx_driver: &mut ContextDriver<Self>)
    where
        E: Event;
}

pub struct ContextDriver<D: Driver> {
    pub ctx: Context,
    pub driver: D,
    pub weak: Option<Weak<RefCell<ContextDriver<D>>>>,
}

pub struct Context {
    curr: Option<Child>,
    last: Option<Child>,
    widget_holders: HashMap<Path, Box<WidgetHolderTrait>>,
    curr_widget_path: Option<Path>,
    listener_manager: ListenerManager,
}

pub struct RootContext<D: Driver>(Rc<ReffCell<ContextDriver>>);

impl<D: Driver> RootContext<D> {
    pub fn new(driver: D) -> RootContext<D> {
        let ctx = Context {
            curr: None,
            last: None,
            widget_holders: HashMap::new(),
            curr_widget_path: None,
            listener_manager: ListenerManager::new(),
            driver: driver,
            weak: None,
        };
        let ctx_driver = ContextDriver {
            ctx: ctx,
            driver: driver,
            weak: None,
        };
        let ctx_driver = Rc::new(RefCell::new(ctx_driver));
        ctx_driver.borrow_mut().weak = Some(Rc::downgrade(&ctx_driver));
        RootContext(ctx_driver)
    }

    // pub fn get(&self) -> &RefCell<Differ> {
    //     &*self.differ
    // }

    pub fn update<W>(&mut self, input: W::Input)
    where
        W: 'static + Widget,
    {
        let mut ctx_driver = self.0.borrow_mut();
        let curr: ChildBuilder<W> = WidgetData::<W>::new(input).into();
        ctx_driver.ctx.curr = Some(curr.into());
        D::schedule_repaint(ctx_driver);
    }
}

fn diff_attributes<D>(ctx_driver: &mut ContextDriver<D>, pf: &PathFrame, last: &Node, curr: &Node) {
    let curr = &curr.attributes;
    let last = &last.attributes;

    for (name, c_value) in curr.iter() {
        match last.get(name) {
            Some(l_value) => {
                if l_value != c_value {
                    D::attribute_changed(ctx_driver, pf, name, c_value);
                }
            }
            None => D::attribute_added(ctx_driver, pf, name, c_value),
        }
    }

    for name in last.keys().filter(|name| !curr.contains_key(name.as_ref())) {
        D::attribute_removed(ctx_driver, pf, name);
    }
}

fn diff_event_listeners<D>(
    ctx_driver: &mut ContextDriver<D>,
    pf: &PathFrame,
    last: &Node,
    curr: &mut Node,
) {
    for listener in curr.event_listeners.values_mut() {
        let widget_path = ctx_driver.ctx.curr_widget_path.as_ref().unwrap().clone();
        ListenerManager::register(
            ctx_driver,
            pf.to_path(),
            widget_path,
            listener.take().unwrap(),
        );
    }

    for type_id in last.event_listeners
        .keys()
        .filter(|type_id| !curr.event_listeners.contains_key(type_id))
    {
        ListenerManager::unregister(ctx_driver, pf.to_path(), *type_id);
    }
}

fn diff_nodes<D>(
    ctx_driver: &mut ContextDriver<D>,
    pf: &PathFrame,
    last: &mut Node,
    curr: &mut Node,
) {
    diff_attributes(ctx_driver, pf, last, curr);
    diff_event_listeners(ctx_driver, pf, last, curr);

    for (index, &mut (ref ident, ref mut curr_child)) in curr.children.iter_mut().enumerate() {
        let pf = pf.add_ident(ident.clone());
        let last_index = match ident {
            &Ident::Key(ref key) => last.keyed_children.get(key).map(|i| *i),
            &Ident::NonKeyedIndex(non_keyed_index) => {
                last.non_keyed_children.get(non_keyed_index).map(|i| *i)
            }
        };
        let last_child = last_index
            .and_then(|i| last.children.get_mut(i))
            .map(|&mut (_, ref mut child)| child);
        diff(ctx_driver, &pf, index, last_child, Some(curr_child));
        if let Some(last_index) = last_index {
            if last_index != index {
                D::node_reordered(ctx_driver, &pf, index);
            }
        }
    }

    // remove non-keyed nodes
    for (non_keyed_index, index) in last.non_keyed_children
        .iter()
        .enumerate()
        .skip(curr.non_keyed_children.len())
    {
        let pf = pf.add_non_keyed_index(non_keyed_index);
        let l = &mut last.children.get_mut(*index).unwrap().1;
        diff(ctx_driver, &pf, 0, Some(l), None);
    }

    // remove keyed nodes
    for (key, index) in last.keyed_children
        .iter()
        .filter(|&(ref key, _)| !curr.keyed_children.contains_key(key))
    {
        let pf = pf.add_key(key.clone());
        let l = &mut last.children.get_mut(*index).unwrap().1;
        diff(ctx_driver, &pf, 0, Some(l), None);
    }
}

fn visit_children<F>(pf: &PathFrame, node: &mut Node, f: &mut F)
where
    F: FnMut(&PathFrame, usize, &mut Child),
{
    for (index, &mut (ref ident, ref mut child)) in node.children.iter_mut().enumerate() {
        let pf = pf.add_ident(ident.clone());
        f(&pf, index, child);
    }
}

fn node_added<D>(ctx_driver: &mut ContextDriver<D>, pf: &PathFrame, index: usize, curr: &mut Node) {
    D::node_added(ctx_driver, pf, index, curr.ty.as_ref());

    for (name, value) in curr.attributes.iter() {
        D::attribute_added(ctx_driver, pf, name, value);
    }

    for listener in curr.event_listeners.values_mut() {
        let widget_path = ctx_driver.ctx.curr_widget_path.as_ref().unwrap().clone();
        ListenerManager::register(
            ctx_driver,
            pf.to_path(),
            widget_path,
            listener.take().unwrap(),
        );
    }

    visit_children(pf, curr, &mut |pf, index, child| {
        diff(ctx_driver, pf, index, None, Some(child));
    });
}

fn node_removed<D>(ctx_driver: &mut ContextDriver<D>, pf: &PathFrame, last: &mut Node) {
    for type_id in last.event_listeners.keys() {
        ListenerManager::unregister(ctx_driver, pf.to_path(), *type_id);
    }

    visit_children(pf, last, &mut |pf, _, child| {
        diff(ctx_driver, pf, 0, Some(child), None);
    });

    D::node_removed(ctx_driver, pf);
}

pub fn diff<D>(
    ctx_driver: &mut ContextDriver<D>,
    pf: &PathFrame,
    index: usize,
    last: Option<&mut Child>,
    curr: Option<&mut Child>,
) {
    match (last, curr) {
        (Some(last), Some(curr)) => {
            match (last, curr) {
                (&mut Child::Node(ref mut l), &mut Child::Node(ref mut c)) if l.ty == c.ty => {
                    diff_nodes(ctx_driver, pf, l, c);
                }
                (&mut Child::Text(ref l), &mut Child::Text(ref c)) => {
                    if l.as_ref() != c {
                        D::text_changed(ctx_driver, pf, c);
                    }
                }
                (
                    &mut Child::Widget(ref last_input, ref mut last_output),
                    &mut Child::Widget(ref mut curr_input, ref mut curr_output),
                ) if last_input.widget_type_id() == curr_input.widget_type_id() =>
                {
                    assert!(last_output.is_some());
                    assert!(curr_output.is_none());
                    ctx_driver.ctx.curr_widget_path = Some(pf.to_path());
                    match curr_input.try_render(ctx_driver, pf.to_path()) {
                        Some(mut rendered) => {
                            diff(
                                ctx_driver,
                                pf,
                                index,
                                last_output.as_mut().map(|o| &mut **o),
                                Some(&mut rendered),
                            );
                            *curr_output = Some(Box::new(rendered));
                        }
                        None => {
                            *curr_output = last_output.take();
                        }
                    }
                }
                (ref mut last, ref mut curr) => {
                    diff(ctx_driver, pf, index, Some(last), None);
                    diff(ctx_driver, pf, index, None, Some(curr));
                }
            }
        }

        // add
        (None, Some(curr)) => {
            match curr {
                &mut Child::Node(ref mut c) => node_added(ctx_driver, pf, index, c),
                &mut Child::Text(ref c) => D::text_added(ctx_driver, pf, index, c.as_ref()),
                &mut Child::Widget(ref mut input, ref mut output) => {
                    assert!(output.is_none());
                    ctx_driver.ctx.curr_widget_path = Some(pf.to_path());
                    let mut rendered = input.try_render(ctx_driver, pf.to_path()).unwrap();
                    diff(ctx_driver, pf, index, None, Some(&mut rendered));
                    *output = Some(Box::new(rendered));
                }
            }
        }

        // remove
        (Some(last), None) => {
            match last {
                &mut Child::Node(ref mut l) => node_removed(ctx_driver, pf, l),
                &mut Child::Text(_) => D::text_removed(ctx_driver, pf),
                &mut Child::Widget(ref input, ref mut output) => {
                    ctx_driver.ctx.curr_widget_path = Some(pf.to_path());
                    let output = output.as_mut().unwrap();
                    diff(ctx_driver, pf, index, Some(&mut *output), None);
                    input.remove(ctx_driver, &pf.to_path());
                }
            }
        }

        (None, None) => {}
    }
}

fn traverse_path<F, P>(child: &mut Child, path: P, f: F)
where
    F: FnOnce(usize, &PathFrame, &mut Child),
    P: AsRef<[Ident]>,
{
    traverse_path_(
        child,
        0,
        &PathFrame::new(),
        path.as_ref().split_last().unwrap().1,
        f,
    );
}

fn traverse_path_<F>(child: &mut Child, index: usize, pf: &PathFrame, path: &[Ident], f: F)
where
    F: FnOnce(usize, &PathFrame, &mut Child),
{
    if let &mut Child::Widget(_, ref mut child) = child {
        traverse_path_(
            child.as_mut().expect("child not rendered"),
            index,
            &pf,
            path,
            f,
        );
        return;
    }
    if let Some((ident, leftover)) = path.split_last() {
        let pf = pf.add_ident(ident.clone());
        match child {
            &mut Child::Node(ref mut node) => {
                let index = match ident {
                    &Ident::Key(ref key) => {
                        *node.keyed_children.get(key).expect("node key not found")
                    }
                    &Ident::NonKeyedIndex(non_keyed_index) => {
                        *node.non_keyed_children
                            .get(non_keyed_index)
                            .expect("node index not found")
                    }
                };
                let child = &mut node.children
                    .get_mut(index)
                    .expect("node not found by index")
                    .1;
                traverse_path_(child, index, &pf, leftover, f);
            }
            &mut Child::Text(..) | &mut Child::Widget(..) => unreachable!(),
        }
    } else {
        f(index, pf, child);
    }
}

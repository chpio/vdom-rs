use diff::{ContextDriver, Driver};
use path::Path;
use widget::Widget;
use widget::WidgetHolderTrait;

use std::any::{Any, TypeId};
use std::cell::RefCell;
use std::collections::HashMap;
use std::fmt;
use std::marker::PhantomData;
use std::ops::Fn;
use std::rc::Weak;

pub trait Event {
    fn event_type(&self) -> &'static str;
}

pub struct ListenerHolder<W, E, F>
where
    W: 'static + Widget,
    E: 'static + Event,
    F: 'static + Fn(&mut W, &E),
{
    listener: F,
    pd: PhantomData<(W, E)>,
}

impl<W, E, F> ListenerHolder<W, E, F>
where
    W: 'static + Widget,
    E: 'static + Event,
    F: 'static + Fn(&mut W, &E),
{
    pub fn new(f: F) -> ListenerHolder<W, E, F> {
        ListenerHolder {
            listener: f,
            pd: PhantomData,
        }
    }
}

impl<W, E, F> fmt::Debug for ListenerHolder<W, E, F>
where
    W: 'static + Widget,
    E: 'static + Event,
    F: 'static + Fn(&mut W, &E),
{
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("ListenerHolder")
            .field("type", &E::EVENT_TYPE)
            .field("listener", &"{Fn}")
            .finish()
    }
}

pub trait Listener: fmt::Debug {
    fn call(&self, widget: &mut WidgetHolderTrait, event: &Any);
    fn event_type_id(&self) -> TypeId;
    fn register_root<D: Driver>(&self, ctx_driver: &mut ContextDriver<D>);
}

impl<W, E, F> Listener for ListenerHolder<W, E, F>
where
    W: 'static + Widget,
    E: 'static + Event,
    F: 'static + Fn(&mut W, &E),
{
    fn call(&self, widget_holder: &mut WidgetHolderTrait, event: &Any) {
        let widget_holder = widget_holder.downcast_mut::<W>().unwrap();
        let event = event.downcast_ref().unwrap();
        (self.listener)(&mut widget_holder.curr_widget, event);
        widget_holder.update_is_dirty();
    }

    fn event_type_id(&self) -> TypeId {
        TypeId::of::<E>()
    }

    fn register_root<D: Driver>(&self, ctx_driver: &mut ContextDriver<D>) {
        D::register_root_event_listener(ctx_driver, |ctx_driver, path, event: E| {
            for len in (0..path.len()).rev() {
                let path = path.iter().skip(len).cloned().collect();
                if let Some(&(ref widget_path, ref listener)) = ctx_driver
                    .ctx
                    .listener_manager
                    .listeners
                    .get(&(path, TypeId::of::<E>()))
                {
                    let widget_holder = ctx_driver.ctx.widget_holders.get_mut(widget_path).unwrap();
                    listener.call(&mut **widget_holder, &event);
                }
            }
        });
    }
}

#[derive(Debug)]
pub struct ListenerManager {
    listeners: HashMap<(Path, TypeId), (Path, Box<Listener>)>,
}

impl ListenerManager {
    pub fn new() -> ListenerManager {
        ListenerManager {
            root_listeners: HashMap::new(),
            listeners: HashMap::new(),
        }
    }

    /// Registers a new listener or replaces an old one.
    pub fn register<D: Driver>(
        ctx_driver: &mut ContextDriver<D>,
        path: Path,
        widget_path: Path,
        listener: Box<Listener>,
    ) {
        use std::collections::hash_map::Entry::{Occupied, Vacant};
        let type_id = listener.event_type_id();
        match ctx_driver
            .ctx
            .listener_manager
            .listeners
            .entry((path, type_id))
        {
            Occupied(mut oe) => *oe.get_mut() = (widget_path, listener),
            Vacant(ve) => {
                listener.register_root(ctx_driver);
                ve.insert((widget_path, listener));
            }
        }
    }

    pub fn unregister<D: Driver>(ctx_driver: &mut ContextDriver<D>, path: Path, type_id: TypeId) {
        use std::collections::hash_map::Entry::{Occupied, Vacant};
        let listener_manager = &mut ctx_driver.ctx.listener_manager;
        if let Some(_) = listener_manager.listeners.remove(&(path, type_id)) {
            match listener_manager.root_listeners.entry(type_id) {
                Occupied(mut oe) => {
                    let remove = {
                        let c = &mut oe.get_mut().0;
                        *c -= 1;
                        *c == 0
                    };
                    if remove {
                        oe.remove().1.remove();
                    }
                }
                Vacant(_) => unreachable!(),
            }
        }
    }
}

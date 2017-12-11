use path::Path;
use widget::Widget;
use diff::Differ;

use stdweb::web;
use stdweb::web::EventListenerHandle;
use stdweb::web::event::ConcreteEvent;
use stdweb::web::IEventTarget;
use stdweb::unstable::TryInto;
use std::collections::HashMap;
use std::any::{Any, TypeId};
use std::ops::Fn;
use std::marker::PhantomData;
use std::fmt;
use std::cell::RefCell;
use std::rc::Rc;

pub struct ListenerHolder<W, E, F>
where
    W: 'static + Widget,
    E: 'static + ConcreteEvent,
    F: 'static + Fn(&mut W, &E),
{
    listener: F,
    pd: PhantomData<(W, E)>,
}

impl<W, E, F> ListenerHolder<W, E, F>
where
    W: 'static + Widget,
    E: 'static + ConcreteEvent,
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
    E: 'static + ConcreteEvent,
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
    fn call(&self, widget: &mut Any, event: &Any);
    fn event_type_id(&self) -> TypeId;
    fn register_root(
        &self,
        root: &web::Node,
        queue: Rc<RefCell<Vec<(i32, TypeId, Box<Any>)>>>,
    ) -> EventListenerHandle;
}

impl<W, E, F> Listener for ListenerHolder<W, E, F>
where
    W: 'static + Widget,
    E: 'static + ConcreteEvent,
    F: 'static + Fn(&mut W, &E),
{
    fn call(&self, widget: &mut Any, event: &Any) {
        let widget = widget.downcast_mut().unwrap();
        let event = *event.downcast_ref().unwrap();
        (self.listener)(widget, event);
    }

    fn event_type_id(&self) -> TypeId {
        TypeId::of::<E>()
    }

    fn register_root(
        &self,
        root: &web::Node,
        queue: Rc<RefCell<Vec<(i32, TypeId, Box<Any>)>>>,
    ) -> EventListenerHandle {
        root.add_event_listener(move |event: E| {
            let node_id: i32 = js!(
                return @{event.as_ref()}.target.__vdom_node_id;
            ).try_into()
                .unwrap();
            queue
                .borrow_mut()
                .push((node_id, TypeId::of::<E>(), Box::new(event)));
        })
    }
}

#[derive(Debug)]
pub struct ListenerManager {
    root_listeners: HashMap<TypeId, (usize, EventListenerHandle)>,
    listeners: HashMap<(Path, TypeId), (Path, Box<Listener>)>,
    queue: Rc<RefCell<Vec<(i32, TypeId, Box<Any>)>>>,
}

impl ListenerManager {
    pub fn new() -> ListenerManager {
        ListenerManager {
            root_listeners: HashMap::new(),
            listeners: HashMap::new(),
            queue: Rc::new(RefCell::new(Vec::new())),
        }
    }

    pub fn handle_events(differ: &mut Differ) {
        let listener_manager = &differ.listener_manager;
        let mut queue = listener_manager.queue.borrow_mut();
        for (node_id, event_type_id, event) in queue.drain(..) {
            let path = differ.node_id_to_path.get(&node_id).unwrap();
            for len in 0..path.len() {
                let path = path.iter().take(len).cloned().collect();
                if let Some(&(ref widget_path, ref listener)) =
                    listener_manager.listeners.get(&(path, event_type_id))
                {
                    let widget = differ.widget_holders.get_mut(widget_path).unwrap();
                    listener.call(widget, &*event);
                }
            }
        }
    }

    /// Registers a new listener or replaces an old one.
    pub fn register(differ: &mut Differ, path: Path, widget_path: Path, listener: Box<Listener>) {
        use std::collections::hash_map::Entry::{Occupied, Vacant};
        let listener_manager = &mut differ.listener_manager;
        let type_id = listener.event_type_id();
        match listener_manager.listeners.entry((path, type_id)) {
            Occupied(mut oe) => *oe.get_mut() = (widget_path, listener),
            Vacant(ve) => {
                match listener_manager.root_listeners.entry(type_id) {
                    Occupied(mut oe) => oe.get_mut().0 += 1,
                    Vacant(ve) => {
                        ve.insert((
                            1,
                            listener.register_root(&differ.root, listener_manager.queue.clone()),
                        ));
                    }
                }
                ve.insert((widget_path, listener));
            }
        }
    }

    pub fn unregister(differ: &mut Differ, path: Path, type_id: TypeId) {
        use std::collections::hash_map::Entry::{Occupied, Vacant};
        let listener_manager = &mut differ.listener_manager;
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

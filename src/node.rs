use path::{Ident, Key};
use widget::{Widget, WidgetData, WidgetDataTrait};
use event::{Listener, ListenerHolder};
use Str;

use stdweb::web::event::ConcreteEvent;
use std::collections::HashMap;
use std::any::TypeId;

#[derive(Debug)]
pub struct Node {
    pub ty: Str,
    pub children: Vec<(Ident, Child)>,
    pub keyed_children: HashMap<Key, usize>,
    pub non_keyed_children: Vec<usize>,
    pub attributes: HashMap<Str, Str>,
    pub event_listeners: HashMap<TypeId, Option<Box<Listener>>>,
}

#[derive(Debug)]
pub struct NodeBuilder {
    ty: Str,
    children: Vec<(Ident, Child)>,
    keyed_children: HashMap<Key, usize>,
    non_keyed_children: Vec<usize>,
    attributes: HashMap<Str, Str>,
    event_listeners: HashMap<TypeId, Option<Box<Listener>>>,
}

impl NodeBuilder {
    pub fn new<T>(ty: T) -> NodeBuilder
    where
        T: Into<Str>,
    {
        NodeBuilder {
            ty: ty.into(),
            children: Vec::new(),
            keyed_children: HashMap::new(),
            non_keyed_children: Vec::new(),
            attributes: HashMap::new(),
            event_listeners: HashMap::new(),
        }
    }

    pub fn build(self) -> Node {
        Node {
            ty: self.ty,
            children: self.children,
            keyed_children: self.keyed_children,
            non_keyed_children: self.non_keyed_children,
            attributes: self.attributes,
            event_listeners: self.event_listeners,
        }
    }

    pub fn add_child<C>(mut self, child: C) -> NodeBuilder
    where
        C: Into<Child>,
    {
        let child = child.into();
        let index = self.children.len();
        let non_keyed_index = self.non_keyed_children.len();
        self.non_keyed_children.push(index);
        self.children.push((Ident::Index(non_keyed_index), child));
        self
    }

    pub fn add_keyed_child<K, C>(mut self, key: K, child: C) -> NodeBuilder
    where
        K: Into<Key>,
        C: Into<Child>,
    {
        let key = key.into();
        let child = child.into();
        let index = self.children.len();
        self.keyed_children.insert(key.clone(), index);
        self.children.push((Ident::Key(key), child));
        self
    }

    pub fn add_children<I>(self, iter: I) -> NodeBuilder
    where
        I: IntoIterator<Item = (Option<Key>, Child)>,
    {
        let mut this = self;
        for (key, child) in iter {
            if let Some(key) = key {
                this = this.add_keyed_child(key, child);
            } else {
                this = this.add_child(child);
            }
        }
        this
    }

    pub fn add_attribute<N, V>(mut self, name: N, value: V) -> NodeBuilder
    where
        N: Into<Str>,
        V: Into<Str>,
    {
        self.attributes.insert(name.into(), value.into());
        self
    }

    pub fn add_attributes<I>(mut self, iter: I) -> NodeBuilder
    where
        I: IntoIterator<Item = (Str, Str)>,
    {
        for (name, value) in iter {
            self.attributes.insert(name, value);
        }
        self
    }

    pub fn add_event_listener<W, E, F>(mut self, f: F) -> NodeBuilder
    where
        W: 'static + Widget,
        E: 'static + ConcreteEvent,
        F: 'static + Fn(&mut W, &E),
    {
        self.event_listeners
            .insert(TypeId::of::<E>(), Some(Box::new(ListenerHolder::new(f))));
        self
    }
}

#[derive(Debug)]
pub enum Child {
    Text(Str),
    Node(Node),
    Widget(Box<WidgetDataTrait>, Option<Box<Child>>),
    Tombstone,
}

impl From<Str> for Child {
    fn from(v: Str) -> Child {
        Child::Text(v)
    }
}

impl From<String> for Child {
    fn from(v: String) -> Child {
        Child::Text(v.into())
    }
}

impl From<&'static str> for Child {
    fn from(v: &'static str) -> Child {
        Child::Text(v.into())
    }
}

impl From<Node> for Child {
    fn from(v: Node) -> Child {
        Child::Node(v)
    }
}

impl<W> From<WidgetData<W>> for Child
where
    W: Widget + 'static,
{
    fn from(v: WidgetData<W>) -> Child {
        Child::Widget(Box::new(v), None)
    }
}

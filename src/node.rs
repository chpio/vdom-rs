use Str;
use event::{Listener, ListenerHolder};
use path::{Ident, Key};
use widget::{Widget, WidgetData, WidgetDataTrait};

use std::any::TypeId;
use std::collections::HashMap;
use std::marker::PhantomData;
use stdweb::web::event::ConcreteEvent;

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
pub struct NodeBuilder<W>
where
    W: 'static + Widget,
{
    ty: Str,
    children: Vec<(Ident, Child)>,
    keyed_children: HashMap<Key, usize>,
    non_keyed_children: Vec<usize>,
    attributes: HashMap<Str, Str>,
    event_listeners: HashMap<TypeId, Option<Box<Listener>>>,
    pd: PhantomData<W>,
}

impl<W> NodeBuilder<W>
where
    W: 'static + Widget,
{
    pub fn new<T>(ty: T) -> NodeBuilder<W>
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
            pd: PhantomData,
        }
    }

    pub fn add_child<C>(mut self, child: C) -> NodeBuilder<W>
    where
        C: Into<ChildBuilder<W>>,
    {
        let child = child.into().into();
        let index = self.children.len();
        let non_keyed_index = self.non_keyed_children.len();
        self.non_keyed_children.push(index);
        self.children
            .push((Ident::NonKeyedIndex(non_keyed_index), child));
        self
    }

    pub fn add_keyed_child<K, C>(mut self, key: K, child: C) -> NodeBuilder<W>
    where
        K: Into<Key>,
        C: Into<ChildBuilder<W>>,
    {
        let key = key.into();
        let child = child.into().into();
        let index = self.children.len();
        self.keyed_children.insert(key.clone(), index);
        self.children.push((Ident::Key(key), child));
        self
    }

    pub fn add_children<I>(self, iter: I) -> NodeBuilder<W>
    where
        I: IntoIterator<Item = (Option<Key>, ChildBuilder<W>)>,
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

    pub fn add_attribute<N, V>(mut self, name: N, value: V) -> NodeBuilder<W>
    where
        N: Into<Str>,
        V: Into<Str>,
    {
        self.attributes.insert(name.into(), value.into());
        self
    }

    pub fn add_attributes<I>(mut self, iter: I) -> NodeBuilder<W>
    where
        I: IntoIterator<Item = (Str, Str)>,
    {
        for (name, value) in iter {
            self.attributes.insert(name, value);
        }
        self
    }

    pub fn add_event_listener<E, F>(mut self, f: F) -> NodeBuilder<W>
    where
        E: 'static + ConcreteEvent,
        F: 'static + Fn(&mut W, &E),
    {
        self.event_listeners
            .insert(TypeId::of::<E>(), Some(Box::new(ListenerHolder::new(f))));
        self
    }
}

impl<W> From<NodeBuilder<W>> for Node
where
    W: 'static + Widget,
{
    fn from(nb: NodeBuilder<W>) -> Node {
        Node {
            ty: nb.ty,
            children: nb.children,
            keyed_children: nb.keyed_children,
            non_keyed_children: nb.non_keyed_children,
            attributes: nb.attributes,
            event_listeners: nb.event_listeners,
        }
    }
}

#[derive(Debug)]
pub enum Child {
    Text(Str),
    Node(Node),
    Widget(Box<WidgetDataTrait>, Option<Box<Child>>),
}

impl<W> From<ChildBuilder<W>> for Child
where
    W: 'static + Widget,
{
    fn from(cb: ChildBuilder<W>) -> Child {
        match cb {
            ChildBuilder::Text(string) => Child::Text(string),
            ChildBuilder::Node(node, _) => Child::Node(node),
            ChildBuilder::Widget(widget_data, child) => Child::Widget(widget_data, child),
        }
    }
}

#[derive(Debug)]
pub enum ChildBuilder<W> {
    Text(Str),
    Node(Node, PhantomData<W>),
    Widget(Box<WidgetDataTrait>, Option<Box<Child>>),
}

impl<W> From<Str> for ChildBuilder<W>
where
    W: Widget + 'static,
{
    fn from(v: Str) -> ChildBuilder<W> {
        ChildBuilder::Text(v)
    }
}

impl<W> From<String> for ChildBuilder<W>
where
    W: Widget + 'static,
{
    fn from(v: String) -> ChildBuilder<W> {
        ChildBuilder::Text(v.into())
    }
}

impl<W> From<&'static str> for ChildBuilder<W>
where
    W: Widget + 'static,
{
    fn from(v: &'static str) -> ChildBuilder<W> {
        ChildBuilder::Text(v.into())
    }
}

impl<W> From<NodeBuilder<W>> for ChildBuilder<W>
where
    W: Widget + 'static,
{
    fn from(nb: NodeBuilder<W>) -> ChildBuilder<W> {
        ChildBuilder::Node(nb.into(), PhantomData)
    }
}

impl<W, WI> From<WidgetData<WI>> for ChildBuilder<W>
where
    W: Widget + 'static,
    WI: Widget + 'static,
{
    fn from(v: WidgetData<WI>) -> ChildBuilder<W> {
        ChildBuilder::Widget(Box::new(v), None)
    }
}

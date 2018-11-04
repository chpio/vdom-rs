use vdom::{
    comp::Comp,
    driver::Driver,
    vdom::{
        attr::{Attr, AttrRefValue, AttrVisitor},
        node::{Node, NodeDiffer, NodeVisitor, Tag, Text},
        path::Path,
    },
};
use web_sys as web;

pub struct WebDriver;

#[derive(Default)]
pub struct AttrStore;

#[derive(Default)]
pub struct TagStore {
    element: Option<web::Element>,
}

#[derive(Default)]
pub struct TextStore {
    text: Option<web::Text>,
}

impl Driver for WebDriver {
    type AttrStore = AttrStore;
    type TagStore = TagStore;
    type TextStore = TextStore;

    fn new_attr_store() -> AttrStore {
        Default::default()
    }

    fn new_tag_store() -> TagStore {
        Default::default()
    }

    fn new_text_store() -> TextStore {
        Default::default()
    }
}

pub struct App<C>
where
    C: Comp<WebDriver>,
{
    root_element: web::Element,
    comp: C,
    comp_input: C::Input,
    last_rendered: C::Rendered,
}

impl<C> App<C>
where
    C: Comp<WebDriver>,
{
    pub fn new(comp: C, comp_input: C::Input, root_element: web::Element) -> App<C> {
        let mut last_rendered = comp.render(&comp_input);
        last_rendered.visit(
            &Path::new(0),
            &mut NodeAddVisitor {
                parent_element: &root_element,
            },
        );
        App {
            root_element,
            comp,
            last_rendered,
            comp_input,
        }
    }

    pub fn set_input(&mut self, comp_input: C::Input) {
        self.comp_input = comp_input;
        let mut curr_rendered = self.comp.render(&self.comp_input);
        curr_rendered.diff(
            &Path::new(0),
            &mut self.last_rendered,
            &mut NodeStdDiffer {
                parent_element: &self.root_element,
            },
        );
        self.last_rendered = curr_rendered;
    }
}

struct NodeAddVisitor<'a> {
    parent_element: &'a web::Element,
}

impl<'a> NodeVisitor<WebDriver> for NodeAddVisitor<'a> {
    fn on_tag<T>(&mut self, path: &Path<'_>, tag: &mut T)
    where
        T: Tag<WebDriver>,
    {
        let elem = web::window()
            .unwrap()
            .document()
            .unwrap()
            .create_element(tag.tag())
            .unwrap();
        tag.visit_attr(&mut AttrAddVisitor {
            parent_element: &elem,
        });
        tag.visit_children(
            path,
            &mut NodeAddVisitor {
                parent_element: &elem,
            },
        );
        AsRef::<web::Node>::as_ref(&self.parent_element)
            .append_child(elem.as_ref())
            .unwrap();
        tag.driver_store().element = Some(elem);
    }

    fn on_text<T>(&mut self, path: &Path<'_>, text: &mut T)
    where
        T: Text<WebDriver>,
    {
        let text_node = web::window()
            .unwrap()
            .document()
            .unwrap()
            .create_text_node(text.get());
        AsRef::<web::Node>::as_ref(&self.parent_element)
            .append_child(text_node.as_ref())
            .unwrap();
        text.driver_store().text = Some(text_node);
    }
}

struct NodeRemoveVisitor;

impl NodeVisitor<WebDriver> for NodeRemoveVisitor {
    fn on_tag<T>(&mut self, path: &Path<'_>, tag: &mut T)
    where
        T: Tag<WebDriver>,
    {
        let elem = tag
            .driver_store()
            .element
            .as_ref()
            .expect("element is None");
        elem.remove();
    }

    fn on_text<T>(&mut self, path: &Path<'_>, text: &mut T)
    where
        T: Text<WebDriver>,
    {
        let text_node = text.driver_store().text.as_ref().expect("text is None");
        let node = AsRef::<web::Node>::as_ref(text_node);
        node.parent_node()
            .expect("parent is None")
            .remove_child(node);
    }
}

struct AttrAddVisitor<'a> {
    parent_element: &'a web::Element,
}

impl<'a> AttrVisitor<WebDriver> for AttrAddVisitor<'a> {
    fn on_attr<A>(&mut self, attr: &mut A)
    where
        A: Attr<WebDriver>,
    {
        let value = match attr.value() {
            AttrRefValue::True => Some(attr.name()),
            AttrRefValue::Null => None,
            AttrRefValue::Str(s) => Some(s),
        };

        if let Some(value) = value {
            self.parent_element
                .set_attribute(attr.name(), value)
                .unwrap();
        }
    }
}

struct NodeStdDiffer<'a> {
    parent_element: &'a web::Element,
}

impl<'a> NodeDiffer<WebDriver> for NodeStdDiffer<'a> {
    fn on_node_added<N>(&mut self, path: &Path<'_>, curr: &mut N)
    where
        N: Node<WebDriver>,
    {
        curr.visit(
            path,
            &mut NodeAddVisitor {
                parent_element: &self.parent_element,
            },
        );
    }

    fn on_node_removed<N>(&mut self, path: &Path<'_>, ancestor: &mut N)
    where
        N: Node<WebDriver>,
    {
        ancestor.visit(path, &mut NodeRemoveVisitor);
    }

    fn on_tag<T>(&mut self, path: &Path<'_>, curr: &mut T, ancestor: &mut T)
    where
        T: Tag<WebDriver>,
    {
        let elem = ancestor
            .driver_store()
            .element
            .take()
            .expect("element is None");
        curr.diff_children(
            path,
            ancestor,
            &mut NodeStdDiffer {
                parent_element: &elem,
            },
        );
        curr.driver_store().element = Some(elem);
    }

    fn on_text<T>(&mut self, path: &Path<'_>, curr: &mut T, ancestor: &mut T)
    where
        T: Text<WebDriver>,
    {
        let text = ancestor.driver_store().text.take().expect("text is None");
        if curr.get() != ancestor.get() {
            AsRef::<web::CharacterData>::as_ref(&text).set_data(curr.get());
        }
        curr.driver_store().text = Some(text);
    }
}

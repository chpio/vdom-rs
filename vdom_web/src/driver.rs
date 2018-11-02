use vdom::{
    comp::Comp,
    driver::Driver,
    vdom::{
        attr::{Attr, AttrRefValue, AttrVisitor},
        node::{Node, NodeVisitor, Tag, Text},
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
    web_node: web::Node,
    comp: C,
    comp_input: C::Input,
    last_rendered: C::Rendered,
}

impl<C> App<C>
where
    C: Comp<WebDriver>,
{
    pub fn new(comp: C, comp_input: C::Input, web_node: web::Node) -> App<C> {
        let mut last_rendered = comp.render(&comp_input);
        last_rendered.visit(
            &Path::new(0),
            &mut NodeAddVisitor {
                parent_web_node: &web_node,
            },
        );
        App {
            web_node,
            comp,
            last_rendered,
            comp_input,
        }
    }
}

struct NodeAddVisitor<'a> {
    parent_web_node: &'a web::Node,
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
            parent_web_elem: &elem,
        });
        tag.visit_children(
            path,
            &mut NodeAddVisitor {
                parent_web_node: elem.as_ref(),
            },
        );
        self.parent_web_node.append_child(elem.as_ref()).unwrap();
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
        self.parent_web_node
            .append_child(text_node.as_ref())
            .unwrap();
        text.driver_store().text = Some(text_node);
    }
}

struct AttrAddVisitor<'a> {
    parent_web_elem: &'a web::Element,
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
            self.parent_web_elem
                .set_attribute(attr.name(), value)
                .unwrap();
        }
    }
}

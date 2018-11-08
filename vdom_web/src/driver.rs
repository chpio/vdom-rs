use crate::Error;
use vdom::{
    comp::Comp,
    driver::Driver,
    vdom::{
        attr::{Attr, AttrRefValue, AttrVisitor},
        node::{Node, NodeDiffer, NodeVisitor, Tag, Text},
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
    pub fn new(comp: C, comp_input: C::Input, root_element: web::Element) -> Result<App<C>, Error> {
        let mut last_rendered = comp.render(&comp_input);
        last_rendered.visit(
            &mut 0,
            &mut NodeAddVisitor {
                parent_element: &root_element,
            },
        )?;
        Ok(App {
            root_element,
            comp,
            last_rendered,
            comp_input,
        })
    }

    pub fn set_input(&mut self, comp_input: C::Input) -> Result<(), Error> {
        self.comp_input = comp_input;
        let mut curr_rendered = self.comp.render(&self.comp_input);
        curr_rendered.diff(
            &mut 0,
            &mut 0,
            &mut self.last_rendered,
            &mut NodeStdDiffer {
                parent_element: &self.root_element,
            },
        )?;
        self.last_rendered = curr_rendered;
        Ok(())
    }
}

struct NodeAddVisitor<'a> {
    parent_element: &'a web::Element,
}

impl<'a> NodeVisitor<WebDriver> for NodeAddVisitor<'a> {
    type Err = Error;

    fn on_tag<T>(&mut self, index: usize, tag: &mut T) -> Result<(), Error>
    where
        T: Tag<WebDriver>,
    {
        let elem = web::window()
            .ok_or("window is None")?
            .document()
            .ok_or("document is None")?
            .create_element(tag.tag())?;
        tag.visit_attr(&mut AttrAddVisitor {
            parent_element: &elem,
        });
        tag.visit_children(&mut NodeAddVisitor {
            parent_element: &elem,
        });
        let parent_node = AsRef::<web::Node>::as_ref(&self.parent_element);
        parent_node.insert_before(
            elem.as_ref(),
            parent_node.child_nodes().get(index as u32).as_ref(),
        )?;
        tag.driver_store().element = Some(elem);
        Ok(())
    }

    fn on_text<T>(&mut self, index: usize, text: &mut T) -> Result<(), Error>
    where
        T: Text<WebDriver>,
    {
        let text_node = web::window()
            .ok_or("window is None")?
            .document()
            .ok_or("document is None")?
            .create_text_node(text.get());
        let parent_node = AsRef::<web::Node>::as_ref(&self.parent_element);
        parent_node.insert_before(
            text_node.as_ref(),
            parent_node.child_nodes().get(index as u32).as_ref(),
        )?;
        text.driver_store().text = Some(text_node);
        Ok(())
    }
}

struct NodeRemoveVisitor;

impl NodeVisitor<WebDriver> for NodeRemoveVisitor {
    type Err = Error;

    fn on_tag<T>(&mut self, _index: usize, tag: &mut T) -> Result<(), Error>
    where
        T: Tag<WebDriver>,
    {
        let elem = tag
            .driver_store()
            .element
            .as_ref()
            .ok_or("element is None")?;
        elem.remove();
        Ok(())
    }

    fn on_text<T>(&mut self, _index: usize, text: &mut T) -> Result<(), Error>
    where
        T: Text<WebDriver>,
    {
        let text_node = text.driver_store().text.as_ref().ok_or("text is None")?;
        let node = AsRef::<web::Node>::as_ref(text_node);
        node.parent_node()
            .ok_or("text has no parent")?
            .remove_child(node)?;
        Ok(())
    }
}

struct AttrAddVisitor<'a> {
    parent_element: &'a web::Element,
}

impl<'a> AttrVisitor<WebDriver> for AttrAddVisitor<'a> {
    type Err = Error;

    fn on_attr<A>(&mut self, attr: &mut A) -> Result<(), Error>
    where
        A: Attr<WebDriver>,
    {
        let value = match attr.value() {
            AttrRefValue::True => Some(attr.name()),
            AttrRefValue::Null => None,
            AttrRefValue::Str(s) => Some(s),
        };

        if let Some(value) = value {
            self.parent_element.set_attribute(attr.name(), value)?;
        }
        Ok(())
    }
}

struct NodeStdDiffer<'a> {
    parent_element: &'a web::Element,
}

impl<'a> NodeDiffer<WebDriver> for NodeStdDiffer<'a> {
    type Err = Error;

    fn on_node_added<N>(&mut self, index: &mut usize, curr: &mut N) -> Result<(), Error>
    where
        N: Node<WebDriver>,
    {
        curr.visit(
            index,
            &mut NodeAddVisitor {
                parent_element: &self.parent_element,
            },
        )
    }

    fn on_node_removed<N>(
        &mut self,
        ancestor_index: &mut usize,
        ancestor: &mut N,
    ) -> Result<(), Error>
    where
        N: Node<WebDriver>,
    {
        ancestor.visit(ancestor_index, &mut NodeRemoveVisitor)
    }

    fn on_tag<T>(
        &mut self,
        _curr_index: usize,
        _ancestor_index: usize,
        curr: &mut T,
        ancestor: &mut T,
    ) -> Result<(), Error>
    where
        T: Tag<WebDriver>,
    {
        let elem = ancestor
            .driver_store()
            .element
            .take()
            .ok_or("element is None")?;
        curr.diff_children(
            ancestor,
            &mut NodeStdDiffer {
                parent_element: &elem,
            },
        )?;
        curr.driver_store().element = Some(elem);
        Ok(())
    }

    fn on_text<T>(
        &mut self,
        _curr_index: usize,
        _ancestor_index: usize,
        curr: &mut T,
        ancestor: &mut T,
    ) -> Result<(), Error>
    where
        T: Text<WebDriver>,
    {
        let text = ancestor.driver_store().text.take().ok_or("text is None")?;
        if curr.get() != ancestor.get() {
            AsRef::<web::CharacterData>::as_ref(&text).set_data(curr.get());
        }
        curr.driver_store().text = Some(text);
        Ok(())
    }
}

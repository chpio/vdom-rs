use crate::Error;
use vdom::{
    driver::Driver,
    vdom::{
        attr::{Attr, AttrDiffer, AttrRefValue, AttrVisitor},
        node::{Comp, CompNode, Node, NodeDiffer, NodeVisitor, Tag, Text},
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

#[derive(Default)]
pub struct CompStore;

impl Driver for WebDriver {
    type AttrStore = AttrStore;
    type TagStore = TagStore;
    type TextStore = TextStore;
    type CompStore = CompStore;

    fn new_attr_store() -> AttrStore {
        Default::default()
    }

    fn new_tag_store() -> TagStore {
        Default::default()
    }

    fn new_text_store() -> TextStore {
        Default::default()
    }

    fn new_comp_store() -> CompStore {
        Default::default()
    }
}

pub struct App<N>
where
    N: Node<WebDriver>,
{
    root_element: web::Element,
    node: N,
}

impl<N> App<N>
where
    N: Node<WebDriver>,
{
    pub fn new(mut node: N, root_element: web::Element) -> Result<App<N>, Error> {
        node.visit(
            &mut 0,
            &mut NodeAddVisitor {
                parent_element: &root_element,
            },
        )?;
        Ok(App { root_element, node })
    }

    pub fn set(&mut self, mut node: N) -> Result<(), Error> {
        node.diff(
            &mut 0,
            &mut 0,
            &mut self.node,
            &mut NodeStdDiffer {
                parent_element: &self.root_element,
            },
        )?;
        self.node = node;
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
        tag.visit_attrs(&mut AttrAddVisitor {
            parent_element: &elem,
        })?;
        tag.visit_children(&mut NodeAddVisitor {
            parent_element: &elem,
        })?;
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

    fn on_comp<C>(
        &mut self,
        index: &mut usize,
        comp: &mut CompNode<WebDriver, C>,
    ) -> Result<(), Self::Err>
    where
        C: Comp<WebDriver>,
    {
        comp.init_comp_ctx();
        comp.visit_rendered(index, self)
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

    fn on_comp<C>(
        &mut self,
        index: &mut usize,
        comp: &mut CompNode<WebDriver, C>,
    ) -> Result<(), Self::Err>
    where
        C: Comp<WebDriver>,
    {
        comp.visit_rendered(index, self)
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
        if let Some(value) = attr_to_str(attr) {
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
        curr.diff_attrs(
            ancestor,
            &mut AttrStdDiffer {
                parent_element: &elem,
            },
        )?;
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

    fn on_comp<C>(
        &mut self,
        curr_index: &mut usize,
        ancestor_index: &mut usize,
        curr: &mut CompNode<WebDriver, C>,
        ancestor: &mut CompNode<WebDriver, C>,
    ) -> Result<(), Self::Err>
    where
        C: Comp<WebDriver>,
    {
        if curr.comp_ctx().is_none() {
            let ctx = ancestor.comp_ctx().expect("ancestor.comp_ctx is None");
            curr.set_comp_ctx(ctx.clone());
        }
        curr.diff_rendered(curr_index, ancestor_index, ancestor, self)
    }
}

struct AttrStdDiffer<'a> {
    parent_element: &'a web::Element,
}

impl<'a> AttrDiffer<WebDriver> for AttrStdDiffer<'a> {
    type Err = Error;

    fn on_diff<A>(&mut self, curr: &mut A, ancestor: &mut A) -> Result<(), Error>
    where
        A: Attr<WebDriver>,
    {
        match (attr_to_str(curr), attr_to_str(ancestor)) {
            (Some(curr_val), Some(ancestor_val)) => {
                if curr_val != ancestor_val {
                    self.parent_element.set_attribute(curr.name(), curr_val)?;
                }
            }
            (Some(curr_val), None) => {
                self.parent_element.set_attribute(curr.name(), curr_val)?;
            }
            (None, Some(_)) => {
                self.parent_element.remove_attribute(curr.name())?;
            }
            (None, None) => {}
        }
        Ok(())
    }
}

fn attr_to_str<A>(attr: &A) -> Option<&str>
where
    A: Attr<WebDriver>,
{
    match attr.value() {
        AttrRefValue::True => Some(attr.name()),
        AttrRefValue::Null => None,
        AttrRefValue::Str(s) => Some(s),
    }
}

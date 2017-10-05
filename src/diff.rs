use {Child, Node};
use path::{Path, PathFrame};

use std::collections::HashMap;
use std::any::Any;
use std::mem;
use stdweb::web::{self, document, INode};

#[derive(Debug)]
pub struct Differ {
    root: web::Node,
    nodes: HashMap<Path, web::Node>,
    pub widget_holders: HashMap<Path, Box<Any>>,
}

impl Differ {
    fn new(root: web::Node) -> Differ {
        Differ {
            root: root,
            nodes: HashMap::new(),
            widget_holders: HashMap::new(),
        }
    }

    fn add_node(&mut self, pf: &PathFrame, index: usize, node: web::Node) {
        {
            let parent = match pf.parent() {
                Some(p) => {
                    self.nodes
                        .get(&p.to_path())
                        .unwrap_or_else(|| panic!("Can't find parent `{}`", p))
                }
                None => &self.root,
            };
            let child_nodes = parent.child_nodes();
            if index < child_nodes.len() {
                parent.insert_before(&node, &child_nodes.iter().nth(index).unwrap());
            } else {
                parent.append_child(&node);
            }
        }

        assert!(
            self.nodes.insert(pf.to_path(), node).is_none(),
            "Node `{}` already inserted",
            pf
        );
    }

    fn node_added(&mut self, pf: &PathFrame, index: usize, ty: &str) {
        let node = document().create_element(ty).into();
        self.add_node(pf, index, node);
    }
    fn node_removed(&mut self, pf: &PathFrame) {
        let node = self.nodes
            .remove(&pf.to_path())
            .unwrap_or_else(|| panic!("Can't find node `{}`", pf));
        js!(
            @(no_return)
            @{&node}.parentNode.removeChild(@{&node});
        );
    }

    fn text_added(&mut self, pf: &PathFrame, index: usize, curr: &str) {
        let node = document().create_text_node(curr).into();
        self.add_node(pf, index, node);
    }
    fn text_changed(&mut self, pf: &PathFrame, curr: &str) {
        let node = self.nodes
            .get(&pf.to_path())
            .unwrap_or_else(|| panic!("Can't find node `{}`", pf));
        js!(
            @(no_return)
            @{node}.nodeValue = @{curr};
        );
    }
    fn text_removed(&mut self, pf: &PathFrame) {
        self.node_removed(pf);
    }

    fn attribute_added(&mut self, pf: &PathFrame, name: &str, value: &str) {
        self.attribute_changed(pf, name, value);
    }
    fn attribute_changed(&mut self, pf: &PathFrame, name: &str, value: &str) {
        let node = self.nodes
            .get(&pf.to_path())
            .unwrap_or_else(|| panic!("Can't find node `{}`", pf));
        js!(
            @(no_return)
            @{node}.setAttribute(@{name}, @{value});
        );
    }
    fn attribute_removed(&mut self, pf: &PathFrame, name: &str) {
        let node = self.nodes
            .get(&pf.to_path())
            .unwrap_or_else(|| panic!("Can't find node `{}`", pf));
        js!(
            @(no_return)
            @{node}.removeAttribute(@{name});
        );
    }

    fn node_reordered(&mut self, pf: &PathFrame, index: usize) {
        let node = self.nodes
            .get(&pf.to_path())
            .unwrap_or_else(|| panic!("Can't find node `{}`", pf));
        js!(
            @(no_return)
            var parent = @{node}.parent;
            parent.insertBefore(@{node}, parent.childNodes[@{index as u32}]);
        );
    }
}

pub struct Context {
    differ: Differ,
    last: Option<Child>,
}

impl Context {
    pub fn new(root: web::Node) -> Context {
        Context {
            differ: Differ::new(root),
            last: None,
        }
    }

    pub fn update(&mut self, mut curr: Child) {
        diff(
            &mut self.differ,
            &PathFrame::new(),
            0,
            self.last.as_mut(),
            Some(&mut curr),
        );
        self.last = Some(curr);
    }
}

fn diff_attributes(differ: &mut Differ, pf: &PathFrame, last: &Node, curr: &Node) {
    let curr = &curr.attributes;
    let last = &last.attributes;

    for (name, c_value) in curr.iter() {
        match last.get(name) {
            Some(l_value) => {
                if l_value != c_value {
                    differ.attribute_changed(pf, name, c_value);
                }
            }
            None => differ.attribute_added(pf, name, c_value),
        }
    }

    for name in last.keys().filter(|name| !curr.contains_key(name.as_ref())) {
        differ.attribute_removed(pf, name);
    }
}

fn diff_nodes(differ: &mut Differ, pf: &PathFrame, last: &mut Node, curr: &mut Node) {
    diff_attributes(differ, pf, &last, curr);

    let mut last_index = 0;
    let mut non_keyed_index = 0;
    {
        let mut curr_it = curr.children.iter_mut().enumerate();
        loop {
            let (index, &mut (ref key, ref mut c)) = match curr_it.next() {
                Some(v) => v,
                None => break,
            };
            match key {
                &Some(ref key) => {
                    let pf = pf.add_key(key.clone());
                    diff(
                        differ,
                        &pf,
                        index,
                        last.keyed_children
                            .get(key)
                            .map(|i| *i)
                            .and_then(|i| last.children.get_mut(i))
                            .map(|&mut (_, ref mut l)| l),
                        Some(c),
                    );
                    differ.node_reordered(&pf, index);
                }
                &None => {
                    match last.children.get_mut(last_index) {
                        Some(&mut (Some(_), ref mut l)) => {
                            let pf = pf.add_index(non_keyed_index);
                            diff(differ, &pf, index, Some(l), Some(c));
                            last_index += 1;
                            non_keyed_index += 1;
                        }
                        Some(&mut (None, _)) => {
                            last_index += 1;
                            continue;
                        }
                        None => {
                            for (_, &mut (ref key, ref mut c)) in curr_it {
                                if key.is_some() {
                                    continue;
                                }
                                let pf = pf.add_index(non_keyed_index);
                                diff(differ, &pf, 0, None, Some(c));
                                non_keyed_index += 1;
                            }
                            break;
                        }
                    }
                }
            }
        }
    }

    // remove non-keyed nodes
    for &mut (_, ref mut l) in last.children[last_index..]
        .iter_mut()
        .filter(|v| v.0.is_none())
    {
        let pf = pf.add_index(non_keyed_index);
        diff(differ, &pf, 0, Some(l), None);
        non_keyed_index += 1;
    }

    // remove keyed nodes
    for (key, l) in last.children
        .iter_mut()
        .filter_map(|&mut (ref k, ref mut l)| k.as_ref().map(|k| (k, l)))
        .filter(|v| !curr.keyed_children.contains_key(&v.0))
    {
        let pf = pf.add_key(key.clone());
        diff(differ, &pf, 0, Some(l), None);
    }
}

fn visit_children<F>(pf: &PathFrame, node: &mut Node, f: &mut F)
where
    F: FnMut(&PathFrame, usize, &mut Child),
{
    let mut non_keyed_index = 0;
    for (index, &mut (ref key, ref mut child)) in node.children.iter_mut().enumerate() {
        match key {
            &Some(ref key) => {
                let pf = pf.add_key(key.clone());
                f(&pf, index, child);
            }
            &None => {
                let pf = pf.add_index(non_keyed_index);
                non_keyed_index += 1;
                f(&pf, index, child);
            }
        }
    }
}

fn node_added(differ: &mut Differ, pf: &PathFrame, index: usize, curr: &mut Node) {
    differ.node_added(pf, index, curr.ty.as_ref());

    for (name, value) in curr.attributes.iter() {
        differ.attribute_added(pf, name, value);
    }

    visit_children(pf, curr, &mut |pf, index, child| {
        diff(differ, pf, index, None, Some(child));
    });
}

fn node_removed(differ: &mut Differ, pf: &PathFrame, last: &mut Node) {
    visit_children(pf, last, &mut |pf, _, child| {
        diff(differ, pf, 0, Some(child), None);
    });

    differ.node_removed(pf);
}

fn diff(
    differ: &mut Differ,
    pf: &PathFrame,
    index: usize,
    last: Option<&mut Child>,
    curr: Option<&mut Child>,
) {
    match (last, curr) {
        (Some(last), Some(curr)) => {
            match (last, curr) {
                (&mut Child::Node(ref mut l), &mut Child::Node(ref mut c)) => {
                    if l.ty != c.ty {
                        node_removed(differ, pf, l);
                        node_added(differ, pf, index, c);
                    } else {
                        diff_nodes(differ, pf, l, c);
                    }
                }
                (&mut Child::Text(ref l), &mut Child::Text(ref c)) => {
                    if l.as_ref() != c {
                        differ.text_changed(pf, c);
                    }
                }
                (
                    &mut Child::Widget(ref last_input, ref mut last_output),
                    &mut Child::Widget(ref curr_input, ref mut curr_output),
                ) => {
                    assert!(last_output.is_some());
                    assert!(curr_output.is_none());
                    match curr_input.render(differ, pf, Some(&**last_input)) {
                        Some(mut rendered) => {
                            diff(
                                differ,
                                pf,
                                index,
                                last_output.as_mut().map(|o| &mut **o),
                                Some(&mut rendered),
                            );
                        }
                        None => {
                            *curr_output = mem::replace(last_output, None);
                        }
                    }
                }
                (ref mut last, ref mut curr) => {
                    diff(differ, pf, index, Some(last), None);
                    diff(differ, pf, index, None, Some(curr));
                }
            }
        }

        // add
        (None, Some(curr)) => {
            match curr {
                &mut Child::Node(ref mut c) => node_added(differ, pf, index, c),
                &mut Child::Text(ref c) => differ.text_added(pf, index, c.as_ref()),
                &mut Child::Widget(ref input, ref mut output) => {
                    assert!(output.is_none());
                    let mut rendered = input.render(differ, pf, None).unwrap();
                    diff(differ, pf, index, None, Some(&mut rendered));
                    *output = Some(Box::new(rendered));
                }
                &mut Child::Tombstone => panic!("curr is a tombstone `{}`", pf),
            }
        }

        // remove
        (Some(last), None) => {
            match last {
                &mut Child::Node(ref mut l) => node_removed(differ, pf, l),
                &mut Child::Text(_) => differ.text_removed(pf),
                &mut Child::Widget(ref input, ref mut output) => {
                    let output = output.as_mut().unwrap();
                    diff(differ, pf, index, Some(&mut *output), None);
                    input.remove(differ, pf);
                }
                &mut Child::Tombstone => panic!("last is a tombstone `{}`", pf),
            }
        }

        (None, None) => {}
    }
}

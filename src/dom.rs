use diff::diff;
use diff::{Context, Driver};
use path::Path;
use path::PathFrame;
use std::collections::HashMap;
use stdweb::web::{self, document, window, RequestAnimationFrameHandle};

pub struct Dom {
    raf: Option<RequestAnimationFrameHandle>,
    pub root: web::Node,
    nodes: HashMap<Path, web::Node>,
    pub node_id_to_path: HashMap<i32, Path>,
    next_node_id: i32,
    root_listeners: HashMap<TypeId, (usize, EventListenerHandle)>,
}

impl Dom {
    fn add_node(
        ctx_driver: &mut ContextDriver<Dom>,
        pf: &PathFrame,
        index: usize,
        node: web::Node,
        node_id: Option<i32>,
    ) {
        let driver = &mut ctx_driver.driver;
        {
            let parent = match pf.parent() {
                Some(ref p) => {
                    driver
                        .nodes
                        .get(&p.to_path())
                        .unwrap_or_else(|| panic!("Can't find parent `{}`", p))
                }
                None => &driver.root,
            };
            js!(
                @(no_return)
                var parent = @{&parent};
                var node = @{&node};
                var nodeId = @{node_id};
                if (nodeId) {
                    node.__vdomNodeId = nodeId;
                }
                parent.insertBefore(node, parent.childNodes[@{index as i32}] || null);
            );
        }
        assert!(
            driver.nodes.insert(pf.to_path(), node).is_none(),
            "Node `{}` already inserted",
            pf
        );
    }
}

impl Driver for Dom {
    fn schedule_repaint(ctx_driver: &mut ContextDriver<Dom>) {
        if self.raf.is_some() {
            return;
        }
        let ctx_driver_copy = ctx_driver.weak.as_ref().unwrap().upgrade();
        ctx_driver.driver.raf = Some(window().request_animation_frame(move |_| {
            let mut ctx_driver = ctx_driver_copy.borrow_mut();
            ctx_driver.driver.raf = None;

            if let Some(mut curr) = ctx_driver.ctx.curr.take() {
                let mut last = ctx_driver.ctx.last.take();
                diff(
                    ctx_driver,
                    &PathFrame::new(),
                    0,
                    last.as_mut(),
                    Some(&mut curr),
                );
                ctx_driver.ctx.last = Some(curr);
            }

            if let Some(mut last) = ctx_driver.ctx.last.take() {
                while let Some((path, mut rendered)) = ctx_driver
                    .ctx
                    .widget_holders
                    .iter_mut()
                    .filter(|&(_, ref widget_holder)| widget_holder.is_dirty())
                    .min_by_key(|&(ref path, _)| path.len())
                    .map(|(path, widget_holder)| (path.clone(), widget_holder.render()))
                {
                    traverse_path(&mut last, path, |index, pf, child| {
                        diff(ctx_driver, pf, index, Some(child), Some(&mut rendered));
                        *child = rendered;
                    });
                }
                ctx_driver.ctx.last = Some(last);
            }
        }));
    }

    fn node_added(ctx_driver: &mut ContextDriver<Dom>, pf: &PathFrame, index: usize, ty: &str) {
        let driver = &mut ctx_driver.driver;
        let node = document().create_element(ty).into();
        let node_id = driver.next_node_id;
        driver.node_id_to_path.insert(node_id, pf.to_path());
        driver.next_node_id += 1;
        driver.add_node(pf, index, node, Some(node_id));
    }
    fn node_removed(ctx_driver: &mut ContextDriver<Dom>, pf: &PathFrame) {
        let node = ctx_driver
            .driver
            .nodes
            .remove(&pf.to_path())
            .unwrap_or_else(|| panic!("Can't find node `{}`", pf));
        js!(
            @(no_return)
            @{&node}.parentNode.removeChild(@{&node});
        );
    }

    fn text_added(ctx_driver: &mut ContextDriver<Dom>, pf: &PathFrame, index: usize, curr: &str) {
        let node = document().create_text_node(curr).into();
        ctx_driver.driver.add_node(pf, index, node, None);
    }
    fn text_changed(ctx_driver: &mut ContextDriver<Dom>, pf: &PathFrame, curr: &str) {
        let node = ctx_driver
            .driver
            .nodes
            .get(&pf.to_path())
            .unwrap_or_else(|| panic!("Can't find node `{}`", pf));
        js!(
            @(no_return)
            @{node}.nodeValue = @{curr};
        );
    }
    fn text_removed(ctx_driver: &mut ContextDriver<Dom>, pf: &PathFrame) {
        ctx_driver.driver.node_removed(pf);
    }

    fn attribute_added(
        ctx_driver: &mut ContextDriver<Dom>,
        pf: &PathFrame,
        name: &str,
        value: &str,
    ) {
        ctx_driver.driver.attribute_changed(pf, name, value);
    }
    fn attribute_changed(
        ctx_driver: &mut ContextDriver<Dom>,
        pf: &PathFrame,
        name: &str,
        value: &str,
    ) {
        let node = ctx_driver
            .driver
            .nodes
            .get(&pf.to_path())
            .unwrap_or_else(|| panic!("Can't find node `{}`", pf));
        js!(
            @(no_return)
            @{node}.setAttribute(@{name}, @{value});
        );
    }
    fn attribute_removed(ctx_driver: &mut ContextDriver<Dom>, pf: &PathFrame, name: &str) {
        let node = ctx_driver
            .driver
            .nodes
            .get(&pf.to_path())
            .unwrap_or_else(|| panic!("Can't find node `{}`", pf));
        js!(
            @(no_return)
            @{node}.removeAttribute(@{name});
        );
    }

    fn node_reordered(ctx_driver: &mut ContextDriver<Dom>, pf: &PathFrame, index: usize) {
        let node = ctx_driver
            .driver
            .nodes
            .get(&pf.to_path())
            .unwrap_or_else(|| panic!("Can't find node `{}`", pf));
        js!(
            @(no_return)
            var parent = @{node}.parent;
            parent.insertBefore(@{node}, parent.childNodes[@{index as u32}]);
        );
    }

    fn register_root_event_listener<E, F>(ctx_driver: &mut ContextDriver<Dom>, f: F)
    where
        F: Fn(&mut ContextDriver<Dom>, &Path, E),
        E: Event,
    {
        match ctx_driver.driver.root_listeners.entry(type_id) {
            Occupied(mut oe) => oe.get_mut().0 += 1,
            Vacant(ve) => {
                let ctx_driver_copy = ctx_driver.weak.upgrade().unwrap();
                let handle = root.add_event_listener(move |event: E| {
                    let ctx_driver = ctx_driver_copy.borrow_mut().unwrap();
                    let node_id: i32 = js!(
                        return @{event.as_ref()}.target.__vdomNodeId;
                    ).try_into()
                        .unwrap();
                    {
                        let path = ctx_driver.driver.node_id_to_path.get(&node_id).unwrap();
                        f(ctx_driver, path, event);
                    }
                    Self::schedule_repaint(ctx_driver);
                });

                ve.insert((1, handle));
            }
        }
    }

    fn unregister_root_event_listener<E>(ctx_driver: &mut ContextDriver<Dom>)
    where
        E: Event,
    {

    }
}

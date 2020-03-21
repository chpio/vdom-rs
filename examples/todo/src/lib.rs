#![feature(existential_type)]
#![feature(proc_macro_hygiene)]

use vdom::{
    driver::Driver,
    vdom::node::{Comp, CompCtx, CompNode, IntoNode, Node},
};
use vdom_macro::html;
use vdom_web::{driver::App, Error};
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use web_sys as web;

fn render<D>(i: usize) -> impl Node<D>
where
    D: Driver,
{
    let c = CompNode::<_, TestComp>::new(i);

    html! {
        div class="wrapper" {
            (c)
        }
    }
}

#[wasm_bindgen]
pub fn main() {
    print_err(|| {
        let win = web::window().ok_or("window is None")?;
        let doc = win.document().ok_or("document is None")?;
        let mut app = App::new(render(0), doc.get_element_by_id("app").unwrap())?;
        let mut c = 0;
        let a = Closure::wrap(Box::new(move || {
            print_err(|| {
                app.set(render(c))?;
                c += 1;
                Ok(())
            })
        }) as Box<FnMut()>);
        win.set_interval_with_callback_and_timeout_and_arguments_0(a.as_ref().unchecked_ref(), 0)?;
        a.forget();
        Ok(())
    });
}

fn print_err<F>(f: F)
where
    F: FnOnce() -> Result<(), Error>,
{
    if let Err(err) = f() {
        let err = format!("Error: {:?}", err);
        web::console::log_1(&err.into());
    }
}

#[derive(Clone, Eq, PartialEq)]
struct TestComp;

impl<D> Comp<D> for TestComp
where
    D: Driver,
{
    type Input = usize;
    existential type Rendered: Node<D>;

    fn new(_: &usize, _: CompCtx<D, Self>) -> Self {
        TestComp
    }

    fn render(&self, input: &Self::Input) -> Self::Rendered {
        let kek = Some(()).filter(|_| input % 2 == 0).map(|_| {
            html! {(CompNode::<_, CompB>::new(*input))}
        });

        html! {
            div test=(input.to_string()) {
                (kek)
            }
        }
    }
}

#[derive(Clone, Eq, PartialEq)]
struct CompB;

// impl Drop for CompB {
//     fn drop(&mut self) {
//         web::console::log_1(&"drop CompB".to_string().into());
//     }
// }

impl<D> Comp<D> for CompB
where
    D: Driver,
{
    type Input = usize;
    existential type Rendered: Node<D>;

    fn new(_: &usize, _: CompCtx<D, Self>) -> Self {
        // web::console::log_1(&"CompB::new".to_string().into());

        CompB
    }

    fn render(&self, input: &Self::Input) -> Self::Rendered {
        let err = format!("ptr: {:p}", self);
        web::console::log_1(&err.into());

        html! {
            (input.to_string().into_node())
        }
    }
}

#[macro_use]
extern crate stdweb;

mod callback;
pub use callback::WidgetCallback;

mod diff;
pub use diff::Context;

mod event;

mod node;
pub use node::{Child, ChildBuilder, Node, NodeBuilder};

mod path;

pub mod widget;
pub use widget::Widget;

use std::borrow::Cow;

pub type Str = Cow<'static, str>;

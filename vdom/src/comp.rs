use crate::driver::Driver;
use crate::vdom::node::Node;

pub trait Comp<D>
where
    D: Driver,
{
    type Input;
    type Rendered: Node<D>;

    fn new(input: &Self::Input) -> Self;

    fn render(&self, input: &Self::Input) -> Self::Rendered;
}

#![deny(bare_trait_objects, anonymous_parameters, elided_lifetimes_in_paths)]

extern crate proc_macro;

mod code_gen;
mod parser;

use crate::parser::Nodes;
use crate::proc_macro::TokenStream;
use syn::parse_macro_input;

#[proc_macro]
pub fn html(input: TokenStream) -> TokenStream {
    let nodes = parse_macro_input!(input as Nodes);
    code_gen::gen_nodes(nodes.nodes).into()
}

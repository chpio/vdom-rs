extern crate proc_macro;

mod parser;

use crate::parser::Node;
use crate::proc_macro::TokenStream;
use quote::quote;
use syn::parse_macro_input;

#[proc_macro]
pub fn html(input: TokenStream) -> TokenStream {
    let node = parse_macro_input!(input as Node);

    (quote!{

    })
    .into()
}

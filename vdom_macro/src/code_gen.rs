use crate::parser::{Attr, AttrValue, Node, Tag};
use quote::{__rt::TokenStream, quote, ToTokens};
use syn::LitStr;

pub fn gen_nodes(nodes: Vec<Node>) -> TokenStream {
    nodes
        .into_iter()
        .map(gen_node)
        .fold(None, |prev_nodes, node| {
            match prev_nodes {
                Some(prev_nodes) => Some(quote!{(#prev_nodes, #node)}),
                None => Some(node),
            }
        })
        .unwrap_or_else(|| quote!{()})
}

fn gen_node(node: Node) -> TokenStream {
    match node {
        Node::Tag(tag) => gen_tag(tag),
        Node::Text(lit_str) => quote!{vdom::vdom::node::TextStatic::new(#lit_str)},
        Node::Expr(expr) => expr.into_token_stream(),
    }
}

fn gen_tag(tag: Tag) -> TokenStream {
    let tag_tag = LitStr::new(&tag.tag.to_string(), tag.tag.span());

    let attrs = tag
        .attrs
        .into_iter()
        .map(gen_attr)
        .map(|attr| quote!{vdom::vdom::attr::AttrListEntry(#attr)})
        .fold(None, |prev_attrs, attr| {
            match prev_attrs {
                Some(prev_attrs) => Some(quote!{(#prev_attrs, #attr)}),
                None => Some(attr),
            }
        })
        .unwrap_or_else(|| quote!{()});

    let children = gen_nodes(tag.children);

    quote!{
        vdom::vdom::node::TagStatic::new(
            #tag_tag,
            #attrs,
            #children,
        )
    }
}

fn gen_attr(attr: Attr) -> TokenStream {
    let name = LitStr::new(&attr.name.to_string(), attr.name.span());

    match attr.value {
        AttrValue::Str(lit_str) => {
            quote!{
                vdom::vdom::attr::AttrStr::new(
                    #name,
                    #lit_str
                )
            }
        }
        AttrValue::Expr(expr) => {
            quote!{
                vdom::vdom::attr::AttrDyn::new(#name, #expr)
            }
        }
        AttrValue::True => {
            quote!{
                vdom::vdom::attr::AttrTrue::new(#name)
            }
        }
    }
}

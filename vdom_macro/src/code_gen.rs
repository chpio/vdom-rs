use crate::parser::{Attr, AttrValue, Node, Tag, Text};
use quote::{__rt::TokenStream, quote};
use syn::LitStr;

pub fn gen_node(node: Node) -> TokenStream {
    match node {
        Node::Tag(tag) => gen_tag(tag),
        Node::Text(text) => gen_text(text),
    }
}

fn gen_text(text: Text) -> TokenStream {
    match text {
        Text::Str(lit_str) => {
            quote!{
                vdom::vdom::node::TextStatic::new(#lit_str)
            }
        }
        Text::Expr(expr) => {
            quote!{
                vdom::vdom::node::TextDyn::new(#expr)
            }
        }
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

    let children = tag
        .children
        .into_iter()
        .map(gen_node)
        .map(|node| quote!{vdom::vdom::node::NodeListEntry(#node)})
        .fold(None, |prev_nodes, node| {
            match prev_nodes {
                Some(prev_nodes) => Some(quote!{(#prev_nodes, #node)}),
                None => Some(node),
            }
        })
        .unwrap_or_else(|| quote!{()});

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

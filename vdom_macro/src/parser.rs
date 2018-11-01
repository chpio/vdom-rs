use syn::{
    braced, bracketed,
    ext::IdentExt,
    parse::{Parse, ParseBuffer, ParseStream, Result},
    punctuated::Punctuated,
    token, Expr, Ident, LitStr, Token,
};

#[derive(Debug)]
pub enum Node {
    Tag(Tag),
    Text(Text),
}

impl Parse for Node {
    fn parse(input: ParseStream) -> Result<Self> {
        let res = if input.fork().parse::<Text>().is_ok() {
            Node::Text(input.parse()?)
        } else {
            Node::Tag(input.parse()?)
        };
        Ok(res)
    }
}

#[derive(Debug)]
pub struct Tag {
    pub tag: Ident,
    pub attrs: Vec<Attr>,
    pub children: Vec<Node>,
}

impl Parse for Tag {
    fn parse(input: ParseStream) -> Result<Self> {
        let tag = Ident::parse_any(input)?;

        let mut attrs = Vec::new();
        while input.fork().parse::<Attr>().is_ok() {
            attrs.push(input.parse()?);
        }

        let mut children = Vec::new();
        if input.peek(token::Brace) {
            let content;
            braced!(content in input);
            while !content.is_empty() {
                children.push(content.parse()?);
            }
        } else if input.peek(Token![;]) {
            input.parse::<Token![;]>()?;
        } else {
            children.push(input.parse()?);
        }

        Ok(Tag {
            tag,
            attrs,
            children,
        })
    }
}

#[derive(Debug)]
pub struct Attr {
    pub name: Ident,
    pub value: AttrValue,
    pub condition: Option<Expr>,
}

impl Parse for Attr {
    fn parse(input: ParseStream) -> Result<Self> {
        let name = Ident::parse_any(input)?;

        let value = if input.peek(Token![=]) {
            input.parse::<Token![=]>()?;
            AttrValue::Str(input.parse()?)
        } else if input.peek(Token![?]) {
            input.parse::<Token![?]>()?;
            AttrValue::True
        } else {
            Err(input.error("expected `?` or `=`"))?
        };

        let condition = if input.peek(token::Bracket) {
            let condition;
            bracketed!(condition in input);
            Some(condition.parse()?)
        } else {
            None
        };

        Ok(Attr {
            name,
            value,
            condition,
        })
    }
}

#[derive(Debug)]
pub enum AttrValue {
    Str(LitStr),
    True,
}

#[derive(Debug)]
pub enum Text {
    Str(LitStr),
}

impl Parse for Text {
    fn parse(input: ParseStream) -> Result<Self> {
        Ok(Text::Str(input.parse()?))
    }
}

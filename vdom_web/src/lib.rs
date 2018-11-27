#![deny(bare_trait_objects, anonymous_parameters, elided_lifetimes_in_paths)]

use wasm_bindgen::JsValue;

pub mod driver;

#[derive(Debug)]
pub enum Error {
    JsValue(JsValue),
    Str(&'static str),
}

impl From<JsValue> for Error {
    fn from(js_value: JsValue) -> Error {
        Error::JsValue(js_value)
    }
}

impl From<&'static str> for Error {
    fn from(s: &'static str) -> Error {
        Error::Str(s)
    }
}

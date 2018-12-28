pub trait Driver {
    type AttrStore;
    type TagStore;
    type TextStore;
    type CompStore;

    fn new_attr_store() -> Self::AttrStore;
    fn new_tag_store() -> Self::TagStore;
    fn new_text_store() -> Self::TextStore;
    fn new_comp_store() -> Self::CompStore;
}

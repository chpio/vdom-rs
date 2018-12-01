use super::*;

pub trait Text<D>
where
    D: Driver,
{
    fn is_static(&self) -> bool;
    fn get(&self) -> &str;
    fn driver_store(&mut self) -> &mut D::TextStore;
}

pub struct TextStatic<D>
where
    D: Driver,
{
    text: &'static str,
    driver_store: D::TextStore,
}

impl<D> TextStatic<D>
where
    D: Driver,
{
    pub fn new(text: &'static str) -> TextStatic<D> {
        TextStatic {
            text,
            driver_store: D::new_text_store(),
        }
    }
}

impl<D> Text<D> for TextStatic<D>
where
    D: Driver,
{
    fn is_static(&self) -> bool {
        true
    }

    fn get(&self) -> &str {
        &self.text
    }

    fn driver_store(&mut self) -> &mut D::TextStore {
        &mut self.driver_store
    }
}

impl<D> Node<D> for TextStatic<D>
where
    D: Driver,
{
    fn visit<NV>(&mut self, index: &mut usize, visitor: &mut NV) -> Result<(), NV::Err>
    where
        NV: NodeVisitor<D>,
    {
        visitor.on_text(*index, self)?;
        *index += 1;
        Ok(())
    }

    fn diff<ND>(
        &mut self,
        curr_index: &mut usize,
        ancestor_index: &mut usize,
        ancestor: &mut Self,
        differ: &mut ND,
    ) -> Result<(), ND::Err>
    where
        ND: NodeDiffer<D>,
    {
        debug_assert_eq!(self.text, ancestor.text);
        differ.on_text(*curr_index, *ancestor_index, self, ancestor)?;
        *curr_index += 1;
        *ancestor_index += 1;
        Ok(())
    }
}

pub struct TextDyn<D>
where
    D: Driver,
{
    text: Cow<'static, str>,
    driver_store: D::TextStore,
}

impl<D> TextDyn<D>
where
    D: Driver,
{
    pub fn new<T>(text: T) -> TextDyn<D>
    where
        T: Into<Cow<'static, str>>,
    {
        TextDyn {
            text: text.into(),
            driver_store: D::new_text_store(),
        }
    }
}

impl<D> Text<D> for TextDyn<D>
where
    D: Driver,
{
    fn is_static(&self) -> bool {
        false
    }

    fn get(&self) -> &str {
        self.text.as_ref()
    }

    fn driver_store(&mut self) -> &mut D::TextStore {
        &mut self.driver_store
    }
}

impl<D> Node<D> for TextDyn<D>
where
    D: Driver,
{
    fn visit<NV>(&mut self, index: &mut usize, visitor: &mut NV) -> Result<(), NV::Err>
    where
        NV: NodeVisitor<D>,
    {
        visitor.on_text(*index, self)?;
        *index += 1;
        Ok(())
    }

    fn diff<ND>(
        &mut self,
        curr_index: &mut usize,
        ancestor_index: &mut usize,
        ancestor: &mut Self,
        differ: &mut ND,
    ) -> Result<(), ND::Err>
    where
        ND: NodeDiffer<D>,
    {
        differ.on_text(*curr_index, *ancestor_index, self, ancestor)?;
        *curr_index += 1;
        *ancestor_index += 1;
        Ok(())
    }
}

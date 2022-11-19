use std::fmt::Display;

use super::input::Input;

// Collection type, reused for reports
#[derive(Debug)]
pub struct Collection<T> {
    pub collection_type: super::basic::Collection,
    pub usage: (u16, u16),
    // "String and Physical indices, as well as delimiters may be associated with collections."
    // TODO delimiter support (when needed)
    pub designator_index: Option<u32>,
    pub string_index: Option<u32>,
    pub items: Vec<CollectionItem<T>>,
}

impl<T> Collection<T> {
    pub fn map<O, F: Fn(&T) -> Option<O>>(&self, f: F) -> Collection<O>
    where
        F: Copy,
    {
        Collection {
            collection_type: self.collection_type,
            usage: self.usage,
            designator_index: self.designator_index,
            string_index: self.string_index,
            items: self
                .items
                .iter()
                .filter_map(|item| match item {
                    CollectionItem::Collection(c) => {
                        let col = c.map(f);

                        Some(CollectionItem::Collection(col))
                    }
                    CollectionItem::Item(item) => f(item).map(CollectionItem::Item),
                })
                .collect(),
        }
    }
}

#[derive(Debug)]
pub enum CollectionItem<T> {
    Collection(Collection<T>),
    Item(T),
}

impl Display for Collection<Vec<Input>> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let items_string = self
            .items
            .iter()
            .map(|item| match item {
                CollectionItem::Collection(c) => format!("{}", c),
                CollectionItem::Item(inputs) => format!(
                    "[{}]",
                    inputs
                        .iter()
                        .map(|input| format!("{}", input))
                        .collect::<Vec<_>>()
                        .join(", ")
                ),
            })
            .collect::<Vec<_>>()
            .join(", ");

        write!(
            f,
            "{:?}({:02x} {:02x})[{}]",
            self.collection_type, self.usage.0, self.usage.1, items_string
        )
    }
}

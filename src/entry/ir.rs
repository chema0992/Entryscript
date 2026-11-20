use super::input::EntryInput;

#[derive(Debug, Clone, Default, PartialEq)]
pub struct EntryScript {
    pub threads: Vec<EntryThread>,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct EntryThread {
    pub blocks: Vec<EntryBlock>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct EntryBlock {
    pub type_id: String,
    /// Entry's params in exact positional order.
    pub params: Vec<EntryInput>,
    /// Entry's statement areas in exact positional order.
    pub statements: Vec<Vec<EntryBlock>>,
    pub x: i64,
    pub y: i64,
}

impl EntryBlock {
    pub fn new(type_id: impl Into<String>) -> Self {
        Self {
            type_id: type_id.into(),
            params: Vec::new(),
            statements: Vec::new(),
            x: 0,
            y: 0,
        }
    }
}

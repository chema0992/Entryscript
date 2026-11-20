use serde_json::Value;

use super::ir::EntryBlock;

#[derive(Debug, Clone, PartialEq)]
pub enum EntryInput {
    /// A missing or intentionally empty Entry parameter.
    Empty,
    /// A value encoded directly into the params array (dropdowns, IDs, etc.).
    Raw(Value),
    /// A dropdown/text field value stored as a bare string in params.
    Field(String),
    /// A string value encoded as Entry's nested `text` block.
    Text(String),
    /// A numeric value encoded as Entry's nested `number` block.
    Number(String),
    /// A boolean value encoded as Entry's `True` / `False` block.
    Boolean(bool),
    /// A nested reporter block.
    Block(Box<EntryBlock>),
}

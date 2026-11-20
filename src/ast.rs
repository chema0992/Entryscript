#[derive(Debug, Clone, PartialEq)]
pub struct Program {
    pub variables: Vec<VariableDeclaration>,
    pub functions: Vec<FunctionDefinition>,
    pub objects: Vec<ObjectSection>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct VariableDeclaration {
    pub name: String,
    pub kind: VariableKind,
    pub initial_value: Expression,
    pub initial_items: Vec<Expression>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VariableKind {
    Scalar,
    List,
}

#[derive(Debug, Clone, PartialEq)]
pub struct FunctionDefinition {
    pub name: String,
    pub params: Vec<FunctionParameter>,
    pub return_value: Option<Expression>,
    pub body: Vec<Statement>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct FunctionParameter {
    pub name: String,
    pub kind: FunctionParameterKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FunctionParameterKind {
    Value,
    Boolean,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ObjectSection {
    pub name: String,
    pub handlers: Vec<EventHandler>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct EventHandler {
    pub event: String,
    pub args: Vec<Expression>,
    pub body: Vec<Statement>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Statement {
    BlockCall {
        name: String,
        args: Vec<Expression>,
        body: Option<Vec<Statement>>,
        else_body: Option<Vec<Statement>>,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub enum Expression {
    String(String),
    Number(String),
    Boolean(bool),
    Null,
    Identifier(String),
    Call {
        name: String,
        args: Vec<Expression>,
    },
    Unary {
        op: UnaryOp,
        value: Box<Expression>,
    },
    Binary {
        left: Box<Expression>,
        op: BinaryOp,
        right: Box<Expression>,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnaryOp {
    Negate,
    Not,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinaryOp {
    Add,
    Subtract,
    Multiply,
    Divide,
    Modulo,
    Equal,
    NotEqual,
    Less,
    LessEqual,
    Greater,
    GreaterEqual,
    And,
    Or,
}

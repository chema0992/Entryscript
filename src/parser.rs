use std::mem::discriminant;

use crate::ast::{
    BinaryOp, EventHandler, Expression, FunctionDefinition, FunctionParameter,
    FunctionParameterKind, ObjectSection, Program, Statement, UnaryOp,
    VariableDeclaration, VariableKind,
};
use crate::error::CompileError;
use crate::lexer::{Token, TokenKind};

pub struct Parser {
    tokens: Vec<Token>,
    current: usize,
}

impl Parser {
    pub fn new(mut tokens: Vec<Token>) -> Self {
        if tokens.is_empty() {
            tokens.push(Token {
                kind: TokenKind::Eof,
                line: 1,
                column: 1,
            });
        }
        Self { tokens, current: 0 }
    }

    pub fn parse(mut self) -> Result<Program, CompileError> {
        let mut variables = Vec::new();
        let mut functions = Vec::new();
        let mut objects = Vec::new();

        while !self.is_at_end() {
            if self.check_identifier("var") {
                variables.push(self.parse_variable_declaration(VariableKind::Scalar)?);
            } else if self.check_identifier("list") {
                variables.push(self.parse_variable_declaration(VariableKind::List)?);
            } else if self.check_identifier("fn") {
                functions.push(self.parse_function_definition()?);
            } else if self.check_simple(&TokenKind::At) {
                objects.push(self.parse_object_section()?);
            } else {
                return Err(self.error_here(
                    "Expected a top-level declaration ('var', 'list', 'fn') or object section ('@')",
                ));
            }
        }

        Ok(Program { variables, functions, objects })
    }

    fn parse_variable_declaration(
        &mut self,
        kind: VariableKind,
    ) -> Result<VariableDeclaration, CompileError> {
        self.advance(); // var / list
        let name = self.expect_identifier("Expected a variable name")?;
        self.expect_simple(TokenKind::Equal, "Expected '=' after the variable name")?;

        let (initial_value, initial_items) = match kind {
            VariableKind::Scalar => (self.parse_expression()?, Vec::new()),
            VariableKind::List => {
                self.expect_simple(TokenKind::LBracket, "Expected '[' to begin a list literal")?;
                let mut items = Vec::new();
                if !self.check_simple(&TokenKind::RBracket) {
                    loop {
                        items.push(self.parse_expression()?);
                        if self.match_simple(&TokenKind::Comma) {
                            if self.check_simple(&TokenKind::RBracket) {
                                break;
                            }
                        } else {
                            break;
                        }
                    }
                }
                self.expect_simple(TokenKind::RBracket, "Expected ']' after list items")?;
                (Expression::Number("0".into()), items)
            }
        };

        Ok(VariableDeclaration { name, kind, initial_value, initial_items })
    }

    fn parse_function_definition(&mut self) -> Result<FunctionDefinition, CompileError> {
        self.advance(); // fn
        let name = self.expect_identifier("Expected a function name after 'fn'")?;
        self.expect_simple(TokenKind::LParen, "Expected '(' after the function name")?;

        let mut params = Vec::new();
        if !self.check_simple(&TokenKind::RParen) {
            loop {
                let param_name = self.expect_identifier("Expected a parameter name")?;
                let kind = if self.match_simple(&TokenKind::Colon) {
                    let type_name = self.expect_identifier("Expected 'value' or 'bool' after ':'")?;
                    match type_name.as_str() {
                        "value" | "string" => FunctionParameterKind::Value,
                        "bool" | "boolean" => FunctionParameterKind::Boolean,
                        _ => return Err(self.error_here(format!(
                            "Unsupported function parameter type '{type_name}'. Use 'value' or 'bool'."
                        ))),
                    }
                } else {
                    FunctionParameterKind::Value
                };
                params.push(FunctionParameter { name: param_name, kind });

                if self.match_simple(&TokenKind::Comma) {
                    if self.check_simple(&TokenKind::RParen) {
                        break;
                    }
                    continue;
                }
                break;
            }
        }
        self.expect_simple(TokenKind::RParen, "Expected ')' after function parameters")?;

        let return_value = if self.match_simple(&TokenKind::Arrow) {
            Some(self.parse_expression()?)
        } else {
            None
        };

        self.expect_simple(TokenKind::LBrace, "Expected '{' before the function body")?;
        let body = self.parse_statement_body_after_open_brace()?;
        Ok(FunctionDefinition { name, params, return_value, body })
    }

    fn parse_object_section(&mut self) -> Result<ObjectSection, CompileError> {
        self.expect_simple(TokenKind::At, "Expected '@' before an object name")?;
        let name = self.expect_identifier("Expected an object name after '@'")?;
        self.expect_simple(TokenKind::LBrace, "Expected '{' after the object name")?;

        let mut handlers = Vec::new();
        while !self.check_simple(&TokenKind::RBrace) && !self.is_at_end() {
            handlers.push(self.parse_event_handler()?);
        }
        self.expect_simple(TokenKind::RBrace, "Expected '}' to close the object section")?;
        Ok(ObjectSection { name, handlers })
    }

    fn parse_event_handler(&mut self) -> Result<EventHandler, CompileError> {
        let keyword = self.expect_identifier("Expected an event declaration")?;
        if keyword != "when" {
            return Err(self.error_here(format!("Expected 'when', found '{keyword}'")));
        }

        let event = self.expect_identifier("Expected an event name after 'when'")?;
        let args = if self.match_simple(&TokenKind::LParen) {
            self.parse_arguments_after_open_paren()?
        } else {
            Vec::new()
        };
        self.expect_simple(TokenKind::LBrace, "Expected '{' after the event declaration")?;
        let body = self.parse_statement_body_after_open_brace()?;
        Ok(EventHandler { event, args, body })
    }

    fn parse_statement_body_after_open_brace(&mut self) -> Result<Vec<Statement>, CompileError> {
        let mut statements = Vec::new();
        while !self.check_simple(&TokenKind::RBrace) && !self.is_at_end() {
            statements.push(self.parse_statement()?);
        }
        self.expect_simple(TokenKind::RBrace, "Expected '}' to close this block body")?;
        Ok(statements)
    }

    fn parse_statement(&mut self) -> Result<Statement, CompileError> {
        let name = self.expect_identifier("Expected a block name")?;

        // Parenthesized calls are the general form. `for 10 {}` and
        // `if condition {}` are intentionally supported as compact forms.
        let args = if self.match_simple(&TokenKind::LParen) {
            self.parse_arguments_after_open_paren()?
        } else if self.check_simple(&TokenKind::LBrace) {
            Vec::new()
        } else {
            vec![self.parse_expression()?]
        };

        let body = if self.match_simple(&TokenKind::LBrace) {
            Some(self.parse_statement_body_after_open_brace()?)
        } else {
            None
        };

        let else_body = if self.check_identifier("else") {
            self.advance();
            self.expect_simple(TokenKind::LBrace, "Expected '{' after 'else'")?;
            Some(self.parse_statement_body_after_open_brace()?)
        } else {
            None
        };

        Ok(Statement::BlockCall { name, args, body, else_body })
    }

    fn parse_arguments_after_open_paren(&mut self) -> Result<Vec<Expression>, CompileError> {
        let mut args = Vec::new();
        if self.match_simple(&TokenKind::RParen) {
            return Ok(args);
        }

        loop {
            args.push(self.parse_expression()?);
            if self.match_simple(&TokenKind::Comma) {
                if self.match_simple(&TokenKind::RParen) {
                    break;
                }
                continue;
            }
            self.expect_simple(TokenKind::RParen, "Expected ',' or ')' after an argument")?;
            break;
        }
        Ok(args)
    }

    fn parse_expression(&mut self) -> Result<Expression, CompileError> {
        self.parse_binary_expression(1)
    }

    fn parse_binary_expression(&mut self, min_precedence: u8) -> Result<Expression, CompileError> {
        let mut left = self.parse_unary_expression()?;
        loop {
            let Some((operator, precedence)) = self.peek_binary_operator() else { break };
            if precedence < min_precedence { break; }
            self.advance();
            let right = self.parse_binary_expression(precedence + 1)?;
            left = Expression::Binary { left: Box::new(left), op: operator, right: Box::new(right) };
        }
        Ok(left)
    }

    fn parse_unary_expression(&mut self) -> Result<Expression, CompileError> {
        if self.match_simple(&TokenKind::Minus) {
            return Ok(Expression::Unary {
                op: UnaryOp::Negate,
                value: Box::new(self.parse_unary_expression()?),
            });
        }
        if self.match_simple(&TokenKind::Bang) {
            return Ok(Expression::Unary {
                op: UnaryOp::Not,
                value: Box::new(self.parse_unary_expression()?),
            });
        }
        self.parse_primary_expression()
    }

    fn parse_primary_expression(&mut self) -> Result<Expression, CompileError> {
        let token = self.advance().clone();
        match token.kind {
            TokenKind::String(value) => Ok(Expression::String(value)),
            TokenKind::Number(value) => Ok(Expression::Number(value)),
            TokenKind::Identifier(name) => match name.as_str() {
                "true" => Ok(Expression::Boolean(true)),
                "false" => Ok(Expression::Boolean(false)),
                "null" => Ok(Expression::Null),
                _ if self.match_simple(&TokenKind::LParen) => {
                    let args = self.parse_arguments_after_open_paren()?;
                    Ok(Expression::Call { name, args })
                }
                _ => Ok(Expression::Identifier(name)),
            },
            TokenKind::LParen => {
                let expression = self.parse_expression()?;
                self.expect_simple(TokenKind::RParen, "Expected ')' after expression")?;
                Ok(expression)
            }
            _ => Err(CompileError::parser(
                token.line,
                token.column,
                "Expected a literal, identifier, reporter call, or parenthesized expression",
            )),
        }
    }

    fn peek_binary_operator(&self) -> Option<(BinaryOp, u8)> {
        match &self.peek().kind {
            TokenKind::OrOr => Some((BinaryOp::Or, 1)),
            TokenKind::AndAnd => Some((BinaryOp::And, 2)),
            TokenKind::EqualEqual => Some((BinaryOp::Equal, 3)),
            TokenKind::BangEqual => Some((BinaryOp::NotEqual, 3)),
            TokenKind::Less => Some((BinaryOp::Less, 4)),
            TokenKind::LessEqual => Some((BinaryOp::LessEqual, 4)),
            TokenKind::Greater => Some((BinaryOp::Greater, 4)),
            TokenKind::GreaterEqual => Some((BinaryOp::GreaterEqual, 4)),
            TokenKind::Plus => Some((BinaryOp::Add, 5)),
            TokenKind::Minus => Some((BinaryOp::Subtract, 5)),
            TokenKind::Star => Some((BinaryOp::Multiply, 6)),
            TokenKind::Slash => Some((BinaryOp::Divide, 6)),
            TokenKind::Percent => Some((BinaryOp::Modulo, 6)),
            _ => None,
        }
    }

    fn expect_identifier(&mut self, message: &str) -> Result<String, CompileError> {
        let token = self.advance().clone();
        match token.kind {
            TokenKind::Identifier(value) => Ok(value),
            _ => Err(CompileError::parser(token.line, token.column, message)),
        }
    }

    fn expect_simple(&mut self, expected: TokenKind, message: &str) -> Result<(), CompileError> {
        if self.check_simple(&expected) {
            self.advance();
            Ok(())
        } else {
            Err(self.error_here(message))
        }
    }

    fn match_simple(&mut self, expected: &TokenKind) -> bool {
        if self.check_simple(expected) {
            self.advance();
            true
        } else {
            false
        }
    }

    fn check_simple(&self, expected: &TokenKind) -> bool {
        discriminant(&self.peek().kind) == discriminant(expected)
    }

    fn check_identifier(&self, expected: &str) -> bool {
        matches!(&self.peek().kind, TokenKind::Identifier(value) if value == expected)
    }

    fn is_at_end(&self) -> bool {
        matches!(&self.peek().kind, TokenKind::Eof)
    }

    fn peek(&self) -> &Token {
        &self.tokens[self.current.min(self.tokens.len() - 1)]
    }

    fn advance(&mut self) -> &Token {
        let index = self.current;
        if !self.is_at_end() { self.current += 1; }
        &self.tokens[index]
    }

    fn error_here(&self, message: impl Into<String>) -> CompileError {
        let token = self.peek();
        CompileError::parser(token.line, token.column, message)
    }
}

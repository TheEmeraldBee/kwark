use crate::{error::Error, op_registry::OpRegistry, spanned::Spanned, token::Token};

pub const CTRL_CHARS: &str = "{([])};,";
pub const OP_CHARS: &str = "!-=+><|&/$%*";

/// The primary way of turning text into readable tokens
pub struct Lexer<'src> {
    chars: Vec<char>,
    len: usize,

    start: bool,
    cursor: usize,

    checkpoints: Vec<usize>,

    registry: &'src OpRegistry,
}

impl<'src> Lexer<'src> {
    /// Turns the passed text into a list of tokens
    pub fn lex(
        text: &str,
        registry: &'src OpRegistry,
    ) -> Result<Vec<Spanned<Token>>, Spanned<Error>> {
        let mut n = Self {
            chars: text.chars().collect(),
            len: text.chars().count(),

            start: true,
            cursor: 0,

            checkpoints: vec![],

            registry,
        };

        n.lex_inner()
    }

    /// Clears all checkpoints
    fn clear_checkpoints(&mut self) {
        self.checkpoints.clear();
    }

    /// Sets the checkpoint char
    fn checkpoint(&mut self) {
        self.checkpoints.push(self.cursor);
    }

    /// Removes the top of the checkpoint stack, but doesn't move the cursor
    fn remove_checkpoint(&mut self) {
        self.checkpoints.pop();
    }

    /// Moves the cursor forward, returning false if at EOF
    fn advance(&mut self) -> bool {
        if self.start {
            self.start = false;
            return self.len > 0;
        }

        if self.cursor + 1 >= self.len {
            return false;
        }

        self.cursor += 1;

        true
    }

    fn back(&mut self) {
        self.cursor = self.cursor.saturating_sub(1);
    }

    /// Advances the cursor, ignoring all whitespace
    fn advance_ignore_whitespace(&mut self) -> bool {
        if !self.advance() {
            return false;
        }

        while self.get().is_whitespace() {
            if !self.advance() {
                return false;
            }
        }

        true
    }

    fn get(&self) -> char {
        self.chars[self.cursor]
    }

    fn create<T>(&self, value: T) -> Spanned<T> {
        Spanned::new(self.cursor, self.cursor, value)
    }

    fn create_checkpoint<T>(&self, value: T) -> Spanned<T> {
        Spanned::new(
            *self.checkpoints.last().unwrap_or(&self.cursor),
            self.cursor,
            value,
        )
    }

    fn lex_inner(&mut self) -> Result<Vec<Spanned<Token>>, Spanned<Error>> {
        let mut res = vec![];
        loop {
            // Keep checkpoints clear
            self.clear_checkpoints();

            if !self.advance_ignore_whitespace() {
                return Ok(res);
            }

            match self.get() {
                // Number
                c if c.is_ascii_digit() => {
                    self.checkpoint();

                    let mut num = c.to_string();

                    let mut decimal = false;
                    while self.advance() {
                        let c = self.get();
                        if c == '.' {
                            if decimal {
                                self.back();
                                break;
                            } else {
                                decimal = true;
                                num.push(c);
                                continue;
                            }
                        }

                        if !self.get().is_ascii_digit() {
                            self.back();
                            break;
                        }

                        num.push(self.get())
                    }

                    res.push(
                        self.create_checkpoint(match decimal {
                            true => Token::Float(
                                num.parse()
                                    .expect("Number was already checked to be a float"),
                            ),
                            false => Token::Int(
                                num.parse()
                                    .expect("Number was already checked to be an int"),
                            ),
                        }),
                    )
                }

                // String
                '"' => {
                    self.checkpoint();

                    let mut str = "".to_string();

                    loop {
                        if !self.advance() {
                            return Err(self.create_checkpoint(Error::UnclosedString));
                        }
                        if self.get() == '"' {
                            break;
                        }

                        if self.get() == '\\' {
                            self.checkpoint();

                            if !self.advance() {
                                self.remove_checkpoint();
                                return Err(self.create_checkpoint(Error::UnclosedString));
                            }

                            match self.get() {
                                't' => str.push('\t'),
                                'r' => str.push('\r'),
                                'n' => str.push('\n'),

                                '\\' => str.push('\\'),

                                '"' => str.push('"'),

                                c => {
                                    return Err(self.create_checkpoint(Error::UnknownEscape(c)));
                                }
                            }

                            self.remove_checkpoint();
                            continue;
                        }

                        str.push(self.get())
                    }

                    res.push(self.create_checkpoint(Token::Str(str)));
                }

                // Ops
                c if OP_CHARS.contains(c) => {
                    self.checkpoint();

                    let mut op = c.to_string();
                    while self.advance() {
                        if !OP_CHARS.contains(self.get()) {
                            self.back();
                            break;
                        }

                        let candidate = format!("{op}{}", self.get());
                        if !self
                            .registry
                            .op_strings()
                            .any(|k| k.starts_with(&candidate))
                        {
                            self.back();
                            break;
                        }

                        op = candidate;
                    }

                    res.push(self.create_checkpoint(Token::Op(op)))
                }

                // Ctrl
                c if CTRL_CHARS.contains(c) => res.push(self.create(Token::Ctrl(c))),

                // Ident
                c if c.is_alphabetic() || c == '_' => {
                    self.checkpoint();

                    let mut ident = c.to_string();
                    while self.advance() {
                        let ch = self.get();
                        if !ch.is_alphanumeric() && ch != '_' {
                            self.back();
                            break;
                        }
                        ident.push(ch)
                    }

                    let tok = match ident.as_str() {
                        "true" => Token::Bool(true),
                        "false" => Token::Bool(false),

                        "let" => Token::Let,

                        "for" => Token::For,
                        "in" => Token::In,
                        "if" => Token::If,
                        "else" => Token::Else,

                        "return" => Token::Return,
                        "break" => Token::Break,

                        "fn" => Token::Fn,

                        _ => Token::Ident(ident),
                    };

                    res.push(self.create_checkpoint(tok));
                }

                // Err
                c => return Err(self.create(Error::UnknownChar(c))),
            }
        }
    }
}

#[cfg(test)]
mod test {
    use std::collections::HashMap;

    use super::*;

    fn registry() -> OpRegistry {
        let mut binary_ops = HashMap::new();
        binary_ops.insert("==".to_string(), ("eq".to_string(), 0));
        binary_ops.insert("!=".to_string(), ("neq".to_string(), 0));

        let mut unary_ops = HashMap::new();
        unary_ops.insert("!".to_string(), "not".to_string());

        OpRegistry {
            binary_ops,
            unary_ops,
        }
    }

    #[test]
    fn test_lex_empty_input_is_empty() {
        assert_eq!(Lexer::lex("", &registry()).unwrap(), vec![]);
    }

    #[test]
    fn test_lex_ctrl() {
        let tokens = Lexer::lex("()", &registry()).unwrap();
        assert_eq!(
            tokens,
            vec![
                Spanned::new(0, 0, Token::Ctrl('(')),
                Spanned::new(1, 1, Token::Ctrl(')'))
            ]
        )
    }

    #[test]
    fn test_lex_op() {
        let tokens = Lexer::lex("== != !=!", &registry()).unwrap();
        assert_eq!(
            tokens,
            vec![
                Spanned::new(0, 1, Token::Op("==".to_string())),
                Spanned::new(3, 4, Token::Op("!=".to_string())),
                Spanned::new(6, 7, Token::Op("!=".to_string())),
                Spanned::new(8, 8, Token::Op("!".to_string())),
            ]
        )
    }

    #[test]
    fn test_lex_op_stops_at_unknown_combo() {
        let tokens = Lexer::lex("!!", &registry()).unwrap();
        assert_eq!(
            tokens,
            vec![
                Spanned::new(0, 0, Token::Op("!".to_string())),
                Spanned::new(1, 1, Token::Op("!".to_string())),
            ]
        )
    }

    #[test]
    fn test_lex_ident() {
        let tokens = Lexer::lex("true false for if else  hi,ec9ho", &registry()).unwrap();
        assert_eq!(
            tokens,
            vec![
                Spanned::new(0, 3, Token::Bool(true)),
                Spanned::new(5, 9, Token::Bool(false)),
                Spanned::new(11, 13, Token::For),
                Spanned::new(15, 16, Token::If),
                Spanned::new(18, 21, Token::Else),
                Spanned::new(24, 25, Token::Ident("hi".to_string())),
                Spanned::new(26, 26, Token::Ctrl(',')),
                Spanned::new(27, 31, Token::Ident("ec9ho".to_string())),
            ]
        )
    }

    #[test]
    fn test_lex_str() {
        let tokens = Lexer::lex(
            r#"
                "abacadaba -> \" \" \n\t "
            "#,
            &registry(),
        )
        .unwrap();
        assert_eq!(
            tokens[0],
            Spanned::new(17, 42, Token::Str("abacadaba -> \" \" \n\t ".to_string()))
        );
    }
    #[test]
    fn test_lex_str_span_error() {
        let err = Lexer::lex(
            r#"
                "...\f..."
            "#,
            &registry(),
        )
        .unwrap_err();
        assert_eq!(err, Spanned::new(21, 22, Error::UnknownEscape('f')));
    }
}

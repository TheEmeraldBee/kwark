use kaon::{lex::Lexer, op_registry::OpRegistry, spanned::Spanned, token::Token};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HighlightKind {
    Keyword,
    Ident,
    Number,
    StringLit,
    Bool,
    Operator,
    Punct,
}

pub fn classify_tokens(source: &str, ops: &OpRegistry) -> Vec<Spanned<HighlightKind>> {
    let Ok(tokens) = Lexer::lex(source, ops) else {
        return vec![];
    };

    tokens
        .into_iter()
        .map(|tok| {
            let kind = match &*tok {
                Token::Int(_) | Token::Float(_) => HighlightKind::Number,
                Token::Str(_) => HighlightKind::StringLit,
                Token::Bool(_) => HighlightKind::Bool,

                Token::Ident(_) => HighlightKind::Ident,

                Token::Let
                | Token::For
                | Token::In
                | Token::If
                | Token::Else
                | Token::Return
                | Token::Break
                | Token::Fn => HighlightKind::Keyword,

                Token::Ctrl(_) => HighlightKind::Punct,
                Token::Op(_) => HighlightKind::Operator,
            };

            Spanned::new(tok.start, tok.end, kind)
        })
        .collect()
}

#[cfg(test)]
mod test {
    use super::*;

    fn ops() -> OpRegistry {
        let mut ops = OpRegistry::default();
        ops.binary_ops
            .insert("+".to_string(), ("add".to_string(), 1));
        ops
    }

    #[test]
    fn test_highlight_classifies_every_token_kind() {
        let kinds: Vec<_> = classify_tokens("let x = 1 + \"s\" + foo", &ops())
            .into_iter()
            .map(|t| *t)
            .collect();

        assert_eq!(
            kinds,
            vec![
                HighlightKind::Keyword,
                HighlightKind::Ident,
                HighlightKind::Operator,
                HighlightKind::Number,
                HighlightKind::Operator,
                HighlightKind::StringLit,
                HighlightKind::Operator,
                HighlightKind::Ident,
            ]
        );
    }

    #[test]
    fn test_highlight_on_invalid_input_is_empty() {
        assert!(classify_tokens("\"unterminated", &ops()).is_empty());
    }
}

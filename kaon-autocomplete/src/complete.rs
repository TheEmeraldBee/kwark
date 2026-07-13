use kaon::{
    engine::Engine, error::Error, lex::Lexer, parse::Parser, scope::Scope, spanned::Spanned,
    token::Token,
};

use crate::Span;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompletionKind {
    Variable,
    Function,
    Keyword,
    Operator,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Completion {
    pub text: String,
    pub kind: CompletionKind,
    pub replace: Span,
}

const KEYWORDS: &[&str] = &[
    "let", "for", "in", "if", "else", "return", "break", "fn", "true", "false",
];

enum Expectation {
    Ident,
    ExprStart,
    Any,
}

pub fn complete_at<Cx>(
    engine: &Engine<Cx>,
    source: &str,
    cursor: usize,
    scope: &Scope,
) -> Vec<Completion> {
    let prefix = char_prefix(source, cursor);

    let Ok(tokens) = Lexer::lex(&prefix, engine.ops()) else {
        return vec![];
    };

    let (partial, replace, context_tokens) = split_partial_ident(&tokens, cursor);

    let expectation = if context_tokens.is_empty() {
        Expectation::ExprStart
    } else {
        match Parser::parse_recovering(engine.ops(), context_tokens)
            .1
            .last()
        {
            Some(err) => expectation_from_error(err),
            None => Expectation::Any,
        }
    };

    let mut candidates = candidates_for(expectation, engine, scope);
    candidates.retain(|(text, _)| text.starts_with(&partial));
    candidates.sort_by(|a, b| rank(a.1).cmp(&rank(b.1)).then_with(|| a.0.cmp(&b.0)));

    candidates
        .into_iter()
        .map(|(text, kind)| Completion {
            text,
            kind,
            replace,
        })
        .collect()
}

fn char_prefix(source: &str, cursor: usize) -> String {
    source.chars().take(cursor).collect()
}

fn split_partial_ident(
    tokens: &[Spanned<Token>],
    cursor: usize,
) -> (String, Span, &[Spanned<Token>]) {
    if let Some(last) = tokens.last()
        && let Token::Ident(text) = &**last
        && last.end + 1 == cursor
    {
        return (
            text.clone(),
            Span::new(last.start, cursor),
            &tokens[..tokens.len() - 1],
        );
    }

    (String::new(), Span::empty_at(cursor), tokens)
}

fn expectation_from_error(err: &Spanned<Error>) -> Expectation {
    let label = match &**err {
        Error::Expected(label, _) => label.as_str(),
        Error::ExpectedFoundEOF(label) => label.as_str(),
        _ => return Expectation::Any,
    };

    match label {
        "name" => Expectation::Ident,
        "expr" | "argument" | "atom" | "condition" | "iterator or loop variable" => {
            Expectation::ExprStart
        }
        _ => Expectation::Any,
    }
}

fn candidates_for<Cx>(
    expectation: Expectation,
    engine: &Engine<Cx>,
    scope: &Scope,
) -> Vec<(String, CompletionKind)> {
    let mut out = vec![];

    let want_exprs = matches!(expectation, Expectation::ExprStart | Expectation::Any);
    let want_ops = matches!(expectation, Expectation::Any);

    if want_exprs {
        out.extend(
            scope
                .names()
                .map(|n| (n.to_string(), CompletionKind::Variable)),
        );
        out.extend(
            engine
                .function_names()
                .map(|n| (n.to_string(), CompletionKind::Function)),
        );
        out.extend(
            KEYWORDS
                .iter()
                .map(|k| (k.to_string(), CompletionKind::Keyword)),
        );
    }

    if want_ops {
        out.extend(
            engine
                .ops()
                .op_strings()
                .map(|o| (o.to_string(), CompletionKind::Operator)),
        );
    }

    out
}

fn rank(kind: CompletionKind) -> u8 {
    match kind {
        CompletionKind::Variable => 0,
        CompletionKind::Function => 1,
        CompletionKind::Keyword => 2,
        CompletionKind::Operator => 3,
    }
}

#[cfg(test)]
mod test {
    use kaon::{engine::Engine, value::Value};

    use super::*;

    fn engine() -> Engine<()> {
        Engine::default_std()
    }

    #[test]
    fn test_complete_at_start_of_line_offers_keywords() {
        let names: Vec<_> = complete_at(&engine(), "tr", 2, &Scope::default())
            .into_iter()
            .map(|c| c.text)
            .collect();

        assert!(names.contains(&"true".to_string()));
    }

    #[test]
    fn test_complete_after_let_offers_no_scope_names() {
        let mut scope = Scope::default();
        scope.register("existing", Value::Null);

        let completions = complete_at(&engine(), "let ", 4, &scope);
        assert!(completions.is_empty());
    }

    #[test]
    fn test_complete_ranks_variables_before_functions_before_keywords() {
        let mut scope = Scope::default();
        scope.register("foo", Value::Null);

        let mut engine = engine();
        engine.register(
            "foobar",
            kaon::engine::FunctionBuilder::new().build(|_| Ok(Value::Null)),
        );

        let completions = complete_at(&engine, "fo", 2, &scope);
        let names: Vec<_> = completions.iter().map(|c| c.text.as_str()).collect();

        assert_eq!(names, vec!["foo", "foobar", "for"]);
    }

    #[test]
    fn test_complete_replace_span_covers_the_partial_identifier() {
        let mut scope = Scope::default();
        scope.register("foo", Value::Null);

        let completions = complete_at(&engine(), "fo", 2, &scope);
        assert_eq!(completions[0].replace, Span::new(0, 2));
    }

    #[test]
    fn test_complete_after_full_expression_offers_operators() {
        let names: Vec<_> = complete_at(&engine(), "1 ", 2, &Scope::default())
            .into_iter()
            .map(|c| c.text)
            .collect();

        assert!(names.contains(&"+".to_string()));
    }
}

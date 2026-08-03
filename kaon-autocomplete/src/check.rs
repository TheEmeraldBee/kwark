use kaon::{
    engine::Engine,
    error::Error,
    expr::{Expr, SpannedExpr},
    lex::Lexer,
    parse::Parser,
    scope::Scope,
    spanned::Spanned,
    value::Value,
};

use crate::Span;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Severity {
    Error,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Diagnostic {
    pub span: Span,
    pub message: String,
    pub severity: Severity,
}

pub fn check_line<Cx>(engine: &Engine<Cx>, source: &str, scope: &Scope) -> Vec<Diagnostic> {
    let tokens = match Lexer::lex(source, engine.ops()) {
        Ok(tokens) => tokens,
        Err(err) => return vec![diagnostic_from_error(&err)],
    };

    let (tree, errors) = Parser::parse_recovering(engine.ops(), &tokens);

    let mut diagnostics: Vec<Diagnostic> = errors.iter().map(diagnostic_from_error).collect();

    let mut scope = scope.clone();
    walk(&tree, engine, &mut scope, &mut diagnostics);

    diagnostics
}

fn diagnostic_from_error(err: &Spanned<Error>) -> Diagnostic {
    Diagnostic {
        span: Span::new(err.start, err.end),
        message: err.to_string(),
        severity: Severity::Error,
    }
}

fn unknown_variable_diagnostic(name: &Spanned<String>) -> Diagnostic {
    Diagnostic {
        span: Span::new(name.start, name.end),
        message: format!("unknown variable `{}`", **name),
        severity: Severity::Error,
    }
}

fn walk<Cx>(expr: &SpannedExpr, engine: &Engine<Cx>, scope: &mut Scope, out: &mut Vec<Diagnostic>) {
    match &***expr {
        Expr::Error(_) => {}
        Expr::Null | Expr::Literal(_) => {}

        Expr::List(items) => {
            for item in items {
                walk(item, engine, scope, out);
            }
        }

        Expr::Let { name, body } => {
            walk(body, engine, scope, out);
            scope.register(name.clone().into_inner(), Value::Null);
        }

        Expr::Assign { name, body } => {
            walk(body, engine, scope, out);
            if scope.get(name.clone().into_inner()).is_none() {
                out.push(unknown_variable_diagnostic(name));
            }
        }

        Expr::Local { name } => {
            let known =
                scope.get(name.clone().into_inner()).is_some() || engine.function(name).is_some();

            if !known {
                out.push(unknown_variable_diagnostic(name));
            }
        }

        Expr::UnaryOp { body, .. } => walk(body, engine, scope, out),

        Expr::BinOp { left, right, .. } => {
            walk(left, engine, scope, out);
            walk(right, engine, scope, out);
        }

        Expr::If { cond, then, else_ } => {
            walk(cond, engine, scope, out);
            walk(then, engine, scope, out);
            if let Some(else_) = else_ {
                walk(else_, engine, scope, out);
            }
        }

        Expr::For {
            name,
            iterator,
            body,
        } => {
            walk(iterator, engine, scope, out);

            scope.push_frame();
            if let Some(name) = name {
                scope.register(name.clone().into_inner(), Value::Null);
            }
            walk(body, engine, scope, out);
            scope.pop_frame();
        }

        Expr::Func { args, body } => {
            scope.push_blocking_frame();
            for arg in args {
                scope.register(arg.clone().into_inner(), Value::Null);
            }
            walk(body, engine, scope, out);
            scope.pop_frame();
        }

        Expr::Then { first, next } => {
            walk(first, engine, scope, out);
            walk(next, engine, scope, out);
        }

        Expr::Block { body } => {
            scope.push_frame();
            walk(body, engine, scope, out);
            scope.pop_frame();
        }

        Expr::Call { body, args } => {
            walk(body, engine, scope, out);
            for arg in args {
                walk(arg, engine, scope, out);
            }
        }
    }
}

#[cfg(test)]
mod test {
    use kaon::engine::Engine;

    use super::*;

    fn engine() -> Engine<()> {
        Engine::default_std()
    }

    #[test]
    fn test_check_valid_line_has_no_diagnostics() {
        let diagnostics = check_line(&engine(), "1 + 2", &Scope::default());
        assert!(diagnostics.is_empty());
    }

    #[test]
    fn test_check_unknown_variable_is_flagged_at_its_span() {
        let diagnostics = check_line(&engine(), "foo", &Scope::default());
        assert_eq!(diagnostics.len(), 1);
        assert_eq!(diagnostics[0].span, Span::new(0, 2));
    }

    #[test]
    fn test_check_known_function_is_not_flagged() {
        let diagnostics = check_line(&engine(), "add", &Scope::default());
        assert!(diagnostics.is_empty());
    }

    #[test]
    fn test_check_let_binding_is_visible_to_later_statements() {
        let diagnostics = check_line(&engine(), "let x = 1; x + 1", &Scope::default());
        assert!(diagnostics.is_empty());
    }

    #[test]
    fn test_check_reports_a_syntax_error_and_an_unknown_variable_together() {
        let diagnostics = check_line(&engine(), "1 + ; bar", &Scope::default());
        assert_eq!(diagnostics.len(), 2);
    }

    #[test]
    fn test_check_leaves_the_caller_scope_untouched() {
        let scope = Scope::default();
        check_line(&engine(), "let x = 1", &scope);
        assert!(scope.get("x").is_none());
    }
}

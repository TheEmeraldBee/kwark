use std::{borrow::Cow, cell::RefCell, rc::Rc};

use kaon::prelude::*;
use kaon_autocomplete::{Autocomplete, HighlightKind};
use rustyline::{
    CompletionType, Config, Context, Editor, Helper, Result,
    completion::{Completer, Pair},
    highlight::{CmdKind, Highlighter},
    hint::Hinter,
    history::DefaultHistory,
    validate::{ValidationContext, ValidationResult, Validator},
};

const RESET: &str = "\x1b[0m";

fn color_for(kind: HighlightKind) -> &'static str {
    match kind {
        HighlightKind::Keyword => "\x1b[38;2;203;166;247m",
        HighlightKind::Ident => "\x1b[38;2;205;214;244m",
        HighlightKind::Number => "\x1b[38;2;250;179;135m",
        HighlightKind::StringLit => "\x1b[38;2;166;227;161m",
        HighlightKind::Bool => "\x1b[38;2;235;160;172m",
        HighlightKind::Operator => "\x1b[38;2;137;220;235m",
        HighlightKind::Punct => "\x1b[38;2;147;153;178m",
    }
}

struct KaonHelper {
    engine: Engine<()>,
    scope: Rc<RefCell<Scope>>,
}

impl Completer for KaonHelper {
    type Candidate = Pair;

    fn complete(&self, line: &str, pos: usize, _ctx: &Context<'_>) -> Result<(usize, Vec<Pair>)> {
        let scope = self.scope.borrow();
        let completions = self.engine.complete_at(line, pos, &scope);

        let start = completions.first().map_or(pos, |c| c.replace.start);
        let pairs = completions
            .into_iter()
            .map(|c| Pair {
                display: c.text.clone(),
                replacement: c.text,
            })
            .collect();

        Ok((start, pairs))
    }
}

impl Hinter for KaonHelper {
    type Hint = String;
}

impl Validator for KaonHelper {
    fn validate(&self, _ctx: &mut ValidationContext) -> Result<ValidationResult> {
        Ok(ValidationResult::Valid(None))
    }
}

impl Highlighter for KaonHelper {
    fn highlight<'l>(&self, line: &'l str, _pos: usize) -> Cow<'l, str> {
        let tokens = self.engine.highlight(line);
        let mut out = String::with_capacity(line.len() + tokens.len() * RESET.len());
        let mut last = 0;

        for tok in tokens {
            let end = tok.end + 1;
            out.push_str(&line[last..tok.start]);
            out.push_str(color_for(*tok));
            out.push_str(&line[tok.start..end]);
            out.push_str(RESET);
            last = end;
        }
        out.push_str(&line[last..]);

        Cow::Owned(out)
    }

    fn highlight_char(&self, _line: &str, _pos: usize, _kind: CmdKind) -> bool {
        true
    }
}

impl Helper for KaonHelper {}

fn main() -> Result<()> {
    let mut engine: Engine<()> = Engine::default_std();
    engine.register(
        "print",
        FunctionBuilder::new()
            .desc("Print the message to stdout")
            .arg("message", "The message to print", None)
            .build(|args| {
                println!("{}", args.value("message")?);
                Ok(Value::Null)
            }),
    );

    let scope = Rc::new(RefCell::new(Scope::default()));

    let config = Config::builder()
        .completion_type(CompletionType::List)
        .build();
    let mut rl: Editor<KaonHelper, DefaultHistory> = Editor::with_config(config)?;
    rl.set_helper(Some(KaonHelper {
        engine,
        scope: scope.clone(),
    }));

    loop {
        let line = match rl.readline("kaon> ") {
            Ok(line) => line,
            Err(
                rustyline::error::ReadlineError::Interrupted | rustyline::error::ReadlineError::Eof,
            ) => {
                break;
            }
            Err(err) => return Err(err),
        };

        if line.trim().is_empty() {
            continue;
        }

        rl.add_history_entry(&line)?;

        let helper = rl.helper().expect("helper is set");

        let diagnostics = helper.engine.check_line(&line, &scope.borrow());
        if !diagnostics.is_empty() {
            for diagnostic in &diagnostics {
                eprintln!("{}", diagnostic.message);
            }
            continue;
        }

        match helper.engine.exec(&line, &mut scope.borrow_mut(), &mut ()) {
            Ok(Value::Null) => {}
            Ok(value) => println!("{value}"),
            Err(err) => eprintln!("{err}"),
        }
    }

    Ok(())
}

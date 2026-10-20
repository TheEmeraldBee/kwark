use std::fmt::Display;

#[derive(Default, Debug, Copy, Clone, PartialEq, Eq)]
pub enum Severity {
    Error,
    Warn,
    #[default]
    Info,
}

impl Display for Severity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Self::Error => "error",
                Self::Warn => "warn",
                Self::Info => "info",
            }
        )
    }
}

#[derive(Default)]
pub struct LogState {
    message: Option<(String, Severity)>,
}

impl LogState {
    pub fn log(&mut self, message: impl Into<String>, severity: Severity) {
        self.message = Some((message.into(), severity))
    }

    pub fn clear(&mut self) {
        self.message = None
    }

    pub fn get(&self) -> Option<&(String, Severity)> {
        self.message.as_ref()
    }
}

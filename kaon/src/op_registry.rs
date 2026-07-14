use std::collections::HashMap;

/// A map from operators to function names
#[derive(Default)]
pub struct OpRegistry {
    pub binary_ops: HashMap<String, (String, u16)>,
    pub unary_ops: HashMap<String, String>,
}

impl OpRegistry {
    /// All operators this registry recognizes
    pub fn op_strings(&self) -> impl Iterator<Item = &str> {
        self.binary_ops
            .keys()
            .map(String::as_str)
            .chain(self.unary_ops.keys().map(String::as_str))
    }
}

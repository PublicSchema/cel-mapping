use cel::Program;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ErrorMode {
    Strict,
    Collect,
    Lenient,
}

impl ErrorMode {
    pub fn parse(s: Option<&str>) -> Self {
        match s.unwrap_or("strict").to_lowercase().as_str() {
            "collect" => ErrorMode::Collect,
            "lenient" => ErrorMode::Lenient,
            _ => ErrorMode::Strict,
        }
    }
}

/// Compiled CEL program plus the authored expression source.
#[derive(Debug)]
pub struct CompiledCel {
    pub program: Program,
    pub source: String,
}

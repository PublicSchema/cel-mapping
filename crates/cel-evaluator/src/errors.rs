//! Mapping errors (spec §9).

use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ErrorCode {
    MissingRequiredValue,
    TypeError,
    ParseError,
    InvalidDate,
    InvalidNumber,
    InvalidCode,
    UnknownCodeSystem,
    UnknownFunction,
    EvaluationError,
    ValidationError,
    InternalError,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MappingError {
    pub code: ErrorCode,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expression: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub record: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub index: Option<usize>,
    pub severity: ErrorSeverity,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ErrorSeverity {
    Error,
    Warning,
}

#[derive(Debug, Error)]
pub enum CompileError {
    #[error("YAML parse error: {0}")]
    Yaml(#[from] serde_yaml::Error),
    #[error("CEL compile error at {path}: {message}\nexpression: {expression}")]
    Cel {
        path: String,
        expression: String,
        message: String,
    },
    #[error("mapping error: {0}")]
    Mapping(String),
}

/// Editor / playground phase for a single-expression diagnostic.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExpressionPhase {
    /// `SecurityLimits` rejected the expression text (e.g. size).
    Limits,
    /// CEL parse / compile failure (`cel::Program::compile`).
    Syntax,
    /// `Program::execute` or JSON conversion failed.
    Evaluation,
}

/// One structured issue for UI (LSP, web editor, IDE).
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ExpressionIssue {
    pub phase: ExpressionPhase,
    pub severity: ErrorSeverity,
    pub code: ErrorCode,
    /// Human-readable diagnostic. For [`ExpressionPhase::Syntax`], includes CEL’s multi-line
    /// formatter (snippet + caret) when available, truncated for very large strings.
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub line: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub column: Option<u32>,
    /// Same as preview [`ExpressionPreviewResult::author_expression`] (editor buffer).
    pub expression: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_path: Option<String>,
}

/// Result of [`crate::evaluator::preview_cel_expression`] — always returned (no `Err`), so UIs can
/// render `issues` and optional `value` in one round-trip.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ExpressionPreviewResult {
    /// Expression as typed in the mapping / editor (before namespaced stdlib rewrite).
    pub author_expression: String,
    /// CEL source after [`crate::expr::rewrite_namespaced_calls`]; `None` if rejected by limits
    /// before rewrite. **Syntax `line` / `column` refer to this string** when present.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rewritten_expression: Option<String>,
    /// JSON result when evaluation succeeds; JSON `null` when there are issues or no value.
    pub value: Option<serde_json::Value>,
    pub issues: Vec<ExpressionIssue>,
    /// Stable hints for tools (e.g. AI agents) about how to interpret `issues` / positions.
    #[serde(default)]
    pub notes: Vec<String>,
}

pub(crate) const PREVIEW_NOTE_SYNTAX_LINES: &str = "Syntax line and column refer to `rewritten_expression` (after namespaced stdlib rewrites, e.g. `code.map_or_default` → `code_map_or_default`). Use `author_expression` for the exact editor buffer text.";
pub(crate) const PREVIEW_NOTE_EVALUATION: &str = "Evaluation errors use the authored text in each issue's `expression` field; `source_path` hints at bindings under `source` when inferable. Pass sample `source` / `context` JSON alongside this payload for automated fixes.";

pub fn truncate_diagnostic_string(s: &str, max_chars: usize) -> String {
    let count = s.chars().count();
    if count <= max_chars {
        s.to_string()
    } else {
        let head: String = s.chars().take(max_chars).collect();
        format!("{head}…\n({count} characters total, truncated for transport)")
    }
}

impl ExpressionPreviewResult {
    pub fn success(
        author_expression: String,
        rewritten_expression: String,
        value: serde_json::Value,
    ) -> Self {
        Self {
            author_expression,
            rewritten_expression: Some(rewritten_expression),
            value: Some(value),
            issues: Vec::new(),
            notes: Vec::new(),
        }
    }

    pub fn from_parts(
        author_expression: String,
        rewritten_expression: Option<String>,
        issues: Vec<ExpressionIssue>,
    ) -> Self {
        let mut notes = Vec::new();
        if rewritten_expression.as_ref().is_some_and(|s| !s.is_empty())
            && issues.iter().any(|i| i.phase == ExpressionPhase::Syntax)
        {
            notes.push(PREVIEW_NOTE_SYNTAX_LINES.to_string());
        }
        if issues
            .iter()
            .any(|i| i.phase == ExpressionPhase::Evaluation)
        {
            notes.push(PREVIEW_NOTE_EVALUATION.to_string());
        }
        Self {
            author_expression,
            rewritten_expression,
            value: None,
            issues,
            notes,
        }
    }

    pub fn is_ok(&self) -> bool {
        self.issues.is_empty()
    }
}

/// Failure when evaluating a single CEL expression (no mapping document).
#[derive(Debug, Error)]
pub enum StandaloneEvalError {
    #[error(transparent)]
    Compile(#[from] CompileError),
    #[error("invalid root binding name `{name}`: {message}")]
    InvalidBindingName { name: String, message: String },
    #[error("evaluation failed: {message}\nexpression: {expression}")]
    Evaluate { message: String, expression: String },
}

impl MappingError {
    pub fn error(
        code: ErrorCode,
        message: impl Into<String>,
        path: Option<String>,
        expression: Option<String>,
    ) -> Self {
        Self {
            code,
            message: message.into(),
            path,
            expression,
            source_path: None,
            record: None,
            index: None,
            severity: ErrorSeverity::Error,
        }
    }

    pub fn with_record(mut self, record: impl Into<String>, index: Option<usize>) -> Self {
        self.record = Some(record.into());
        self.index = index;
        self
    }

    pub fn warning(mut self) -> Self {
        self.severity = ErrorSeverity::Warning;
        self
    }
}

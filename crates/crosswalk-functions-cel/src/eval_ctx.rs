//! Per-evaluation thread-local context (ctx.* for host functions, warnings).

use std::cell::RefCell;

thread_local! {
    static CTX: RefCell<Option<FunctionRequestContext>> = const { RefCell::new(None) };
    static WARNINGS: RefCell<Vec<String>> = const { RefCell::new(Vec::new()) };
}

#[derive(Clone, Debug, Default)]
pub struct FunctionRequestContext {
    pub country: Option<String>,
    pub timezone: Option<String>,
    pub today: Option<String>,
}

impl FunctionRequestContext {
    pub fn from_json(ctx: &serde_json::Value) -> Self {
        Self {
            country: ctx
                .get("country")
                .and_then(|value| value.as_str())
                .map(ToString::to_string),
            timezone: ctx
                .get("timezone")
                .and_then(|value| value.as_str())
                .map(ToString::to_string),
            today: ctx
                .get("today")
                .and_then(|value| value.as_str())
                .map(ToString::to_string),
        }
    }
}

pub fn set_eval_ctx(ctx: FunctionRequestContext) {
    CTX.with(|c| *c.borrow_mut() = Some(ctx));
}

pub fn clear_eval_ctx() {
    CTX.with(|c| *c.borrow_mut() = None);
}

pub fn eval_ctx_get(path: &[&str]) -> Option<serde_json::Value> {
    CTX.with(|c| {
        let ctx = c.borrow();
        let ctx = ctx.as_ref()?;
        match path {
            ["country"] => ctx.country.clone().map(serde_json::Value::String),
            ["timezone"] => ctx.timezone.clone().map(serde_json::Value::String),
            ["today"] => ctx.today.clone().map(serde_json::Value::String),
            _ => None,
        }
    })
}

pub fn take_warnings() -> Vec<String> {
    WARNINGS.with(|w| {
        let mut v = w.borrow_mut();
        std::mem::take(&mut *v)
    })
}

pub fn clear_warnings() {
    WARNINGS.with(|w| w.borrow_mut().clear());
}

pub fn push_warning(msg: String) {
    WARNINGS.with(|w| w.borrow_mut().push(msg));
}

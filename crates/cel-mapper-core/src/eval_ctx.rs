//! Per-evaluation thread-local context (ctx.* for host functions, warnings).

use serde_json::Value as JsonValue;
use std::cell::RefCell;

thread_local! {
    static CTX: RefCell<Option<JsonValue>> = const { RefCell::new(None) };
    static WARNINGS: RefCell<Vec<String>> = const { RefCell::new(Vec::new()) };
}

pub fn set_eval_ctx(ctx: JsonValue) {
    CTX.with(|c| *c.borrow_mut() = Some(ctx));
}

pub fn clear_eval_ctx() {
    CTX.with(|c| *c.borrow_mut() = None);
}

pub fn eval_ctx_get(path: &[&str]) -> Option<JsonValue> {
    CTX.with(|c| {
        let root = c.borrow();
        let v = root.as_ref()?;
        let mut cur = v.clone();
        for p in path {
            cur = cur.get(*p)?.clone();
        }
        Some(cur)
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

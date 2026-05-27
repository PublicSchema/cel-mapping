//! Compatibility facade for helper request context and warnings.

pub fn set_eval_ctx(ctx: serde_json::Value) {
    mapping_functions_cel::eval_ctx::set_eval_ctx(
        mapping_functions_cel::FunctionRequestContext::from_json(&ctx),
    );
}

pub fn clear_eval_ctx() {
    mapping_functions_cel::eval_ctx::clear_eval_ctx();
}

pub fn eval_ctx_get(path: &[&str]) -> Option<serde_json::Value> {
    mapping_functions_cel::eval_ctx::eval_ctx_get(path)
}

pub fn take_warnings() -> Vec<String> {
    mapping_functions_cel::eval_ctx::take_warnings()
}

pub fn clear_warnings() {
    mapping_functions_cel::eval_ctx::clear_warnings();
}

pub fn push_warning(msg: String) {
    mapping_functions_cel::eval_ctx::push_warning(msg);
}

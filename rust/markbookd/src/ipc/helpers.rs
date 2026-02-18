#[allow(dead_code)]
pub fn method_in(method: &str, methods: &[&str]) -> bool {
    methods.iter().any(|m| *m == method)
}

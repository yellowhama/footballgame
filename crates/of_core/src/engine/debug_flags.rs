use std::sync::OnceLock;

fn env_flag_enabled(name: &str) -> bool {
    std::env::var(name)
        .map(|value| {
            matches!(
                value.trim().to_ascii_lowercase().as_str(),
                "1" | "true" | "yes" | "on"
            )
        })
        .unwrap_or(false)
}

pub fn action_debug_enabled() -> bool {
    if !cfg!(debug_assertions) {
        return false;
    }
    static FLAG: OnceLock<bool> = OnceLock::new();
    *FLAG.get_or_init(|| env_flag_enabled("OF_DEBUG_ACTIONS"))
}

pub fn match_debug_enabled() -> bool {
    if !cfg!(debug_assertions) {
        return false;
    }
    static FLAG: OnceLock<bool> = OnceLock::new();
    *FLAG.get_or_init(|| env_flag_enabled("OF_DEBUG_MATCH"))
}

// Fatal exit with a message, combined with panic="abort" in Cargo.toml
#[macro_export]
macro_rules! fatal {
    ($fmt:expr, $($x:expr),*) => {{
        tracing::error!($fmt, $($x,)*);
        panic!("Unhandled error")
    }};
}

// Evaluate a Result, and returns the contained Ok if possible,
// else fatal with the provided message
#[macro_export]
macro_rules! fatal_if_err {
    ($eval:expr; $msg:expr) => {{
        let result = $eval;
        if let Ok(result_) = result {
            result_
        } else {
            crate::fatal!($msg,)
        }
    }};
}

#[macro_export]
macro_rules! thread {
    ($func: expr $(, $name_for_thread: expr)? $(,)?) => {{
        std::thread::Builder::new()
            $(.name($name_for_thread))?
            .spawn($func)
            .map_err(|e| $crate::errors::NuclErrors::IO(e.to_string()))
    }}
}

#[macro_export]
/// Specify a global value that can be accessed from anywhere in the application.
/// Positional arguments:
/// - `$name`: The name of the global value. This will be the name of the variable that holds the value.
/// - `$type`: The type of the global value.
/// - `$init`: The initial value of the global value.
/// - `$set_fn`: The name of the function that will be used to set the global value.
/// - `$get_fn`: The name of the function that will be used to get the global value.
///
/// The macro will also automatically generate boilerplate code for unit tests to work correctly.
macro_rules! global_value {
    ($name:ident, $type:ty, $init:expr, $set_fn:ident, $get_fn:ident) => {
        use std::sync::{RwLock, RwLockReadGuard};

        static $name: RwLock<$type> = RwLock::new($init);

        pub fn $set_fn(value: $type) {
            *$name.write().expect("Failed to acquire RwLock for write") = value;
        }

        pub fn $get_fn() -> RwLockReadGuard<'static, $type> {
            $name.read().expect("Failed to acquire RwLock for read")
        }
    };
}

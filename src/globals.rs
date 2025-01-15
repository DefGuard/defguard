#[macro_export]
/// Specify a global value that can be accessed from anywhere in the application.
/// Pass the name of the global value, the type of the value, the initial value, the function name to set the value, and the function name to get the value.
///
/// The macro will also automatically generate boilerplate code for unit tests to work correctly.
macro_rules! global_value {
    ($name:ident, $type:ty, $init:expr, $set_fn:ident, $get_fn:ident) => {
        use std::sync::RwLock;
        #[cfg(not(test))]
        use std::sync::RwLockReadGuard;

        #[cfg(test)]
        thread_local! {
            static $name: RwLock<$type> = const { RwLock::new($init) };
        }

        #[cfg(not(test))]
        static $name: RwLock<$type> = RwLock::new($init);

        #[cfg(not(test))]
        pub fn $set_fn(value: $type) {
            *$name.write().expect("Failed to acquire lock on the mutex.") = value;
        }

        #[cfg(not(test))]
        pub fn $get_fn() -> RwLockReadGuard<'static, $type> {
            $name.read().expect("Failed to acquire lock on the mutex.")
        }

        #[cfg(test)]
        pub fn $set_fn(new_value: $type) {
            $name.with(|value| {
                *value.write().expect("Failed to acquire lock on the mutex.") = new_value;
            });
        }

        // This is not really a 1:1 replacement for the non-test RwLockReadGuard<'static, $type> as the RwLock may be tried to be
        // dereferenced
        #[cfg(test)]
        pub fn $get_fn() -> $type {
            $name.with(|value| {
                value
                    .read()
                    .expect("Failed to acquire lock on the mutex.")
                    .clone()
            })
        }
    };
}

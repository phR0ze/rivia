/// Unwrap the value on Ok or return false on Err
///
/// ### Examples
/// ```
/// use rivia::prelude::*;
///
/// unwrap_or_false!(Path::new("foo").to_string());
/// ```
#[macro_export]
macro_rules! unwrap_or_false {
    ($e:expr) => {
        match $e {
            Ok(v) => v,
            Err(_) => return false,
        }
    };
}

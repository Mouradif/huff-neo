/// Constant Bytecode Generation Module
pub mod constants;

/// Statement Bytecode Generation Module
pub mod statements;

/// Argument Call Module
pub mod arg_calls;

/// Builtin Function Module
pub mod builtin_function;

/// Prelude wraps common utilities.
pub mod prelude {
    pub use super::{arg_calls::*, constants::*, statements::*};
}

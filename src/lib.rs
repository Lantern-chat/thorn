#![recursion_limit = "256"]
#![allow(unused, dead_code, clippy::wrong_self_convention)]

pub extern crate postgres_types as pg;
pub extern crate thorn_macros;
pub extern crate tokio_postgres as pgt;

#[doc(hidden)]
pub extern crate paste;

#[macro_use]
pub mod macros;

pub mod literal;
pub mod name;
pub mod ty;

#[cfg(feature = "extensions")]
pub mod extensions;

#[cfg(feature = "generate")]
pub mod generate;

#[macro_use]
pub mod table;

#[macro_use]
pub mod enums;

#[macro_use]
pub mod func;

pub use enums::EnumType;
pub use table::{Table, TableExt};

#[cfg(test)]
mod test {
    use pg::Type;

    use super::*;

    use enums::TestEnum;
    use table::TestTable;

    tables! {
        pub struct Users in MySchema {
            Id: Type::INT8,
            UserName: Type::VARCHAR,
        }

        pub struct Messages in MySchema {
            Id: Type::INT8,
            Author: Type::INT8,
            Content: Type::TEXT,
        }
    }

    enums! {
        #[allow(clippy::enum_variant_names)]
        pub enum EventCode in TestSchema {
            MessageCreate,
            MessageUpdate,
            MessageDelete,
        }

        #[allow(clippy::enum_variant_names)]
        pub enum EventCode2 as "event_code3" {
            MessageCreate,
            MessageUpdate,
            MessageDelete,
        }
    }
}

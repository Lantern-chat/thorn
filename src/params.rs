use postgres_types::ToSql;

pub trait Parameters {
    type LEN: ga::ArrayLength;

    fn as_params(&self) -> ga::GenericArray<&(dyn ToSql + Sync), Self::LEN>;

    fn all_vars() -> ga::GenericArray<crate::Var, Self::LEN>;
}

impl<T> Parameters for &T
where
    T: Parameters,
{
    type LEN = <T as Parameters>::LEN;

    #[inline]
    fn as_params(&self) -> ga::GenericArray<&(dyn ToSql + Sync), Self::LEN> {
        <T as Parameters>::as_params(*self)
    }

    #[inline]
    fn all_vars() -> ga::GenericArray<crate::Var, Self::LEN> {
        <T as Parameters>::all_vars()
    }
}

#[macro_export]
macro_rules! params {
    (@COUNT ) => ($crate::ga::typenum::U0);
    (@COUNT $f:ident $($xf:ident)*) => ($crate::ga::typenum::Add1<$crate::params!(@COUNT $($xf)*)>);

    // TODO: Replace this with a muncher that doens't create new impl blocks
    (@FIELD struct $name:ident $(<$($q:tt),*>)? $(where $($w:tt),*)? {
        $offset:expr;
    }) => {};

    (@FIELD struct $name:ident $(<$($q:tt),*>)? $(where $($w:tt),*)? {
        $offset:expr;
        $field_vis:vis $field_name:ident = $field_column:expr;
        $($rest:tt)*
    }) => {
        impl $(<$($q),*>)? $name $(<$($q),*>)? $(where $($w),*)? {
            #[inline]
            $field_vis fn $field_name() -> $crate::Var {
                $crate::Var::at($field_column, $offset)
            }
        }

        $crate::params! {
            @FIELD struct $name $(<$($q),*>)? $(where $($w),*)? {
                $offset + 1; $($rest)*
            }
        }
    };

    ($(#[$meta:meta])* $vis:vis struct $name:ident $(<$($q:tt),*>)? $(where $($w:tt),*)? {
        $($(#[$field_meta:meta])* $field_vis:vis $field_name:ident : $field_ty:ty = $field_column:expr),*$(,)?
    }) => {
        $(#[$meta])*
        $vis struct $name $(<$($q),*>)? $(where $($w),*)? {
            $($(#[$field_meta])* $field_vis $field_name: $field_ty),*
        }

        impl $(<$($q),*>)? $crate::Parameters for $name $(<$($q),*>)? $(where $($w),*)? {
            type LEN = $crate::params!(@COUNT $($field_name)*);

            #[allow(non_snake_case)]
            fn as_params(&self) -> $crate::ga::GenericArray<&(dyn $crate::pg::ToSql + Sync), Self::LEN> {
                $crate::ga::GenericArray::from([$(&self.$field_name as _),*])
            }

            fn all_vars() -> $crate::ga::GenericArray<$crate::Var, Self::LEN> {
                $crate::ga::GenericArray::from([$(Self::$field_name()),*])
            }
        }

        $crate::params! {
            @FIELD struct $name $(<$($q),*>)? $(where $($w),*)? {
                1; $($field_vis $field_name = $field_column;)*
            }
        }
    };
}

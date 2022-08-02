use postgres_types::ToSql;

pub trait Parameters {
    type LEN: for<'a> ga::ArrayLength<&'a (dyn ToSql + Sync)>;

    fn as_params<'a>(&'a self) -> ga::GenericArray<&'a (dyn ToSql + Sync), Self::LEN>;
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
            fn as_params<'__PARAMETERS_LIFETIME>(&'__PARAMETERS_LIFETIME self) ->
                $crate::ga::GenericArray<&'__PARAMETERS_LIFETIME (dyn $crate::pg::ToSql + Sync), Self::LEN>
            {
                $crate::ga::GenericArray::from([
                    $(&self.$field_name as _),*
                ])
            }
        }

        $crate::params! {
            @FIELD struct $name $(<$($q),*>)? $(where $($w),*)? {
                1; $($field_vis $field_name = $field_column;)*
            }
        }
    };
}

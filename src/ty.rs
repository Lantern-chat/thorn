use pg::{Kind, Type};

pub trait TypeExt {
    fn is_boolean(&self) -> bool;
    fn is_array(&self) -> bool;
    fn is_composite(&self) -> bool;
}

impl TypeExt for Type {
    fn is_boolean(&self) -> bool {
        matches!(*self, Type::BOOL | Type::BOOL_ARRAY)
    }

    fn is_array(&self) -> bool {
        matches!(self.kind(), Kind::Array(_))
    }

    fn is_composite(&self) -> bool {
        matches!(self.kind(), Kind::Composite(_))
    }
}

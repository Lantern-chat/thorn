#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Schema {
    None,
    Named(&'static str),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Name {
    Default(&'static str),
    Custom(&'static str),
}

impl Schema {
    #[doc(hidden)]
    pub const fn set(self, name: &'static str) -> Self {
        Schema::Named(name)
    }
}

impl Name {
    #[doc(hidden)]
    pub const fn custom(self, name: &'static str) -> Self {
        Name::Custom(name)
    }

    pub const fn name(&self) -> &'static str {
        match *self {
            Name::Default(name) => name,
            Name::Custom(name) => name,
        }
    }
}

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

#[derive(Debug, Clone, Copy, thiserror::Error)]
pub enum NameError {
    #[error("Names must be at least 1 character long!")]
    NameTooShort,

    #[error("Names must start with an alphabetic character!")]
    NonAlphaStart,

    #[error("Names must only contain alphanumeric characters!")]
    InvalidName,
}

fn valid_name_start(c: char) -> bool {
    c.is_alphabetic() || c == '_'
}

fn valid_name_char(c: char) -> bool {
    c.is_alphanumeric() || c == '_' || c == '$' // ['_', '$'].contains(&c)
}

impl NameError {
    pub(crate) fn check_name(name: &'static str) -> Result<&'static str, Self> {
        let mut chars = name.chars();

        match chars.next() {
            None => return Err(NameError::NameTooShort),
            Some(c) if !valid_name_start(c) => return Err(NameError::NonAlphaStart),
            _ => {}
        }

        if !chars.all(valid_name_char) {
            return Err(NameError::InvalidName);
        }

        Ok(name)
    }
}

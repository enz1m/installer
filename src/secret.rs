use std::ops::{Deref, DerefMut};
use zeroize::{Zeroize, Zeroizing};

#[derive(Clone, Default, PartialEq, Eq)]
pub(crate) struct Secret(Zeroizing<String>);
impl Secret {
    pub(crate) fn clear(&mut self) {
        self.0.zeroize();
    }
}
impl std::fmt::Debug for Secret {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("[redacted]")
    }
}
impl From<String> for Secret {
    fn from(value: String) -> Self {
        Self(Zeroizing::new(value))
    }
}
impl From<&str> for Secret {
    fn from(value: &str) -> Self {
        value.to_owned().into()
    }
}
impl Deref for Secret {
    type Target = String;
    fn deref(&self) -> &String {
        &self.0
    }
}
impl DerefMut for Secret {
    fn deref_mut(&mut self) -> &mut String {
        &mut self.0
    }
}
impl PartialEq<&str> for Secret {
    fn eq(&self, other: &&str) -> bool {
        self.as_str() == *other
    }
}

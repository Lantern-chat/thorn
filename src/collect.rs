use std::{
    collections::{btree_map::Entry, BTreeMap},
    fmt::{self, Write},
    ops::Deref,
};

use pg::Type;

#[derive(Default)]
pub struct Collector {
    pub map: BTreeMap<usize, Type>,
    pub len: usize,
}

impl Collector {
    pub fn push(&mut self, t: Type) -> usize {
        self.len += 1;
        self.insert(self.len, t);
        self.len
    }

    pub fn insert(&mut self, idx: usize, t: Type) {
        match self.map.entry(idx) {
            Entry::Occupied(t2) => {
                assert_eq!(t, *t2.get(), "Specified placeholders have differing types")
            }
            Entry::Vacant(v) => {
                v.insert(t);
            }
        }
    }
}

const _: Option<&dyn Collectable> = None;

pub trait Collectable {
    fn collect(&self, w: &mut dyn Write, t: &mut Collector) -> fmt::Result;

    fn to_string(&self) -> (String, Collector) {
        let mut t = Collector::default();
        let mut w = String::new();

        self.collect(&mut w, &mut t).unwrap();

        (w, t)
    }

    fn needs_wrapping(&self) -> bool {
        false
    }

    fn _collect(&self, w: &mut dyn Write, t: &mut Collector) -> fmt::Result {
        if self.needs_wrapping() {
            w.write_char('(')?;
            self.collect(w, t)?;
            w.write_char(')')
        } else {
            self.collect(w, t)
        }
    }

    //fn is_boolean(&self) -> bool;
    //fn is_array(&self) -> bool;
    //fn is_composite(&self) -> bool;
}

impl<T> Collectable for T
where
    T: Deref,
    <T as Deref>::Target: Collectable,
{
    #[inline]
    fn collect(&self, w: &mut dyn Write, t: &mut Collector) -> fmt::Result {
        (**self).collect(w, t)
    }

    #[inline]
    fn to_string(&self) -> (String, Collector) {
        (**self).to_string()
    }

    #[inline]
    fn needs_wrapping(&self) -> bool {
        (**self).needs_wrapping()
    }

    #[inline]
    fn _collect(&self, w: &mut dyn Write, t: &mut Collector) -> fmt::Result {
        (**self)._collect(w, t)
    }
}

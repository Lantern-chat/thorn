use super::*;
use crate::{
    collect::{Collectable, Collector},
    *,
};

use std::fmt::{self, Write};

pub struct CallQuery {
    proc: Call,
}

impl CallQuery {
    pub fn new(proc: Call) -> Self {
        CallQuery { proc }
    }
}

impl Collectable for CallQuery {
    fn collect(&self, w: &mut dyn Write, t: &mut Collector) -> fmt::Result {
        w.write_str("CALL ")?;
        self.proc._collect(w, t)
    }
}

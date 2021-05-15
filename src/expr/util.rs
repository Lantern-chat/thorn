use super::*;

pub fn collect_delimited<E: Collectable>(
    exprs: impl IntoIterator<Item = E>,
    paren_wrap: bool,
    delimiter: &'static str,
    w: &mut dyn Write,
    t: &mut Collector,
) -> fmt::Result {
    if paren_wrap {
        w.write_str("(")?;
    }

    let mut iter = exprs.into_iter();

    if let Some(first) = iter.next() {
        first._collect(w, t)?;
    }

    for expr in iter {
        w.write_str(delimiter)?;
        expr._collect(w, t)?;
    }

    if paren_wrap {
        w.write_str(")")?;
    }

    Ok(())
}

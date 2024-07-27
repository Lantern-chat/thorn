use futures_util::{Stream, StreamExt};
use pg::ToSql;
use pgt::{Client, Row, RowStream};

use crate::macros::{Query, SqlFormatError};

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("SQL format error: {0}")]
    SqlFormat(#[from] SqlFormatError),

    #[error("Postgres error: {0}")]
    Postgres(#[from] pgt::Error),
}

#[allow(async_fn_in_trait)]
pub trait ClientExt {
    async fn query_stream2<'a, E: From<Row>>(
        &self,
        query: Result<Query<'a, E>, SqlFormatError>,
    ) -> Result<impl Stream<Item = Result<E, Error>>, Error>;

    async fn query2<'a, E: From<Row>>(
        &self,
        query: Result<Query<'a, E>, SqlFormatError>,
    ) -> Result<Vec<E>, Error> {
        let mut stream = std::pin::pin!(self.query_stream2(query).await?);

        let mut rows = Vec::new();
        while let Some(row) = stream.next().await {
            rows.push(row?);
        }
        Ok(rows)
    }
}

impl ClientExt for Client {
    async fn query_stream2<'a, E: From<Row>>(
        &self,
        query: Result<Query<'a, E>, SqlFormatError>,
    ) -> Result<impl Stream<Item = Result<E, Error>>, Error> {
        fn slice_iter<'a>(s: &'a [&'a (dyn ToSql + Sync)]) -> impl ExactSizeIterator<Item = &'a dyn ToSql> + 'a {
            s.iter().map(|s| *s as _)
        }

        let mut query = query?;

        let stmt = self.prepare_typed(&query.q, &query.param_tys).await?;

        let stream = self.query_raw(&stmt, slice_iter(&query.params)).await?;

        Ok(stream.map(|r| match r {
            Ok(row) => Ok(E::from(row)),
            Err(e) => Err(e.into()),
        }))
    }
}

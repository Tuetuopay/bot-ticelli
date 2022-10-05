/*!
 * Diesel plugin for pagination
 * https://github.com/diesel-rs/diesel/blob/master/examples/postgres/advanced-blog-cli/src/pagination.rs
 */

use diesel::{pg::Pg, prelude::*, query_builder::*, sql_types::BigInt};
use diesel_async::{methods::LoadQuery, AsyncPgConnection, RunQueryDsl};

pub trait Paginate: Sized {
    fn paginate(self, page: i64) -> Paginated<Self>;
}

impl<T> Paginate for T {
    fn paginate(self, page: i64) -> Paginated<Self> {
        Paginated { query: self, per_page: 20, page, offset: (page - 1) * 20 }
    }
}

#[derive(Debug, Clone, Copy, QueryId)]
pub struct Paginated<T> {
    query: T,
    page: i64,
    per_page: i64,
    offset: i64,
}

impl<'a, T: 'a> Paginated<T> {
    #[allow(dead_code)]
    pub fn per_page(self, per_page: i64) -> Self {
        Paginated { per_page, offset: (self.page - 1) * per_page, ..self }
    }

    #[allow(dead_code)]
    pub async fn load_and_count<U>(self, conn: &mut AsyncPgConnection) -> QueryResult<(Vec<U>, i64)>
    where
        Self: LoadQuery<'a, AsyncPgConnection, (U, i64)>,
        U: Send,
    {
        let res: Vec<(U, i64)> = self.load(conn).await?;
        let count = res.get(0).map(|x| x.1).unwrap_or(0);
        let data = res.into_iter().map(|x| x.0).collect();
        Ok((data, count))
    }
}

impl<T: Query> Query for Paginated<T> {
    type SqlType = (T::SqlType, BigInt);
}

// impl<T> RunQueryDsl<AsyncPgConnection> for Paginated<T> {}

impl<T> QueryFragment<Pg> for Paginated<T>
where
    T: QueryFragment<Pg>,
    // i64: ToSql<BigInt, DB>,
{
    fn walk_ast<'b>(&'b self, mut out: AstPass<'_, 'b, Pg>) -> QueryResult<()> {
        out.push_sql("SELECT *, COUNT(*) OVER () FROM (");
        self.query.walk_ast(out.reborrow())?;
        out.push_sql(") t LIMIT ");
        out.push_bind_param::<BigInt, _>(&self.per_page)?;
        out.push_sql(" OFFSET ");
        out.push_bind_param::<BigInt, _>(&self.offset)?;
        Ok(())
    }
}

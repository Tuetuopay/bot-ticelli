/*!
 * Diesel plugin for pagination
 * https://github.com/diesel-rs/diesel/blob/master/examples/postgres/advanced-blog-cli/src/pagination.rs
 */

use diesel::backend::Backend;
use diesel::prelude::{QueryResult, RunQueryDsl};
use diesel::query_builder::{AstPass, Query, QueryFragment};
use diesel::query_dsl::LoadQuery;
use diesel::sql_types::BigInt;
use diesel::Connection;

pub trait Paginate: Sized {
    fn paginate(self, page: i64) -> Paginated<Self>;
}

impl<T> Paginate for T {
    fn paginate(self, page: i64) -> Paginated<Self> {
        Paginated {
            query: self,
            per_page: 20,
            page,
        }
    }
}

#[derive(Debug, Clone, Copy, QueryId)]
pub struct Paginated<T> {
    query: T,
    page: i64,
    per_page: i64,
}

impl<T> Paginated<T> {
    #[allow(dead_code)]
    pub fn per_page(self, per_page: i64) -> Self {
        Paginated { per_page, ..self }
    }

    #[allow(dead_code)]
    pub fn load_and_count<C, U>(self, conn: &C) -> QueryResult<(Vec<U>, i64)>
    where
        C: Connection,
        Self: LoadQuery<C, (U, i64)>,
    {
        let res: Vec<(U, i64)> = self.load(conn)?;
        let count = res.get(0).map(|x| x.1).unwrap_or(0);
        let data = res.into_iter().map(|x| x.0).collect();
        Ok((data, count))
    }
}

impl<T: Query> Query for Paginated<T> {
    type SqlType = (T::SqlType, BigInt);
}

impl<C: Connection, T> RunQueryDsl<C> for Paginated<T> {}

impl<DB, T> QueryFragment<DB> for Paginated<T>
where
    DB: Backend,
    T: QueryFragment<DB>,
{
    fn walk_ast(&self, mut out: AstPass<DB>) -> QueryResult<()> {
        out.push_sql("SELECT *, COUNT(*) OVER () FROM (");
        self.query.walk_ast(out.reborrow())?;
        out.push_sql(") t LIMIT ");
        out.push_bind_param::<BigInt, _>(&self.per_page)?;
        out.push_sql(" OFFSET ");
        let offset = (self.page - 1) * self.per_page;
        out.push_bind_param::<BigInt, _>(&offset)?;
        Ok(())
    }
}

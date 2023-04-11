use super::{Create, List, NoParams, Update};
use crate::error::Result;
use crate::infra_cache::InfraCache;
use crate::models::Retrieve;
use crate::schema::{
    BufferStop, Catenary, Detector, OperationalPoint, RailJson, RailjsonError, Route, Signal,
    SpeedSection, Switch, SwitchType, TrackSection, TrackSectionLink,
};
use crate::tables::osrd_infra_infra;
use crate::tables::osrd_infra_infra::dsl;
use crate::views::pagination::{Paginate, PaginatedResponse};
use crate::{generated_data, DbPool};
use actix_web::web::{block, Data};
use chrono::{NaiveDateTime, Utc};
use derivative::Derivative;
use diesel::result::Error as DieselError;
use diesel::sql_types::{BigInt, Bool, Nullable, Text};
use diesel::{sql_query, PgConnection, QueryDsl, RunQueryDsl};
use diesel::{Connection, ExpressionMethods};
use editoast_derive::Model;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

pub const RAILJSON_VERSION: &str = "3.2.0";

#[derive(
    Clone,
    QueryableByName,
    Queryable,
    Debug,
    Serialize,
    Insertable,
    Deserialize,
    Identifiable,
    Model,
    AsChangeset,
    Derivative,
)]
#[diesel(table_name = osrd_infra_infra)]
#[model(retrieve, delete, create, update)]
#[model(table = "osrd_infra_infra")]
#[derivative(Default)]
pub struct Infra {
    #[diesel(deserialize_as = i64)]
    pub id: Option<i64>,
    #[diesel(deserialize_as = String)]
    pub name: Option<String>,
    #[derivative(Default(value = "RAILJSON_VERSION.into()"))]
    pub railjson_version: String,
    #[serde(skip_serializing)]
    #[diesel(deserialize_as = Uuid)]
    pub owner: Option<Uuid>,
    #[derivative(Default(value = "'0'.into()"))]
    pub version: String,
    #[diesel(deserialize_as = Option<String>)]
    #[derivative(Default(value = "Some(Some('0'.into()))"))]
    pub generated_version: Option<Option<String>>,
    #[derivative(Default(value = "false"))]
    pub locked: bool,
    #[diesel(deserialize_as = NaiveDateTime)]
    pub created: Option<NaiveDateTime>,
    #[derivative(Default(value = "Utc::now().naive_utc()"))]
    pub modified: NaiveDateTime,
}

impl Infra {
    pub fn retrieve_for_update(conn: &mut PgConnection, infra_id: i64) -> Result<Infra> {
        let infra = dsl::osrd_infra_infra
            .for_update()
            .find(infra_id)
            .first(conn)?;
        Ok(infra)
    }

    pub fn list_for_update(conn: &mut PgConnection) -> Vec<Infra> {
        dsl::osrd_infra_infra
            .for_update()
            .load::<Self>(conn)
            .expect("List infra query failed")
    }

    pub fn all(conn: &mut PgConnection) -> Vec<Infra> {
        dsl::osrd_infra_infra
            .load::<Self>(conn)
            .expect("List infra query failed")
    }

    pub fn bump_version(&self, conn: &mut PgConnection) -> Result<Self> {
        let new_version = self
            .version
            .parse::<u32>()
            .expect("Cannot convert version into an Integer")
            + 1;
        let infra_id = self.id.unwrap();
        let new_version = new_version.to_string();
        let mut infra = self.clone();
        infra.version = new_version;
        let infra = infra.update_conn(conn, infra_id)?.unwrap();
        Ok(infra)
    }

    pub async fn persist(self, railjson: RailJson, db_pool: Data<DbPool>) -> Result<Infra> {
        block::<_, Result<_>>(move || {
            if railjson.version != RAILJSON_VERSION {
                return Err(RailjsonError::WrongVersion(railjson.version).into());
            }
            let mut conn = db_pool.get()?;

            conn.transaction(|conn| {
                let infra = self.create_conn(conn)?;
                let infra_id = infra.id.unwrap();
                BufferStop::persist_batch(&railjson.buffer_stops, infra_id, conn)?;
                Catenary::persist_batch(&railjson.catenaries, infra_id, conn)?;
                Detector::persist_batch(&railjson.detectors, infra_id, conn)?;
                OperationalPoint::persist_batch(&railjson.operational_points, infra_id, conn)?;
                Route::persist_batch(&railjson.routes, infra_id, conn)?;
                Signal::persist_batch(&railjson.signals, infra_id, conn)?;
                Switch::persist_batch(&railjson.switches, infra_id, conn)?;
                SpeedSection::persist_batch(&railjson.speed_sections, infra_id, conn)?;
                SwitchType::persist_batch(&railjson.switch_types, infra_id, conn)?;
                TrackSectionLink::persist_batch(&railjson.track_section_links, infra_id, conn)?;
                TrackSection::persist_batch(&railjson.track_sections, infra_id, conn)?;
                Ok(infra)
            })
        })
        .await
        .unwrap()
    }

    pub async fn clone(infra_id: i64, db_pool: Data<DbPool>, new_name: String) -> Result<Infra> {
        let infra_to_clone = Infra::retrieve(db_pool.clone(), infra_id).await?.unwrap();
        block::<_, Result<_>>(move || {
        let mut conn = db_pool.get()?;
        match sql_query(
            "INSERT INTO osrd_infra_infra (name, railjson_version, owner, version, generated_version, locked, created, modified
            )
            SELECT $1, $2, '00000000-0000-0000-0000-000000000000', $3, $4, $5, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP FROM osrd_infra_infra
             WHERE id = $6
             RETURNING *",
        )
        .bind::<Text, _>(new_name)
        .bind::<Text, _>(RAILJSON_VERSION)
        .bind::<Text,_>(infra_to_clone.version)
        .bind::<Nullable<Text>,_>(infra_to_clone.generated_version.unwrap())
        .bind::<Bool,_>(infra_to_clone.locked)
        .bind::<BigInt,_>(infra_id)
        .get_result::<Infra>(&mut conn)
        {
            Ok(infra) => Ok(infra),
            Err(err) => Err(err.into()),
        }})
        .await
        .unwrap()
    }

    /// Refreshes generated data if not up to date and returns whether they were refreshed.
    /// `force` argument allows us to refresh it in any cases.
    /// This function will update `generated_version` accordingly.
    /// If refreshed you need to call `invalidate_after_refresh` to invalidate layer cache
    pub fn refresh(
        &self,
        conn: &mut PgConnection,
        force: bool,
        infra_cache: &InfraCache,
    ) -> Result<bool> {
        // Check if refresh is needed
        if !force
            && self.generated_version.is_some()
            && &self.version == self.generated_version.as_ref().unwrap().as_ref().unwrap()
        {
            return Ok(false);
        }

        generated_data::refresh_all(conn, self.id.unwrap(), infra_cache)?;

        // Update generated infra version
        let mut infra = self.clone();
        infra.generated_version = Some(Some(self.version.clone()));
        infra.update_conn(conn, self.id.unwrap())?;

        Ok(true)
    }

    /// Clear generated data of the infra
    /// This function will update `generated_version` acordingly.
    pub fn clear(&self, conn: &mut PgConnection) -> Result<bool> {
        generated_data::clear_all(conn, self.clone().id.unwrap())?;
        let mut infra = self.clone();
        infra.generated_version = Some(None);
        infra.update_conn(conn, self.id.unwrap())?;
        Ok(true)
    }
}

impl List<NoParams> for Infra {
    fn list_conn(
        conn: &mut diesel::PgConnection,
        page: i64,
        page_size: i64,
        _: NoParams,
    ) -> Result<PaginatedResponse<Self>> {
        dsl::osrd_infra_infra
            .distinct()
            .paginate(page, page_size)
            .load_and_count(conn)
    }
}

#[cfg(test)]
pub mod tests {
    use super::Infra;
    use crate::client::PostgresConfig;
    use crate::models::Create;
    use actix_web::test as actix_test;
    use chrono::Utc;
    use diesel::result::Error;
    use diesel::{Connection, PgConnection};
    use uuid::Uuid;

    pub fn build_test_infra() -> Infra {
        Infra {
            name: Some("test".into()),
            created: Some(Utc::now().naive_utc()),
            owner: Some(Uuid::nil()),
            ..Default::default()
        }
    }

    pub async fn test_infra_transaction(fn_test: fn(&mut PgConnection, Infra)) {
        let infra = build_test_infra();
        let mut conn = PgConnection::establish(&PostgresConfig::default().url()).unwrap();
        conn.test_transaction::<_, Error, _>(|conn| {
            let infra = infra.create_conn(conn).unwrap();
            fn_test(conn, infra);
            Ok(())
        })
    }

    #[actix_test]
    async fn create_infra() {
        test_infra_transaction(|_, infra| {
            assert_eq!("test", infra.name.unwrap());
        })
        .await;
    }
}
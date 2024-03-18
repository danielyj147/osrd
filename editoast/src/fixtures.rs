#[cfg(test)]
pub mod tests {
    use std::io::Cursor;

    use crate::client::PostgresConfig;
    use crate::models::train_schedule::Mrsp;
    use crate::models::{
        self, Identifiable, Infra, Pathfinding, PathfindingChangeset, ResultPosition, ResultStops,
        ResultTrain, Scenario, SimulationOutput, SimulationOutputChangeset, Timetable,
        TrainSchedule,
    };
    use crate::modelsv2::projects::Tags;
    use crate::modelsv2::rolling_stock_livery::RollingStockLiveryModel;
    use crate::modelsv2::scenario::Scenario as ScenarioV2;
    use crate::modelsv2::timetable::Timetable as TimetableV2;
    use crate::modelsv2::train_schedule::TrainSchedule as TrainScheduleV2;
    use crate::modelsv2::{
        self, Changeset, Document, ElectricalProfileSet, Model, Project, RollingStockModel, Study,
    };
    use crate::schema::electrical_profiles::{ElectricalProfile, ElectricalProfileSetData};
    use crate::schema::v2::trainschedule::TrainScheduleBase;
    use crate::schema::{RailJson, TrackRange};
    use crate::views::infra::InfraForm;
    use crate::views::v2::train_schedule::TrainScheduleForm;
    use crate::DbPool;

    use actix_web::web::Data;
    use chrono::Utc;
    use diesel_async::pooled_connection::AsyncDieselConnectionManager;
    use futures::executor;
    use postgis_diesel::types::LineString;
    use rstest::*;
    use std::fmt;

    pub struct TestFixture<T: modelsv2::DeleteStatic<i64> + Identifiable + Send> {
        pub model: T,
        pub db_pool: Data<DbPool>,
        pub infra: Option<Infra>,
    }

    impl<T: modelsv2::DeleteStatic<i64> + Identifiable + Send + fmt::Debug> fmt::Debug
        for TestFixture<T>
    {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            write!(f, "Fixture {:?} {:?}", self.model, self.infra)
        }
    }

    impl<T: modelsv2::DeleteStatic<i64> + Identifiable + Send> TestFixture<T> {
        pub fn id(&self) -> i64 {
            self.model.get_id()
        }

        pub fn new(model: T, db_pool: Data<DbPool>) -> Self {
            TestFixture {
                model,
                db_pool,
                infra: None,
            }
        }
    }

    impl<T: modelsv2::DeleteStatic<i64> + models::Create + Identifiable + Send> TestFixture<T> {
        pub async fn create_legacy(model: T, db_pool: Data<DbPool>) -> Self {
            TestFixture {
                model: model.create(db_pool.clone()).await.unwrap(),
                db_pool,
                infra: None,
            }
        }
    }

    impl<T> TestFixture<T>
    where
        T: modelsv2::Model + modelsv2::DeleteStatic<i64> + Identifiable + Send,
        T::Changeset: modelsv2::Create<T> + Send,
    {
        pub async fn create(cs: T::Changeset, db_pool: Data<DbPool>) -> Self {
            use modelsv2::Create;
            let conn = &mut db_pool.get().await.unwrap();
            TestFixture {
                model: cs.create(conn).await.unwrap(),
                db_pool,
                infra: None,
            }
        }
    }

    impl<T: modelsv2::DeleteStatic<i64> + Identifiable + Send> Drop for TestFixture<T> {
        fn drop(&mut self) {
            let mut conn = executor::block_on(self.db_pool.get()).unwrap();
            let _ = executor::block_on(T::delete_static(&mut conn, self.id()));
            if let Some(infra) = &self.infra {
                let _ = executor::block_on(<Infra as models::Delete>::delete_conn(
                    &mut conn,
                    infra.id.unwrap(),
                ));
            }
        }
    }

    #[async_trait::async_trait]
    impl<T: models::Delete> modelsv2::DeleteStatic<i64> for T {
        async fn delete_static(
            conn: &mut diesel_async::AsyncPgConnection,
            id: i64,
        ) -> crate::error::Result<bool> {
            T::delete_conn(conn, id).await
        }
    }

    #[fixture]
    pub fn db_pool() -> Data<DbPool> {
        let pg_config_url = PostgresConfig::default()
            .url()
            .expect("cannot get postgres config url");
        let config =
            AsyncDieselConnectionManager::<diesel_async::AsyncPgConnection>::new(pg_config_url);
        let pool = DbPool::builder(config).build().unwrap();
        Data::new(pool)
    }

    pub fn get_fast_rolling_stock(name: &str) -> Changeset<RollingStockModel> {
        let rs: Changeset<RollingStockModel> =
            serde_json::from_str(include_str!("./tests/example_rolling_stock_1.json"))
                .expect("Unable to parse");
        rs.name(name.to_string())
    }

    pub async fn named_fast_rolling_stock(
        name: &str,
        db_pool: Data<DbPool>,
    ) -> TestFixture<RollingStockModel> {
        let rs = get_fast_rolling_stock(name);
        TestFixture::create(rs, db_pool).await
    }

    pub fn get_other_rolling_stock(name: &str) -> Changeset<RollingStockModel> {
        let rs: Changeset<RollingStockModel> = serde_json::from_str(include_str!(
            "./tests/example_rolling_stock_2_energy_sources.json"
        ))
        .expect("Unable to parse");
        rs.name(name.to_string())
    }

    pub fn get_trainschedule_json_array() -> &'static str {
        include_str!("./tests/train_schedules/simple_array.json")
    }

    pub async fn named_other_rolling_stock(
        name: &str,
        db_pool: Data<DbPool>,
    ) -> TestFixture<RollingStockModel> {
        let rs = get_other_rolling_stock(name);
        TestFixture::create(rs, db_pool).await
    }

    async fn make_train_schedule(
        db_pool: Data<DbPool>,
        path_id: i64,
        timetable_id: i64,
        rolling_stock_id: i64,
    ) -> TrainSchedule {
        let train_schedule = TrainSchedule {
            path_id,
            timetable_id,
            rolling_stock_id,
            ..serde_json::from_str::<TrainSchedule>(include_str!("./tests/train_schedule.json"))
                .expect("Unable to parse")
        };
        use models::Create;
        train_schedule.create(db_pool).await.unwrap()
    }

    #[derive(Debug)]
    pub struct TrainScheduleV2FixtureSet {
        pub train_schedule: TestFixture<TrainScheduleV2>,
        pub timetable: TestFixture<TimetableV2>,
    }

    #[fixture]
    pub async fn train_schedule_v2(
        #[future] timetable_v2: TestFixture<TimetableV2>,
        db_pool: Data<DbPool>,
    ) -> TrainScheduleV2FixtureSet {
        let timetable = timetable_v2.await;
        let train_schedule_base: TrainScheduleBase =
            serde_json::from_str(include_str!("./tests/train_schedules/simple.json"))
                .expect("Unable to parse");
        let train_schedule_form = TrainScheduleForm {
            timetable_id: timetable.id(),
            train_schedule: train_schedule_base,
        };

        let train_schedule = TestFixture::create(train_schedule_form.into(), db_pool).await;

        TrainScheduleV2FixtureSet {
            train_schedule,
            timetable,
        }
    }

    #[derive(Debug)]
    pub struct TrainScheduleFixtureSet {
        pub train_schedule: TestFixture<TrainSchedule>,
        pub project: TestFixture<Project>,
        pub study: TestFixture<Study>,
        pub scenario: TestFixture<Scenario>,
        pub timetable: TestFixture<Timetable>,
        pub infra: TestFixture<Infra>,
        pub path: TestFixture<Pathfinding>,
        pub rolling_stock: TestFixture<RollingStockModel>,
    }

    pub async fn train_schedule_with_scenario(name: &str) -> TrainScheduleFixtureSet {
        let ScenarioFixtureSet {
            project,
            study,
            scenario,
            timetable,
            infra,
        } = scenario_fixture_set().await;

        let pathfinding = pathfinding(db_pool()).await;
        let mut rs_name = "fast_rolling_stock_".to_string();
        rs_name.push_str(name);
        let rolling_stock = named_fast_rolling_stock(&rs_name, db_pool()).await;
        let ts_model = make_train_schedule(
            db_pool(),
            pathfinding.id(),
            timetable.id(),
            rolling_stock.id(),
        )
        .await;
        let train_schedule: TestFixture<TrainSchedule> = TestFixture::new(ts_model, db_pool());
        TrainScheduleFixtureSet {
            train_schedule,
            project,
            study,
            scenario,
            timetable,
            infra,
            path: pathfinding,
            rolling_stock,
        }
    }

    pub struct ScenarioFixtureSet {
        pub project: TestFixture<Project>,
        pub study: TestFixture<Study>,
        pub scenario: TestFixture<Scenario>,
        pub timetable: TestFixture<Timetable>,
        pub infra: TestFixture<Infra>,
    }

    #[fixture]
    pub async fn scenario_fixture_set() -> ScenarioFixtureSet {
        let project = project(db_pool());
        let StudyFixtureSet { project, study } = study_fixture_set(db_pool(), project).await;
        let empty_infra = empty_infra(db_pool()).await;
        let timetable = timetable(db_pool()).await;
        let scenario_model = Scenario {
            name: Some(String::from("scenario_γ")),
            infra_id: Some(empty_infra.id()),
            timetable_id: Some(timetable.id()),
            study_id: Some(study.id()),
            creation_date: Some(Utc::now().naive_utc()),
            ..Scenario::default()
        };
        let scenario = TestFixture::create_legacy(scenario_model, db_pool()).await;
        ScenarioFixtureSet {
            project,
            study,
            scenario,
            timetable,
            infra: empty_infra,
        }
    }

    pub struct StudyFixtureSet {
        pub project: TestFixture<Project>,
        pub study: TestFixture<Study>,
    }

    #[fixture]
    pub async fn study_fixture_set(
        db_pool: Data<DbPool>,
        #[future] project: TestFixture<Project>,
    ) -> StudyFixtureSet {
        let project = project.await;
        let study_model = Study::changeset()
            .name("test_study".into())
            .description("test".into())
            .creation_date(Utc::now().naive_utc())
            .business_code("AAA".into())
            .service_code("BBB".into())
            .creation_date(Utc::now().naive_utc())
            .last_modification(Utc::now().naive_utc())
            .budget(Some(0))
            .tags(Tags::default())
            .state("some_state".into())
            .study_type("some_type".into())
            .project_id(project.id());
        StudyFixtureSet {
            project,
            study: TestFixture::create(study_model, db_pool).await,
        }
    }

    #[fixture]
    pub async fn project(db_pool: Data<DbPool>) -> TestFixture<Project> {
        let project_model = Project::changeset()
            .name("test_project".to_owned())
            .objectives("".to_owned())
            .description("".to_owned())
            .funders("".to_owned())
            .budget(Some(0))
            .creation_date(Utc::now().naive_utc())
            .last_modification(Utc::now().naive_utc())
            .tags(Tags::default());
        TestFixture::create(project_model, db_pool).await
    }

    #[fixture]
    pub async fn timetable(db_pool: Data<DbPool>) -> TestFixture<Timetable> {
        let timetable_model = Timetable {
            id: None,
            name: Some(String::from("with_electrical_profiles")),
        };
        TestFixture::create_legacy(timetable_model, db_pool).await
    }

    #[fixture]
    pub async fn timetable_v2(db_pool: Data<DbPool>) -> TestFixture<TimetableV2> {
        TestFixture::create(
            TimetableV2::changeset().electrical_profile_set_id(None),
            db_pool,
        )
        .await
    }

    pub struct ScenarioV2FixtureSet {
        pub project: TestFixture<Project>,
        pub study: TestFixture<Study>,
        pub scenario: TestFixture<ScenarioV2>,
        pub timetable: TestFixture<TimetableV2>,
        pub infra: TestFixture<Infra>,
    }

    #[fixture]
    pub async fn scenario_v2_fixture_set(
        db_pool: Data<DbPool>,
        #[future] timetable_v2: TestFixture<TimetableV2>,
        #[future] project: TestFixture<Project>,
    ) -> ScenarioV2FixtureSet {
        let StudyFixtureSet { project, study } = study_fixture_set(db_pool.clone(), project).await;
        let empty_infra = empty_infra(db_pool.clone()).await;
        let timetable = timetable_v2.await;
        let scenario = ScenarioV2::changeset()
            .name("test_scenario_v2".to_string())
            .description("test_scenario_v2 description".to_string())
            .creation_date(Utc::now().naive_utc())
            .last_modification(Utc::now().naive_utc())
            .tags(Tags::default())
            .timetable_id(timetable.model.get_id())
            .study_id(study.model.get_id())
            .infra_id(empty_infra.model.get_id());
        let scenario = TestFixture::create(scenario, db_pool).await;
        ScenarioV2FixtureSet {
            project,
            study,
            scenario,
            timetable,
            infra: empty_infra,
        }
    }

    #[fixture]
    pub async fn document_example(db_pool: Data<DbPool>) -> TestFixture<Document> {
        let img = image::open("src/tests/example_rolling_stock_image_1.gif").unwrap();
        let mut img_bytes: Vec<u8> = Vec::new();
        assert!(img
            .write_to(
                &mut Cursor::new(&mut img_bytes),
                image::ImageOutputFormat::Png
            )
            .is_ok());
        TestFixture::create(
            Document::changeset()
                .content_type(String::from("img/png"))
                .data(img_bytes),
            db_pool,
        )
        .await
    }
    pub struct RollingStockLiveryFixture {
        pub rolling_stock_livery: TestFixture<RollingStockLiveryModel>,
        pub rolling_stock: TestFixture<RollingStockModel>,
        pub image: TestFixture<Document>,
    }

    pub async fn rolling_stock_livery(
        name: &str,
        db_pool: Data<DbPool>,
    ) -> RollingStockLiveryFixture {
        let mut rs_name = "fast_rolling_stock_".to_string();
        rs_name.push_str(name);
        let rolling_stock = named_fast_rolling_stock(&rs_name, db_pool.clone()).await;
        let image = document_example(db_pool.clone()).await;

        let rolling_stock_livery = RollingStockLiveryModel::changeset()
            .name("test_livery".to_string())
            .rolling_stock_id(rolling_stock.id())
            .compound_image_id(Some(image.id()));

        RollingStockLiveryFixture {
            rolling_stock_livery: TestFixture::create(rolling_stock_livery, db_pool).await,
            rolling_stock,
            image,
        }
    }

    #[fixture]
    pub async fn electrical_profile_set(
        db_pool: Data<DbPool>,
    ) -> TestFixture<ElectricalProfileSet> {
        TestFixture::create(
            serde_json::from_str::<Changeset<ElectricalProfileSet>>(include_str!(
                "./tests/electrical_profile_set.json"
            ))
            .expect("Unable to parse"),
            db_pool,
        )
        .await
    }

    #[fixture]
    pub async fn dummy_electrical_profile_set(
        db_pool: Data<DbPool>,
    ) -> TestFixture<ElectricalProfileSet> {
        let ep_set_data = ElectricalProfileSetData {
            levels: vec![ElectricalProfile {
                value: "A".to_string(),
                power_class: "1".to_string(),
                track_ranges: vec![TrackRange::default()],
            }],
            level_order: Default::default(),
        };
        let ep_set = ElectricalProfileSet::changeset()
            .name("test".to_string())
            .data(ep_set_data.clone());
        TestFixture::create(ep_set, db_pool).await
    }

    #[fixture]
    pub async fn empty_infra(db_pool: Data<DbPool>) -> TestFixture<Infra> {
        let infra_form = InfraForm {
            name: String::from("test_infra"),
        };
        TestFixture::create_legacy(Infra::from(infra_form), db_pool).await
    }

    async fn make_small_infra(db_pool: Data<DbPool>) -> Infra {
        let railjson: RailJson = serde_json::from_str(include_str!(
            "../../tests/data/infras/small_infra/infra.json"
        ))
        .unwrap();
        let infra = Infra::from(InfraForm {
            name: "small_infra".to_owned(),
        });
        infra.persist(railjson, db_pool).await.unwrap()
    }

    /// Provides an [Infra] based on small_infra
    ///
    /// The infra is imported once for each test using the fixture and
    /// deleted afterwards. The small_infra railjson file is located
    /// at `editoast/tests/small_infra.json`. Any modification of that
    /// file is likely to impact the tests using this fixture. Likewise, if any
    /// modification of the infra itself (cf.
    /// `python/railjson_generator/railjson_generator/scripts/examples/small_infra.py`)
    /// is made, the `editoast/tests/small_infra/small_infra.json` file should be updated
    /// to the latest infra description.
    #[fixture]
    pub async fn small_infra(db_pool: Data<DbPool>) -> TestFixture<Infra> {
        TestFixture::new(make_small_infra(db_pool.clone()).await, db_pool)
    }

    #[fixture]
    pub async fn pathfinding(db_pool: Data<DbPool>) -> TestFixture<Pathfinding> {
        let small_infra = make_small_infra(db_pool.clone()).await;
        let pf_cs = PathfindingChangeset {
            infra_id: small_infra.id,
            payload: Some(
                serde_json::from_str(include_str!(
                    "tests/small_infra/pathfinding_fixture_payload.json"
                ))
                .unwrap(),
            ),
            length: Some(46776.),
            slopes: Some(diesel_json::Json(vec![])),
            curves: Some(diesel_json::Json(vec![])),
            geographic: Some(LineString::new(None)),
            schematic: Some(LineString::new(None)),
            owner: Some(Default::default()),
            created: Some(Default::default()),
            ..Default::default()
        };
        use models::Create;
        let pf = pf_cs.create(db_pool.clone()).await.unwrap().into();
        TestFixture {
            model: pf,
            db_pool,
            infra: Some(small_infra),
        }
    }

    pub struct TrainScheduleWithSimulationOutputFixtureSet {
        pub project: TestFixture<Project>,
        pub study: TestFixture<Study>,
        pub scenario: TestFixture<Scenario>,
        pub timetable: TestFixture<Timetable>,
        pub infra: TestFixture<Infra>,
        pub train_schedule: TestFixture<TrainSchedule>,
        pub simulation_output: TestFixture<SimulationOutput>,
        pub pathfinding: TestFixture<Pathfinding>,
        pub rolling_stock: TestFixture<RollingStockModel>,
    }

    pub async fn train_with_simulation_output_fixture_set(
        name: &str,
        db_pool: Data<DbPool>,
    ) -> TrainScheduleWithSimulationOutputFixtureSet {
        let ScenarioFixtureSet {
            project,
            study,
            scenario,
            timetable,
            infra,
        } = scenario_fixture_set().await;

        let mut rs_name = "fast_rolling_stock_".to_string();
        rs_name.push_str(name);
        let rolling_stock = named_fast_rolling_stock(&rs_name, db_pool.clone()).await;
        let pathfinding = pathfinding(db_pool.clone()).await;
        let train_schedule = make_train_schedule(
            db_pool.clone(),
            pathfinding.id(),
            timetable.id(),
            rolling_stock.id(),
        )
        .await;

        let result_train: ResultTrain = ResultTrain {
            stops: vec![
                ResultStops {
                    time: 0.0,
                    duration: 0.0,
                    position: 0.0,
                    ch: None,
                },
                ResultStops {
                    time: 110.90135448736316,
                    duration: 1.0,
                    position: 2417.6350658673214,
                    ch: None,
                },
            ],
            head_positions: vec![ResultPosition {
                time: 0.0,
                track_section: "TA7".to_string(),
                offset: 0.0,
                path_offset: 0.0,
            }],
            ..Default::default()
        };

        let simulation_output = SimulationOutputChangeset {
            mrsp: Some(diesel_json::Json(Mrsp::default())),
            base_simulation: Some(diesel_json::Json(result_train)),
            eco_simulation: Some(None),
            electrification_ranges: Some(diesel_json::Json(Vec::default())),
            power_restriction_ranges: Some(diesel_json::Json(Vec::default())),
            train_schedule_id: Some(train_schedule.id),
            ..Default::default()
        };
        use models::Create;
        let simulation_output: SimulationOutput = simulation_output
            .create(db_pool.clone())
            .await
            .unwrap()
            .into();

        let train_schedule = TestFixture::new(train_schedule, db_pool.clone());
        let simulation_output = TestFixture::new(simulation_output, db_pool);
        TrainScheduleWithSimulationOutputFixtureSet {
            project,
            study,
            scenario,
            timetable,
            infra,
            train_schedule,
            simulation_output,
            pathfinding,
            rolling_stock,
        }
    }
}

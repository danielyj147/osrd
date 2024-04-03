CREATE TABLE work_schedule_group (
    id int8 PRIMARY KEY GENERATED BY DEFAULT AS IDENTITY,
    creation_date timestamptz NOT NULL,
    name varchar(255) NOT NULL UNIQUE
);

CREATE TABLE work_schedule (
    id int8 PRIMARY KEY GENERATED BY DEFAULT AS IDENTITY,
    start_date_time timestamptz NOT NULL,
    end_date_time timestamptz NOT NULL,
    track_ranges jsonb NOT NULL,
    obj_id varchar(255) NOT NULL,
    work_schedule_type smallint NOT NULL,
    work_schedule_group_id int8 NOT NULL REFERENCES work_schedule_group(id) ON DELETE CASCADE
);

CREATE INDEX "work_schedule_start_date_time" ON "work_schedule" ("start_date_time");
CREATE INDEX "work_schedule_end_date_time" ON "work_schedule" ("end_date_time");

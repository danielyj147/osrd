table! {
    osrd_infra_infra {
        id -> Integer,
        name -> Text,
    }
}

table! {
    osrd_infra_tracksectionlayer {
        id -> Integer,
        obj_id -> Text,
        infra_id -> Integer,
    }
}

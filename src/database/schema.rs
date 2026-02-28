diesel::table! {
    #[sql_name = "user"]
    users (mxid) {
        mxid -> Text,
        uin -> Nullable<Text>,
        management_room -> Nullable<Text>,
        space_room -> Nullable<Text>,
    }
}

diesel::table! {
    puppet (uin) {
        uin -> Text,
        avatar -> Nullable<Text>,
        avatar_url -> Nullable<Text>,
        avatar_set -> Bool,
        displayname -> Nullable<Text>,
        name_quality -> SmallInt,
        name_set -> Bool,
        last_sync -> BigInt,
        custom_mxid -> Nullable<Text>,
        access_token -> Nullable<Text>,
        next_batch -> Nullable<Text>,
        enable_presence -> Bool,
    }
}

diesel::table! {
    portal (uid, receiver) {
        uid -> Text,
        receiver -> Text,
        mxid -> Nullable<Text>,
        name -> Text,
        name_set -> Bool,
        topic -> Text,
        topic_set -> Bool,
        avatar -> Text,
        avatar_url -> Nullable<Text>,
        avatar_set -> Bool,
        encrypted -> Bool,
        last_sync -> BigInt,
        first_event_id -> Nullable<Text>,
        next_batch_id -> Nullable<Text>,
    }
}

diesel::table! {
    message (chat_uid, chat_receiver, msg_id) {
        chat_uid -> Text,
        chat_receiver -> Text,
        msg_id -> Text,
        mxid -> Text,
        sender -> Text,
        timestamp -> BigInt,
        sent -> Bool,
        error -> Nullable<Text>,
        #[sql_name = "type"]
        msg_type -> Text,
    }
}

diesel::allow_tables_to_appear_in_same_query!(
    users,
    puppet,
    portal,
    message,
);

// 从 Diesel 生成的 schema 裁剪；仅保留借用与登记所需表。
// 省略 maintenance_holds 表及其 join 声明；设备上的维修身份列供状态重建。

diesel::table! {
    devices (id) {
        id -> Text,
        number -> Text,
        status -> Text,
        borrower_id -> Nullable<Text>,
        version -> BigInt,
        maintenance_work_order_id -> Nullable<Text>,
    }
}

diesel::table! {
    movements (id) {
        id -> Integer,
        device_id -> Text,
        actor_id -> Text,
        kind -> Text,
    }
}

diesel::joinable!(movements -> devices (device_id));

diesel::allow_tables_to_appear_in_same_query!(devices, movements,);

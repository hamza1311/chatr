use crate::{services, websocket};
use common::websocket::{MessagePayload, OpCode};
use common::{Message, MessageType, Room, RoomMember, User};
use sqlx::types::Uuid;
use sqlx::PgConnection;
use std::sync::Arc;

pub async fn create(db: &mut PgConnection, room: Room) -> anyhow::Result<Room> {
    let Room { name, uuid, .. } = room;

    Ok(sqlx::query_as!(
        Room,
        "
            insert into rooms(name, uuid)
            values ($1, $2)
            returning *;
        ",
        name,
        uuid
    )
    .fetch_one(db)
    .await?)
}

pub async fn get(db: &mut PgConnection, uuid: Uuid) -> anyhow::Result<Option<Room>> {
    let result = sqlx::query_as!(
        Room,
        "
            select *
            from rooms
            where uuid = $1;
        ",
        uuid
    )
    .fetch_one(db)
    .await;

    services::optional_value_or_err(result)
}

pub async fn join(
    db: &mut PgConnection,
    room: &Room,
    user: &User,
    has_elevated_perms: bool,
) -> anyhow::Result<RoomMember> {
    let ret = sqlx::query!(
        "
            insert into room_members(room_id, user_id, has_elevated_permissions)
            values ($1, $2, $3)
            returning *;
        ",
        room.uuid,
        user.uuid,
        has_elevated_perms
    )
    .fetch_one(&mut *db)
    .await?;
    let member = RoomMember {
        room: room.clone(),
        user: user.clone(),
        has_elevated_permissions: ret.has_elevated_permissions,
        joined_at: ret.joined_at,
    };

    // notify user
    websocket::send_message(
        Arc::new(MessagePayload {
            op: OpCode::RoomJoin,
            data: room.clone(), // maybe find a way to do this without cloning
        }),
        |uuid| uuid == user.uuid,
    )
    .await;

    // send the message that user joined
    services::message::create(
        db,
        Message::new_with_type(
            user.clone(),
            room.clone(),
            "".to_string(),
            MessageType::RoomJoin,
        ),
    )
    .await?;
    Ok(member)
}

pub async fn user_in_room(db: &mut PgConnection, room: &Room, user: &User) -> anyhow::Result<bool> {
    Ok(sqlx::query!(
        "
                select (count(*) = 1) as is_in_room
                from room_members
                where room_id = $1
                  and user_id = $2;
            ",
        room.uuid,
        user.uuid
    )
    .fetch_one(db)
    .await?
    .is_in_room
    .unwrap_or(false))
}

pub async fn get_with_user(db: &mut PgConnection, user: &User) -> anyhow::Result<Vec<Room>> {
    Ok(sqlx::query_as!(
        Room,
        "
            select r.*
            from room_members
                left join rooms r on r.uuid = room_members.room_id
            where user_id = $1;
        ",
        user.uuid
    )
    .fetch_all(db)
    .await?)
}

pub async fn get_room_members(
    db: &mut PgConnection,
    room: Room,
) -> anyhow::Result<Vec<RoomMember>> {
    let returned = sqlx::query!(
        "
            select u.username   as user_username,
                   u.uuid       as user_uuid,
                   u.password   as user_password,
                   u.created_at as user_created_at,
                   has_elevated_permissions,
                   joined_at
            from room_members
            left join rooms r on r.uuid = room_members.room_id
            left join users u on u.uuid = room_members.user_id
            where room_id = $1;
        ",
        room.uuid
    )
    .fetch_all(db)
    .await?;

    let mapped = returned
        .into_iter()
        .map(|value| RoomMember {
            user: User {
                uuid: value.user_uuid,
                username: value.user_username,
                password: value.user_password,
                created_at: value.user_created_at,
            },
            room: room.clone(),
            has_elevated_permissions: value.has_elevated_permissions,
            joined_at: value.joined_at,
        })
        .collect::<Vec<RoomMember>>();

    Ok(mapped)
}

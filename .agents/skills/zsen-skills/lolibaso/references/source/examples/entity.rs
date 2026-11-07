//! Entity 宏的完整用法示例；使用时参考此文件，无需再查阅 macros 源码。
//! 运行：cargo run --example entity --features entity
use lolibaso::entity::{Entity, EntityId, IntoParts, Reconstitute};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct UserId(u64);
impl EntityId for UserId {}

#[derive(Entity)]
#[entity(patchable)]
#[readonly(derive(Debug))]
#[state(derive(Debug))]
#[patch(derive(Debug))]
struct User {
    #[entity(id)] // 省略时默认选择名为 id 的字段。
    key: UserId,
    #[entity(patch)]
    name: String,
    #[entity(patch)]
    nickname: Option<String>,
    email: String,
}

// 默认生成 UserState（不含身份）、IntoParts，并要求手写 Reconstitute。
impl Reconstitute for User {
    type State = UserState;
    type Error = &'static str;

    fn reconstitute(id: Self::Id, state: Self::State) -> Result<Self, Self::Error> {
        if state.name.trim().is_empty() {
            return Err("姓名不能为空");
        }
        Ok(state.into_entity_unchecked(id))
    }
}

impl User {
    fn update(&mut self, patch: UserPatch) -> Result<(), &'static str> {
        let UserPatch {
            key,
            name,
            nickname,
        } = patch;
        if key != self.key {
            return Err("身份不匹配");
        }
        if let Some(name) = name {
            if name.trim().is_empty() {
                return Err("姓名不能为空");
            }
            self.name = name;
        }
        if let Some(nickname) = nickname {
            self.nickname = nickname;
        }
        Ok(())
    }
}

// no_state 不生成 State / IntoParts，也不要求 Reconstitute。
#[derive(Entity)]
#[entity(no_state)]
struct Session {
    id: UserId,
    token: String,
}

fn main() -> Result<(), &'static str> {
    let mut user = User::reconstitute(
        UserId(1),
        UserState {
            name: "Alice".into(),
            nickname: Some("Ali".into()),
            email: "alice@example.com".into(),
        },
    )?;
    assert_eq!(user.id(), &UserId(1));

    let UserReadonly {
        key,
        name,
        nickname,
        email,
    } = user.readonly();
    let _: (&UserId, &String, &Option<String>, &String) = (key, name, nickname, email);

    // None = 不改，Some(None) = 清空，Some(Some(value)) = 设置。
    user.update(UserPatch {
        key: UserId(1),
        name: None,
        nickname: Some(None),
    })?;

    let (
        id,
        UserState {
            name,
            nickname,
            email,
        },
    ) = user.into_parts();
    let _: (String, Option<String>, String) = (name, nickname, email);
    assert_eq!(id, UserId(1));

    let session = Session {
        id,
        token: "secret".into(),
    };
    assert_eq!(session.readonly().token, "secret");
    Ok(())
}

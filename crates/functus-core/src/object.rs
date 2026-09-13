//! 圏の対象 (Object)。データ型・API スキーマ・UI 状態を表す。

/// 対象を一意に識別する ID。
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ObjectId(pub String);

impl ObjectId {
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl From<&str> for ObjectId {
    fn from(id: &str) -> Self {
        Self::new(id)
    }
}

/// 対象の構造。積 (構造体) と余積 (タグ付きユニオン) を表現する。
#[derive(Debug, Clone, PartialEq)]
pub enum ObjectKind {
    /// これ以上分解しないスカラー型 (文字列・数値・UUID 等)。
    Scalar,
    /// 積: フィールド名と対象からなるレコード型。
    Product(Vec<(String, ObjectId)>),
    /// 余積: バリアント名と対象からなるタグ付きユニオン型。
    Coproduct(Vec<(String, ObjectId)>),
}

/// 圏の対象。
#[derive(Debug, Clone, PartialEq)]
pub struct Object {
    pub id: ObjectId,
    pub kind: ObjectKind,
}

impl Object {
    pub fn scalar(id: impl Into<String>) -> Self {
        Self {
            id: ObjectId::new(id),
            kind: ObjectKind::Scalar,
        }
    }

    pub fn product(id: impl Into<String>, fields: Vec<(String, ObjectId)>) -> Self {
        Self {
            id: ObjectId::new(id),
            kind: ObjectKind::Product(fields),
        }
    }

    pub fn coproduct(id: impl Into<String>, variants: Vec<(String, ObjectId)>) -> Self {
        Self {
            id: ObjectId::new(id),
            kind: ObjectKind::Coproduct(variants),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scalar_object_has_no_fields() {
        let user_id = Object::scalar("UserId");
        assert_eq!(user_id.kind, ObjectKind::Scalar);
    }

    #[test]
    fn product_object_holds_fields_in_order() {
        let user = Object::product(
            "User",
            vec![
                ("id".to_string(), ObjectId::from("UserId")),
                ("name".to_string(), ObjectId::from("String")),
            ],
        );
        match &user.kind {
            ObjectKind::Product(fields) => {
                assert_eq!(fields[0].0, "id");
                assert_eq!(fields[1].0, "name");
            }
            other => panic!("Product を期待したが {other:?} だった"),
        }
    }

    #[test]
    fn coproduct_object_holds_variants() {
        let fetch_state = Object::coproduct(
            "UserFetch",
            vec![
                ("Idle".to_string(), ObjectId::from("Unit")),
                ("Success".to_string(), ObjectId::from("User")),
            ],
        );
        match &fetch_state.kind {
            ObjectKind::Coproduct(variants) => assert_eq!(variants.len(), 2),
            other => panic!("Coproduct を期待したが {other:?} だった"),
        }
    }
}

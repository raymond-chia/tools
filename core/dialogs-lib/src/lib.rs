use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use strum_macros::{Display, EnumIter, EnumString};

#[derive(Debug, Deserialize, Serialize, Default)]
pub struct Script {
    #[serde(default)]
    pub function_signatures: Vec<String>,
    #[serde(default)]
    pub nodes: BTreeMap<String, Node>,
}

#[derive(Debug, Deserialize, Serialize, Default, Clone, Copy)]
pub struct Pos {
    pub x: f32,
    pub y: f32,
}

#[derive(Debug, Deserialize, Serialize, EnumString, Display, EnumIter, Clone)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Node {
    Dialogue {
        pos: Pos,
        dialogues: Vec<DialogueEntry>,
        actions: Option<Vec<Action>>,
        next_node: String,
    },
    Option {
        pos: Pos,
        options: Vec<OptionEntry>,
    },
    Battle {
        pos: Pos,
        results: Vec<BattleResult>,
    },
    Condition {
        pos: Pos,
        conditions: Vec<ConditionNodeEntry>,
    },
    End {
        pos: Pos,
    },
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct DialogueEntry {
    pub speaker: String,
    pub text: String,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Action {
    pub function: String,
    // sort params when serializing to TOML
    // params is not big enough to use HashMap
    pub params: BTreeMap<String, toml::Value>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct OptionEntry {
    pub text: String,
    pub next_node: String,
    pub conditions: Option<Vec<ConditionCheckEntry>>,
    pub actions: Option<Vec<Action>>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct BattleResult {
    pub result: String,
    pub next_node: String,
    pub conditions: Option<Vec<ConditionCheckEntry>>,
    pub actions: Option<Vec<Action>>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct ConditionNodeEntry {
    pub function: String,
    // sort params when serializing to TOML
    // params is not big enough to use HashMap
    pub params: BTreeMap<String, toml::Value>,
    pub next_node: String,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct ConditionCheckEntry {
    pub function: String,
    // sort params when serializing to TOML
    // params is not big enough to use HashMap
    pub params: BTreeMap<String, toml::Value>,
}

// 輔助方法：從 Node 獲取 pos
impl Node {
    pub fn pos(&self) -> Pos {
        match self {
            Node::Dialogue { pos, .. }
            | Node::Option { pos, .. }
            | Node::Battle { pos, .. }
            | Node::Condition { pos, .. }
            | Node::End { pos } => *pos,
        }
    }

    pub fn set_pos(&mut self, p: Pos) {
        match self {
            Node::Dialogue { pos, .. }
            | Node::Option { pos, .. }
            | Node::Battle { pos, .. }
            | Node::Condition { pos, .. }
            | Node::End { pos } => *pos = p,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_node_pos_and_set_pos() {
        let mut node = Node::Dialogue {
            pos: Pos { x: 1.0, y: 2.0 },
            dialogues: vec![DialogueEntry {
                speaker: "A".to_string(),
                text: "Hello".to_string(),
            }],
            actions: None,
            next_node: "next".to_string(),
        };
        let p = node.pos();
        assert_eq!(p.x, 1.0);
        assert_eq!(p.y, 2.0);

        node.set_pos(Pos { x: 3.0, y: 4.0 });
        let p2 = node.pos();
        assert_eq!(p2.x, 3.0);
        assert_eq!(p2.y, 4.0);
    }

    #[test]
    fn test_node_option_pos_and_set_pos() {
        let opt = Node::Option {
            pos: Pos { x: -1.5, y: 0.5 },
            options: vec![OptionEntry {
                text: "opt".to_string(),
                next_node: "n".to_string(),
                conditions: None,
                actions: None,
            }],
        };
        let p = opt.pos();
        assert_eq!(p.x, -1.5);
        assert_eq!(p.y, 0.5);

        let mut opt2 = opt;
        opt2.set_pos(Pos { x: 2.5, y: -3.5 });
        let p2 = opt2.pos();
        assert_eq!(p2.x, 2.5);
        assert_eq!(p2.y, -3.5);
    }
}

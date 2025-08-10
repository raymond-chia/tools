use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap};
use strum_macros::{Display, EnumIter, EnumString};

#[derive(Debug, Deserialize, Serialize, Default)]
pub struct Script {
    #[serde(default)]
    pub function_signatures: Vec<String>,
    #[serde(default)]
    pub nodes: HashMap<String, Node>,
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
            Node::Dialogue { pos, .. } => *pos,
            Node::Option { pos, .. } => *pos,
            Node::Battle { pos, .. } => *pos,
            Node::Condition { pos, .. } => *pos,
            Node::End { pos } => *pos,
        }
    }

    pub fn set_pos(&mut self, p: Pos) {
        match self {
            Node::Dialogue { pos, .. } => *pos = p,
            Node::Option { pos, .. } => *pos = p,
            Node::Battle { pos, .. } => *pos = p,
            Node::Condition { pos, .. } => *pos = p,
            Node::End { pos } => *pos = p,
        }
    }
}

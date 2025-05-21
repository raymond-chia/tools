use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use strum_macros::{Display, EnumString};
use toml;

// TOML 結構體定義
#[derive(Deserialize, Serialize, Debug)]
pub struct Script {
    pub function_signature: Vec<String>,
    pub node: HashMap<String, Node>,
}

#[derive(Deserialize, Serialize, Debug, EnumString, Display)]
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
        outcomes: Vec<Outcome>,
    },
    Condition {
        pos: Pos,
        conditions: Vec<ConditionNodeEntry>,
    },
    End {
        pos: Pos,
    },
}

#[derive(Deserialize, Serialize, Debug, Default)]
pub struct Pos {
    pub x: f32,
    pub y: f32,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct DialogueEntry {
    pub speaker: String,
    pub text: String,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct Action {
    pub function: String,
    pub params: HashMap<String, toml::Value>,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct OptionEntry {
    pub text: String,
    pub next_node: String,
    pub conditions: Option<Vec<ConditionCheckEntry>>,
    pub actions: Option<Vec<Action>>,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct Outcome {
    pub result: String,
    pub next_node: String,
    pub conditions: Option<Vec<ConditionCheckEntry>>,
    pub actions: Option<Vec<Action>>,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct ConditionNodeEntry {
    pub function: String,
    pub params: HashMap<String, toml::Value>,
    pub next_node: String,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct ConditionCheckEntry {
    pub function: String,
    pub params: HashMap<String, toml::Value>,
}

// 輔助方法：從 Node 獲取 pos
impl Node {
    pub fn pos(&self) -> &Pos {
        match self {
            Node::Dialogue { pos, .. } => pos,
            Node::Option { pos, .. } => pos,
            Node::Battle { pos, .. } => pos,
            Node::Condition { pos, .. } => pos,
            Node::End { pos } => pos,
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

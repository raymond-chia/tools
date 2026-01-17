//! 戰鬥統計模組
//!
//! 收集戰鬥數據以協助設計技能平衡

use std::collections::HashMap;

use crate::{TeamID, UnitID};
use skills_lib::{AttackResult, DefenseResult, SkillID};

/// 死亡原因
#[derive(Debug, Clone)]
pub enum DeathCause {
    /// 被技能擊殺
    Skill {
        killer_id: UnitID,
        skill_id: SkillID,
    },
    /// 燃燒傷害
    Burn,
    /// 坑洞墜落
    Pit {
        pusher_id: UnitID,
        skill_id: SkillID,
    },
}

/// 戰鬥統計總表
#[derive(Debug, Default, Clone)]
pub struct BattleStatistics {
    /// 按單位統計
    pub unit_stats: HashMap<UnitID, UnitStatistics>,
    /// 戰鬥回合數
    pub total_turns: usize,
}

/// 單位統計
#[derive(Debug, Default, Clone)]
pub struct UnitStatistics {
    /// 單位名稱（用於顯示，即使單位死亡也能查詢）
    pub unit_name: String,
    /// 隊伍 ID
    pub team: TeamID,
    /// 傷害統計
    pub damage: DamageStatistics,
    /// 命中統計
    pub hit: HitStatistics,
    /// 資源消耗統計
    pub resource: ResourceStatistics,
    /// 技能使用統計
    pub skill_stats: HashMap<SkillID, SkillStatistics>,
    /// 死亡統計
    pub death: DeathStatistics,
}

/// 傷害統計
#[derive(Debug, Default, Clone)]
pub struct DamageStatistics {
    /// 造成的總傷害
    pub damage_dealt: i32,
    /// 受到的總傷害
    pub damage_taken: i32,
    /// 造成的最高單次傷害
    pub max_single_damage: i32,
    /// 造成的治療總量
    pub healing_dealt: i32,
    /// 受到的治療總量
    pub healing_received: i32,
}

/// 命中統計
#[derive(Debug, Default, Clone)]
pub struct HitStatistics {
    /// 攻擊次數（嘗試命中的次數）
    pub attack_count: usize,
    /// 命中次數
    pub hit_count: usize,
    /// 爆擊次數
    pub critical_count: usize,
    /// 完全命中次數（亂數 > 95）
    pub guaranteed_hit_count: usize,
    /// 被攻擊次數
    pub attacked_count: usize,
    /// 閃避次數
    pub evade_count: usize,
    /// 格擋次數
    pub block_count: usize,
    /// 完全閃避次數（亂數 <= 5）
    pub guaranteed_evade_count: usize,
    /// 豁免成功次數
    pub save_success_count: usize,
    /// 豁免失敗次數
    pub save_fail_count: usize,
}

/// 資源消耗統計
#[derive(Debug, Default, Clone)]
pub struct ResourceStatistics {
    /// MP 消耗總量
    pub mp_consumed: i32,
    /// 行動次數（施放技能次數）
    pub action_count: usize,
    /// 反應次數
    pub reaction_count: usize,
    /// 移動格數
    pub tiles_moved: usize,
}

/// 技能統計
#[derive(Debug, Default, Clone)]
pub struct SkillStatistics {
    /// 使用次數
    pub use_count: usize,
    /// 造成的總傷害
    pub total_damage: i32,
    /// 命中次數
    pub hit_count: usize,
    /// 攻擊次數
    pub attack_count: usize,
    /// 爆擊次數
    pub critical_count: usize,
}

/// 死亡統計
#[derive(Debug, Default, Clone)]
pub struct DeathStatistics {
    /// 擊殺數
    pub kills: usize,
    /// 死亡數（0 或 1）
    pub deaths: usize,
    /// 各技能擊殺數
    pub killing_blows: HashMap<SkillID, usize>,
    /// 死亡原因
    pub death_cause: Option<DeathCause>,
}

impl BattleStatistics {
    pub fn new() -> Self {
        Self::default()
    }

    /// 註冊單位（在戰鬥開始時呼叫，確保死亡後仍能查詢名稱與隊伍）
    pub fn register_unit(&mut self, unit_id: UnitID, name: String, team: TeamID) {
        let unit_stats = self.unit_stats.entry(unit_id).or_default();
        unit_stats.unit_name = name;
        unit_stats.team = team;
    }

    /// 記錄技能施放
    pub fn record_skill_cast(
        &mut self,
        caster_id: UnitID,
        skill_id: &SkillID,
        mp_cost: i32,
        is_reaction: bool,
    ) {
        let unit_stats = self.unit_stats.entry(caster_id).or_default();

        unit_stats.resource.mp_consumed += mp_cost;
        if is_reaction {
            unit_stats.resource.reaction_count += 1;
        } else {
            unit_stats.resource.action_count += 1;
        }

        let skill_stats = unit_stats.skill_stats.entry(skill_id.clone()).or_default();
        skill_stats.use_count += 1;
    }

    /// 記錄攻擊嘗試
    pub fn record_attack(&mut self, attacker_id: UnitID, target_id: UnitID, skill_id: &SkillID) {
        // 攻擊者統計
        let attacker_stats = self.unit_stats.entry(attacker_id).or_default();
        attacker_stats.hit.attack_count += 1;

        let skill_stats = attacker_stats
            .skill_stats
            .entry(skill_id.clone())
            .or_default();
        skill_stats.attack_count += 1;

        // 目標統計
        let target_stats = self.unit_stats.entry(target_id).or_default();
        target_stats.hit.attacked_count += 1;
    }

    /// 記錄命中結果
    pub fn record_hit(
        &mut self,
        attacker_id: UnitID,
        target_id: UnitID,
        skill_id: &SkillID,
        defense: DefenseResult,
        attack: AttackResult,
    ) {
        let attacker_stats = self.unit_stats.entry(attacker_id).or_default();
        let skill_stats = attacker_stats
            .skill_stats
            .entry(skill_id.clone())
            .or_default();

        // 記錄爆擊（不管防禦結果）
        match attack {
            AttackResult::Critical => {
                attacker_stats.hit.critical_count += 1;
                skill_stats.critical_count += 1;
            }
            AttackResult::Normal | AttackResult::NoAttack => {}
        }

        // 記錄防禦結果
        match defense {
            DefenseResult::Hit => {
                attacker_stats.hit.hit_count += 1;
                skill_stats.hit_count += 1;
            }
            DefenseResult::GuaranteedHit => {
                attacker_stats.hit.hit_count += 1;
                attacker_stats.hit.guaranteed_hit_count += 1;
                skill_stats.hit_count += 1;
            }
            DefenseResult::Evaded => {
                let target_stats = self.unit_stats.entry(target_id).or_default();
                target_stats.hit.evade_count += 1;
            }
            DefenseResult::Blocked => {
                let target_stats = self.unit_stats.entry(target_id).or_default();
                target_stats.hit.block_count += 1;
            }
            DefenseResult::GuaranteedEvade => {
                let target_stats = self.unit_stats.entry(target_id).or_default();
                target_stats.hit.evade_count += 1;
                target_stats.hit.guaranteed_evade_count += 1;
            }
        }
    }

    /// 記錄 HP 變化（傷害或治療）
    pub fn record_hp_change(
        &mut self,
        source_id: UnitID,
        target_id: UnitID,
        skill_id: &SkillID,
        amount: i32,
    ) {
        if amount < 0 {
            // 傷害
            let damage = -amount;

            let source_stats = self.unit_stats.entry(source_id).or_default();
            source_stats.damage.damage_dealt += damage;
            if damage > source_stats.damage.max_single_damage {
                source_stats.damage.max_single_damage = damage;
            }

            let skill_stats = source_stats
                .skill_stats
                .entry(skill_id.clone())
                .or_default();
            skill_stats.total_damage += damage;

            let target_stats = self.unit_stats.entry(target_id).or_default();
            target_stats.damage.damage_taken += damage;
        } else if amount > 0 {
            // 治療
            let source_stats = self.unit_stats.entry(source_id).or_default();
            source_stats.damage.healing_dealt += amount;

            let target_stats = self.unit_stats.entry(target_id).or_default();
            target_stats.damage.healing_received += amount;
        }
    }

    /// 記錄豁免結果
    pub fn record_save(&mut self, target_id: UnitID, success: bool) {
        let target_stats = self.unit_stats.entry(target_id).or_default();
        if success {
            target_stats.hit.save_success_count += 1;
        } else {
            target_stats.hit.save_fail_count += 1;
        }
    }

    /// 記錄移動
    pub fn record_movement(&mut self, unit_id: UnitID, tiles: usize) {
        let unit_stats = self.unit_stats.entry(unit_id).or_default();
        unit_stats.resource.tiles_moved += tiles;
    }

    /// 記錄回合結束
    pub fn record_turn_end(&mut self) {
        self.total_turns += 1;
    }

    /// 記錄單位死亡
    pub fn record_death(&mut self, victim_id: UnitID, cause: DeathCause) {
        let victim_stats = self.unit_stats.entry(victim_id).or_default();
        victim_stats.death.deaths = 1;
        victim_stats.death.death_cause = Some(cause.clone());

        // 記錄擊殺者的統計
        match cause {
            DeathCause::Skill {
                killer_id,
                skill_id,
            } => {
                let killer_stats = self.unit_stats.entry(killer_id).or_default();
                killer_stats.death.kills += 1;
                *killer_stats
                    .death
                    .killing_blows
                    .entry(skill_id)
                    .or_default() += 1;
            }
            DeathCause::Pit {
                pusher_id,
                skill_id,
            } => {
                let pusher_stats = self.unit_stats.entry(pusher_id).or_default();
                pusher_stats.death.kills += 1;
                *pusher_stats
                    .death
                    .killing_blows
                    .entry(skill_id)
                    .or_default() += 1;
            }
            DeathCause::Burn => {
                // 簡單起見，燃燒死亡不計入任何人的擊殺
            }
        }
    }

    /// 記錄燃燒傷害（狀態效果造成的傷害）
    pub fn record_burn_damage(&mut self, unit_id: UnitID, damage: i32) {
        let stats = self.unit_stats.entry(unit_id).or_default();
        stats.damage.damage_taken += damage;
    }
}

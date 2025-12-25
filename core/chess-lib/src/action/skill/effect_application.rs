//! effect_application.rs：
//! - 負責效果套用邏輯
//! - 包含 Hp/Mp 效果、推擠效果、狀態效果等的實際套用
use crate::*;
use skills_lib::*;
use std::collections::BTreeMap;

use super::CRITICAL_HIT_MULTIPLIER;
use super::hit_resolution::{AttackResult, SaveResult, calc_save_result};
use super::targeting::calc_direction_manhattan;

/// 推擠結果
enum PushResult {
    Destination(Pos), // 可以推到目標位置
    Stopped(String),  // 停止推擠並回傳訊息
}

/// 處理空地效果（無單位的位置）
pub(super) fn apply_effects_to_empty_tile(
    board: &mut Board,
    skill: &Skill,
    caster_pos: Pos,
    pos: Pos,
) -> Vec<String> {
    let mut msgs = vec![];
    for effect in &skill.effects {
        if let Some(msg) = apply_effect_to_pos(
            board,
            effect,
            caster_pos,
            pos,
            AttackResult::NoAttack,
            SaveResult::NoSave,
        ) {
            msgs.push(format!("空地 {pos:?} 受到效果：{msg}"));
        }
    }
    msgs
}

/// 套用所有效果並返回訊息（用於完全命中或普通命中）
pub(super) fn apply_all_effects(
    board: &mut Board,
    skills: &BTreeMap<SkillID, Skill>,
    caster_id: UnitID,
    unit_id: UnitID,
    caster_pos: Pos,
    pos: Pos,
    skill: &Skill,
    attack_result: AttackResult,
) -> Result<Vec<String>, Error> {
    let func = "apply_all_effects";
    let mut msgs = vec![];

    for effect in &skill.effects {
        let save_result = calc_save_result(board, skills, caster_id, unit_id, skill, effect)
            .map_err(|e| Error::Wrap {
                func,
                source: Box::new(e),
            })?;

        if let Some(msg) =
            apply_effect_to_pos(board, effect, caster_pos, pos, attack_result, save_result)
        {
            msgs.push(msg);
        }
    }

    Ok(msgs)
}

/// 套用效果（格擋時 Hp 效果減傷，其他效果正常套用）
pub(super) fn apply_effects_with_block(
    board: &mut Board,
    skills: &BTreeMap<SkillID, Skill>,
    caster_id: UnitID,
    unit_id: UnitID,
    caster_pos: Pos,
    pos: Pos,
    skill: &Skill,
    attack_result: AttackResult,
    block_reduction: i32,
) -> Result<Vec<String>, Error> {
    let func = "apply_effects_with_block";
    let mut results = vec![];

    for effect in &skill.effects {
        match effect {
            Effect::Hp {
                value,
                target_type,
                shape,
            } => {
                let block_percentage = (block_reduction as f32) / 100.0;
                let damage_reduction = (value.abs() as f32 * block_percentage).ceil() as i32;
                let final_value = *value + damage_reduction; // value 是負值

                // 建立減傷後的 effect
                let reduced_effect = Effect::Hp {
                    value: final_value,
                    target_type: target_type.clone(),
                    shape: shape.clone(),
                };

                let save_result =
                    calc_save_result(board, skills, caster_id, unit_id, skill, &reduced_effect)
                        .map_err(|e| Error::Wrap {
                            func,
                            source: Box::new(e),
                        })?;

                if let Some(msg) = apply_effect_to_pos(
                    board,
                    &reduced_effect,
                    caster_pos,
                    pos,
                    attack_result,
                    save_result,
                ) {
                    results.push(format!(
                        "減傷 {damage_reduction} ({block_reduction}%)：{msg}"
                    ));
                }
            }
            _ => {
                // 其他效果不受格擋影響，直接套用
                let save_result = calc_save_result(
                    board, skills, caster_id, unit_id, skill, effect,
                )
                .map_err(|e| Error::Wrap {
                    func,
                    source: Box::new(e),
                })?;

                if let Some(msg) =
                    apply_effect_to_pos(board, effect, caster_pos, pos, attack_result, save_result)
                {
                    results.push(format!("但效果不受影響：{msg}"));
                }
            }
        }
    }

    Ok(results)
}

/// 判斷推擠目的地（處理 Cliff 越過邏輯）
fn determine_push_destination(
    board: &Board,
    next_pos: Pos,
    step: (isize, isize),
    unit_type: &str,
) -> PushResult {
    let tile = match board.get_tile(next_pos) {
        Some(t) => t,
        None => {
            return PushResult::Stopped(format!("單位 {} 被推到邊界並停止", unit_type));
        }
    };

    match &tile.object {
        Some(Object::Cliff { orientation }) => {
            // 檢查推擠方向是否與 Cliff 方向一致
            let direction_matches = match orientation {
                Orientation::Up => step.1 < 0,    // 往上推
                Orientation::Down => step.1 > 0,  // 往下推
                Orientation::Left => step.0 < 0,  // 往左推
                Orientation::Right => step.0 > 0, // 往右推
            };

            // 方向不一致，無法越過 Cliff
            if !direction_matches {
                return PushResult::Stopped(format!("單位 {} 被推到懸崖並停止", unit_type));
            }

            // 方向一致，越過 Cliff 到下一格
            let beyond_x = next_pos.x as isize + step.0;
            let beyond_y = next_pos.y as isize + step.1;

            // 檢查左上邊界
            if beyond_x < 0 || beyond_y < 0 {
                return PushResult::Stopped(format!("單位 {} 越過懸崖後會到達邊界", unit_type));
            }

            let beyond_pos = Pos {
                x: beyond_x as usize,
                y: beyond_y as usize,
            };

            // 檢查右下邊界（用 get_tile 判斷是否超出棋盤）
            if board.get_tile(beyond_pos).is_none() {
                return PushResult::Stopped(format!("單位 {} 越過懸崖後會到達邊界", unit_type));
            }

            PushResult::Destination(beyond_pos)
        }
        Some(Object::Pit) => PushResult::Destination(next_pos),
        _ => {
            if !tile.object.as_ref().map_or(true, |obj| obj.is_passable()) {
                PushResult::Stopped(format!("單位 {} 被推到障礙物並停止", unit_type))
            } else {
                PushResult::Destination(next_pos)
            }
        }
    }
}

/// 推擠效果的輔助函數
fn apply_shove_effect(
    board: &mut Board,
    caster_pos: Pos,
    mut target_pos: Pos,
    distance: &usize,
) -> Option<String> {
    // 只有在格子上有單位時才處理
    let unit_id = board.pos_to_unit(target_pos)?;
    let unit = board.units.get(&unit_id)?;
    let unit_type = unit.unit_template_type.clone();

    // 計算推擠方向
    let step = calc_direction_manhattan(caster_pos, target_pos);
    let mut pushed = 0usize;

    for _ in 0..*distance {
        let (next_x, next_y) = (
            target_pos.x as isize + step.0,
            target_pos.y as isize + step.1,
        );

        if next_x < 0 || next_y < 0 {
            return Some(format!("單位 {} 被推到邊界並停止", unit_type));
        }

        let next_pos = Pos {
            x: next_x as usize,
            y: next_y as usize,
        };

        // 決定最終位置
        let final_pos = match determine_push_destination(board, next_pos, step, &unit_type) {
            PushResult::Destination(pos) => pos,
            PushResult::Stopped(msg) => return Some(msg),
        };

        // 檢查最終位置是否有其他單位
        if let Some(other_id) = board.pos_to_unit(final_pos) {
            let other_type = board
                .units
                .get(&other_id)
                .map(|u| u.unit_template_type.clone())
                .unwrap_or_default();
            return Some(format!(
                "單位 {} 與 單位 {} 相撞並停止",
                unit_type, other_type
            ));
        }

        // 執行移動
        if let Err(e) = board.unit_map.move_unit(unit_id, target_pos, final_pos) {
            // 無法移動（不應發生）
            return Some(format!(
                "單位 {} 無法被移動並停止 (err: {:?})",
                unit_type, e
            ));
        }

        target_pos = final_pos;
        pushed += 1;

        // 檢查掉落
        if let Some(tile) = board.get_tile(final_pos) {
            if matches!(tile.object, Some(Object::Pit)) {
                // 單位掉落坑洞，立即死亡
                if let Some(unit) = board.units.get_mut(&unit_id) {
                    unit.hp = 0;
                }
                return Some(format!("單位 {} 被推入坑洞並墜落死亡！", unit_type));
            }
        }
    }

    Some(format!(
        "單位 {} 被推擠了 {} 格 到 ({}, {})",
        unit_type, pushed, target_pos.x, target_pos.y
    ))
}

/// 將單一效果套用到指定座標（單一 entry-point，方便後續擴充/重構）
pub(super) fn apply_effect_to_pos(
    board: &mut Board,
    effect: &Effect,
    caster_pos: Pos,
    target_pos: Pos,
    attack_result: AttackResult,
    save_result: SaveResult,
) -> Option<String> {
    // 根據攻擊結果計算 multiplier
    let multiplier = match attack_result {
        AttackResult::NoAttack => 1,
        AttackResult::Normal => 1,
        AttackResult::Critical => CRITICAL_HIT_MULTIPLIER,
    };

    match effect {
        Effect::Hp { value, .. } => {
            let unit_id = board.pos_to_unit(target_pos)?;
            let unit = board.units.get_mut(&unit_id)?;
            let old_hp = unit.hp;

            // 套用倍率（僅對 Hp 效果）
            let modified_value = *value * multiplier;
            unit.hp += modified_value;

            // HP 上限限制
            if unit.hp > unit.max_hp {
                unit.hp = unit.max_hp;
            }

            let new_hp = unit.hp;
            Some(format!(
                "單位 {} HP: {old_hp} → {new_hp}",
                &unit.unit_template_type,
            ))
        }
        Effect::Mp { value, .. } => {
            let unit_id = board.pos_to_unit(target_pos)?;
            let unit = board.units.get_mut(&unit_id)?;
            let old_mp = unit.mp;

            unit.mp += value;

            // MP 上限限制
            if unit.mp > unit.max_mp {
                unit.mp = unit.max_mp;
            }

            // MP 下限限制
            if unit.mp < 0 {
                unit.mp = 0;
            }

            let new_mp = unit.mp;
            Some(format!(
                "單位 {} MP: {old_mp} → {new_mp}",
                &unit.unit_template_type,
            ))
        }
        Effect::MaxHp {
            duration, value, ..
        } => Some(format!("[未實作] MaxHp {value}, 持續 {duration} 回合",)),
        Effect::MaxMp {
            duration, value, ..
        } => Some(format!("[未實作] MaxMp {value}, 持續 {duration} 回合",)),
        Effect::Initiative {
            duration, value, ..
        } => Some(format!("[未實作] Initiative {value}, 持續 {duration} 回合",)),
        Effect::Accuracy {
            value, duration, ..
        } => Some(format!(
            "[未實作] Accuracy 效果 +{value}, 持續 {duration} 回合"
        )),
        Effect::Evasion {
            value, duration, ..
        } => Some(format!(
            "[未實作] Evasion 效果 +{value}%, 持續 {duration} 回合"
        )),
        Effect::Block {
            value, duration, ..
        } => Some(format!(
            "[未實作] Block 效果 +{value}%, 持續 {duration} 回合"
        )),
        Effect::BlockReduction {
            value, duration, ..
        } => Some(format!(
            "[未實作] BlockReduction 效果 +{value}%, 持續 {duration} 回合"
        )),
        Effect::MovePoints {
            value, duration, ..
        } => Some(format!("[未實作] 單位移動 {value}, 持續 {duration} 回合")),
        Effect::HitAndRun { .. } => Some(format!("[未實作] 打帶跑")),
        Effect::Shove { distance, .. } => {
            apply_shove_effect(board, caster_pos, target_pos, distance)
        }
        Effect::Potency {
            value, duration, ..
        } => Some(format!(
            "[未實作] Potency 效果 +{value}, 持續 {duration} 回合"
        )),
        Effect::Resistance {
            value,
            duration,
            save_type,
            ..
        } => Some(format!(
            "[未實作] Resistance 效果（{:?}）+{value}, 持續 {duration} 回合",
            save_type
        )),
        Effect::Burn { duration, .. } | Effect::Silence { duration, .. } => {
            match save_result {
                SaveResult::Success => {
                    // 豁免成功，抵抗狀態
                    Some(format!("豁免成功！抵抗了 {:?} 效果", effect))
                }
                SaveResult::Failure => {
                    // 豁免失敗，施加狀態
                    let target_unit_id = board.pos_to_unit(target_pos)?;
                    let target = board.units.get_mut(&target_unit_id)?;
                    target.status_effects.push(effect.clone());

                    Some(format!("豁免失敗！{:?} 效果持續 {} 回合", effect, duration))
                }
                SaveResult::NoSave => {
                    // 不應該發生（Burn/Silence 一定需要豁免）
                    Some(format!("[錯誤] {:?} 效果需要豁免判定", effect))
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::{BTreeSet, HashMap};

    fn prepare_test_board(
        pos: Pos,
        extra_unit_pos: Option<Vec<Pos>>,
    ) -> (Board, UnitID, BTreeMap<SkillID, Skill>) {
        let data = include_str!("../../../tests/unit.json");
        let v: serde_json::Value = serde_json::from_str(data).unwrap();
        let template: UnitTemplate = serde_json::from_value(v["UnitTemplate"].clone()).unwrap();
        let marker: UnitMarker = serde_json::from_value(v["UnitMarker"].clone()).unwrap();
        let team: Team = serde_json::from_value(v["Team"].clone()).unwrap();
        let teams = HashMap::from([(team.id.clone(), team.clone())]);
        let skills = {
            let slash_data = include_str!("../../../tests/skill_slash.json");
            let slash_skill: Skill = serde_json::from_str(slash_data).unwrap();
            let shoot_data = include_str!("../../../tests/skill_shoot.json");
            let shoot_skill: Skill = serde_json::from_str(shoot_data).unwrap();
            let splash_data = include_str!("../../../tests/skill_splash.json");
            let splash_skill: Skill = serde_json::from_str(splash_data).unwrap();
            BTreeMap::from([
                ("shoot".to_string(), shoot_skill),
                ("slash".to_string(), slash_skill),
                ("splash".to_string(), splash_skill),
            ])
        };
        let template = {
            let mut template = template;
            template.skills = skills.iter().map(|(id, _)| id.clone()).collect();
            template
        };
        let unit = Unit::from_template(&marker, &template, &skills).unwrap();
        let unit_id = unit.id;

        let mut unit_map = UnitMap::default();
        unit_map.insert(unit_id, pos);
        let mut units = HashMap::from([(unit_id, unit)]);

        if let Some(pos_list) = extra_unit_pos {
            let mut next_id = unit_id;
            for p in pos_list {
                next_id += 1;
                let extra_template = template.clone();
                let mut extra_unit =
                    Unit::from_template(&marker, &extra_template, &skills).unwrap();
                extra_unit.id = next_id;
                unit_map.insert(extra_unit.id, p);
                units.insert(extra_unit.id, extra_unit);
            }
        }

        let board = Board {
            tiles: vec![vec![Tile::default(); 10]; 10],
            teams,
            unit_map,
            units,
        };
        (board, unit_id, skills)
    }

    #[test]
    fn test_apply_effect_hp() {
        let (mut board, _unit_id, _skills) =
            prepare_test_board(Pos { x: 1, y: 1 }, Some(vec![Pos { x: 1, y: 2 }]));
        let target_pos = Pos { x: 1, y: 2 };
        let target_unit_id = board.pos_to_unit(target_pos).unwrap();

        let effect = Effect::Hp {
            target_type: TargetType::Enemy,
            shape: Shape::Point,
            value: -20,
        };

        let orig_hp = board.units.get(&target_unit_id).unwrap().hp;

        let msg = apply_effect_to_pos(
            &mut board,
            &effect,
            Pos { x: 1, y: 1 },
            target_pos,
            AttackResult::Normal,
            SaveResult::NoSave,
        );

        assert!(msg.is_some());
        let new_hp = board.units.get(&target_unit_id).unwrap().hp;
        assert_eq!(new_hp, orig_hp - 20);
    }

    #[test]
    fn test_apply_effect_hp_critical() {
        let (mut board, _unit_id, _skills) =
            prepare_test_board(Pos { x: 1, y: 1 }, Some(vec![Pos { x: 1, y: 2 }]));
        let target_pos = Pos { x: 1, y: 2 };
        let target_unit_id = board.pos_to_unit(target_pos).unwrap();

        let effect = Effect::Hp {
            target_type: TargetType::Enemy,
            shape: Shape::Point,
            value: -20,
        };

        let orig_hp = board.units.get(&target_unit_id).unwrap().hp;

        let msg = apply_effect_to_pos(
            &mut board,
            &effect,
            Pos { x: 1, y: 1 },
            target_pos,
            AttackResult::Critical,
            SaveResult::NoSave,
        );

        assert!(msg.is_some());
        let new_hp = board.units.get(&target_unit_id).unwrap().hp;
        // 爆擊應該造成 2 倍傷害
        assert_eq!(new_hp, orig_hp - 40);
    }

    #[test]
    fn test_apply_effect_mp() {
        let (mut board, _unit_id, _skills) =
            prepare_test_board(Pos { x: 1, y: 1 }, Some(vec![Pos { x: 1, y: 2 }]));
        let target_pos = Pos { x: 1, y: 2 };
        let target_unit_id = board.pos_to_unit(target_pos).unwrap();

        // 設置初始 MP
        board.units.get_mut(&target_unit_id).unwrap().mp = 50;
        board.units.get_mut(&target_unit_id).unwrap().max_mp = 100;

        let effect = Effect::Mp {
            target_type: TargetType::Enemy,
            shape: Shape::Point,
            value: 30,
        };

        let msg = apply_effect_to_pos(
            &mut board,
            &effect,
            Pos { x: 1, y: 1 },
            target_pos,
            AttackResult::Normal,
            SaveResult::NoSave,
        );

        assert!(msg.is_some());
        assert!(msg.unwrap().contains("MP: 50 → 80"));
        assert_eq!(board.units.get(&target_unit_id).unwrap().mp, 80);
    }

    #[test]
    fn test_apply_effect_mp_overflow() {
        let (mut board, _unit_id, _skills) =
            prepare_test_board(Pos { x: 1, y: 1 }, Some(vec![Pos { x: 1, y: 2 }]));
        let target_pos = Pos { x: 1, y: 2 };
        let target_unit_id = board.pos_to_unit(target_pos).unwrap();

        board.units.get_mut(&target_unit_id).unwrap().mp = 90;
        board.units.get_mut(&target_unit_id).unwrap().max_mp = 100;

        let effect = Effect::Mp {
            target_type: TargetType::Enemy,
            shape: Shape::Point,
            value: 50,
        };

        let msg = apply_effect_to_pos(
            &mut board,
            &effect,
            Pos { x: 1, y: 1 },
            target_pos,
            AttackResult::Normal,
            SaveResult::NoSave,
        );

        assert!(msg.is_some());
        assert!(msg.unwrap().contains("MP: 90 → 100"));
        assert_eq!(board.units.get(&target_unit_id).unwrap().mp, 100);
    }

    #[test]
    fn test_apply_effect_mp_underflow() {
        let (mut board, _unit_id, _skills) =
            prepare_test_board(Pos { x: 1, y: 1 }, Some(vec![Pos { x: 1, y: 2 }]));
        let target_pos = Pos { x: 1, y: 2 };
        let target_unit_id = board.pos_to_unit(target_pos).unwrap();

        board.units.get_mut(&target_unit_id).unwrap().mp = 20;
        board.units.get_mut(&target_unit_id).unwrap().max_mp = 100;

        let effect = Effect::Mp {
            target_type: TargetType::Enemy,
            shape: Shape::Point,
            value: -50,
        };

        let msg = apply_effect_to_pos(
            &mut board,
            &effect,
            Pos { x: 1, y: 1 },
            target_pos,
            AttackResult::Normal,
            SaveResult::NoSave,
        );

        assert!(msg.is_some());
        assert!(msg.unwrap().contains("MP: 20 → 0"));
        assert_eq!(board.units.get(&target_unit_id).unwrap().mp, 0);
    }

    #[test]
    fn test_shove_basic() {
        let (mut board, _unit_id, _skills) =
            prepare_test_board(Pos { x: 1, y: 1 }, Some(vec![Pos { x: 1, y: 2 }]));
        let target_pos = Pos { x: 1, y: 2 };
        let target_unit_id = board.pos_to_unit(target_pos).unwrap();

        let effect = Effect::Shove {
            target_type: TargetType::Enemy,
            shape: Shape::Point,
            distance: 1,
        };

        let msg = apply_effect_to_pos(
            &mut board,
            &effect,
            Pos { x: 1, y: 1 },
            target_pos,
            AttackResult::Normal,
            SaveResult::NoSave,
        );

        assert!(msg.is_some());
        assert!(msg.unwrap().contains("被推擠了 1 格 到 (1, 3)"));

        let new_pos = board.unit_to_pos(target_unit_id).unwrap();
        assert_eq!(new_pos, Pos { x: 1, y: 3 });
    }

    #[test]
    fn test_shove_into_pit() {
        let (mut board, _unit_id, _skills) =
            prepare_test_board(Pos { x: 1, y: 1 }, Some(vec![Pos { x: 1, y: 2 }]));
        let target_pos = Pos { x: 1, y: 2 };
        let target_unit_id = board.pos_to_unit(target_pos).unwrap();

        // 在 (1,3) 放置 Pit
        board.tiles[3][1].object = Some(Object::Pit);

        let effect = Effect::Shove {
            target_type: TargetType::Enemy,
            shape: Shape::Point,
            distance: 1,
        };

        let msg = apply_effect_to_pos(
            &mut board,
            &effect,
            Pos { x: 1, y: 1 },
            target_pos,
            AttackResult::Normal,
            SaveResult::NoSave,
        );

        assert!(msg.is_some());
        assert!(msg.unwrap().contains("被推入坑洞並墜落死亡"));

        // 檢查單位 HP = 0
        assert_eq!(board.units.get(&target_unit_id).unwrap().hp, 0);

        // 檢查單位位置在 Pit 上
        let new_pos = board.unit_to_pos(target_unit_id).unwrap();
        assert_eq!(new_pos, Pos { x: 1, y: 3 });
    }

    #[test]
    fn test_shove_over_cliff_into_pit() {
        let (mut board, _unit_id, _skills) =
            prepare_test_board(Pos { x: 1, y: 1 }, Some(vec![Pos { x: 1, y: 2 }]));
        let target_pos = Pos { x: 1, y: 2 };
        let target_unit_id = board.pos_to_unit(target_pos).unwrap();

        // 在 (1,3) 放置向下的 Cliff
        board.tiles[3][1].object = Some(Object::Cliff {
            orientation: Orientation::Down,
        });

        // 在 (1,4) 放置 Pit
        board.tiles[4][1].object = Some(Object::Pit);

        let effect = Effect::Shove {
            target_type: TargetType::Enemy,
            shape: Shape::Point,
            distance: 1,
        };

        let msg = apply_effect_to_pos(
            &mut board,
            &effect,
            Pos { x: 1, y: 1 },
            target_pos,
            AttackResult::Normal,
            SaveResult::NoSave,
        );

        assert!(msg.is_some());
        assert!(msg.unwrap().contains("被推入坑洞並墜落死亡"));

        // 單位應該在 (1,4)
        let new_pos = board.unit_to_pos(target_unit_id).unwrap();
        assert_eq!(new_pos, Pos { x: 1, y: 4 });

        // HP = 0
        assert_eq!(board.units.get(&target_unit_id).unwrap().hp, 0);
    }

    #[test]
    fn test_shove_cliff_wrong_direction() {
        let (mut board, _unit_id, _skills) =
            prepare_test_board(Pos { x: 1, y: 1 }, Some(vec![Pos { x: 1, y: 2 }]));
        let target_pos = Pos { x: 1, y: 2 };
        let target_unit_id = board.pos_to_unit(target_pos).unwrap();

        // 在 (1,3) 放置向上的 Cliff（但推擠方向是向下）
        board.tiles[3][1].object = Some(Object::Cliff {
            orientation: Orientation::Up,
        });

        let effect = Effect::Shove {
            target_type: TargetType::Enemy,
            shape: Shape::Point,
            distance: 1,
        };

        let msg = apply_effect_to_pos(
            &mut board,
            &effect,
            Pos { x: 1, y: 1 },
            target_pos,
            AttackResult::Normal,
            SaveResult::NoSave,
        );

        assert!(msg.is_some());
        assert!(msg.unwrap().contains("被推到懸崖並停止"));

        // 單位應該還在 (1,2)（沒有移動）
        let new_pos = board.unit_to_pos(target_unit_id).unwrap();
        assert_eq!(new_pos, Pos { x: 1, y: 2 });
    }

    #[test]
    fn test_shove_over_cliff_collision() {
        let (mut board, _unit_id, _skills) = prepare_test_board(
            Pos { x: 1, y: 1 },
            Some(vec![Pos { x: 1, y: 2 }, Pos { x: 1, y: 4 }]),
        );
        let target_pos = Pos { x: 1, y: 2 };
        let target_unit_id = board.pos_to_unit(target_pos).unwrap();

        // 在 (1,3) 放置向下的 Cliff
        board.tiles[3][1].object = Some(Object::Cliff {
            orientation: Orientation::Down,
        });

        let effect = Effect::Shove {
            target_type: TargetType::Enemy,
            shape: Shape::Point,
            distance: 1,
        };

        let msg = apply_effect_to_pos(
            &mut board,
            &effect,
            Pos { x: 1, y: 1 },
            target_pos,
            AttackResult::Normal,
            SaveResult::NoSave,
        );

        assert!(msg.is_some());
        assert!(msg.unwrap().contains("相撞並停止"));

        // 單位應該還在原位 (1,2)
        let new_pos = board.unit_to_pos(target_unit_id).unwrap();
        assert_eq!(new_pos, Pos { x: 1, y: 2 });
    }

    #[test]
    fn test_shove_beyond_boundary() {
        let (mut board, _unit_id, _skills) =
            prepare_test_board(Pos { x: 1, y: 8 }, Some(vec![Pos { x: 1, y: 9 }]));
        let target_pos = Pos { x: 1, y: 9 };
        let target_unit_id = board.pos_to_unit(target_pos).unwrap();

        let effect = Effect::Shove {
            target_type: TargetType::Enemy,
            shape: Shape::Point,
            distance: 1,
        };

        let msg = apply_effect_to_pos(
            &mut board,
            &effect,
            Pos { x: 1, y: 8 },
            target_pos,
            AttackResult::Normal,
            SaveResult::NoSave,
        );

        assert!(msg.is_some());
        assert!(msg.unwrap().contains("被推到邊界並停止"));

        // 單位應該還在 (1,9)
        let new_pos = board.unit_to_pos(target_unit_id).unwrap();
        assert_eq!(new_pos, Pos { x: 1, y: 9 });
    }
}

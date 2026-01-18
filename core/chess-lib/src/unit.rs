//! unit.rs：
//! - 定義單位（Unit）、單位模板（UnitTemplate）等資料結構，僅負責靜態資料與屬性，不含戰鬥邏輯。
//! - 所有單位屬性衍生值（如先攻 initiative、移動力等）之計算，應實作於 unit.rs 內部方法或輔助函式。
//! - 不負責戰鬥流程與判定（如命中、閃避、格擋、傷害計算等）。
use crate::*;
use serde::{Deserialize, Serialize};
use skills_lib::*;
use std::collections::{BTreeMap, BTreeSet};

const MAX_INITIATIVE_RANDOM: i32 = 6;

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Team {
    pub id: TeamID,
    pub color: RGB,
}

#[derive(Debug, Default, Clone, Deserialize, Serialize)]
pub struct UnitTemplate {
    pub name: UnitTemplateType,
    pub skills: BTreeSet<String>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct UnitMarker {
    pub id: UnitID,
    pub unit_template_type: UnitTemplateType,
    pub team: TeamID,
    pub pos: Pos,
}

#[derive(Debug)]
pub struct Unit {
    pub id: UnitID,
    pub unit_template_type: UnitTemplateType,
    pub team: TeamID,
    pub moved: MovementCost,
    pub move_points: MovementCost,
    pub has_cast_skill_this_turn: bool,
    pub reactions_used_this_turn: ReactionCount,
    pub max_reactions_per_turn: ReactionCount,
    pub hp: i32,
    pub max_hp: i32,
    pub mp: i32,
    pub max_mp: i32,
    pub skills: BTreeSet<String>,
    pub status_effects: Vec<Effect>,
}

impl Unit {
    pub fn from_template(
        marker: &UnitMarker,
        template: &UnitTemplate,
        skills: &BTreeMap<SkillID, Skill>,
    ) -> Result<Self, Error> {
        let func = "Unit::from_template";

        // 驗證所有技能都存在
        for skill_id in &template.skills {
            if !skills.contains_key(skill_id) {
                return Err(Error::SkillNotFound {
                    func,
                    skill_id: skill_id.clone(),
                });
            }
        }

        // 使用統一函數計算衍生值
        let (max_hp, max_mp, move_points, max_reactions_per_turn) =
            calculate_derived_stats(&template.skills, skills).wrap_context(func)?;

        Ok(Unit {
            id: marker.id,
            unit_template_type: marker.unit_template_type.clone(),
            team: marker.team.clone(),
            moved: 0,
            move_points,
            has_cast_skill_this_turn: false,
            reactions_used_this_turn: 0,
            max_reactions_per_turn,
            hp: max_hp,
            max_hp,
            mp: max_mp,
            max_mp,
            skills: template.skills.clone(),
            status_effects: Vec::new(),
        })
    }

    /// 使用當前 unit.skills 與技能表重算衍生屬性
    pub fn recalc_from_skills(&mut self, skills: &BTreeMap<SkillID, Skill>) -> Result<(), Error> {
        let func = "Unit::recalc_from_skills";

        // 使用統一函數計算衍生值
        let (max_hp, max_mp, move_points, max_reactions_per_turn) =
            calculate_derived_stats(&self.skills, skills).wrap_context(func)?;

        self.max_hp = max_hp;
        self.hp = max_hp; // 重置 HP 為新的最大值
        self.max_mp = max_mp;
        self.mp = max_mp; // 重置 MP 為新的最大值
        self.move_points = move_points;
        self.max_reactions_per_turn = max_reactions_per_turn;

        Ok(())
    }
}

/// 簡單累加型態的 skills_to_xxx 函式生成 macro
macro_rules! impl_simple_skills_stat {
    ($(#[$meta:meta])* $fn_name:ident, $effect_variant:ident) => {
        $(#[$meta])*
        pub fn $fn_name(
            skill_ids: impl Iterator<Item = impl AsRef<str>>,
            skills: &BTreeMap<SkillID, Skill>,
        ) -> Result<i32, Error> {
            let func = stringify!($fn_name);
            aggregate_skill_effect(skill_ids, skills, |effect| {
                if let Effect::$effect_variant { value, .. } = effect {
                    *value
                } else {
                    0
                }
            })
            .wrap_context(func)
        }
    };
}

/// 累加型態的 skills_to_xxx 函式生成 macro（保證非負，可指定返回類型）
macro_rules! impl_nonnegative_skills_stat {
    ($(#[$meta:meta])* $fn_name:ident, $effect_variant:ident, $return_type:ty) => {
        $(#[$meta])*
        pub fn $fn_name(
            skill_ids: impl Iterator<Item = impl AsRef<str>>,
            skills: &BTreeMap<SkillID, Skill>,
        ) -> Result<$return_type, Error> {
            let func = stringify!($fn_name);
            let total = aggregate_skill_effect(skill_ids, skills, |effect| {
                if let Effect::$effect_variant { value, .. } = effect {
                    *value
                } else {
                    0
                }
            })
            .wrap_context(func)?;
            let nonnegative = if total < 0 { 0 } else { total };
            nonnegative.try_into().map_err(|_| Error::InvalidImplementation {
                func,
                detail: format!("無法轉換為 {}", stringify!($return_type)),
            })
        }
    };
}

/// 通用的 skill effect 累加 helper function
/// matcher 閉包對每個 effect 判斷是否匹配，匹配時返回 value，不匹配時返回 0
fn aggregate_skill_effect<F>(
    skill_ids: impl Iterator<Item = impl AsRef<str>>,
    skills: &BTreeMap<SkillID, Skill>,
    matcher: F,
) -> Result<i32, Error>
where
    F: Fn(&Effect) -> i32,
{
    let func = "aggregate_skill_effect";
    let sum: i32 = skill_ids
        .map(|skill_id| {
            skills
                .get(skill_id.as_ref())
                .ok_or_else(|| Error::SkillNotFound {
                    func,
                    skill_id: skill_id.as_ref().to_string(),
                })
        })
        .collect::<Result<Vec<_>, _>>()? // 先收集所有 Result，遇錯就返回
        .iter()
        .flat_map(|skill| skill.effects.iter())
        .map(matcher)
        .sum();
    Ok(sum)
}

/// 計算單位本回合的 initiative 值
/// - 1D 隨機
/// - 技能 initiative 加總（i32）
/// - 未來可擴充 buff/debuff、裝備等
pub fn calc_initiative(
    rng: &mut impl rand::Rng,
    skill_ids: impl Iterator<Item = impl AsRef<str>>,
    skills: &BTreeMap<SkillID, Skill>,
) -> Result<i32, Error> {
    let func = "calc_initiative";
    let roll = rng.random_range(1..=MAX_INITIATIVE_RANDOM);
    let skill_initiative = skills_to_initiative(skill_ids, skills).wrap_context(func)?;
    Ok(roll + skill_initiative)
}

impl_simple_skills_stat!(skills_to_max_hp, MaxHp);
impl_simple_skills_stat!(skills_to_max_mp, MaxMp);
impl_simple_skills_stat!(skills_to_initiative, Initiative);
impl_simple_skills_stat!(skills_to_accuracy, Accuracy);
impl_simple_skills_stat!(skills_to_evasion, Evasion);
impl_simple_skills_stat!(skills_to_block, Block);
impl_simple_skills_stat!(skills_to_block_reduction, BlockReduction);

// 以下統計保證非負（負值會被設為 0）
impl_nonnegative_skills_stat!(skills_to_flanking, Flanking, i32);
impl_nonnegative_skills_stat!(skills_to_move_points, MovePoints, MovementCost);
impl_nonnegative_skills_stat!(skills_to_max_reactions, MaxReactions, ReactionCount);

pub fn skills_to_hit_and_run(
    skill_ids: impl Iterator<Item = impl AsRef<str>>,
    skills: &BTreeMap<SkillID, Skill>,
) -> Result<bool, Error> {
    let func = "skills_to_hit_and_run";

    for skill_id in skill_ids {
        let skill = skills
            .get(skill_id.as_ref())
            .ok_or_else(|| Error::SkillNotFound {
                func,
                skill_id: skill_id.as_ref().to_string(),
            })?;
        if skill
            .effects
            .iter()
            .any(|e| matches!(e, Effect::HitAndRun { .. }))
        {
            return Ok(true);
        }
    }
    Ok(false)
}

/// 計算單位對特定 Tag 的施法效力總和
/// 尋找所有 effect 為 Effect::Potency 且 tag 匹配的技能，並加總其 value
pub fn skills_to_potency(
    skill_ids: impl Iterator<Item = impl AsRef<str>>,
    skills: &BTreeMap<SkillID, Skill>,
    target_tag: &Tag,
) -> Result<i32, Error> {
    let func = "skills_to_potency";
    aggregate_skill_effect(skill_ids, skills, |effect| {
        if let Effect::Potency { value, tag, .. } = effect {
            if tag == target_tag {
                return *value;
            }
        }
        0
    })
    .wrap_context(func)
}

/// 計算單位對特定豁免類型的抗性總和
/// 尋找所有 effect 為 Effect::Resistance 且 save_type 匹配的技能，並加總其 value
pub fn skills_to_resistance(
    skill_ids: impl Iterator<Item = impl AsRef<str>>,
    skills: &BTreeMap<SkillID, Skill>,
    target_save_type: &SaveType,
) -> Result<i32, Error> {
    let func = "skills_to_resistance";
    aggregate_skill_effect(skill_ids, skills, |effect| {
        if let Effect::Resistance {
            value, save_type, ..
        } = effect
        {
            if save_type == target_save_type {
                return *value;
            }
        }
        0
    })
    .wrap_context(func)
}

/// 檢查單位的技能中是否有 Sense 效果覆蓋目標位置
pub fn skills_to_sense(
    skill_ids: impl Iterator<Item = impl AsRef<str>>,
    skills: &BTreeMap<SkillID, Skill>,
    distance: usize,
) -> Result<bool, Error> {
    let func = "skills_to_sense";

    for skill_id in skill_ids {
        let skill = skills
            .get(skill_id.as_ref())
            .ok_or_else(|| Error::SkillNotFound {
                func,
                skill_id: skill_id.as_ref().to_string(),
            })?;

        if effects_to_sense(skill.effects.iter(), distance) {
            return Ok(true);
        }
    }

    Ok(false)
}

/// 從效果集合中根據距離計算光照等級（CarriesLight 效果）
/// 如果有多個 CarriesLight 效果，取最亮的光照等級
pub fn effects_to_light_level<'a>(
    effects: impl Iterator<Item = &'a Effect>,
    distance: usize,
) -> LightLevel {
    let mut max_light = LightLevel::Darkness;

    for effect in effects {
        if let Effect::CarriesLight {
            bright_range,
            dim_range,
            ..
        } = effect
        {
            let light_level = if *bright_range > 0 && distance <= *bright_range {
                LightLevel::Bright
            } else if *dim_range > 0 && distance <= *dim_range {
                LightLevel::Dim
            } else {
                LightLevel::Darkness
            };

            max_light = max_light.max(light_level);

            // 早期退出：已經是最亮
            if max_light == LightLevel::Bright {
                return LightLevel::Bright;
            }
        }
    }

    max_light
}

/// 檢查效果集合中是否有 Sense 效果覆蓋目標位置
pub fn effects_to_sense<'a>(
    mut effects: impl Iterator<Item = &'a Effect>,
    distance: usize,
) -> bool {
    effects.any(|effect| {
        if let Effect::Sense { range, .. } = effect {
            distance <= *range
        } else {
            false
        }
    })
}

use inner::*;
mod inner {
    use super::*;

    /// 計算單位的所有衍生屬性
    ///
    /// 此函數提供統一的衍生值計算邏輯，被 from_template 和 recalc_from_skills 共同使用
    pub fn calculate_derived_stats(
        skill_ids: &BTreeSet<String>,
        skills: &BTreeMap<SkillID, Skill>,
    ) -> Result<(i32, i32, MovementCost, ReactionCount), Error> {
        let func = "calculate_derived_stats";
        let max_hp = skills_to_max_hp(skill_ids.iter(), skills).wrap_context(func)?;

        let max_mp = skills_to_max_mp(skill_ids.iter(), skills).wrap_context(func)?;

        let move_points = skills_to_move_points(skill_ids.iter(), skills).wrap_context(func)?;

        let max_reactions_per_turn =
            skills_to_max_reactions(skill_ids.iter(), skills).wrap_context(func)?;

        Ok((max_hp, max_mp, move_points, max_reactions_per_turn))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json;
    use std::collections::HashMap;

    #[test]
    fn test_deserialize_unit() {
        let data = include_str!("../tests/unit.json");
        let v: serde_json::Value = serde_json::from_str(data).unwrap();
        // 從 skill_sprint.json 載入 sprint 技能，並載入 max_hp / max_mp 技能以覆蓋對應欄位
        let sprint_data = include_str!("../tests/skill_sprint.json");
        let sprint_skill: Skill = serde_json::from_str(sprint_data).unwrap();
        let max_hp_data = include_str!("../tests/skill_max_hp.json");
        let max_hp_skill: Skill = serde_json::from_str(max_hp_data).unwrap();
        let max_mp_data = include_str!("../tests/skill_max_mp.json");
        let max_mp_skill: Skill = serde_json::from_str(max_mp_data).unwrap();

        // 測試 Team
        let team: Team = serde_json::from_value(v["Team"].clone()).unwrap();
        assert_eq!(team.id, "t1");
        assert_eq!(team.color, (255, 0, 0));

        // 測試 UnitTemplate
        let template: UnitTemplate = serde_json::from_value(v["UnitTemplate"].clone()).unwrap();
        assert_eq!(template.name, "knight");
        assert_eq!(template.skills.len(), 2);
        assert!(template.skills.contains("sprint"));
        assert!(template.skills.contains("slash"));

        // 測試 UnitMarker
        let marker: UnitMarker = serde_json::from_value(v["UnitMarker"].clone()).unwrap();
        assert_eq!(marker.id, 42);
        assert_eq!(marker.unit_template_type, "knight");
        assert_eq!(marker.team, "t1");
        assert_eq!(marker.pos, Pos { x: 0, y: 0 });

        // 測試 Unit::from_template
        let skills_map = BTreeMap::from([
            ("sprint".to_string(), sprint_skill),
            ("max_hp".to_string(), max_hp_skill),
            ("max_mp".to_string(), max_mp_skill),
        ]);

        fn with_skills(mut template: UnitTemplate, skills: &[&str]) -> UnitTemplate {
            template.skills = skills.iter().map(|s| s.to_string()).collect();
            template
        }
        let test_data = [
            (
                vec![],
                HashMap::from([("move_points", 0), ("max_hp", 0), ("max_mp", 0)]),
            ),
            (
                vec!["sprint"],
                HashMap::from([("move_points", 30), ("max_hp", 0), ("max_mp", 0)]),
            ),
            (
                vec!["max_hp"],
                HashMap::from([("move_points", 0), ("max_hp", 10), ("max_mp", 0)]),
            ),
            (
                vec!["max_mp"],
                HashMap::from([("move_points", 0), ("max_hp", 0), ("max_mp", 5)]),
            ),
            (
                vec!["sprint", "max_hp", "max_mp"],
                HashMap::from([("move_points", 30), ("max_hp", 10), ("max_mp", 5)]),
            ),
        ];

        for (skills, expect) in test_data {
            let template = with_skills(template.clone(), &skills);
            let unit = Unit::from_template(&marker, &template, &skills_map).unwrap();
            assert_eq!(unit.id, marker.id);
            assert_eq!(unit.unit_template_type, marker.unit_template_type);
            assert_eq!(unit.team, marker.team);
            assert_eq!(unit.moved, 0);
            assert_eq!(unit.move_points, expect["move_points"] as usize);
            assert_eq!(unit.has_cast_skill_this_turn, false);
            assert_eq!(unit.reactions_used_this_turn, 0);
            assert_eq!(unit.max_reactions_per_turn, 0);
            assert_eq!(unit.hp, expect["max_hp"]);
            assert_eq!(unit.max_hp, expect["max_hp"]);
            assert_eq!(unit.mp, expect["max_mp"]);
            assert_eq!(unit.max_mp, expect["max_mp"]);
            assert_eq!(unit.skills.len(), skills.len());
            for skill in skills {
                assert!(unit.skills.contains(skill));
            }
        }
    }

    #[test]
    fn test_unit_template_default() {
        let default = UnitTemplate::default();
        assert_eq!(default.name, "");
        assert!(default.skills.is_empty());
    }

    #[test]
    fn test_unit_struct_fields_extreme() {
        let unit = Unit {
            id: 999,
            unit_template_type: "超級戰士".to_string(),
            team: "t99".to_string(),
            moved: usize::MAX,
            move_points: usize::MAX,
            has_cast_skill_this_turn: true,
            reactions_used_this_turn: usize::MAX,
            max_reactions_per_turn: usize::MAX,
            hp: i32::MIN,
            max_hp: i32::MAX,
            mp: i32::MIN,
            max_mp: i32::MAX,
            skills: ["超級技能".to_string()].iter().cloned().collect(),
            status_effects: Vec::new(),
        };
        assert_eq!(unit.id, 999);
        assert_eq!(unit.unit_template_type, "超級戰士");
        assert_eq!(unit.team, "t99");
        assert_eq!(unit.moved, usize::MAX);
        assert_eq!(unit.move_points, usize::MAX);
        assert!(unit.has_cast_skill_this_turn);
        assert_eq!(unit.reactions_used_this_turn, usize::MAX);
        assert_eq!(unit.max_reactions_per_turn, usize::MAX);
        assert_eq!(unit.hp, i32::MIN);
        assert_eq!(unit.max_hp, i32::MAX);
        assert_eq!(unit.mp, i32::MIN);
        assert_eq!(unit.max_mp, i32::MAX);
        assert!(unit.skills.contains("超級技能"));
    }

    #[test]
    fn test_unit_from_template_skill_not_found() {
        let marker = UnitMarker {
            id: 1,
            unit_template_type: "knight".to_string(),
            team: "t1".to_string(),
            pos: Pos { x: 0, y: 0 },
        };
        let template = UnitTemplate {
            name: "knight".to_string(),
            skills: ["not_exist_skill".to_string()].iter().cloned().collect(),
        };
        let skills_map = BTreeMap::new();
        let result = Unit::from_template(&marker, &template, &skills_map);
        match result {
            Err(Error::SkillNotFound { skill_id, .. }) => assert_eq!(skill_id, "not_exist_skill"),
            _ => panic!("Should return Error::SkillNotFound"),
        }
    }

    #[test]
    fn test_skills_to_initiative() {
        let mut skills = BTreeMap::new();
        // 無技能
        assert_eq!(skills_to_initiative(skills.keys(), &skills).unwrap(), 0);

        // 一個 initiative 技能
        let mut skill1 = Skill::default();
        skill1.effects = vec![Effect::Initiative {
            value: 2,
            target_type: Default::default(),
            shape: Default::default(),
            duration: 0,
        }];
        let key_a = "a".to_string();
        skills.insert(key_a.clone(), skill1);
        assert_eq!(skills_to_initiative(skills.keys(), &skills).unwrap(), 2);

        // 多個 initiative 技能
        let mut skill2 = Skill::default();
        skill2.effects = vec![Effect::Initiative {
            value: 3,
            target_type: Default::default(),
            shape: Default::default(),
            duration: 0,
        }];
        let key_b = "b".to_string();
        skills.insert(key_b.clone(), skill2);
        assert_eq!(skills_to_initiative(skills.keys(), &skills).unwrap(), 5);

        // 非 initiative 類型技能不影響
        let mut skill3 = Skill::default();
        skill3.effects = vec![Effect::MaxHp {
            value: 99,
            target_type: Default::default(),
            shape: Default::default(),
            duration: 0,
        }];
        let key_c = "c".to_string();
        skills.insert(key_c.clone(), skill3);
        assert_eq!(skills_to_initiative(skills.keys(), &skills).unwrap(), 5);
    }

    #[test]
    fn test_calc_initiative() {
        let mut rng = rand::rng();
        let mut skills = BTreeMap::new();
        // 無技能
        let result = calc_initiative(&mut rng, skills.keys(), &skills).unwrap();
        assert!(result >= 1 && result <= MAX_INITIATIVE_RANDOM);

        // 有 initiative 技能
        let mut skill = Skill::default();
        skill.effects = vec![Effect::Initiative {
            value: 3,
            target_type: Default::default(),
            shape: Default::default(),
            duration: 0,
        }];
        let key = "test".to_string();
        skills.insert(key.clone(), skill);
        let result = calc_initiative(&mut rng, skills.keys(), &skills).unwrap();
        assert!(result >= 4 && result <= 9);

        // 有 initiative 技能
        let mut skill = Skill::default();
        skill.effects = vec![
            Effect::Initiative {
                value: 3,
                target_type: Default::default(),
                shape: Default::default(),
                duration: 0,
            },
            Effect::Initiative {
                value: 2,
                target_type: Default::default(),
                shape: Default::default(),
                duration: 0,
            },
        ];
        let key = "test".to_string();
        skills.insert(key.clone(), skill);
        let result = calc_initiative(&mut rng, skills.keys(), &skills).unwrap();
        assert!(result >= 6 && result <= 11);
    }

    #[test]
    fn test_skills_to_max_hp() {
        let mut skills = BTreeMap::new();
        // 無技能
        assert_eq!(skills_to_max_hp(skills.keys(), &skills).unwrap(), 0);

        // 一個 MaxHp 技能
        let mut skill1 = Skill::default();
        skill1.effects = vec![Effect::MaxHp {
            value: 10,
            target_type: Default::default(),
            shape: Default::default(),
            duration: 0,
        }];
        let key_a = "a".to_string();
        skills.insert(key_a.clone(), skill1);
        assert_eq!(skills_to_max_hp(skills.keys(), &skills).unwrap(), 10);

        // 多個 MaxHp 技能
        let mut skill2 = Skill::default();
        skill2.effects = vec![Effect::MaxHp {
            value: 20,
            target_type: Default::default(),
            shape: Default::default(),
            duration: 0,
        }];
        let key_b = "b".to_string();
        skills.insert(key_b.clone(), skill2);
        assert_eq!(skills_to_max_hp(skills.keys(), &skills).unwrap(), 30);

        // 非 MaxHp 類型技能不影響
        let mut skill3 = Skill::default();
        skill3.effects = vec![Effect::Initiative {
            value: 99,
            target_type: Default::default(),
            shape: Default::default(),
            duration: 0,
        }];
        let key_c = "c".to_string();
        skills.insert(key_c.clone(), skill3);
        assert_eq!(skills_to_max_hp(skills.keys(), &skills).unwrap(), 30);
    }

    #[test]
    fn test_skills_to_max_mp() {
        let mut skills = BTreeMap::new();
        // 無技能
        assert_eq!(skills_to_max_mp(skills.keys(), &skills).unwrap(), 0);

        // 一個 MaxMp 技能
        let mut skill1 = Skill::default();
        skill1.effects = vec![Effect::MaxMp {
            value: 5,
            target_type: Default::default(),
            shape: Default::default(),
            duration: 0,
        }];
        let key_a = "a".to_string();
        skills.insert(key_a.clone(), skill1);
        assert_eq!(skills_to_max_mp(skills.keys(), &skills).unwrap(), 5);

        // 多個 MaxMp 技能
        let mut skill2 = Skill::default();
        skill2.effects = vec![Effect::MaxMp {
            value: 10,
            target_type: Default::default(),
            shape: Default::default(),
            duration: 0,
        }];
        let key_b = "b".to_string();
        skills.insert(key_b.clone(), skill2);
        assert_eq!(skills_to_max_mp(skills.keys(), &skills).unwrap(), 15);

        // 非 MaxMp 類型技能不影響
        let mut skill3 = Skill::default();
        skill3.effects = vec![Effect::Initiative {
            value: 99,
            target_type: Default::default(),
            shape: Default::default(),
            duration: 0,
        }];
        let key_c = "c".to_string();
        skills.insert(key_c.clone(), skill3);
        assert_eq!(skills_to_max_mp(skills.keys(), &skills).unwrap(), 15);
    }

    #[test]
    fn test_skills_to_evasion() {
        let mut skills = BTreeMap::new();
        // 無技能
        assert_eq!(skills_to_evasion(skills.keys(), &skills).unwrap(), 0);

        // 一個 evasion 技能
        let mut skill1 = Skill::default();
        skill1.effects = vec![Effect::Evasion {
            value: 2,
            target_type: Default::default(),
            shape: Default::default(),
            duration: 0,
        }];
        let key_a = "a".to_string();
        skills.insert(key_a.clone(), skill1);
        assert_eq!(skills_to_evasion(skills.keys(), &skills).unwrap(), 2);

        // 多個 evasion 技能
        let mut skill2 = Skill::default();
        skill2.effects = vec![Effect::Evasion {
            value: 3,
            target_type: Default::default(),
            shape: Default::default(),
            duration: 0,
        }];
        let key_b = "b".to_string();
        skills.insert(key_b.clone(), skill2);
        assert_eq!(skills_to_evasion(skills.keys(), &skills).unwrap(), 5);

        // 非 evasion 類型技能不影響
        let mut skill3 = Skill::default();
        skill3.effects = vec![Effect::MaxHp {
            value: 99,
            target_type: Default::default(),
            shape: Default::default(),
            duration: 0,
        }];
        let key_c = "c".to_string();
        skills.insert(key_c.clone(), skill3);
        assert_eq!(skills_to_evasion(skills.keys(), &skills).unwrap(), 5);
    }

    #[test]
    fn test_skills_to_block() {
        let mut skills = BTreeMap::new();
        // 無技能
        assert_eq!(skills_to_block(skills.keys(), &skills).unwrap(), 0);

        // 一個 block 技能
        let mut skill1 = Skill::default();
        skill1.effects = vec![Effect::Block {
            value: 2,
            target_type: Default::default(),
            shape: Default::default(),
            duration: 0,
        }];
        let key_a = "a".to_string();
        skills.insert(key_a.clone(), skill1);
        assert_eq!(skills_to_block(skills.keys(), &skills).unwrap(), 2);

        // 多個 block 技能
        let mut skill2 = Skill::default();
        skill2.effects = vec![Effect::Block {
            value: 3,
            target_type: Default::default(),
            shape: Default::default(),
            duration: 0,
        }];
        let key_b = "b".to_string();
        skills.insert(key_b.clone(), skill2);
        assert_eq!(skills_to_block(skills.keys(), &skills).unwrap(), 5);

        // 非 block 類型技能不影響
        let mut skill3 = Skill::default();
        skill3.effects = vec![Effect::MaxHp {
            value: 99,
            target_type: Default::default(),
            shape: Default::default(),
            duration: 0,
        }];
        let key_c = "c".to_string();
        skills.insert(key_c.clone(), skill3);
        assert_eq!(skills_to_block(skills.keys(), &skills).unwrap(), 5);
    }

    #[test]
    fn test_skills_to_block_reduction() {
        let mut skills = BTreeMap::new();
        // 無技能，應返回 0
        assert_eq!(
            skills_to_block_reduction(skills.keys(), &skills).unwrap(),
            0
        );

        // 一個 BlockReduction 技能
        let mut skill1 = Skill::default();
        skill1.effects = vec![Effect::BlockReduction {
            value: 20,
            target_type: Default::default(),
            shape: Default::default(),
            duration: 0,
        }];
        let key_a = "a".to_string();
        skills.insert(key_a.clone(), skill1);
        assert_eq!(
            skills_to_block_reduction(skills.keys(), &skills).unwrap(),
            20
        );

        // 多個 BlockReduction 技能
        let mut skill2 = Skill::default();
        skill2.effects = vec![Effect::BlockReduction {
            value: 30,
            target_type: Default::default(),
            shape: Default::default(),
            duration: 0,
        }];
        let key_b = "b".to_string();
        skills.insert(key_b.clone(), skill2);
        assert_eq!(
            skills_to_block_reduction(skills.keys(), &skills).unwrap(),
            50
        );

        // 非 BlockReduction 類型技能不影響
        let mut skill3 = Skill::default();
        skill3.effects = vec![Effect::MaxHp {
            value: 99,
            target_type: Default::default(),
            shape: Default::default(),
            duration: 0,
        }];
        let key_c = "c".to_string();
        skills.insert(key_c.clone(), skill3);
        assert_eq!(
            skills_to_block_reduction(skills.keys(), &skills).unwrap(),
            50
        );
    }

    #[test]
    fn test_skills_to_move_points_negative() {
        let mut skills = BTreeMap::new();
        // 負數 move_points，應回傳 0
        let mut skill1 = Skill::default();
        skill1.effects = vec![Effect::MovePoints {
            value: -10,
            target_type: Default::default(),
            shape: Default::default(),
            duration: 0,
        }];
        let key_a = "a".to_string();
        skills.insert(key_a.clone(), skill1);
        assert_eq!(skills_to_move_points(skills.keys(), &skills).unwrap(), 0);

        // 正負混合，總和為負，仍回傳 0
        let mut skill2 = Skill::default();
        skill2.effects = vec![Effect::MovePoints {
            value: 5,
            target_type: Default::default(),
            shape: Default::default(),
            duration: 0,
        }];
        let key_b = "b".to_string();
        skills.insert(key_b.clone(), skill2);
        assert_eq!(skills_to_move_points(skills.keys(), &skills).unwrap(), 0);

        // 正負混合，總和為正
        let mut skill3 = Skill::default();
        skill3.effects = vec![Effect::MovePoints {
            value: 20,
            target_type: Default::default(),
            shape: Default::default(),
            duration: 0,
        }];
        let key_c = "c".to_string();
        skills.insert(key_c.clone(), skill3);
        assert_eq!(skills_to_move_points(skills.keys(), &skills).unwrap(), 15);
    }

    #[test]
    fn test_recalc_from_skills_updates_stats() {
        let mut skills = BTreeMap::new();
        let mut s1 = Skill::default();
        s1.effects = vec![Effect::MaxHp {
            value: 20,
            target_type: Default::default(),
            shape: Default::default(),
            duration: 0,
        }];
        let mut s2 = Skill::default();
        s2.effects = vec![Effect::MaxMp {
            value: 5,
            target_type: Default::default(),
            shape: Default::default(),
            duration: 0,
        }];
        let mut s3 = Skill::default();
        s3.effects = vec![Effect::MovePoints {
            value: 7,
            target_type: Default::default(),
            shape: Default::default(),
            duration: 0,
        }];
        skills.insert("a".to_string(), s1);
        skills.insert("b".to_string(), s2);
        skills.insert("c".to_string(), s3);

        let mut unit = Unit {
            id: 1,
            unit_template_type: "t".to_string(),
            team: "team".to_string(),
            moved: 0,
            move_points: 0,
            has_cast_skill_this_turn: false,
            reactions_used_this_turn: 0,
            max_reactions_per_turn: 0,
            hp: 0,
            max_hp: 0,
            mp: 0,
            max_mp: 0,
            skills: ["a".to_string(), "b".to_string(), "c".to_string()]
                .iter()
                .cloned()
                .collect(),
            status_effects: Vec::new(),
        };

        unit.recalc_from_skills(&skills).unwrap();
        assert_eq!(unit.max_hp, 20);
        assert_eq!(unit.hp, 20);
        assert_eq!(unit.max_mp, 5);
        assert_eq!(unit.mp, 5);
        assert_eq!(unit.move_points, 7);
        assert_eq!(unit.max_reactions_per_turn, 0);
    }

    #[test]
    fn test_recalc_from_skills_with_max_reactions() {
        let mut skills = BTreeMap::new();
        let mut s1 = Skill::default();
        s1.effects = vec![Effect::MaxReactions {
            value: 2,
            target_type: Default::default(),
            shape: Default::default(),
            duration: 0,
        }];
        skills.insert("reaction_skill".to_string(), s1);

        let mut unit = Unit {
            id: 1,
            unit_template_type: "t".to_string(),
            team: "team".to_string(),
            moved: 0,
            move_points: 0,
            has_cast_skill_this_turn: false,
            reactions_used_this_turn: 0,
            max_reactions_per_turn: 0,
            hp: 0,
            max_hp: 0,
            mp: 0,
            max_mp: 0,
            skills: ["reaction_skill".to_string()].iter().cloned().collect(),
            status_effects: Vec::new(),
        };

        unit.recalc_from_skills(&skills).unwrap();
        assert_eq!(unit.max_reactions_per_turn, 2);
    }

    #[test]
    fn test_recalc_from_skills_negative_move_points_sets_zero() {
        let mut skills = BTreeMap::new();
        let mut s1 = Skill::default();
        s1.effects = vec![Effect::MovePoints {
            value: -20,
            target_type: Default::default(),
            shape: Default::default(),
            duration: 0,
        }];
        skills.insert("neg".to_string(), s1);

        let mut unit = Unit {
            id: 2,
            unit_template_type: "t".to_string(),
            team: "team".to_string(),
            moved: 0,
            move_points: 10, // initial value should be overwritten
            has_cast_skill_this_turn: false,
            reactions_used_this_turn: 0,
            max_reactions_per_turn: 0,
            hp: 1,
            max_hp: 1,
            mp: 1,
            max_mp: 1,
            skills: ["neg".to_string()].iter().cloned().collect(),
            status_effects: Vec::new(),
        };

        unit.recalc_from_skills(&skills).unwrap();
        assert_eq!(unit.move_points, 0);
    }

    #[test]
    fn test_skills_to_potency() {
        let mut skills = BTreeMap::new();

        // 技能 1：Fire Potency +15
        let mut s1 = Skill::default();
        s1.effects = vec![Effect::Potency {
            target_type: TargetType::Caster,
            shape: Shape::Point,
            tag: Tag::Fire,
            value: 15,
            duration: -1,
        }];
        skills.insert("fire_pot1".to_string(), s1);

        // 技能 2：Fire Potency +10
        let mut s2 = Skill::default();
        s2.effects = vec![Effect::Potency {
            target_type: TargetType::Caster,
            shape: Shape::Point,
            tag: Tag::Fire,
            value: 10,
            duration: -1,
        }];
        skills.insert("fire_pot2".to_string(), s2);

        // 技能 3：Debuff Potency +20
        let mut s3 = Skill::default();
        s3.effects = vec![Effect::Potency {
            target_type: TargetType::Caster,
            shape: Shape::Point,
            tag: Tag::Debuff,
            value: 20,
            duration: -1,
        }];
        skills.insert("debuff_pot".to_string(), s3);

        // 測試 Fire tag 的 potency
        let fire_potency = skills_to_potency(skills.keys(), &skills, &Tag::Fire);
        assert_eq!(fire_potency.unwrap(), 25);

        // 測試 Debuff tag 的 potency
        let debuff_potency = skills_to_potency(skills.keys(), &skills, &Tag::Debuff);
        assert_eq!(debuff_potency.unwrap(), 20);

        // 測試 Physical tag 的 potency（沒有）
        let physical_potency = skills_to_potency(skills.keys(), &skills, &Tag::Physical);
        assert_eq!(physical_potency.unwrap(), 0);
    }

    #[test]
    fn test_skills_to_resistance() {
        let mut skills = BTreeMap::new();

        // 技能 1：Fortitude +10
        let mut s1 = Skill::default();
        s1.effects = vec![Effect::Resistance {
            target_type: TargetType::Caster,
            shape: Shape::Point,
            save_type: SaveType::Fortitude,
            value: 10,
            duration: -1,
        }];
        skills.insert("fort1".to_string(), s1);

        // 技能 2：Fortitude +5
        let mut s2 = Skill::default();
        s2.effects = vec![Effect::Resistance {
            target_type: TargetType::Caster,
            shape: Shape::Point,
            save_type: SaveType::Fortitude,
            value: 5,
            duration: -1,
        }];
        skills.insert("fort2".to_string(), s2);

        // 技能 3：Will +20
        let mut s3 = Skill::default();
        s3.effects = vec![Effect::Resistance {
            target_type: TargetType::Caster,
            shape: Shape::Point,
            save_type: SaveType::Will,
            value: 20,
            duration: -1,
        }];
        skills.insert("will1".to_string(), s3);

        let fortitude_resistance =
            skills_to_resistance(skills.keys(), &skills, &SaveType::Fortitude);
        assert_eq!(fortitude_resistance.unwrap(), 15);

        let will_resistance = skills_to_resistance(skills.keys(), &skills, &SaveType::Will);
        assert_eq!(will_resistance.unwrap(), 20);

        let reflex_resistance = skills_to_resistance(skills.keys(), &skills, &SaveType::Reflex);
        assert_eq!(reflex_resistance.unwrap(), 0);
    }

    #[test]
    fn test_skills_to_max_reactions() {
        let mut skills = BTreeMap::new();
        // 無技能
        assert_eq!(skills_to_max_reactions(skills.keys(), &skills).unwrap(), 0);

        // 一個 MaxReactions 技能
        let mut skill1 = Skill::default();
        skill1.effects = vec![Effect::MaxReactions {
            value: 1,
            target_type: Default::default(),
            shape: Default::default(),
            duration: 0,
        }];
        let key_a = "a".to_string();
        skills.insert(key_a.clone(), skill1);
        assert_eq!(skills_to_max_reactions(skills.keys(), &skills).unwrap(), 1);

        // 多個 MaxReactions 技能
        let mut skill2 = Skill::default();
        skill2.effects = vec![Effect::MaxReactions {
            value: 2,
            target_type: Default::default(),
            shape: Default::default(),
            duration: 0,
        }];
        let key_b = "b".to_string();
        skills.insert(key_b.clone(), skill2);
        assert_eq!(skills_to_max_reactions(skills.keys(), &skills).unwrap(), 3);

        // 非 MaxReactions 類型技能不影響
        let mut skill3 = Skill::default();
        skill3.effects = vec![Effect::MaxHp {
            value: 99,
            target_type: Default::default(),
            shape: Default::default(),
            duration: 0,
        }];
        let key_c = "c".to_string();
        skills.insert(key_c.clone(), skill3);
        assert_eq!(skills_to_max_reactions(skills.keys(), &skills).unwrap(), 3);
    }

    #[test]
    fn test_skills_to_max_reactions_negative() {
        let mut skills = BTreeMap::new();
        // 負數 max_reactions，應回傳 0
        let mut skill1 = Skill::default();
        skill1.effects = vec![Effect::MaxReactions {
            value: -2,
            target_type: Default::default(),
            shape: Default::default(),
            duration: 0,
        }];
        let key_a = "a".to_string();
        skills.insert(key_a.clone(), skill1);
        assert_eq!(skills_to_max_reactions(skills.keys(), &skills).unwrap(), 0);

        // 正負混合，總和為負，仍回傳 0
        let mut skill2 = Skill::default();
        skill2.effects = vec![Effect::MaxReactions {
            value: 1,
            target_type: Default::default(),
            shape: Default::default(),
            duration: 0,
        }];
        let key_b = "b".to_string();
        skills.insert(key_b.clone(), skill2);
        assert_eq!(skills_to_max_reactions(skills.keys(), &skills).unwrap(), 0);

        // 正負混合，總和為正
        let mut skill3 = Skill::default();
        skill3.effects = vec![Effect::MaxReactions {
            value: 5,
            target_type: Default::default(),
            shape: Default::default(),
            duration: 0,
        }];
        let key_c = "c".to_string();
        skills.insert(key_c.clone(), skill3);
        assert_eq!(skills_to_max_reactions(skills.keys(), &skills).unwrap(), 4);
    }
}

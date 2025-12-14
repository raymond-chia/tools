# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

This is a **tactical turn-based RPG game** with an integrated editor, written in Rust. The project consists of:
- **Core game libraries** (`chess-lib`, `skills-lib`, `dialogs-lib`) - game logic, combat system, skills, and dialogue
- **Editor** - GUI tool for designing maps, units, skills, and game content using egui/eframe

**Game Design Philosophy**: Tactical planning + terrain utilization + resource management. Players should be able to overcome randomness through strategic preparation (buffs, positioning, equipment). See `README-設計機制.md` for detailed design vision (in Traditional Chinese).

## Build & Test Commands

### Building
```bash
# Build all workspace members
cargo build

# Build editor (default member)
cargo build -p editor

# Build specific library
cargo build -p chess-lib
```

### Testing
```bash
# Run all core library tests (recommended)
./scripts/test_core.bat        # Windows
bash scripts/test_core.sh       # Linux/WSL (if available)

# Or manually test each core library
cd core/chess-lib && cargo test
cd core/skills-lib && cargo test
cd core/dialogs-lib && cargo test

# Run single test
cargo test -p chess-lib test_name

# Run tests with output
cargo test -p chess-lib -- --nocapture
```

### Test Coverage
```bash
# Generate coverage reports for all core libraries
bash scripts/test_core_coverage.sh

# Output: coverage/*.html and coverage/lcov.info
# Individual reports: coverage/chess-lib.html, coverage/skills-lib.html, etc.
```

**Important**: Do NOT write tests for:
- `ai.rs` modules
- `editor` crate
- Inner functions (only test public APIs)

### Running the Editor
```bash
cargo run -p editor
```

## Architecture

### Core Libraries

**chess-lib** - Main game logic library
- `lib.rs` - Type definitions, re-exports, global constants
- `board.rs` - Board, Tile, Terrain, Object definitions; tile/unit position mapping
- `unit.rs` - Unit, UnitTemplate, Team; unit stat calculations (initiative, evasion, block, block_reduction, move points)
- `battle.rs` - Battle flow, turn order management
- `action/` - Movement, skill casting, pathfinding
  - `movement.rs` - Movement cost calculation, pathfinding (Dijkstra)
  - `skill.rs` - Skill casting, hit/evade/block resolution, effect application
  - `algo.rs` - Shape calculations (Point, Circle, Line, Cone), Bresenham line algorithm
- `ai.rs` - AI decision making, action scoring (NO TESTS)
- `error.rs` - Custom error types with rich context

**skills-lib** - Skill system
- Skill definitions with tags, range, cost, accuracy, crit_rate, effects
- Effect types: Hp, Mp, MaxHp, MaxMp, Initiative, Evasion, Block, BlockReduction, MovePoints, Burn, HitAndRun, Shove
- TargetType: Caster, Ally, Enemy, etc.
- Shape: Point, Circle, Line, Cone

**dialogs-lib** - Dialogue system
- Dialogue trees and conversation flow

### Key Design Patterns

**Terrain vs Object Separation**
```rust
pub struct Tile {
    pub terrain: Terrain,      // Base terrain (affects movement cost)
    pub object: Option<Object>, // Objects on terrain (affects passability)
}
```
- **Terrain**: Plain, Hill, Mountain, Forest, ShallowWater, DeepWater
- **Object**: Wall, Tree, Cliff, Pit, Tent2, Tent15
- This allows combinations like "mountain with a wall" or "plain with a cliff"

**Combat Resolution Flow**
1. Calculate hit score: `accuracy + random(1-100)`
2. Check critical failure (≤5%) or critical success (>95%)
3. Check critical hit: if `random > (100 - crit_rate)`, mark as crit (2.0x damage multiplier)
4. Calculate evasion: `hit_score - target_evasion`
5. If evaded, no effect; otherwise calculate block
6. If blocked, apply block reduction (base 10%, max 80% from BlockReduction skills), then apply crit multiplier if applicable
7. If not blocked, apply full effect, then apply crit multiplier if applicable

**Block vs BlockReduction**:
- **Block**: Blocking chance (used in hit calculation: `hit_score - evasion - block`)
- **BlockReduction**: Damage reduction percentage when blocked (from passive skills, base 10%, max 80%)
- These are separate systems - units can have high block chance but low reduction, or vice versa

**Percentage-based System** (NOT d20/dice)
- Accuracy, evasion, block are numeric modifiers
- Hit chance displayed as percentages (intuitive for players)
- Randomness uses `random_range(1..=100)`, not dice notation

**Damage System**:
- **Fixed damage** (not damage ranges)
- Rationale: Supports tactical planning, already have sufficient randomness from hit/evade/block/crit
- Randomness comes from whether attack hits/crits, not damage amount
- Aligns with "strategic preparation overcomes randomness" design philosophy

### Module Responsibilities (Strict Separation)

- **lib.rs**: Only type aliases, re-exports, constants
- **board.rs**: Tile/terrain/object definitions, board initialization, position queries
- **unit.rs**: Unit data structures, stat calculations (initiative, evasion, etc.)
- **battle.rs**: Battle flow, turn management (NOT combat resolution)
- **action/skill.rs**: Skill casting, hit/evade/block logic, effect application
- **action/movement.rs**: Movement cost, pathfinding, passability checks
- **ai.rs**: AI decision making only

### Data Serialization

- Uses `serde` with TOML format for game data
- Board configs, unit templates, skills stored as TOML files
- Test data in JSON format (see `core/chess-lib/tests/*.json`)

## Code Style & Conventions

**Language**: All code, comments, and documentation in **Traditional Chinese (繁體中文)**

**Key Principles** (from `.roo/rules/rules.md`):
1. Data-driven design, type safety, comprehensive error handling
2. Rust idiomatic code, best practices, strict type checking
3. NO magic numbers/strings - use constants or enums
4. Multi-step reasoning: analyze → design → implement → refactor
5. Read entire file before modification, make minimal necessary changes
6. When uncertain, ask user for clarification - do NOT decide independently

**Error Handling**:
- Use `Result<T, E>` pattern
- Custom error types with **rich context**: include failing input values, operation target, specific failure reasons
- Error messages should explain WHAT failed, WHY it failed, and WHERE it failed
- Example: `Error::SkillNotFound { func: "cast_skill", skill_id: "fireball" }`

**Testing**:
- Tests can only modify code logic if side effects make testing difficult
- Do NOT modify code logic for other testing reasons
- Test coverage tracked in `coverage/` directory

**Import Statements**:
- All `use` statements at file top, never inside functions

**Comments**:
- Update comments when changing code
- Don't delete still-correct comments
- Avoid overly trivial details in comments

## Current Status

**Completed implementations**:
1. ✅ Terrain-based tactics: `Object::Pit` with `Shove` instant-kill mechanics
2. ✅ Critical hit system with fixed 2.0x multiplier (based on `crit_rate`)
3. ✅ Block and BlockReduction separation (blocking chance vs damage reduction %)
4. ✅ Mp effect handling in skill casting

**Next priorities** (see `README-設計機制.md`):
1. MP as daily resource (doesn't restore between battles)
2. Test maps for terrain tactics in editor
3. Skill diversity (stable vs risky skills with different accuracy/damage/MP tradeoffs)
4. Buff skills for accuracy/evasion/block_reduction enhancement

See `README-設計機制.md` for full design roadmap and implementation priorities.

## Editor Structure

The `editor` crate uses egui for GUI:
- Visual map editor for terrain, objects, unit placement
- Skill editor for defining skill effects
- Unit template editor
- Player progression editor
- AI configuration editor

**NO TESTS** for editor code - it's a GUI tool for designers.

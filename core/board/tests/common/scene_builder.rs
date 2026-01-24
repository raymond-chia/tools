use board::types::{Pos, SceneError};
use std::collections::HashMap;

pub struct SceneBuilder {
    width: usize,
    height: usize,
    #[allow(dead_code)]
    symbols: HashMap<Pos, char>,
}

impl SceneBuilder {
    /// 從文本解析場景
    /// 格式：
    /// 5x5 board
    ///
    /// P . . . .
    /// . . E . .
    /// . . . . .
    /// . . . . .
    /// . . . . .
    pub fn parse(input: &str) -> Result<Self, SceneError> {
        let mut lines = input.lines().map(|l| l.trim()).filter(|l| !l.is_empty());

        // 解析第一行維度
        let dimension_line = lines
            .next()
            .ok_or_else(|| SceneError::ParseError("缺少維度行".to_string()))?;

        let (width, height) = Self::parse_dimensions(dimension_line)?;

        // 解析棋盤
        let mut symbols = HashMap::new();
        for (row, line) in lines.enumerate() {
            if row >= height {
                return Err(SceneError::ParseError(format!(
                    "列數超過預期 (期望 {}，收到 {})",
                    height,
                    row + 1
                )));
            }

            let cells: Vec<&str> = line.split_whitespace().collect();
            if cells.len() != width {
                return Err(SceneError::ParseError(format!(
                    "第 {} 列的寬度不匹配 (期望 {}，收到 {})",
                    row,
                    width,
                    cells.len()
                )));
            }

            for (col, cell) in cells.iter().enumerate() {
                if cell.len() != 1 {
                    return Err(SceneError::ParseError(format!("無效格子：'{}'", cell)));
                }

                let ch = cell.chars().next().unwrap();
                if ch != '.' {
                    let pos = Pos::new(col, row);
                    Self::validate_symbol(ch)?;
                    symbols.insert(pos, ch);
                }
            }
        }

        Ok(SceneBuilder {
            width,
            height,
            symbols,
        })
    }

    fn parse_dimensions(line: &str) -> Result<(usize, usize), SceneError> {
        // 期望格式："5x5 board" 或 "5x5"
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.is_empty() {
            return Err(SceneError::InvalidDimensions("缺少維度資訊".to_string()));
        }

        let dimension_part = parts[0];
        let dims: Vec<&str> = dimension_part.split('x').collect();

        if dims.len() != 2 {
            return Err(SceneError::InvalidDimensions(format!(
                "無效格式，期望 'WxH'，收到 '{}'",
                dimension_part
            )));
        }

        let width = dims[0]
            .parse::<usize>()
            .map_err(|_| SceneError::InvalidDimensions(format!("寬度不是數字：{}", dims[0])))?;

        let height = dims[1]
            .parse::<usize>()
            .map_err(|_| SceneError::InvalidDimensions(format!("高度不是數字：{}", dims[1])))?;

        if width == 0 || height == 0 {
            return Err(SceneError::InvalidDimensions(
                "寬度和高度必須大於 0".to_string(),
            ));
        }

        Ok((width, height))
    }

    fn validate_symbol(ch: char) -> Result<(), SceneError> {
        match ch {
            'P' | 'E' => Ok(()),
            _ => Err(SceneError::InvalidSymbol(ch)),
        }
    }

    pub fn width(&self) -> usize {
        self.width
    }

    pub fn height(&self) -> usize {
        self.height
    }

    #[allow(dead_code)]
    pub fn symbols(&self) -> &HashMap<Pos, char> {
        &self.symbols
    }
}

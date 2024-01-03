// Copyright (c) 2022 Huawei Technologies Co.,Ltd. All rights reserved.
//
// sysMaster is licensed under Mulan PSL v2.
// You can use this software according to the terms and conditions of the Mulan
// PSL v2.
// You may obtain a copy of Mulan PSL v2 at:
//         http://license.coscl.org.cn/MulanPSL2
// THIS SOFTWARE IS PROVIDED ON AN "AS IS" BASIS, WITHOUT WARRANTIES OF ANY
// KIND, EITHER EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO
// NON-INFRINGEMENT, MERCHANTABILITY OR FIT FOR A PARTICULAR PURPOSE.
// See the Mulan PSL v2 for more details.

//! ShowTable can be used to display contents in a table format
//! ```rust,ignore
//!     This is called column
//!              |
//!              v
//! Cell(0,0) Cell(0,1) Cell(0,2) -> This is called Line, or row
//! Cell(1,0) Cell(1,1) Cell(1,2)
//! Cell(2,0)    ...       ...
//! ```
use std::fmt::Display;

/// The alignment of one cell
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum CellAlign {
    ///
    Left,
    ///
    Right,
    ///
    Center,
}

/// The color of one cell
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum CellColor {
    ///
    Empty,
    ///
    Grey,
    ///
    Red,
    ///
    Green,
    ///
    Yellow,
    ///
    Blue,
    ///
    Purple,
}

impl From<CellColor> for String {
    /// Change the CellColor to the magic string
    fn from(cell_color: CellColor) -> Self {
        match cell_color {
            CellColor::Empty => "[37m".to_string(),
            CellColor::Grey => "[30m".to_string(),
            CellColor::Red => "[31m".to_string(),
            CellColor::Green => "[32m".to_string(),
            CellColor::Yellow => "[33m".to_string(),
            CellColor::Blue => "[34m".to_string(),
            CellColor::Purple => "[35m".to_string(),
        }
    }
}

/// Table Cell
pub struct Cell {
    color: CellColor,
    align: CellAlign,
    underline: bool,
    left_space: bool,
    right_space: bool,
    /* this cell's width, don't consider cells in other lines */
    width: usize,
    x_index: usize,
    split_content: Vec<String>,
}

impl Cell {
    /// Create a new cell
    pub fn new(ori_content: Option<&str>, x_index: usize) -> Self {
        let mut width = 0;
        let mut split_content: Vec<String> = Vec::new();
        match ori_content {
            None => {
                width = 0;
                split_content.push(String::new());
            }
            Some(ori_content) => {
                for cell_line in ori_content.split('\n') {
                    width = std::cmp::max(width, cell_line.len());
                    split_content.push(cell_line.to_string());
                }
            }
        }

        Cell {
            color: CellColor::Empty,
            underline: false,
            align: CellAlign::Left,
            left_space: true,
            right_space: true,
            width,
            x_index,
            split_content,
        }
    }

    /// Set the color of one cell
    pub fn set_color(&mut self, color: CellColor) {
        self.color = color;
    }

    /// Set the alignment of one cell
    pub fn set_align(&mut self, align: CellAlign) {
        self.align = align;
    }

    /// Set the cell's content underlined
    pub fn set_underline(&mut self, use_underline: bool) {
        self.underline = use_underline;
    }

    /// Set if keep the cell's left space
    pub fn set_left_space(&mut self, use_space: bool) {
        self.left_space = use_space;
    }

    /// Set if keep the cell's right space
    pub fn set_right_space(&mut self, use_space: bool) {
        self.right_space = use_space;
    }

    /// Print one line of a cell out
    ///
    /// * i: print which line
    ///
    /// * width: the cell's width
    ///
    /// * height: the cell's height
    pub fn format_cell_line(&self, i: usize, width: usize, height: usize) -> String {
        let mut res = String::new();
        if i >= self.split_content.len() {
            res += &" ".repeat(width);
        } else {
            res += &self.split_content[i];
        }
        if self.align == CellAlign::Left && width > res.len() {
            res += &" ".repeat(width - res.len());
        } else if self.align == CellAlign::Right && width > res.len() {
            res = " ".repeat(width - res.len()) + &res;
        } else if self.align == CellAlign::Center && width > res.len() {
            let left_size = (width - res.len()) / 2;
            res = " ".repeat(left_size) + &res;
            res += &" ".repeat(width - res.len());
        }
        if self.left_space {
            res = " ".to_string() + &res;
        }
        if self.right_space {
            res += " ";
        }
        let mut prefix = String::new();
        if self.color != CellColor::Empty {
            prefix = "\x1b".to_string() + &String::from(self.color);
        }
        if i == height - 1 && self.underline {
            prefix += "\x1b[4m";
        }
        if !prefix.is_empty() {
            res = prefix + &res + "\x1b[0m";
        }
        res
    }
}

/// Table Line
pub struct Line {
    /// Height of this total line
    height: usize,
    /// Width of this line, don't consider other lines
    per_widths: Vec<usize>,
    /// Cells of this line
    cells: Vec<Cell>,
}

impl Line {
    /// Create an empty line
    pub fn empty() -> Self {
        Self {
            per_widths: Vec::new(),
            height: 0,
            cells: Vec::new(),
        }
    }
    /// Create a new line by a Vec<&str>
    pub fn new(ori_contents: Vec<&str>) -> Self {
        let mut per_widths = Vec::new();
        let mut height = 0;
        let mut cells = Vec::new();
        for (x_index, ori_content) in ori_contents.into_iter().enumerate() {
            let cell = Cell::new(Some(ori_content), x_index);
            height = std::cmp::max(cell.split_content.len(), height);
            per_widths.push(cell.width);
            cells.push(cell);
        }
        Self {
            height,
            per_widths,
            cells,
        }
    }
}

/// ShowTable
pub struct ShowTable {
    /* width of the table, don't consider left_space or right_space. */
    global_widths: Vec<usize>,
    lines: Vec<Line>,
}

impl ShowTable {
    /// Create a new empty ShowTable
    pub fn new() -> Self {
        Self {
            global_widths: Vec::new(),
            lines: Vec::new(),
        }
    }

    /// Add a ShowTableLine to ShowTable
    pub fn add_show_table_item(&mut self, show_table_line: &impl ShowTableLine) {
        let line = show_table_line.to_vec();
        self.add_line(line);
    }

    /// Add a single line to ShowTable
    pub fn add_line(&mut self, ori_contents: Vec<&str>) {
        let line = Line::new(ori_contents);
        /* The table is empty. */
        if self.global_widths.is_empty() {
            for v in &line.per_widths {
                self.global_widths.push(*v);
            }
        } else {
            if line.per_widths.len() != self.global_widths.len() {
                log::error!("Can not add this line to ShowTable, their lengths are different.");
                return;
            }
            for i in 0..line.per_widths.len() {
                self.global_widths[i] = std::cmp::max(self.global_widths[i], line.per_widths[i]);
            }
        }
        self.lines.push(line);
    }

    /// Set all cells's alignment to left
    pub fn set_all_cell_align_left(&mut self) {
        for i in 0..self.lines.len() {
            for j in 0..self.lines[i].cells.len() {
                self.lines[i].cells[j].align = CellAlign::Left;
            }
        }
    }

    /// Set all cell's alignment to right
    pub fn set_all_cell_align_right(&mut self) {
        for i in 0..self.lines.len() {
            for j in 0..self.lines[i].cells.len() {
                self.lines[i].cells[j].align = CellAlign::Right;
            }
        }
    }

    /// Set all cells' alignment to center
    pub fn set_all_cell_align_center(&mut self) {
        for i in 0..self.lines.len() {
            for j in 0..self.lines[i].cells.len() {
                self.lines[i].cells[j].align = CellAlign::Center;
            }
        }
    }

    /// Set a certain row's alignment
    pub fn set_one_row_align(&mut self, i: usize, align: CellAlign) {
        for j in 0..self.lines[i].cells.len() {
            self.lines[i].cells[j].align = align;
        }
    }

    /// Set current row's alignment
    ///
    /// This is useful when one add a new line, and wants to change its format immediately.
    pub fn set_current_row_align(&mut self, align: CellAlign) {
        let total_line = self.lines.len();
        if total_line < 1 {
            log::info!("Failed to set current row's align.");
            return;
        }
        for j in 0..self.lines[total_line - 1].cells.len() {
            self.lines[total_line - 1].cells[j].align = align;
        }
    }

    /// Set a certain column's alignment
    pub fn set_one_col_align(&mut self, j: usize, align: CellAlign) {
        for i in 0..self.lines.len() {
            self.lines[i].cells[j].align = align;
        }
    }

    /// Set a cell's alignment
    pub fn set_one_cell_align(&mut self, i: usize, j: usize, align: CellAlign) {
        self.lines[i].cells[j].align = align;
    }

    /// Set all cells' split space
    pub fn set_all_cell_space(&mut self, left_space: bool, right_space: bool) {
        for i in 0..self.lines.len() {
            for j in 0..self.lines[i].cells.len() {
                self.lines[i].cells[j].left_space = left_space;
                self.lines[i].cells[j].right_space = right_space;
            }
        }
    }

    /// Set one row's split space
    pub fn set_one_row_space(&mut self, i: usize, left_space: bool, right_space: bool) {
        for j in 0..self.lines[i].cells.len() {
            self.lines[i].cells[j].left_space = left_space;
            self.lines[i].cells[j].right_space = right_space;
        }
    }

    /// Set one column's split space
    pub fn set_one_col_space(&mut self, j: usize, left_space: bool, right_space: bool) {
        for i in 0..self.lines.len() {
            self.lines[i].cells[j].left_space = left_space;
            self.lines[i].cells[j].right_space = right_space;
        }
    }

    /// Set one cell's split space
    pub fn set_one_cell_space(&mut self, i: usize, j: usize, left_space: bool, right_space: bool) {
        self.lines[i].cells[j].left_space = left_space;
        self.lines[i].cells[j].right_space = right_space;
    }

    /// Set all cells' underline
    pub fn set_all_cell_underline(&mut self, use_underline: bool) {
        for i in 0..self.lines.len() {
            for j in 0..self.lines[i].cells.len() {
                self.lines[i].cells[j].underline = use_underline;
            }
        }
    }

    /// Set one row's underline
    pub fn set_one_row_underline(&mut self, i: usize, use_underline: bool) {
        for j in 0..self.lines[i].cells.len() {
            self.lines[i].cells[j].underline = use_underline;
        }
    }

    /// Set the current row's underline
    pub fn set_current_row_underline(&mut self, use_underline: bool) {
        let total_line = self.lines.len();
        if total_line < 1 {
            log::info!("Failed to set current row's underline.");
            return;
        }
        for j in 0..self.lines[total_line - 1].cells.len() {
            self.lines[total_line - 1].cells[j].underline = use_underline;
        }
    }

    /// Set one column's underline
    pub fn set_one_col_underline(&mut self, j: usize, use_underline: bool) {
        for i in 0..self.lines.len() {
            self.lines[i].cells[j].underline = use_underline;
        }
    }

    /// Set one cell's underline
    pub fn set_one_cell_underline(&mut self, i: usize, j: usize, use_underline: bool) {
        self.lines[i].cells[j].underline = use_underline;
    }

    /// Set all cells' color
    pub fn set_all_cell_color(&mut self, color: CellColor) {
        for i in 0..self.lines.len() {
            for j in 0..self.lines[i].cells.len() {
                self.lines[i].cells[j].color = color;
            }
        }
    }

    /// Set one row's color
    pub fn set_one_row_color(&mut self, i: usize, color: CellColor) {
        for j in 0..self.lines[i].cells.len() {
            self.lines[i].cells[j].color = color;
        }
    }

    /// Set current row's color
    pub fn set_current_row_color(&mut self, color: CellColor) {
        let total_line = self.lines.len();
        if total_line < 1 {
            log::info!("Failed to set current row's color.");
            return;
        }
        for j in 0..self.lines[total_line - 1].cells.len() {
            self.lines[total_line - 1].cells[j].color = color;
        }
    }

    /// Set one column's color
    pub fn set_one_col_color(&mut self, j: usize, color: CellColor) {
        for i in 0..self.lines.len() {
            self.lines[i].cells[j].color = color;
        }
    }

    /// Set one cell's color
    pub fn set_one_cell_color(&mut self, i: usize, j: usize, color: CellColor) {
        self.lines[i].cells[j].color = color;
    }
}

impl Default for ShowTable {
    fn default() -> Self {
        Self::new()
    }
}

impl Display for ShowTable {
    /// Print the whole ShowTable out
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut res = String::new();
        /* print one line */
        for line in &self.lines {
            /* The cell can be multi-cell_line, print one by one */
            for i in 0..line.height {
                for cell in &line.cells {
                    res += &cell.format_cell_line(i, self.global_widths[cell.x_index], line.height);
                }
                res += "\n";
            }
        }
        write!(f, "{}", res.trim_end())
    }
}

/// Struct can implement this trait to display in ShowTable
pub trait ShowTableLine {
    /// change the struct to the string vector
    fn to_vec(&self) -> Vec<&str> {
        todo!()
    }
}

mod tests {
    use super::ShowTableLine;

    struct TestItem {
        value1: String,
        value2: String,
        value3: String,
    }

    impl ShowTableLine for TestItem {
        fn to_vec(&self) -> Vec<&str> {
            vec![&self.value1, &self.value2, &self.value3]
        }
    }

    #[test]
    fn run_test() {
        use super::ShowTable;
        let mut table1 = ShowTable::new();
        table1.add_line(vec!["AAA", "BBBB", "CCCCCCCCCC"]);
        table1.add_line(vec!["12345", "123", "123"]);
        assert_eq!(
            table1.to_string(),
            " AAA    BBBB  CCCCCCCCCC \n 12345  123   123"
        );
        table1.set_all_cell_align_right();
        assert_eq!(
            table1.to_string(),
            "   AAA  BBBB  CCCCCCCCCC \n 12345   123         123"
        );
        table1.set_all_cell_align_left();
        table1.set_one_row_align(0, crate::show_table::CellAlign::Right);
        assert_eq!(
            table1.to_string(),
            "   AAA  BBBB  CCCCCCCCCC \n 12345  123   123"
        );
        table1.set_all_cell_align_right();
        table1.set_one_col_align(0, crate::show_table::CellAlign::Left);
        assert_eq!(
            table1.to_string(),
            " AAA    BBBB  CCCCCCCCCC \n 12345   123         123"
        );

        let test_item1 = TestItem {
            value1: "AAA".to_string(),
            value2: "BBBB".to_string(),
            value3: "CCCCCCCCCC".to_string(),
        };
        let test_item2 = TestItem {
            value1: "12345".to_string(),
            value2: "123".to_string(),
            value3: "123".to_string(),
        };
        let mut table2 = ShowTable::new();
        table2.add_show_table_item(&test_item1);
        table2.add_show_table_item(&test_item2);
        assert_eq!(
            table2.to_string(),
            " AAA    BBBB  CCCCCCCCCC \n 12345  123   123"
        );
    }
}

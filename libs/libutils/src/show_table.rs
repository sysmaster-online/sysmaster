//! ShowTable can be used to display contents in a table format

#[derive(Clone, Copy)]
/// Cell Align
pub enum CellAlign {
    /// Align to left
    Left,
    /// Align to right
    Right,
}

/// ShowTable can be used to display contents in a table format
#[derive(Default)]
pub struct ShowTable {
    row_length: usize,
    column_length: usize,
    cell_width: Vec<usize>,
    cell_align: Vec<CellAlign>,
    original_contents: Vec<Vec<String>>,
    show_contents: Vec<Vec<String>>,
}

/// Struct can implement this trait to display in ShowTable
pub trait ShowTableLine {
    /// change the struct to the string vector
    fn to_vec(&self) -> Vec<String> {
        todo!()
    }
}

impl ShowTable {
    /// Create a new ShowTable
    pub fn new() -> Self {
        Self {
            row_length: 0,
            column_length: 0,
            cell_width: vec![],
            cell_align: vec![],
            original_contents: vec![],
            show_contents: vec![],
        }
    }
    /// Add a new line to ShowTable
    pub fn add_line(&mut self, line_content: Vec<String>) {
        let content_length = line_content.len();
        let mut new_line = line_content;
        if self.row_length == 0 {
            self.column_length = content_length;
            self.cell_width = vec![0; self.column_length];
            self.cell_align = vec![CellAlign::Left; self.column_length];
        }
        // remove string at the end, if the new line is longer
        if content_length > self.column_length {
            for _ in 0..content_length - self.column_length {
                new_line.pop();
            }
        }

        // add empty string at the end, if the new line is shorter.
        if self.column_length > content_length {
            for _ in 0..self.column_length - content_length {
                new_line.push(String::new());
            }
        }

        #[allow(clippy::needless_range_loop)]
        for i in 0..self.column_length {
            self.cell_width[i] = std::cmp::max(self.cell_width[i], new_line[i].len() + 2);
        }

        self.original_contents.push(new_line);
        self.row_length += 1;
    }

    /// Add a ShowTableLine to ShowTable
    pub fn add_show_table_item(&mut self, show_table_line: &impl ShowTableLine) {
        let line_content = show_table_line.to_vec();
        self.add_line(line_content);
    }

    fn cell_align_left(cell: &mut String, length: usize) {
        // add a space to split different cell
        *cell = " ".to_string() + cell;
        if length > cell.len() {
            for _ in 0..length - cell.len() {
                // append spaces to the end
                *cell += " ";
            }
        } else {
            *cell = cell[0..length].to_string();
        }
    }

    fn cell_align_right(cell: &mut String, length: usize) {
        // add a space to split different cell
        *cell += " ";
        if length > cell.len() {
            let mut spaces = String::new();
            for _ in 0..length - cell.len() {
                // append spaces to the end
                spaces += " ";
            }
            *cell = spaces + cell;
        } else {
            *cell = cell[0..length].to_string();
        }
    }

    /// Align all the cell content to the left and update
    pub fn align_left(&mut self) {
        self.show_contents.clear();
        for line in &self.original_contents {
            let mut show_line: Vec<String> = Vec::new();
            #[allow(clippy::needless_range_loop)]
            for i in 0..self.column_length {
                let mut show_cell = line[i].to_string();
                Self::cell_align_left(&mut show_cell, self.cell_width[i]);
                show_line.push(show_cell);
            }
            self.show_contents.push(show_line);
        }
    }

    /// Align all the cell content to the right and update
    pub fn align_right(&mut self) {
        self.show_contents.clear();
        for line in &self.original_contents {
            let mut show_line: Vec<String> = Vec::new();
            #[allow(clippy::needless_range_loop)]
            for i in 0..self.column_length {
                let mut show_cell = line[i].to_string();
                Self::cell_align_right(&mut show_cell, self.cell_width[i]);
                show_line.push(show_cell);
            }
            self.show_contents.push(show_line);
        }
    }

    /// Align the cell content by user's define
    pub fn align_define(&mut self) {
        self.show_contents.clear();
        for line in &self.original_contents {
            let mut show_line: Vec<String> = Vec::new();
            #[allow(clippy::needless_range_loop)]
            for i in 0..self.column_length {
                let mut show_cell = line[i].to_string();
                match self.cell_align[i] {
                    CellAlign::Left => Self::cell_align_left(&mut show_cell, self.cell_width[i]),
                    CellAlign::Right => Self::cell_align_right(&mut show_cell, self.cell_width[i]),
                }
                show_line.push(show_cell);
            }
            self.show_contents.push(show_line);
        }
    }

    /// Set all cell aligns to left
    pub fn set_cell_align_left(&mut self) {
        self.cell_align = vec![CellAlign::Left; self.column_length];
    }

    /// Set all cell aligns to right
    pub fn set_cell_align_right(&mut self) {
        self.cell_align = vec![CellAlign::Right; self.column_length];
    }

    /// Set one cell align to left
    pub fn set_one_cell_align_left(&mut self, column: usize) {
        if column >= self.column_length {
            self.cell_align[self.column_length - 1] = CellAlign::Left;
            return;
        }
        self.cell_align[column] = CellAlign::Left;
    }

    /// Set one cell align to right
    pub fn set_one_cell_align_right(&mut self, column: usize) {
        if column >= self.column_length {
            self.cell_align[self.column_length - 1] = CellAlign::Right;
            return;
        }
        self.cell_align[column] = CellAlign::Right;
    }

    /// Set all cell aligns by user's define
    pub fn set_all_cell_align_define(&mut self, cell_align: Vec<CellAlign>) {
        self.cell_align = cell_align;
    }
}

impl std::fmt::Display for ShowTable {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut res = String::new();
        for line in &self.show_contents {
            for cell in line {
                res += cell;
            }
            res += "\n";
        }
        write!(f, "{}", res.trim_end())
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
        fn to_vec(&self) -> Vec<String> {
            vec![
                self.value1.to_string(),
                self.value2.to_string(),
                self.value3.to_string(),
            ]
        }
    }

    #[test]
    fn run_test() {
        use super::ShowTable;
        let mut table1 = ShowTable::new();
        table1.add_line(vec![
            "AAA".to_string(),
            "BBBB".to_string(),
            "CCCCCCCCCC".to_string(),
        ]);
        table1.add_line(vec![
            "12345".to_string(),
            "123".to_string(),
            "123".to_string(),
        ]);
        table1.align_left();
        assert_eq!(
            table1.to_string(),
            " AAA    BBBB  CCCCCCCCCC \n 12345  123   123"
        );
        table1.align_right();
        assert_eq!(
            table1.to_string(),
            "   AAA  BBBB  CCCCCCCCCC \n 12345   123         123"
        );
        table1.set_one_cell_align_right(0);
        table1.align_define();
        assert_eq!(
            table1.to_string(),
            "   AAA  BBBB  CCCCCCCCCC \n 12345  123   123"
        );
        table1.set_cell_align_right();
        table1.set_one_cell_align_left(0);
        table1.align_define();
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
        table2.align_left();
        assert_eq!(
            table2.to_string(),
            " AAA    BBBB  CCCCCCCCCC \n 12345  123   123"
        );
    }
}

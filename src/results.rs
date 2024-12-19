use std::cmp::max;
use std::collections::HashMap;
use std::fmt::{Display, Formatter};
use csv::Writer;

#[derive(Debug, Clone)]
pub struct Results {
    pub items: Vec<Item>,
    pub columns: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct Item {
    pub name: String,
    pub columns: HashMap<String, Datum>,
    pub children: Vec<Item>,
}

#[derive(Debug, Clone)]
pub enum Datum {
    Percent(f64),
    Count(i64),
    Stat(f64),
}

impl ToString for Datum {
    fn to_string(&self) -> String {
        match self {
            Datum::Percent(p) => format!("{:.1}%", p * 100.0),
            Datum::Count(c) => c.to_string(),
            Datum::Stat(s) => format!("{:.1}", s),
        }
    }
}

const CHILD_PREFIX: &str = "  ";
const COLUMNS_SEPARATOR: &str = " | ";

impl Item {
    #[must_use]
    fn name_column_width(&self) -> usize {
        max(
            self.name.len(),
            self.children
                .iter()
                .map(|c| c.name_column_width() + CHILD_PREFIX.len())
                .max()
                .unwrap_or(0),
        )
    }
}

impl Results {
    #[must_use]
    fn names_width(&self) -> usize {
        self.items.iter().map(|i| i.name_column_width()).max().unwrap_or(0)
    }

    #[must_use]
    fn column_width(&self, column: &str) -> usize {
        fn item_width(item: &Item, column: &str) -> usize {
            let mut width = item.columns.get(column).map_or(0, |s| s.to_string().len());
            for child in &item.children {
                width = max(width, item_width(child, column));
            }
            width
        }
        self.items.iter()
            .map(|i| item_width(i, column))
            .max()
            .unwrap_or(0)
    }

    pub fn write_to_csv<W: std::io::Write>(&self, csv_writer: &mut Writer<W>) -> csv::Result<()> {
        // Header
        csv_writer.write_field("")?;
        for column in &self.columns {
            csv_writer.write_field(column.to_string())?;
        }
        csv_writer.write_record(None::<&[u8]>)?;

        // Items
        let results_printer = ResultsPrinter::new(self);
        for item in self.items.iter() {
            for row in results_printer.item_strings(item, 0) {
                for (_, datum, _) in row.iter() {
                    csv_writer.write_field(datum)?;
                }
                csv_writer.write_record(None::<&[u8]>)?;
            }
        }

        Ok(())
    }
}

impl Display for Results {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let results_printer = ResultsPrinter::new(self);
        f.write_str(&results_printer.header())?;
        for i in self.items.iter() {
            f.write_str(&results_printer.item(i, 0))?;
        }
        Ok(())
    }
}

struct ResultsPrinter<'a> {
    results: &'a Results,
    names_width: usize,
    columns_widths: Vec<usize>,
}

enum Alignment {
    Left,
    Right,
}

impl <'a> ResultsPrinter<'a> {
    fn new(results: &'a Results) -> Self {
        Self {
            results,
            names_width: results.names_width(),
            columns_widths: results.columns.iter().map(|c| max(c.len(), results.column_width(c))).collect(),
        }
    }

    fn header(&self) -> String {
        let mut s = String::new();
        s.push_str(&" ".repeat(self.names_width));
        for (index, column) in self.results.columns.iter().enumerate() {
            s.push_str(COLUMNS_SEPARATOR);
            s.push_str(&" ".repeat(self.columns_widths[index] - column.len()));
            s.push_str(&column.to_string());
        }
        s.push_str("\n");
        s
    }

    fn item_strings(&self, item: &Item, depth: usize) -> Vec<Vec<(Alignment, String, usize)>> {
        let mut rows = Vec::new();

        let mut row = Vec::new();
        let mut name = String::new();
        if depth > 0 {
            name.push_str(&" ".repeat((depth - 1) * CHILD_PREFIX.len()));
            name.push_str(CHILD_PREFIX);
        }
        name.push_str(&item.name);
        row.push((Alignment::Left, name, self.names_width));
        for (index, column) in self.results.columns.iter().enumerate() {
            let datum = item.columns.get(column).map_or_else(|| "".to_string(), |d| d.to_string());
            row.push((Alignment::Right, datum, self.columns_widths[index]));
        }
        rows.push(row);

        for child in &item.children {
            rows.extend(self.item_strings(child, depth + 1));
        }
        rows
    }

    fn item(&self, item: &Item, depth: usize) -> String {
        let mut s = String::new();
        for row in self.item_strings(item, depth) {
            let item_strings = row;
            let columns_count = item_strings.len();
            for (index, (alignment, datum, width)) in item_strings.into_iter().enumerate() {
                let padding = &" ".repeat(width - datum.len());
                match alignment {
                    Alignment::Left => {
                        s.push_str(&datum);
                        s.push_str(padding);
                    },
                    Alignment::Right => {
                        s.push_str(padding);
                        s.push_str(&datum);
                    },
                }
                if index < columns_count - 1 {
                    s.push_str(COLUMNS_SEPARATOR);
                }
            }
            s.push_str("\n");
        }
        return s;
    }
}

// pub fn merge_results<I: IntoIterator<Item = (String, Results)>>(results: I) -> Results {
//     let mut items = Vec::new();
//     let mut columns = Vec::new();
//     for (name, results) in results {
//         for column in results.columns {
//             if !columns.contains(&column) {
//                 columns.push(column);
//             }
//         }
//         items.push(Item {
//             name,
//             columns: HashMap::new(),
//             children: results.items,
//         });
//     }
//     Results {
//         items,
//         columns,
//     }
// }

#[cfg(test)]
mod test {
    use std::collections::HashMap;
    use crate::results::{Datum, Item, Results};

    #[test]
    fn test1() {
        let result = Results {
            columns: vec!["1".to_string(), "2".to_string(), "c3".to_string()],
            items: vec![
                Item {
                    name: "item1".to_string(),
                    columns: {
                        let mut map = HashMap::new();
                        map.insert("1".to_string(), Datum::Count(1));
                        map.insert("2".to_string(), Datum::Percent(1.23123));
                        map.insert("c3".to_string(), Datum::Count(3));
                        map
                    },
                    children: vec![
                        Item {
                            name: "child".to_string(),
                            columns: HashMap::new(),
                            children: vec![],
                        }
                    ],
                },
            ],
        };
        assert_eq!(
            result.to_string(),
            "        | 1 |      2 | c3\nitem1   | 1 | 123.1% |  3\n  child |   |        |   \n",
        );
    }

    // #[test]
    // fn test_merge_results() {
    //     let results = vec![
    //         ("a".to_string(), Results {
    //             columns: vec!["c3".to_string()],
    //             items: vec![
    //                 Item {
    //                     name: "item1".to_string(),
    //                     columns: {
    //                         let mut map = HashMap::new();
    //                         map.insert("c3".to_string(), Datum::Count(3));
    //                         map
    //                     },
    //                     children: vec![
    //                         Item {
    //                             name: "child".to_string(),
    //                             columns: HashMap::new(),
    //                             children: vec![],
    //                         }
    //                     ],
    //                 },
    //             ],
    //         }),
    //         ("b".to_string(), Results {
    //             columns: vec!["1".to_string()],
    //             items: vec![
    //                 Item {
    //                     name: "item1".to_string(),
    //                     columns: {
    //                         let mut map = HashMap::new();
    //                         map.insert("1".to_string(), Datum::Count(1));
    //                         map.insert("2".to_string(), Datum::Percent(1.23123));
    //                         map.insert("c3".to_string(), Datum::Count(3));
    //                         map
    //                     },
    //                     children: vec![],
    //                 },
    //             ],
    //         }),
    //     ];
    //     let result = super::merge_results(results);
    //     assert_eq!(
    //         result.to_string(),
    //         "          | c3 | 1\na         |    |  \n  item1   |  3 |  \n    child |    |  \nb         |    |  \n  item1   |  3 | 1\n",
    //     );
    // }
}

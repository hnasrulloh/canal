mod collab;
mod parser;

#[derive(Debug, Clone)]
pub struct Notebook {
    language: String,
    blocks: Vec<Block>,
}

#[derive(Debug, Clone)]
enum Block {
    Text(Text, TextConfig),
    Code(Code, CodeConfig),
    Math(Math, MathConfig),
    Table(Table, TableConfig),
    Media(Media, MediaConfig),
}

#[derive(Debug, Clone)]
struct Text {
    line: String,
}

#[derive(Debug, Clone)]
struct TextConfig {
    composite: bool,
}

#[derive(Debug, Clone)]
struct Code {
    lines: Vec<String>,
}

#[derive(Debug, Clone)]
struct CodeConfig {
    echo: bool,
    executable: bool,
}

#[derive(Debug, Clone)]
struct Math {
    content: String,
}

#[derive(Debug, Clone)]
struct MathConfig {}

#[derive(Debug, Clone)]
struct Table {
    header: TableHeader,
    rows: Vec<TableRow>,
}

#[derive(Debug, Clone)]
struct TableHeader {
    cells: Vec<TableCell>,
}

#[derive(Debug, Clone)]
struct TableRow {
    cells: Vec<TableCell>,
}

#[derive(Debug, Clone)]
struct TableCell {
    content: String,
}

#[derive(Debug, Clone)]
struct TableConfig {}

#[derive(Debug, Clone)]
struct Media {
    source: String,
}

#[derive(Debug, Clone)]
struct MediaConfig {}

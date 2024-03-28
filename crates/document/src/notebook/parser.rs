use nom::{
    branch::alt,
    bytes::complete::{tag, take_while1},
    character::{
        complete::{line_ending, newline, not_line_ending, space0, space1},
        is_alphanumeric,
    },
    combinator::{map, opt, peek},
    multi::{many0, separated_list1},
    sequence::{delimited, preceded, separated_pair, terminated, tuple},
    IResult,
};

const TAB: &str = "  ";

#[derive(Debug, Clone)]
pub struct NotebookParsed {
    language: String,
    blocks: Vec<Block>,
}

impl NotebookParsed {
    pub fn parse(source: &str) -> Self {
        Self {
            language: unimplemented!(),
            blocks: unimplemented!(),
        }
    }
}

type KeyValue = (String, String);

#[derive(Debug, Clone, PartialEq)]
enum Block {
    Construct {
        name: String,
        options: Vec<KeyValue>,
        childs: Vec<Block>,
    },
    Value(String),
}

fn parse_valid_name(input: &str) -> IResult<&str, &str> {
    let allowed_char = |c| is_alphanumeric(c as u8) || c == '_';
    take_while1(allowed_char)(input)
}

fn parse_value_string(input: &str) -> IResult<&str, &str> {
    let allowed_char = |c| is_alphanumeric(c as u8) || "._-".contains(c);
    take_while1(allowed_char)(input)
}

fn parse_key_value(input: &str) -> IResult<&str, KeyValue> {
    let (input, (key, value)) = separated_pair(
        parse_valid_name,
        tuple((space0, tag(":"), space0)),
        parse_value_string,
    )(input)?;

    Ok((input, (key.to_string(), value.to_string())))
}

fn parse_key_value_list(input: &str) -> IResult<&str, Vec<KeyValue>> {
    let separator = delimited(space0, tag(","), space0);
    let bracket_open = terminated(tag("["), space0);
    let bracket_close = preceded(space0, tag("]"));

    let kv_list = delimited(
        bracket_open,
        separated_list1(separator, parse_key_value),
        bracket_close,
    );

    let (input, kv_list) = opt(kv_list)(input)?;

    let config = match kv_list {
        None => vec![],
        Some(list) => list,
    };

    Ok((input, config))
}

fn indented_parse_line(indent_level: usize) -> impl Fn(&str) -> IResult<&str, &str> {
    move |input| {
        let indent_spaces = TAB.repeat(indent_level);
        let indentation = tag(indent_spaces.as_str());

        let (input, line) = preceded(indentation, not_line_ending)(input)?;
        let (input, _) = opt(line_ending)(input)?;

        Ok((input, line))
    }
}

// fn indented_parse_lines(indent_level: usize) -> impl Fn(&str) -> IResult<&str, Vec<&str>> {
//     move |input: &str| {
//         let line = indented_parse_line(indent_level);
//         let (input, lines) = many0(line)(input)?;

//         Ok((input, lines))
//     }
// }

fn indented_parse_construct_info(
    indent_level: usize,
) -> impl Fn(&str) -> IResult<&str, (&str, Vec<KeyValue>)> {
    move |input| {
        let indent_spaces = TAB.repeat(indent_level);
        let indent = tag(indent_spaces.as_str());

        let (input, (_, _, _, name, _, options)) = tuple((
            indent,
            tag("block"),
            space1,
            parse_valid_name,
            space1,
            parse_key_value_list,
        ))(input)?;

        Ok((input, (name, options)))
    }
}

fn custom_parse_child_start_mark(mark: String) -> impl Fn(&str) -> IResult<&str, ()> {
    move |input| {
        let (input, _) = preceded(
            space0,
            terminated(tag(mark.as_str()), tuple((space0, newline))),
        )(input)?;

        Ok((input, ()))
    }
}

fn indented_parse_child_end_mark(
    indent_level: usize,
) -> impl Fn(&str) -> IResult<&str, Option<()>> {
    move |input| {
        let indent_spaces = TAB.repeat(indent_level);
        let indent = tag(indent_spaces.as_str());

        let (input, opt) = opt(peek(tuple((newline, indent, tag("block")))))(input)?;

        Ok((input, opt.map(|_| ())))
    }
}

fn indented_parse_construct(indent_level: usize) -> impl Fn(&str) -> IResult<&str, Block> {
    move |input| {
        let (input, (name, options)) = indented_parse_construct_info(indent_level)(input)?;

        let (input, childs) = delimited(
            custom_parse_child_start_mark("=".to_string()),
            many0(indented_parse_line(indent_level + 1)),
            indented_parse_child_end_mark(indent_level),
        )(input)?;

        Ok((
            input,
            Block::Construct {
                name: name.to_string(),
                options,
                childs: trim_empty_strings_at_vec_end(childs)
                    .into_iter()
                    .map(|s| Block::Value(s.to_string()))
                    .collect(),
            },
        ))
    }
}

fn indented_parse_block_with_child_as_block(
    indent_level: usize,
) -> impl Fn(&str) -> IResult<&str, Block> {
    move |input| {
        let (input, (name, options)) = indented_parse_construct_info(indent_level)(input)?;

        let (input, childs) = delimited(
            custom_parse_child_start_mark("=*".to_string()),
            many0(indented_parse_line(indent_level + 1)),
            indented_parse_child_end_mark(indent_level),
        )(input)?;

        Ok((
            input,
            Block::Construct {
                name: name.to_string(),
                options,
                childs: trim_empty_strings_at_vec_end(childs)
                    .into_iter()
                    .map(|s| Block::Value(s.to_string()))
                    .collect(),
            },
        ))
    }
}

fn indented_parse_blocks(indent_level: usize) -> impl Fn(&str) -> IResult<&str, Vec<Block>> {
    move |input| many0(indented_parse_construct(indent_level))(input)
}

fn indented_block(indent_level: usize) -> impl Fn(&str) -> IResult<&str, Block> {
    move |input| {
        alt((
            map(indented_parse_line(indent_level), |s| {
                Block::Value(s.to_string())
            }),
            indented_parse_block_with_child_as_block(indent_level),
        ))(input)
    }
}

fn trim_empty_strings_at_vec_end(values: Vec<&str>) -> Vec<&str> {
    let mut values: Vec<&str> = values
        .into_iter()
        .rev()
        .skip_while(|s| s.is_empty())
        .collect();

    values.reverse();
    values
}

#[cfg(test)]
mod tests {
    use super::*;
    use helpers::*;
    use indoc::indoc;

    #[googletest::test]
    fn parsing_a_key_value_pair() {
        let input = "a: 1.00";
        let result = parse_key_value(input);
        let expected = ("a".to_string(), "1.00".to_string());
        assert_parsed(result, expected);
    }

    #[googletest::test]
    fn parsing_a_list_of_key_value_pairs() {
        let input = "[ a:1 ,b:2]";
        let result = parse_key_value_list(input);
        let expected = vec![
            ("a".to_string(), "1".to_string()),
            ("b".to_string(), "2".to_string()),
        ];
        assert_parsed(result, expected);
    }

    #[googletest::test]
    fn parsing_an_indented_line() {
        let input = "  value indent 1";
        let result = indented_parse_line(1)(input);
        let expected = "value indent 1";
        assert_parsed(result, expected);
    }

    // #[googletest::test]
    // fn parsing_indented_multiple_lines() {
    //     let input = format!(
    //         "{}{}\n{}{}",
    //         TAB, "value with indent 1", TAB, "value with indent 1"
    //     );
    //     let result = indented_parse_lines(1)(input.as_str());
    //     let expected = vec!["value with indent 1", "value with indent 1"];
    //     assert_parsed(result, expected);
    // }

    #[googletest::test]
    fn parsing_a_block() {
        let input = indoc! {"
            block executable [echo: true] =
              value in first line
              value in second line
        "};
        let result = indented_parse_construct(0)(input);
        let expected = Block::Construct {
            name: "executable".to_string(),
            options: vec![("echo".to_string(), "true".to_string())],
            childs: vec![
                Block::Value("value in first line".to_string()),
                Block::Value("value in second line".to_string()),
            ],
        };
        assert_parsed(result, expected);
    }

    #[googletest::test]
    fn parsing_multi_blocks() {
        // Be careful with indents on empty lines!
        // Empty lines are required to be indented
        let input = indoc! {"
            block text =
              value1
              
            block text =
              value2
              
        "};
        let result = indented_parse_blocks(0)(input);
        let expected = vec![
            Block::Construct {
                name: "text".to_string(),
                options: vec![],
                childs: vec![Block::Value("value1".to_string())],
            },
            Block::Construct {
                name: "text".to_string(),
                options: vec![],
                childs: vec![Block::Value("value2".to_string())],
            },
        ];
        assert_parsed(result, expected);
    }

    #[googletest::test]
    fn parsing_a_block_with_empty_config() {
        let input = indoc! {"
            block text =
              value
        "};
        let result = indented_parse_construct(0)(input);
        let expected = Block::Construct {
            name: "text".to_string(),
            options: vec![],
            childs: vec![Block::Value("value".to_string())],
        };
        assert_parsed(result, expected);
    }

    #[ignore = "reason"]
    #[googletest::test]
    fn parsing_a_nested_block() {
        // Be careful with indents on empty lines!
        // Empty lines are required to be indented
        let input = indoc! {"
            block table =*
              block table_header =
                h1
                h2
                
              block table_row =
                r1
                r2
              
        "};
        let result = indented_parse_construct(0)(input);
        let expected = Block::Construct {
            name: "table".to_string(),
            options: vec![],
            childs: vec![
                Block::Construct {
                    name: "table_header".to_string(),
                    options: vec![],
                    childs: vec![
                        Block::Value("h1".to_string()),
                        Block::Value("h2".to_string()),
                    ],
                },
                Block::Construct {
                    name: "table_row".to_string(),
                    options: vec![],
                    childs: vec![
                        Block::Value("r1".to_string()),
                        Block::Value("r2".to_string()),
                    ],
                },
            ],
        };
        assert_parsed(result, expected);
    }

    mod helpers {
        use std::fmt::{Debug, Display};

        use googletest::prelude::*;
        use nom::IResult;

        pub(super) fn assert_parsed<I, O, E>(result: IResult<I, O, E>, expected: O)
        where
            O: Debug + PartialEq,
            E: Display + Debug,
        {
            if let Err(ref err) = result {
                let message = match err {
                    nom::Err::Error(msg) => format!("Parse error:\n{}", msg),
                    nom::Err::Failure(msg) => format!("Parse failure:\n{}", msg),
                    nom::Err::Incomplete(_) => "Incomplete input".to_string(),
                };

                // Panic with better error message
                let is_parsing_success = false;
                assert_that!(is_parsing_success, eq(true), "{}", message);
            }

            // Safe to unwrap after error check
            let (_, parsed) = result.unwrap();

            assert_that!(parsed, eq(expected));
        }
    }
}

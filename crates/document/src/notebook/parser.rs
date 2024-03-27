use nom::{
    bytes::complete::{tag, take_while1},
    character::{
        complete::{line_ending, not_line_ending, space0},
        is_alphanumeric,
    },
    combinator::opt,
    multi::{many0, separated_list1},
    sequence::{delimited, preceded, separated_pair, terminated, tuple},
    IResult,
};

const TAB: &str = "  ";

pub struct NotebookParser;

#[derive(Debug, Clone, PartialEq)]
struct KeyValue(String, String);

#[derive(Debug, Clone, PartialEq)]
struct BlockExpression {
    name: String,
    config: Vec<KeyValue>,
    childs: Vec<BlockChild>,
}

#[derive(Debug, Clone, PartialEq)]
enum BlockChild {
    Value(String),
    Expression(BlockExpression),
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

    Ok((input, KeyValue(key.to_string(), value.to_string())))
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

fn indented_parse_block_child_value(
    indent_level: usize,
) -> impl Fn(&str) -> IResult<&str, Vec<BlockChild>> {
    move |input: &str| {
        let line = indented_parse_line(indent_level);
        let (input, lines) = many0(line)(input)?;

        Ok((
            input,
            lines
                .iter()
                .map(|line| BlockChild::Value(line.to_string()))
                .collect(),
        ))
    }
}

fn indented_parse_block(indent_level: usize) -> impl Fn(&str) -> IResult<&str, BlockExpression> {
    move |input| {
        let indent_spaces = TAB.repeat(indent_level);
        // let indentation = tag(indent_spaces.as_str());

        let block_name = parse_valid_name;
        let block_config = parse_key_value_list;

        let (input, (name, _, config, _)) =
            tuple((block_name, space0, block_config, space0))(input)?;

        Ok((
            input,
            BlockExpression {
                name: name.to_string(),
                config,
                childs: vec![],
            },
        ))
    }
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
        let expected = KeyValue("a".to_string(), "1.00".to_string());
        assert_parsed(result, expected);
    }

    #[googletest::test]
    fn parsing_a_list_of_key_value_pairs() {
        let input = "[ a:1 ,b:2]";
        let result = parse_key_value_list(input);
        let expected = vec![
            KeyValue("a".to_string(), "1".to_string()),
            KeyValue("b".to_string(), "2".to_string()),
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

    #[googletest::test]
    fn parsing_a_block_child_as_value() {
        let input = format!(
            "{}{}\n{}{}",
            TAB, "value with indent 1", TAB, "value with indent 1"
        );
        let result = indented_parse_block_child_value(1)(input.as_str());
        let expected = vec![
            BlockChild::Value("value with indent 1".to_string()),
            BlockChild::Value("value with indent 1".to_string()),
        ];
        assert_parsed(result, expected);
    }

    // TODO: Starts here
    #[googletest::test]
    #[ignore = "Work value first"]
    fn parsing_a_block_with_value() {
        let input = indoc! {"
            text [composite: true] {
              value in first line 
              value in second line
            }
        "};
        let result = indented_parse_block(0)(input);
        let expected = BlockExpression {
            name: "text".to_string(),
            config: vec![KeyValue("composite".to_string(), "true".to_string())],
            childs: vec![
                BlockChild::Value("value in first line".to_string()),
                BlockChild::Value("value in second line".to_string()),
            ],
        };
        assert_parsed(result, expected);
    }

    #[googletest::test]
    #[ignore = "Work expression first"]
    fn parsing_a_block_with_empty_config() {
        let input = indoc! {"
            text {
              value
            }
        "};
        let result = indented_parse_block(0)(input);
        let expected = BlockExpression {
            name: "text".to_string(),
            config: vec![],
            childs: vec![],
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
                    nom::Err::Error(msg) => format!("Parse error: \"{}\"", msg),
                    nom::Err::Failure(msg) => format!("Parse failure: {}", msg),
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

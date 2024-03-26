use nom::{
    bytes::complete::{tag, take_while1},
    character::{complete::space0, is_alphanumeric},
    multi::separated_list1,
    sequence::{delimited, preceded, separated_pair, terminated, tuple},
    IResult,
};

pub struct NotebookParser;

#[derive(Debug, Clone, PartialEq)]
struct KeyValue(String, String);

fn parse_name(input: &str) -> IResult<&str, &str> {
    let allowed_char = |c| is_alphanumeric(c as u8) || c == '_';
    take_while1(allowed_char)(input)
}

fn parse_value_string(input: &str) -> IResult<&str, &str> {
    let allowed_char = |c| is_alphanumeric(c as u8) || "._-".contains(c);
    take_while1(allowed_char)(input)
}

fn parse_key_value(input: &str) -> IResult<&str, KeyValue> {
    let (input, (key, value)) = separated_pair(
        parse_name,
        tuple((space0, tag(":"), space0)),
        parse_value_string,
    )(input)?;

    Ok((input, KeyValue(key.to_string(), value.to_string())))
}

fn parse_key_value_list(input: &str) -> IResult<&str, Vec<KeyValue>> {
    let separator = delimited(space0, tag(","), space0);
    let bracket_open = terminated(tag("["), space0);
    let bracket_close = preceded(space0, tag("]"));

    let (input, kv_list) = delimited(
        bracket_open,
        separated_list1(separator, parse_key_value),
        bracket_close,
    )(input)?;

    Ok((input, kv_list))
}

#[cfg(test)]
mod tests {
    use super::*;
    use helpers::*;

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

use nom::{
    branch::alt,
    bytes::complete::{tag, take_till, take_till1, take_while, take_while1},
    character::{
        complete::{alpha1, alphanumeric1, anychar, char},
        is_alphanumeric, is_space,
    },
    combinator::cut,
    error::context,
    multi::many1,
    sequence::{delimited, preceded, separated_pair},
    IResult,
};

pub struct NotebookParser;

#[derive(Debug, Clone, PartialEq)]
struct KeyValue(String, String);

fn is_valid_char_name(c: char) -> bool {
    is_alphanumeric(c as u8) || c == '_'
}

fn spaces(i: &str) -> IResult<&str, &str> {
    let chars = " \t\r\n";
    take_while(move |c| chars.contains(c))(i)
}

fn parser_name(input: &str) -> IResult<&str, &str> {
    take_while1(is_valid_char_name)(input)
}

fn parser_key_value(input: &str) -> IResult<&str, KeyValue> {
    let pkey = preceded(spaces, parser_name);
    let pseparator = cut(preceded(spaces, char(':')));
    let pvalue = preceded(spaces, take_till1(|c| is_space(c as u8)));
    let (remaining, (key, value)) = separated_pair(pkey, pseparator, pvalue)(input)?;
    Ok((remaining, KeyValue(key.to_string(), value.to_string())))
}

#[cfg(test)]
mod tests {
    use super::*;
    use helpers::*;

    #[googletest::test]
    fn parsing_a_valid_name() {
        let input = "conf_key1";
        let result = parser_name(input);
        let expected = "conf_key1";
        assert_parsed(result, expected);
    }

    #[googletest::test]
    fn parsing_a_key_value_pair() {
        let input = "   conf_key : value_of_1.00 ";
        let result = parser_key_value(input);
        let expected = KeyValue("conf_key".to_string(), "value_of_1.00".to_string());
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
                    nom::Err::Error(msg) => format!("Parse error: {}", msg),
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

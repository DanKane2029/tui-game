use crate::cli::operation::Opperation;
use nom::{IResult, Parser, branch::alt, bytes::complete::tag};

pub fn parse_opperation(input: &str) -> IResult<&str, Opperation> {
    alt((parse_create_opperation, parse_delete_opperation)).parse(input)
}

pub fn parse_create_opperation(input: &str) -> IResult<&str, Opperation> {
    let (remaining_str, _) = tag("create")(input)?;
    Ok((remaining_str, Opperation::Create))
}

pub fn parse_delete_opperation(input: &str) -> IResult<&str, Opperation> {
    let (remaining_str, _) = tag("delete")(input)?;
    Ok((remaining_str, Opperation::Delete {}))
}

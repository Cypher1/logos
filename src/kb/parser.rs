//! Nom parsers for Knowledge Base structures.
//! Provides round-trip logic between Display strings and internal types.

use crate::kb::rule::Rule;
use crate::kb::tuple::{Ent, Slot, Tuple, TupleTemplate};
use nom::IResult;
use nom::Parser;
use nom::branch::alt;
use nom::bytes::complete::{tag, take_while1};
use nom::character::complete::{alphanumeric1, multispace0, multispace1};
use nom::combinator::map;
use nom::multi::separated_list0;
use nom::sequence::{delimited, preceded};

/// Parse an Entity representation.
pub fn parse_ent(input: &str) -> IResult<&str, Ent> {
    let entity = map(preceded(tag("Entity"), alphanumeric1), |s: &str| {
        let id = s.parse::<u64>().expect("Invalid Entity ID");
        Ent::Entity(id)
    });

    let tuple_ent = map(preceded(tag("Tuple"), alphanumeric1), |s: &str| {
        let id = s.parse::<u64>().expect("Invalid Tuple ID");
        Ent::Tuple(id)
    });

    let str_quoted = map(
        delimited(tag("'"), take_while1(|c| c != '\''), tag("'")),
        |s: &str| Ent::Str(s.to_string()),
    );

    let fallback = map(alphanumeric1, |s: &str| {
        if s.chars()
            .any(|c| c.is_ascii_digit() || c == ' ' || c == '\'')
        {
            Ent::Str(s.to_string())
        } else if s.chars().all(|c| c.is_ascii_digit()) {
            Ent::I64(s.parse::<i64>().unwrap())
        } else {
            Ent::Str(s.to_string())
        }
    });

    alt((entity, tuple_ent, str_quoted, fallback)).parse(input)
}

/// Parse a Slot (Constant or Placeholder).
pub fn parse_slot(input: &str) -> IResult<&str, Slot> {
    let placeholder = map(preceded(tag("?"), alphanumeric1), |s: &str| {
        Slot::Placeholder(s.to_string())
    });

    let constant_parser = map(parse_ent, Slot::Constant);

    alt((placeholder, constant_parser)).parse(input)
}

/// Parse a Tuple: "subject predicate object: confidence"
pub fn parse_tuple(input: &str) -> IResult<&str, Tuple> {
    let (i1, subject) = parse_ent(input)?;
    let (i2, _) = multispace1(i1)?;
    let (i3, predicate) = parse_ent(i2)?;
    let (i4, _) = multispace1(i3)?;
    let (i5, object) = parse_ent(i4)?;
    let (i6, _) = multispace1(i5)?;
    let (i7, _) = tag(":")(i6)?;
    let (i8, _) = multispace0(i7)?;
    let (i9, conf_raw) = take_while1(|c: char| c.is_ascii_digit() || c == '.' || c == '-')(i8)?;

    Ok((
        i9,
        Tuple {
            subject: subject.into(),
            predicate: predicate.into(),
            object: object.into(),
            confidence: conf_raw.parse::<f32>().unwrap_or(1.0),
        },
    ))
}

/// Parse a TupleTemplate: "subject predicate object: confidence"
pub fn parse_tuple_template(input: &str) -> IResult<&str, TupleTemplate> {
    let (i1, subject) = parse_slot(input)?;
    let (i2, _) = multispace1(i1)?;
    let (i3, predicate) = parse_slot(i2)?;
    let (i4, _) = multispace1(i3)?;
    let (i5, object) = parse_slot(i4)?;
    let (i6, _) = multispace1(i5)?;
    let (i7, _) = tag(":")(i6)?;
    let (i8, _) = multispace0(i7)?;
    let (i9, conf_raw) = take_while1(|c: char| c.is_ascii_digit() || c == '.' || c == '-')(i8)?;

    Ok((
        i9,
        TupleTemplate {
            subject: subject.into(),
            predicate: predicate.into(),
            object: object.into(),
            confidence: conf_raw.parse::<f32>().unwrap_or(1.0),
        },
    ))
}

/// Parse Rule definition: "Rule(Premises: [P1, P2], Conclusion: ..., Confidence: ...)"
pub fn parse_rule(input: &str) -> IResult<&str, Rule> {
    let (i1, _) = tag("Rule(")(input)?;
    let (i2, _) = tag("Premises: ")(i1)?;
    let (i3, _) = tag("[")(i2)?;
    let (i4, premises_raw) = separated_list0(
        delimited(multispace0, tag(","), multispace0),
        parse_tuple_template,
    )
    .parse(i3)?;
    let (i5, _) = tag("]")(i4)?;
    let (i6, _) = multispace1(i5)?;

    let (i7, _) = tag("Conclusion: ")(i6)?;
    let (i8, conclusion) = parse_tuple_template(i7)?;
    let (i9, _) = multispace1(i8)?;

    let (i10, _) = tag("Confidence: ")(i9)?;
    let (i11, conf_raw) = take_while1(|c: char| c.is_ascii_digit() || c == '.' || c == '-')(i10)?;
    let confidence = conf_raw.parse::<f32>().unwrap_or(1.0);

    let (i12, _) = tag(")")(i11)?;

    Ok((
        i12,
        Rule {
            premises: premises_raw,
            conclusion: vec![conclusion],
            confidence,
        },
    ))
}

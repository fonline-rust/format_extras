pub use nom::{
    self,
    branch::alt,
    call,
    combinator::{cond, cut, map, map_opt, map_parser, map_res, opt, peek, recognize, value},
    do_parse,
    error::{ErrorKind, ParseError},
    multi::{count, fold_many0, fold_many_m_n, many0, many_m_n, separated_list},
    sequence::{delimited, pair, preceded, separated_pair, terminated, tuple},
    IResult,
};
use nom::{
    error::VerboseError, AsChar, Compare, InputIter, InputLength, InputTake, InputTakeAtPosition,
    Offset, Slice,
};
pub mod complete {
    pub use nom::{
        bytes::complete::{tag, take_till, take_till1, take_while1},
        character::complete::{
            alphanumeric1, char, digit1, line_ending, multispace0, not_line_ending, one_of, space0,
            space1,
        },
    };
}
use std::ops::{Range, RangeFrom, RangeTo};
pub use std::str::FromStr;

pub use arrayvec::ArrayVec;
use complete::*;

pub trait StringLikeInput:
    InputTakeAtPosition<Item = Self::Char>
    + PartialEq
    + Copy
    + Offset
    + Slice<RangeTo<usize>>
    + Slice<RangeFrom<usize>>
    + Slice<Range<usize>>
    + InputIter<Item = Self::Char>
    + InputLength
    + Compare<&'static str>
    + InputTake
{
    type Char: AsChar + Clone;
    type ParseError;
    fn trim(self) -> Self;
    // TODO: use FromExternalError from nom 0.6+
    fn parse<T: FromStr>(self) -> Result<T, Self::ParseError>;
    fn err_to_string<T>(
        self,
        res: IResult<Self, T, VerboseError<Self>>,
    ) -> Result<(Self, T), String>;
}

impl<'a> StringLikeInput for &'a [u8] {
    type Char = u8;
    type ParseError = ();

    fn trim(self) -> Self {
        let mut bytes = self;
        while let [first, rest @ ..] = bytes {
            if first.is_ascii_whitespace() {
                bytes = rest;
            } else {
                break;
            }
        }
        while let [rest @ .., last] = bytes {
            if last.is_ascii_whitespace() {
                bytes = rest;
            } else {
                break;
            }
        }
        bytes
    }

    fn parse<T: FromStr>(self) -> Result<T, Self::ParseError> {
        let str = std::str::from_utf8(self).ok().ok_or(())?;
        FromStr::from_str(str).ok().ok_or(())
    }

    fn err_to_string<T>(
        self,
        res: IResult<Self, T, VerboseError<Self>>,
    ) -> Result<(Self, T), String> {
        nom_err_to_string_bytes(self, res)
    }
}
impl<'a> StringLikeInput for &'a str {
    type Char = char;
    type ParseError = ();

    fn trim(self) -> Self {
        self.trim()
    }

    fn parse<T: FromStr>(self) -> Result<T, Self::ParseError> {
        FromStr::from_str(self).ok().ok_or(())
    }

    fn err_to_string<T>(
        self,
        res: IResult<Self, T, VerboseError<Self>>,
    ) -> Result<(Self, T), String> {
        nom_err_to_string(self, res)
    }
}

#[allow(dead_code)]
pub fn slice_has_none<T>(slice: &[Option<T>]) -> bool {
    slice.iter().all(|item| item.is_none())
}

pub fn flag<I: Clone, O, E: ParseError<I>, F>(fun: F) -> impl Fn(I) -> IResult<I, bool, E>
where
    F: Fn(I) -> IResult<I, O, E>,
{
    move |i| map(opt(&fun), |option| option.is_some())(i)
}

pub fn unsigned_number<I: StringLikeInput, E: ParseError<I>, T: FromStr>(i: I) -> IResult<I, T, E> {
    map_res(digit1, I::parse)(i)
}

pub fn space0_number<'a, E: ParseError<&'a str>, T: FromStr>(i: &'a str) -> IResult<&'a str, T, E> {
    preceded(space0, map_res(idigit1, T::from_str))(i)
}

pub fn space1_number<'a, E: ParseError<&'a str>, T: FromStr>(i: &'a str) -> IResult<&'a str, T, E> {
    preceded(space1, map_res(idigit1, T::from_str))(i)
}

pub fn idigit1<'a, E: ParseError<&'a str>>(i: &'a str) -> IResult<&'a str, &'a str, E> {
    recognize(preceded(opt(char('-')), digit1))(i)
}

// REPLACEMENT_CHARACTER \u{FFFD}
pub fn optional_str<'a, E: ParseError<&'a str>>(
    i: &'a str,
) -> IResult<&'a str, Option<&'a str>, E> {
    alt((
        map(char('-'), |_| None),
        map(word, Some),
        map(space0, |_| None),
    ))(i)
}

pub fn opt_flatten<'a, E: ParseError<&'a str>, O, F>(
    f: F,
) -> impl Fn(&'a str) -> IResult<&'a str, Option<O>, E>
where
    F: Fn(&'a str) -> IResult<&'a str, Option<Option<O>>, E>,
{
    move |i| {
        let (i, res) = f(i)?;
        let res = match res {
            Some(Some(some)) => Some(some),
            _ => None,
        };
        Ok((i, res))
    }
}

pub fn word<'a, E: ParseError<&'a str>>(i: &'a str) -> IResult<&'a str, &'a str, E> {
    take_till1(|ch| "\r\n\t ".contains(ch))(i)
}

pub fn line<'a, E: ParseError<&'a str>>(i: &'a str) -> IResult<&'a str, &'a str, E> {
    let (rest, (line, _)) = pair(
        take_till1(|ch| "\r\n".contains(ch)),
        alt((eof, line_ending)),
    )(i)?;
    Ok((rest, line.trim()))
}

pub fn some_text<T: StringLikeInput, E: ParseError<T>>(i: T) -> IResult<T, T, E> {
    let (rest, line) = take_till1(|ch: T::Char| "\r\n".contains(ch.as_char()))(i)?;
    Ok((rest, line.trim()))
}

pub fn optional_text<T: StringLikeInput, E: ParseError<T>>(i: T) -> IResult<T, T, E> {
    let (rest, line) = take_till(|ch: T::Char| "\r\n".contains(ch.as_char()))(i)?;
    Ok((rest, line.trim()))
}

pub fn eof<'a, E: ParseError<&'a str>>(i: &'a str) -> IResult<&'a str, &'a str, E> {
    use nom::error_position;

    if i.input_len() == 0 {
        Ok((i, i))
    } else {
        Err(nom::Err::Error(error_position!(i, ErrorKind::Eof)))
    }
}

pub fn int_bool<'a, E: ParseError<&'a str>>(i: &'a str) -> IResult<&'a str, bool, E> {
    alt((value(false, char('0')), value(true, char('1'))))(i)
}

pub fn fixed_list_of_numbers<'a, E: ParseError<&'a str>, T: FromStr>(
    len: usize,
) -> impl Fn(&'a str) -> IResult<&'a str, Vec<T>, E> {
    move |i| count(space0_number, len)(i)
}

pub fn t_rn<T, E: ParseError<T>>(i: T) -> IResult<T, T, E>
where
    T: StringLikeInput,
{
    recognize(pair(space0, line_ending))(i)
}

pub fn end_of_line<'a, E: ParseError<&'a str>>(i: &'a str) -> IResult<&'a str, &'a str, E> {
    recognize(pair(space0, alt((line_ending, eof))))(i)
}

pub fn section<'a, E: ParseError<&'a str>>(
    name: &'a str,
) -> impl Fn(&'a str) -> IResult<&'a str, &'a str, E> {
    move |i| delimited(char('['), tag(name), pair(char(']'), end_of_line))(i)
}

pub fn section_ext<'a, O, F, E: ParseError<&'a str>>(
    parser: F,
) -> impl Fn(&'a str) -> IResult<&'a str, O, E>
where
    F: Fn(&'a str) -> IResult<&'a str, O, E>,
{
    move |i| delimited(char('['), &parser, pair(char(']'), end_of_line))(i)
}

pub fn curly_delimited<T: StringLikeInput, E: ParseError<T>, O, F>(
    parser: F,
) -> impl Fn(T) -> IResult<T, O, E>
where
    F: Fn(T) -> IResult<T, O, E>,
{
    move |i| delimited(char('{'), &parser, char('}'))(i)
}

pub fn space0_delimited<'a, E: ParseError<&'a str>, O, F>(
    parser: F,
) -> impl Fn(&'a str) -> IResult<&'a str, O, E>
where
    F: Fn(&'a str) -> IResult<&'a str, O, E>,
{
    move |i| delimited(space0, &parser, space0)(i)
}

pub fn not_closing_curly<I: StringLikeInput, E: ParseError<I>>(i: I) -> IResult<I, I, E> {
    take_till(|ch: I::Char| ch.as_char() == '}')(i)
}

pub fn apply<T: StringLikeInput, E: ParseError<T>, O, F>(
    i: &mut T,
    parser: F,
) -> Result<O, nom::Err<E>>
where
    F: Fn(T) -> IResult<T, O, E>,
{
    let (left, res) = parser(*i)?;
    *i = left;
    Ok(res)
}

pub fn cut_apply<T: StringLikeInput, E: ParseError<T>, O, F>(
    i: &mut T,
    parser: F,
) -> Result<O, nom::Err<E>>
where
    F: Fn(T) -> IResult<T, O, E>,
{
    let (left, res) = cut(parser)(*i)?;
    *i = left;
    Ok(res)
}

pub fn kv<'a, E: ParseError<&'a str>, O, F>(
    key: &'a str,
    parser: F,
) -> impl Fn(&'a str) -> IResult<&'a str, O, E>
where
    F: Fn(&'a str) -> IResult<&'a str, O, E>,
{
    move |i| preceded(tag(key), delimited(space1, &parser, end_of_line))(i)
}
pub fn kv_sep<'a, E: ParseError<&'a str>, O, F>(
    key: &'a str,
    sep: &'a str,
    parser: F,
) -> impl Fn(&'a str) -> IResult<&'a str, O, E>
where
    F: Fn(&'a str) -> IResult<&'a str, O, E>,
{
    move |i| {
        preceded(
            tag(key),
            delimited(tuple((space0, tag(sep), space0)), &parser, end_of_line),
        )(i)
    }
}

pub fn kv_eq<'a, E: ParseError<&'a str>, O, F>(
    key: &'a str,
    parser: F,
) -> impl Fn(&'a str) -> IResult<&'a str, O, E>
where
    F: Fn(&'a str) -> IResult<&'a str, O, E>,
{
    kv_sep(key, "=", parser)
}

pub fn kv_ext<'a, E: ParseError<&'a str>, O, O2, F, K>(
    key: K,
    parser: F,
) -> impl Fn(&'a str) -> IResult<&'a str, O, E>
where
    F: Fn(&'a str) -> IResult<&'a str, O, E>,
    K: Fn(&'a str) -> IResult<&'a str, O2, E>,
{
    move |i| preceded(&key, delimited(space1, &parser, end_of_line))(i)
}

pub fn kv_kv<'a, E: ParseError<&'a str>, O, O2, F, K>(
    key: K,
    parser: F,
) -> impl Fn(&'a str) -> IResult<&'a str, (O2, O), E>
where
    F: Fn(&'a str) -> IResult<&'a str, O, E>,
    K: Fn(&'a str) -> IResult<&'a str, O2, E>,
{
    move |i| tuple((&key, delimited(space1, &parser, end_of_line)))(i)
}

pub fn kv_kv_sep<'a, E: ParseError<&'a str>, O, O2, F, K>(
    key: K,
    sep: &'a str,
    parser: F,
) -> impl Fn(&'a str) -> IResult<&'a str, (O2, O), E>
where
    F: Fn(&'a str) -> IResult<&'a str, O, E>,
    K: Fn(&'a str) -> IResult<&'a str, O2, E>,
{
    move |i| {
        tuple((
            &key,
            delimited(tuple((space0, tag(sep), space0)), &parser, end_of_line),
        ))(i)
    }
}

pub fn key_int<'a, E: ParseError<&'a str>, O: FromStr>(
    key: &'a str,
) -> impl Fn(&'a str) -> IResult<&'a str, O, E> {
    kv(key, integer)
}

pub fn opt_kv<'a, E: ParseError<&'a str>, O, F>(
    key: &'a str,
    parser: F,
) -> impl Fn(&'a str) -> IResult<&'a str, Option<O>, E>
where
    F: Fn(&'a str) -> IResult<&'a str, O, E>,
{
    move |i| opt(preceded(tag(key), delimited(space1, &parser, end_of_line)))(i)
}

pub fn opt_kv_ext<'a, E: ParseError<&'a str>, O, O2, F, K>(
    key: K,
    parser: F,
) -> impl Fn(&'a str) -> IResult<&'a str, Option<O>, E>
where
    F: Fn(&'a str) -> IResult<&'a str, O, E>,
    K: Fn(&'a str) -> IResult<&'a str, O2, E>,
{
    move |i| opt(preceded(&key, delimited(space1, &parser, end_of_line)))(i)
}

pub fn opt_key_int<'a, E: ParseError<&'a str>, O: FromStr>(
    key: &'a str,
) -> impl Fn(&'a str) -> IResult<&'a str, Option<O>, E> {
    opt_kv(key, integer)
}

pub fn integer<'a, E: ParseError<&'a str>, T: FromStr>(i: &'a str) -> IResult<&'a str, T, E> {
    map_res(idigit1, FromStr::from_str)(i)
}

pub fn many_array<I, O, E, F, A: arrayvec::Array<Item = O>>(
    m: usize,
    f: F,
) -> impl Fn(I) -> IResult<I, ArrayVec<A>, E>
where
    I: Clone + PartialEq,
    F: Fn(I) -> IResult<I, O, E>,
    E: ParseError<I>,
{
    move |i: I| {
        if A::CAPACITY == 0 {
            return Ok((i, ArrayVec::new()));
        }

        let mut res = ArrayVec::new();
        let mut input = i.clone();
        let mut count: usize = 0;

        loop {
            let _i = input.clone();
            match f(_i) {
                Ok((i, o)) => {
                    // do not allow parsers that do not consume input (causes infinite loops)
                    if i == input {
                        return Err(nom::Err::Error(E::from_error_kind(
                            input,
                            ErrorKind::ManyMN,
                        )));
                    }

                    res.push(o);
                    input = i;
                    count += 1;

                    if count == A::CAPACITY {
                        return Ok((input, res));
                    }
                }
                Err(nom::Err::Error(e)) => {
                    if count < m {
                        return Err(nom::Err::Error(E::append(input, ErrorKind::ManyMN, e)));
                    } else {
                        return Ok((input, res));
                    }
                }
                Err(e) => {
                    return Err(e);
                }
            }
        }
    }
}

pub fn count_array<I, O, E, F, A: arrayvec::Array<Item = O>>(
    f: F,
) -> impl Fn(I) -> IResult<I, ArrayVec<A>, E>
where
    I: Clone + PartialEq,
    F: Fn(I) -> IResult<I, O, E>,
    E: ParseError<I>,
{
    move |i: I| {
        let mut input = i.clone();
        let mut res = ArrayVec::<A>::new();

        for _ in 0..A::CAPACITY {
            let input_ = input.clone();
            match f(input_) {
                Ok((i, o)) => {
                    res.push(o);
                    input = i;
                }
                Err(nom::Err::Error(e)) => {
                    return Err(nom::Err::Error(E::append(i, ErrorKind::Count, e)));
                }
                Err(e) => {
                    return Err(e);
                }
            }
        }

        Ok((input, res))
    }
}

pub fn count_cap<I, O, E, F>(f: F, count: usize) -> impl Fn(I) -> IResult<I, Vec<O>, E>
where
    I: Clone + PartialEq,
    F: Fn(I) -> IResult<I, O, E>,
    E: ParseError<I>,
{
    move |i: I| {
        let mut input = i.clone();
        let mut res = Vec::with_capacity(count);

        for _index in 0..count {
            let input_ = input.clone();
            match f(input_) {
                Ok((i, o)) => {
                    res.push(o);
                    input = i;
                }
                Err(nom::Err::Error(e)) => {
                    return Err(nom::Err::Error(E::append(i, ErrorKind::Count, e)));
                }
                Err(e) => {
                    return Err(e);
                }
            }
        }

        Ok((input, res))
    }
}

pub fn count_indexed<I, O, E, F>(f: F, count: usize) -> impl Fn(I) -> IResult<I, Vec<O>, E>
where
    I: Clone + PartialEq,
    F: Fn(I, usize) -> IResult<I, O, E>,
    E: ParseError<I>,
{
    move |i: I| {
        let mut input = i.clone();
        let mut res = Vec::with_capacity(count);

        for index in 0..count {
            let input_ = input.clone();
            match f(input_, index) {
                Ok((i, o)) => {
                    res.push(o);
                    input = i;
                }
                Err(nom::Err::Error(e)) => {
                    return Err(nom::Err::Error(E::append(i, ErrorKind::Count, e)));
                }
                Err(e) => {
                    return Err(e);
                }
            }
        }

        Ok((input, res))
    }
}

pub fn many_key_index_int<'a, E: ParseError<&'a str>>(
    prefix: &'a str,
    count: usize,
) -> impl Fn(&'a str) -> IResult<&'a str, Vec<Option<i32>>, E> {
    move |i| {
        count_indexed(
            |i, index| {
                opt_kv_ext(
                    pair(tag(prefix), char((b'0' + index as u8) as char)),
                    integer,
                )(i)
            },
            count,
        )(i)
    }
}

pub fn owned_err(
    e: nom::Err<(&str, nom::error::ErrorKind)>,
) -> nom::Err<(String, nom::error::ErrorKind)> {
    use nom::Err;
    match e {
        Err::Incomplete(n) => Err::Incomplete(n),
        Err::Failure((str, kind)) => Err::Failure((str.to_owned(), kind)),
        Err::Error((str, kind)) => Err::Error((str.to_owned(), kind)),
    }
}

pub fn err_to_kind<I, O>(res: IResult<I, O, (I, ErrorKind)>) -> Result<O, ErrorKind> {
    match res {
        Ok((_rest, val)) => Ok(val),
        Err(err) => match err {
            nom::Err::Error((_rest, err)) => Err(err),
            nom::Err::Failure((_rest, err)) => Err(err),
            nom::Err::Incomplete(_) => Err(ErrorKind::Eof),
        },
    }
}

pub fn nom_err_to_string<'a, O>(
    text: &'a str,
    res: IResult<&'a str, O, VerboseError<&'a str>>,
) -> Result<(&'a str, O), String> {
    match res {
        Ok(ok) => Ok(ok),
        Err(err) => Err({
            //println!("{:#?}", err);
            match err {
                nom::Err::Error(err) => format!("Error: {}", nom::error::convert_error(text, err)),
                nom::Err::Failure(err) => {
                    format!("Failure: {}", nom::error::convert_error(text, err))
                }
                nom::Err::Incomplete(needed) => format!("Incomplete: {:?}", needed),
            }
        }),
    }
}

pub fn nom_err_to_string_bytes<'a, O>(
    bytes: &'a [u8],
    res: IResult<&'a [u8], O, VerboseError<&'a [u8]>>,
) -> Result<(&'a [u8], O), String> {
    fn map_err<'a, F: 'a, T: 'a>(
        err: &'a VerboseError<F>,
        map: impl Fn(&'a F) -> T,
    ) -> VerboseError<T> {
        VerboseError {
            errors: err
                .errors
                .iter()
                .map(|(slice, kind)| (map(slice), kind.clone()))
                .collect(),
        }
    }
    let err_converter = |err: VerboseError<&[u8]>| {
        let string_err = map_err(&err, |slice| String::from_utf8_lossy(slice));
        let ref_err = map_err(&string_err, |cow| cow.as_ref());
        let text = String::from_utf8_lossy(bytes);
        nom::error::convert_error(&text, ref_err)
    };
    match res {
        Ok(ok) => Ok(ok),
        Err(err) => Err({
            //println!("{:#?}", err);
            match err {
                nom::Err::Error(err) => format!("Error: {}", err_converter(err)),
                nom::Err::Failure(err) => format!("Failure: {}", err_converter(err)),
                nom::Err::Incomplete(needed) => format!("Incomplete: {:?}", needed),
            }
        }),
    }
}

#[macro_export(local_inner_macros)]
macro_rules! parse_struct(
    ($input:ident, $($name:ident)::* {
       $($field:ident: $val:expr,)*
    }$(, {$($field2:ident: $val2:expr,)*})?) => {{
        let (inner_input, ($($field,)*)) = tuple((
            $($val,)*
        ))($input)?;
        (inner_input, $($name)::* {
            $($field,)*
            $($($field2: $val2,)*)?
        })
    }}
);

pub fn separated_list_first_unchecked<I, O, O2, E, F, G>(
    sep: G,
    f: F,
) -> impl Fn(I) -> IResult<I, Vec<O>, E>
where
    I: Clone + PartialEq,
    F: Fn(I) -> IResult<I, O, E>,
    G: Fn(I) -> IResult<I, O2, E>,
    E: ParseError<I>,
{
    move |i: I| {
        let mut res = Vec::new();
        let mut i = i.clone();

        match f(i.clone()) {
            Err(nom::Err::Error(_)) => return Ok((i, res)),
            Err(e) => return Err(e),
            Ok((i1, o)) => {
                //unchecked
                /*if i1 == i {
                    return Err(Err::Error(E::from_error_kind(i1, ErrorKind::SeparatedList)));
                }*/

                res.push(o);
                i = i1;
            }
        }

        loop {
            match sep(i.clone()) {
                Err(nom::Err::Error(_)) => return Ok((i, res)),
                Err(e) => return Err(e),
                Ok((i1, _)) => {
                    if i1 == i {
                        return Err(nom::Err::Error(E::from_error_kind(
                            i1,
                            ErrorKind::SeparatedList,
                        )));
                    }

                    match f(i1.clone()) {
                        Err(nom::Err::Error(_)) => return Ok((i, res)),
                        Err(e) => return Err(e),
                        Ok((i2, o)) => {
                            if i2 == i {
                                return Err(nom::Err::Error(E::from_error_kind(
                                    i2,
                                    ErrorKind::SeparatedList,
                                )));
                            }

                            res.push(o);
                            i = i2;
                        }
                    }
                }
            }
        }
    }
}

pub fn cond_err<I: Clone, O, E: ParseError<I>, F>(b: bool, f: F) -> impl Fn(I) -> IResult<I, O, E>
where
    F: Fn(I) -> IResult<I, O, E>,
{
    move |input: I| {
        if b {
            match f(input) {
                Ok((i, o)) => Ok((i, o)),
                Err(e) => Err(e),
            }
        } else {
            Err(nom::Err::Error(E::from_error_kind(
                input,
                ErrorKind::NoneOf,
            )))
        }
    }
}

#[cfg(test)]
mod tests {
    use nom::error::VerboseError;

    use super::*;
    #[test]
    fn test_idigit1() {
        let parser = idigit1::<VerboseError<&str>>;
        assert_eq!(Ok(("   ", "123456")), parser("123456   "));
        assert_eq!(Ok(("   ", "-123456")), parser("-123456   "));
    }
    #[test]
    fn test_parsed_number() {
        let parser = integer::<VerboseError<&str>, i32>;
        assert_eq!(Ok(("   ", 123456)), parser("123456   "));
        assert_eq!(Ok(("   ", -123456)), parser("-123456   "));
    }

    #[test]
    fn test_t_rn() {
        let parser = t_rn::<_, VerboseError<&str>>;
        assert_eq!(Ok(("", "\n")), parser("\n"));
        assert_eq!(Ok(("", "\r\n")), parser("\r\n"));
    }
}

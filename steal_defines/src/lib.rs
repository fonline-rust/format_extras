use std::{collections::BTreeMap, fmt, path::Path};

use anyhow::Context;
use quote::quote;

pub struct AllDefines(Vec<(String, Value)>);

impl AllDefines {
    pub fn steal_file(defines_path: impl AsRef<Path>) -> anyhow::Result<AllDefines> {
        let file = std::fs::read_to_string(defines_path.as_ref())
            .context("read file to steal defines from")?;
        Self::steal(&file)
    }
    pub fn steal(defines_text: &str) -> anyhow::Result<AllDefines> {
        let mut defines = vec![];
        for line in defines_text.lines() {
            match parse_line(line.trim()) {
                Ok((define, value)) => {
                    log::debug!("Ok, define: {define:?}, value: {value:?}, line: {line:?}");
                    defines.push((define.into(), value));
                }
                Err(err) => {
                    log::log!(err.log_level(), "Err: {err:#}, line: {line:?}");
                }
            }
        }
        Ok(AllDefines(defines))
    }
    pub fn distil_enum<I: EnumIndex, E: DistilEnum<I>>(&self, prefix: &str, distil: E, default_option: EnumOptions) -> StolenEnum<'_, I> {
        let mut map = BTreeMap::default();
        let iter = distil.transform(prefix, self.0.iter().map(|(define, value)| (define.as_str(), *value)));
        for (define, value) in iter {
            if let Some(index) = distil.to_index(define, value, map.keys().next_back().copied()) {
                if let Some((old, _)) = map.insert(index, (define, default_option)) {
                    log::warn!("#{index}: {old:?} is replaced with {define:?}");
                }
            } else {
                log::debug!("{define:?} = {value:?}: discarded index");
            }
        }

        StolenEnum {
            map,
            prefix: prefix.into(),
        }
    }
}

pub trait DistilEnum<I> {
    fn transform<'a, 's>(&'a self, prefix: &'a str, iter: impl 'a + Iterator<Item = (&'s str, Value)>) -> impl 'a + Iterator<Item = (&'s str, Value)>;
    fn to_index(&self, define: &str, value: Value, last: Option<I>) -> Option<I>;
}

#[derive(Default)]
pub struct DistilDecEnum{
    //until: Option<String>,
}

impl<I: EnumIndex> DistilEnum<I> for DistilDecEnum {
    fn transform<'a, 's>(&'a self, prefix: &'a str, iter: impl 'a + Iterator<Item = (&'s str, Value)>) -> impl 'a + Iterator<Item = (&'s str, Value)> {
        iter
            //.take_while(|(define, _)| Some(*define) != self.until.as_deref())
            .filter_map(move |(define, value)| define.strip_prefix(prefix).map(|define| (define, value)))
    }
    fn to_index(&self, define: &str, value: Value, last: Option<I>) -> Option<I> {
        let Value::Dec(dec) = value else {
            return None;
        };
        let index: I = dec.try_into().ok()?;
        if last.is_some_and(|last| index < last) {
            log::warn!(
                "#{index}: {define:?} is smaller than last {}",
                last.unwrap()
            );
        }
        Some(index)
    }
    
}

pub trait EnumIndex: Copy + Ord + TryFrom<i64> + fmt::Display + quote::ToTokens {
    const TYPE_NAME: &'static str;
}

macro_rules! impl_enum_index {
    ($($ty:ident)*) => {
        $(
            impl EnumIndex for $ty {
                const TYPE_NAME: &'static str = stringify!($ty);
            }
        )*
    };
}
impl_enum_index!(usize u64 u32 u16 u8 isize i64 i32 i16 i8);

#[derive(Clone, Copy)]
pub struct EnumOptions {
    pub variant: bool,
    pub constant: bool,
}

pub struct StolenEnum<'a, I> {
    map: BTreeMap<I, (&'a str, EnumOptions)>,
    prefix: String,
}

impl<I: EnumIndex> StolenEnum<'_, I> {
    pub fn unparse(&self, name: &str) -> anyhow::Result<String> {
        param_rs(
            name,
            &self.prefix,
            self.map.iter().map(|(index, define)| (*index, *define)),
        )
    }
    pub fn split(&mut self, prefix: &str) -> Self {
        let mut map = BTreeMap::default();
        self.map.retain(|index, (define, options)| {
            if let Some(define) = define.strip_prefix(prefix) {
                map.insert(*index, (define, *options));
                false
            } else {
                true
            }
        });
        Self {
            map,
            prefix: format!("{}{prefix}", self.prefix),
        }
    }
    pub fn last(&mut self) -> Option<(I, &'_ str, &mut EnumOptions)> {
        let (value, (define, options)) = self.map.iter_mut().last()?;
        Some((*value, *define, options))
    }
    pub fn count(&self) -> usize {
        self.map.len()
    }
    /*
    pub fn set_option(&mut self, full_define: &str, set_options: EnumOptions) -> anyhow::Result<()> {
        if let Some(needle) = full_define.strip_prefix(&self.prefix) {
            let (_, options) = self.map.values_mut().find(|(define, _)| *define == needle).context("can't find define")?;
            *options = set_options;
            Ok(())
        } else {
            anyhow::bail!("can't strip prefix {} from {}", self.prefix, full_define);
        }
    }
    pub fn set_last_option(&mut self, ensure_define: Option<&str>, set_options: EnumOptions) -> anyhow::Result<()> {
        let (last_define, last_option) = self.map.values_mut().last().context("empty enum")?;
        if let Some(ensure_define) = ensure_define {
            if let Some(check) = ensure_define.strip_prefix(&self.prefix) {
                if *last_define != check {
                    anyhow::bail!("ensure failed: {} != {}", last_define, check);
                }
            } else {
                anyhow::bail!("can't strip prefix {} from {}", self.prefix, ensure_define);
            }
        }
        *last_option = set_options;
        Ok(())
    }
    */
}

#[derive(Debug, thiserror::Error)]
enum Error {
    #[error("not a define")]
    NotDefine,
    #[error("define without second whitspace")]
    NoSpace,
    #[error("define without parnthesis")]
    NoParenthesis,
    #[error("define with non-number value")]
    NonNumber,
    #[error("define is macro")]
    Macro,
    #[error("define is include guard")]
    IncludeGuard,
    #[error("can't convert float to uint")]
    FloatToUint,
    #[error("can't convert int to uint")]
    IntToUint,
}

impl Error {
    fn log_level(&self) -> log::Level {
        match self {
            Self::NotDefine | Self::Macro | Self::IncludeGuard => log::Level::Debug,
            Self::NoParenthesis | Self::NoSpace | Self::IntToUint => log::Level::Error,
            Self::NonNumber | Self::FloatToUint => log::Level::Warn,
        }
    }
}

fn parens(input: &str) -> Option<&str> {
    let (value, _rest) = input.strip_prefix('(')?.split_once(')')?;
    Some(value.trim())
}

fn value(input: &str) -> Result<Value, Error> {
    let input = input.trim_start();
    if input.starts_with('#') {
        Err(Error::Macro)
    } else if let Some(input) = input.strip_prefix("uint") {
        let raw_value = parens(input.trim_start()).ok_or(Error::NoParenthesis)?;
        parse_value(raw_value)?.to_uint32()
    } else {
        let raw_value = parens(input).ok_or(Error::NoParenthesis)?;
        parse_value(raw_value)
    }
}

fn parse_value(raw_value: &str) -> Result<Value, Error> {
    if let Some(hex) = raw_value.strip_prefix("0x") {
        i64::from_str_radix(hex, 16).ok().map(Value::Hex)
    } else if raw_value.contains('.') {
        raw_value
            .trim_end_matches('f')
            .parse()
            .ok()
            .map(Value::Float)
    } else {
        raw_value.parse().ok().map(Value::Dec)
    }
    .ok_or(Error::NonNumber)
}

#[derive(Debug, Clone, Copy)]
#[allow(dead_code)]
pub enum Value {
    Dec(i64),
    Hex(i64),
    Float(f64),
}

impl Value {
    fn to_uint32(self) -> Result<Value, Error> {
        match self {
            Value::Dec(int) if int < i32::MIN as i64 => Err(Error::IntToUint),
            Value::Dec(int) if int.is_negative() => Ok(Value::Dec(int + u32::MAX as i64)),
            Value::Hex(_) => Err(Error::IntToUint),
            Value::Dec(_) => Ok(self),
            Value::Float(_) => Err(Error::FloatToUint),
        }
    }
}

fn parse_line(line: &str) -> Result<(&str, Value), Error> {
    let rest = line
        .strip_prefix("#define ")
        .ok_or(Error::NotDefine)?
        .trim_start();
    if rest.starts_with("__") && rest.ends_with("__") {
        return Err(Error::IncludeGuard);
    }
    let (define, rest) = rest
        .split_once(|c: char| c.is_whitespace())
        .ok_or(Error::NoSpace)?;
    let value = value(rest)?;
    Ok((define, value))
}

fn param_rs<'a, I: EnumIndex>(
    name: &str,
    prefix: &str,
    iter: impl Iterator<Item = (I, (&'a str, EnumOptions))>,
) -> anyhow::Result<String> {
    use convert_case::{Case, Casing};
    let name = quote::format_ident!("{name}");
    let mut variant_to_value = vec![];
    let mut value_to_variant = vec![];
    let mut variant_as_str = vec![];
    let mut constants = vec![];

    let index_repr = quote::format_ident!("{}", I::TYPE_NAME);
    for (value, (define, options)) in iter {
        let variant = options.variant.then(|| quote::format_ident!("{}", define.to_case(Case::Pascal)));
        let constant = options.constant.then(|| quote::format_ident!("{prefix}{define}"));
        if let Some(variant) = &variant {
            variant_to_value.push(quote! { #variant = #value, });
            value_to_variant.push(quote! { #value => Self::#variant, });
            let original = format!("{prefix}{define}");
            variant_as_str.push(quote! { Self::#variant => #original, });
            if let Some(constant) = constant {
                constants.push(quote! { pub const #constant: Self = Self::#variant; });
            }
        } else if let Some(constant) = constant {
            constants.push(quote! { pub const #constant: #index_repr = #value; });
        }        
    }
    
    let tokens = quote::quote!(
        #[allow(dead_code)]
        #[repr(#index_repr)]
        #[derive(Debug, PartialOrd, Ord, PartialEq, Eq, Clone, Copy)]
        pub enum #name {
            #(
                #variant_to_value
            )*
        }
        #[allow(dead_code)]
        impl #name {
            #(
                #constants
            )*
            pub fn try_from_index(index: #index_repr) -> Option<Self> {
                Some(match index {
                    #(
                        #value_to_variant
                    )*
                    _ => return None,
                })
            }
            pub fn as_str(&self) -> &'static str {
                match self {
                    #(
                        #variant_as_str
                    )*
                }
            }
        }
    );
    let syntax_tree = syn::parse2(tokens).context("parse token stream as file")?;
    Ok(prettyplease::unparse(&syntax_tree))
}

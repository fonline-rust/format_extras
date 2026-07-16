use std::{
    collections::BTreeMap,
    fmt::{self, UpperHex},
    ops::Bound,
    path::Path,
};

use anyhow::{Context, bail};
use proc_macro2::{Literal, TokenStream};
use quote::quote;
use syn::{Ident, parse::Parser as _};

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

    pub fn distil_enum<I: EnumIndex, E: DistilEnum<I>>(
        &self,
        prefix: &str,
        distil: E,
        default_options: EnumOptions,
    ) -> StolenEnum<I> {
        let mut map = BTreeMap::default();
        let iter = distil.transform(
            prefix,
            self.0
                .iter()
                .map(|(define, value)| (define.as_str(), *value)),
        );
        for (define, value) in iter {
            if let Some(index) = distil.to_index(define, value, map.keys().next_back().copied()) {
                let mut options = default_options;
                if options.index_formatting.is_none() {
                    options.index_formatting = IndexFormatting::from_value(value, index);
                }
                if let Some((old, _)) = map.insert(index, (define.to_string(), options)) {
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

    pub fn distil_constants<T: ConstantValue>(
        &self,
        names: &[impl AsRef<str>],
    ) -> anyhow::Result<StolenConstants> {
        let mut stolen: BTreeMap<_, _> = names.iter().map(|str| (str.as_ref(), None)).collect();
        for &(ref name, value) in &self.0 {
            if let Some(stolen) = stolen.get_mut(name.as_str()) {
                if stolen.is_some() {
                    bail!("two constants with the same name: {name}");
                }
                if let Some(constant) = T::try_from_value(value) {
                    *stolen = Some(StolenConstant {
                        name: quote::format_ident!("{name}"),
                        repr: quote::format_ident!("{}", T::TYPE_NAME),
                        value: constant.format(value).with_context(|| {
                            format!("can't re-parse literal: {constant}, value: {value:?}")
                        })?,
                    });
                } else {
                    anyhow::bail!(
                        "can't convert constant {name:?} from {value:?} to {}",
                        std::any::type_name::<T>()
                    );
                }
            }
        }
        let mut constants = Vec::with_capacity(stolen.len());
        for (name, value) in stolen {
            if let Some(value) = value {
                constants.push(value);
            } else {
                anyhow::bail!("can't find constant {name:?}");
            }
        }
        Ok(StolenConstants { constants })
    }
}

#[derive(Default)]
pub struct StolenConstants {
    constants: Vec<StolenConstant>,
}
impl StolenConstants {
    pub fn join(mut self, mut other: Self) -> Self {
        self.constants.append(&mut other.constants);
        self
    }

    pub fn distil_from<T: ConstantValue>(
        mut self,
        defines: &AllDefines,
        names: &[impl AsRef<str>],
    ) -> anyhow::Result<Self> {
        let constants = defines.distil_constants::<T>(names)?;
        Ok(self.join(constants))
    }
}

struct StolenConstant {
    name: Ident,
    repr: Ident,
    value: Literal,
}

impl Unparse for StolenConstants {
    fn to_tokens(&self, name: &str, attributes: &[syn::Attribute]) -> anyhow::Result<TokenStream> {
        let module = quote::format_ident!("{name}");
        let mut constants = vec![];

        for StolenConstant { name, repr, value } in &self.constants {
            constants.push(quote::quote!(pub const #name: #repr = #value;));
        }

        Ok(quote::quote!(
            #(#attributes)*
            pub mod #module {
                #(
                    #constants
                )*
            }
        ))
    }
}

pub trait ConstantValue: Sized + Copy + fmt::Display + quote::ToTokens {
    const TYPE_NAME: &'static str;

    fn try_from_value(value: Value) -> Option<Self>;
    fn format(&self, value: Value) -> Option<Literal>;
}

macro_rules! impl_constant_value {
    (int; $($ty:ident)*) => {
        $(
            impl ConstantValue for $ty {
                const TYPE_NAME: &'static str = stringify!($ty);

                fn try_from_value(value: Value) -> Option<Self> {
                    match value {
                        Value::Dec(num) => num.try_into().ok(),
                        Value::Hex(num) => num.try_into().ok(),
                        Value::Float(_) => None,
                    }
                }
                fn format(&self, value: Value) -> Option<Literal> {
                    if matches!(value, Value::Hex(_)) {
                        format!("0x{self:X}{}", <$ty as ConstantValue>::TYPE_NAME).parse::<Literal>().ok()
                    } else {
                        format!("{self}{}", <$ty as ConstantValue>::TYPE_NAME).parse::<Literal>().ok()
                    }
                }
            }
        )*
    };
    (float; $($ty:ident)*) => {
        $(
            impl ConstantValue for $ty {
                const TYPE_NAME: &'static str = stringify!($ty);

                fn try_from_value(value: Value) -> Option<Self> {
                    match value {
                        Value::Float(num) => Some(num as Self),
                        Value::Dec(_) | Value::Hex(_) => None,
                    }
                }

                fn format(&self, _value: Value) -> Option<Literal> {
                    format!("{self}{}", <$ty as ConstantValue>::TYPE_NAME).parse::<Literal>().ok()
                }
            }
        )*
    };
}
impl_constant_value!(int; usize u64 u32 u16 u8 isize i64 i32 i16 i8);
impl_constant_value!(float; f32 f64);

pub trait DistilEnum<I> {
    fn transform<'a, 's>(
        &'a self,
        prefix: &'a str,
        iter: impl 'a + Iterator<Item = (&'s str, Value)>,
    ) -> impl 'a + Iterator<Item = (&'s str, Value)>;
    fn to_index(&self, define: &str, value: Value, last: Option<I>) -> Option<I>;
}

pub struct DistilDecEnum {
    pub allow_hex: bool,
    pub allow_dec: bool,
    //until: Option<String>,
}

impl Default for DistilDecEnum {
    fn default() -> Self {
        Self {
            allow_hex: false,
            allow_dec: true,
        }
    }
}

impl<I: EnumIndex> DistilEnum<I> for DistilDecEnum {
    fn transform<'a, 's>(
        &'a self,
        prefix: &'a str,
        iter: impl 'a + Iterator<Item = (&'s str, Value)>,
    ) -> impl 'a + Iterator<Item = (&'s str, Value)> {
        iter
            //.take_while(|(define, _)| Some(*define) != self.until.as_deref())
            .filter_map(move |(define, value)| {
                define.strip_prefix(prefix).map(|define| (define, value))
            })
    }

    fn to_index(&self, define: &str, value: Value, last: Option<I>) -> Option<I> {
        let value = match value {
            Value::Dec(value) if self.allow_dec => value,
            Value::Hex(value) if self.allow_hex => value,
            _ => return None,
        };
        let index: I = value.try_into().ok()?;
        if last.is_some_and(|last| index < last) {
            log::warn!(
                "#{index}: {define:?} is smaller than last {}",
                last.unwrap()
            );
        }
        Some(index)
    }
}

pub trait EnumIndex:
    Copy + Ord + TryFrom<i128> + TryInto<i128> + fmt::Display + quote::ToTokens + UpperHex
{
    const TYPE_NAME: &'static str;
    const MAX: Self;

    fn format_index(&self, how: IndexFormatting) -> Literal {
        match how {
            IndexFormatting::Hex { padding } => {
                format!("0x{self:0width$X}", width = padding as usize)
                    .parse::<Literal>()
                    .unwrap()
            }
            IndexFormatting::Dec => format!("{self}").parse::<Literal>().unwrap(),
        }
    }
}

#[test]
fn test_hex_padding() {
    let index = 1u8;
    let formatting = IndexFormatting::from_value(Value::Hex(index as _), index).unwrap();
    assert_eq!(&index.format_index(formatting).to_string(), "0x01")
}

macro_rules! impl_enum_index {
    ($($ty:ident)*) => {
        $(
            impl EnumIndex for $ty {
                const TYPE_NAME: &'static str = stringify!($ty);
                const MAX: Self = Self::MAX;
            }
        )*
    };
}
impl_enum_index!(usize u64 u32 u16 u8 isize i64 i32 i16 i8);

#[derive(Clone, Copy)]
pub struct EnumOptions {
    pub variant: bool,
    pub constant: bool,
    pub index_formatting: Option<IndexFormatting>,
}

#[derive(Clone, Copy, Default)]
pub enum IndexFormatting {
    #[default]
    Dec,
    Hex {
        padding: u32,
    },
}

impl IndexFormatting {
    fn from_value<E: EnumIndex>(value: Value, _enum_index: E) -> Option<Self> {
        match value {
            Value::Dec(..) => Some(IndexFormatting::Dec),
            Value::Hex(..) => {
                let max: i128 = E::MAX.try_into().ok().unwrap();
                if max < 0 {
                    None
                } else {
                    let padding = ((max as u128).next_power_of_two() - 1).trailing_ones() / 4;
                    Some(IndexFormatting::Hex { padding })
                }
            }
            Value::Float(..) => None,
        }
    }
}

pub struct StolenEnum<I> {
    map: BTreeMap<I, (String, EnumOptions)>,
    prefix: String,
}

pub trait Unparse {
    fn to_tokens(&self, name: &str, attributes: &[syn::Attribute]) -> anyhow::Result<TokenStream>;
    fn unparse(&self, name: &str, attributes: &[impl AsRef<str>]) -> anyhow::Result<String> {
        let mut attrs = vec![];
        for attr in attributes {
            attrs.append(
                &mut syn::Attribute::parse_outer
                    .parse_str(attr.as_ref())
                    .context("parse attributes")?,
            );
        }
        let tokens = self.to_tokens(name, &attrs)?;
        if let Ok(syntax_tree) = syn::parse2(tokens.clone()) {
            Ok(prettyplease::unparse(&syntax_tree))
        } else {
            Ok(tokens.to_string())
        }
    }
}

impl<I: EnumIndex> StolenEnum<I> {
    pub fn split(&mut self, prefix: &str) -> Self {
        let mut map = BTreeMap::default();
        self.map.retain(|index, (define, options)| {
            if let Some(define) = define.strip_prefix(prefix) {
                map.insert(*index, (define.to_string(), *options));
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

    pub fn get_value_with_prefix(&self, define_to_find: &str) -> anyhow::Result<I> {
        let to_find = define_to_find.strip_prefix(&self.prefix).with_context(|| {
            format!(
                "can't strip prefix ({:?}) from define {define_to_find:?}",
                self.prefix
            )
        })?;
        let (value, _) = self
            .map
            .iter()
            .find(|(_, (define, _))| *define == to_find)
            .with_context(|| format!("can't find {define_to_find:?}"))?;
        Ok(*value)
    }

    pub fn get_next(&self, index: I) -> Option<(&str, I)> {
        let (value, (key, _)) = self.map.range(index..).next()?;
        Some((key.as_str(), *value))
    }

    pub fn get_mut_without_prefix(
        &mut self,
        define_to_find: &str,
    ) -> Option<(I, &'_ str, &mut EnumOptions)> {
        let (value, (define, options)) = self
            .map
            .iter_mut()
            .find(|(_, (define, _))| *define == define_to_find)?;
        Some((*value, define.as_str(), options))
    }

    pub fn last(&mut self) -> Option<(I, &'_ str, &mut EnumOptions)> {
        let (value, (define, options)) = self.map.iter_mut().last()?;
        Some((*value, define.as_str(), options))
    }

    pub fn count(&self) -> usize {
        self.map.len()
    }

    pub fn without<S: AsRef<str>>(
        mut self,
        defines: impl IntoIterator<Item = S>,
    ) -> anyhow::Result<Self> {
        for without in defines.into_iter() {
            let without = without.as_ref();
            let without_prefix = without.strip_prefix(&self.prefix).with_context(|| {
                format!(
                    "can't strip prefix ({:?}) from without {without:?}",
                    self.prefix
                )
            })?;
            let (_last_value, _define, option) = self
                .get_mut_without_prefix(without_prefix)
                .with_context(|| format!("can't find without {without:?}"))?;
            option.variant = false;
        }
        Ok(self)
    }

    pub fn with<S: AsRef<str>>(
        mut self,
        defines: impl IntoIterator<Item = (S, I, EnumOptions)>,
    ) -> anyhow::Result<Self> {
        for (define, key, options) in defines {
            let define = define.as_ref();
            let value = define.strip_prefix(&self.prefix).with_context(|| {
                format!(
                    "can't strip prefix ({:?}) from define {define:?}",
                    self.prefix
                )
            })?;

            if let Some((old, _)) = self.map.insert(key, (value.to_string(), options)) {
                bail!("#{key}: {old:?} is replaced with {define:?}");
            }
        }
        Ok(self)
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
        i128::from_str_radix(hex, 16).ok().map(Value::Hex)
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
    Dec(i128),
    Hex(i128),
    Float(f64),
}

impl Value {
    fn to_uint32(self) -> Result<Value, Error> {
        match self {
            Value::Dec(int) if int < i32::MIN as i128 => Err(Error::IntToUint),
            Value::Dec(int) if int.is_negative() => Ok(Value::Dec(int + u32::MAX as i128)),
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

fn mask_keywords(word: &str) -> &str {
    match word {
        "Self" => "This",
        _ => word,
    }
}

fn to_pascal_ident(define: &str) -> syn::Ident {
    use convert_case::{Case, Casing};
    let pascal = define.to_case(Case::Pascal);
    let masked = mask_keywords(&pascal);
    quote::format_ident!("{masked}")
}

impl<I: EnumIndex> Unparse for StolenEnum<I> {
    fn to_tokens(&self, name: &str, attributes: &[syn::Attribute]) -> anyhow::Result<TokenStream> {
        let prefix = &self.prefix;
        let iter = self
            .map
            .iter()
            .map(|(index, (define, options))| (*index, define.as_str(), *options));

        let name = quote::format_ident!("{name}");
        let mut variant_to_value = vec![];
        let mut value_to_variant = vec![];
        let mut variant_as_str = vec![];
        let mut constants = vec![];

        let index_repr = quote::format_ident!("{}", I::TYPE_NAME);
        for (value, define, options) in iter {
            let value = value.format_index(options.index_formatting.unwrap_or_default());
            let variant = options.variant.then(|| to_pascal_ident(define));
            let constant = options
                .constant
                .then(|| quote::format_ident!("{prefix}{define}"));
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

        Ok(quote::quote!(
            #[allow(dead_code)]
            #[repr(#index_repr)]
            #[derive(Debug, PartialOrd, Ord, PartialEq, Eq, Clone, Copy)]
            #(#attributes)*
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
                pub const fn try_from_index(index: #index_repr) -> Option<Self> {
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
        ))
    }
}

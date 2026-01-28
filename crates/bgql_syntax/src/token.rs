//! Token kinds and structures for Better GraphQL.

use bgql_core::Span;

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

/// The kind of a token in Better GraphQL.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[repr(u8)]
pub enum TokenKind {
    // Special tokens
    Eof,
    Error,
    Whitespace,
    Newline,
    Comment,

    // Literals
    Ident,
    IntLiteral,
    FloatLiteral,
    StringLiteral,
    BlockStringLiteral,

    // Keywords - Type definitions
    Type,
    Interface,
    Union,
    Enum,
    Input,
    Scalar,
    Schema,
    Extend,
    Implements,
    Fragment,
    On,
    Directive,
    Opaque,

    // Keywords - Operations
    Query,
    Mutation,
    Subscription,

    // Keywords - Type modifiers
    Option,
    List,
    Extends,

    // Keywords - Type alias
    Alias,

    // Keywords - Directive modifiers
    Repeatable,

    // Keywords - Values
    True,
    False,
    Null,

    // Keywords - Import
    Import,
    From,

    // Punctuation
    LBrace,
    RBrace,
    LParen,
    RParen,
    LBracket,
    RBracket,
    LAngle,
    RAngle,
    Colon,
    ColonColon,
    Comma,
    Dot,
    Spread,
    Eq,
    Pipe,
    Amp,
    At,
    Bang,
    Question,
    Dollar,
}

impl TokenKind {
    #[must_use]
    pub const fn is_keyword(self) -> bool {
        matches!(
            self,
            Self::Type
                | Self::Interface
                | Self::Union
                | Self::Enum
                | Self::Input
                | Self::Scalar
                | Self::Schema
                | Self::Extend
                | Self::Implements
                | Self::Fragment
                | Self::On
                | Self::Directive
                | Self::Opaque
                | Self::Query
                | Self::Mutation
                | Self::Subscription
                | Self::Option
                | Self::List
                | Self::Extends
                | Self::Alias
                | Self::Repeatable
                | Self::True
                | Self::False
                | Self::Null
                | Self::Import
                | Self::From
        )
    }

    #[must_use]
    pub const fn is_trivia(self) -> bool {
        matches!(self, Self::Whitespace | Self::Newline | Self::Comment)
    }

    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Eof => "<eof>",
            Self::Error => "<error>",
            Self::Whitespace => "<whitespace>",
            Self::Newline => "<newline>",
            Self::Comment => "<comment>",
            Self::Ident => "<ident>",
            Self::IntLiteral => "<int>",
            Self::FloatLiteral => "<float>",
            Self::StringLiteral => "<string>",
            Self::BlockStringLiteral => "<block-string>",
            Self::Type => "type",
            Self::Interface => "interface",
            Self::Union => "union",
            Self::Enum => "enum",
            Self::Input => "input",
            Self::Scalar => "scalar",
            Self::Schema => "schema",
            Self::Extend => "extend",
            Self::Implements => "implements",
            Self::Fragment => "fragment",
            Self::On => "on",
            Self::Directive => "directive",
            Self::Opaque => "opaque",
            Self::Query => "query",
            Self::Mutation => "mutation",
            Self::Subscription => "subscription",
            Self::Option => "Option",
            Self::List => "List",
            Self::Extends => "extends",
            Self::Alias => "alias",
            Self::Repeatable => "repeatable",
            Self::True => "true",
            Self::False => "false",
            Self::Null => "null",
            Self::Import => "import",
            Self::From => "from",
            Self::LBrace => "{",
            Self::RBrace => "}",
            Self::LParen => "(",
            Self::RParen => ")",
            Self::LBracket => "[",
            Self::RBracket => "]",
            Self::LAngle => "<",
            Self::RAngle => ">",
            Self::Colon => ":",
            Self::ColonColon => "::",
            Self::Comma => ",",
            Self::Dot => ".",
            Self::Spread => "...",
            Self::Eq => "=",
            Self::Pipe => "|",
            Self::Amp => "&",
            Self::At => "@",
            Self::Bang => "!",
            Self::Question => "?",
            Self::Dollar => "$",
        }
    }

    #[must_use]
    pub fn from_keyword(s: &str) -> Option<Self> {
        match s {
            "type" => Some(Self::Type),
            "interface" => Some(Self::Interface),
            "union" => Some(Self::Union),
            "enum" => Some(Self::Enum),
            "input" => Some(Self::Input),
            "scalar" => Some(Self::Scalar),
            "schema" => Some(Self::Schema),
            "extend" => Some(Self::Extend),
            "implements" => Some(Self::Implements),
            "fragment" => Some(Self::Fragment),
            "on" => Some(Self::On),
            "directive" => Some(Self::Directive),
            "opaque" => Some(Self::Opaque),
            "query" => Some(Self::Query),
            "mutation" => Some(Self::Mutation),
            "subscription" => Some(Self::Subscription),
            "Option" => Some(Self::Option),
            "List" => Some(Self::List),
            "extends" => Some(Self::Extends),
            "alias" => Some(Self::Alias),
            "repeatable" => Some(Self::Repeatable),
            "true" => Some(Self::True),
            "false" => Some(Self::False),
            "null" => Some(Self::Null),
            "import" => Some(Self::Import),
            "from" => Some(Self::From),
            _ => None,
        }
    }
}

impl std::fmt::Display for TokenKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// A token with its kind and source span.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct Token {
    pub kind: TokenKind,
    pub span: Span,
}

impl Token {
    #[must_use]
    #[inline]
    pub const fn new(kind: TokenKind, span: Span) -> Self {
        Self { kind, span }
    }

    #[must_use]
    #[inline]
    pub const fn is_eof(&self) -> bool {
        matches!(self.kind, TokenKind::Eof)
    }

    #[must_use]
    #[inline]
    pub const fn is_trivia(&self) -> bool {
        self.kind.is_trivia()
    }

    #[must_use]
    #[inline]
    pub const fn len(&self) -> u32 {
        self.span.len()
    }

    #[must_use]
    #[inline]
    pub const fn is_empty(&self) -> bool {
        self.span.is_empty()
    }
}

/// Directive locations.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum DirectiveLocation {
    // Type system
    Schema,
    Scalar,
    Object,
    FieldDefinition,
    ArgumentDefinition,
    Interface,
    Union,
    Enum,
    EnumValue,
    InputObject,
    InputFieldDefinition,

    // Executable
    Query,
    Mutation,
    Subscription,
    Field,
    FragmentDefinition,
    FragmentSpread,
    InlineFragment,
    VariableDefinition,
}

impl DirectiveLocation {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Schema => "SCHEMA",
            Self::Scalar => "SCALAR",
            Self::Object => "OBJECT",
            Self::FieldDefinition => "FIELD_DEFINITION",
            Self::ArgumentDefinition => "ARGUMENT_DEFINITION",
            Self::Interface => "INTERFACE",
            Self::Union => "UNION",
            Self::Enum => "ENUM",
            Self::EnumValue => "ENUM_VALUE",
            Self::InputObject => "INPUT_OBJECT",
            Self::InputFieldDefinition => "INPUT_FIELD_DEFINITION",
            Self::Query => "QUERY",
            Self::Mutation => "MUTATION",
            Self::Subscription => "SUBSCRIPTION",
            Self::Field => "FIELD",
            Self::FragmentDefinition => "FRAGMENT_DEFINITION",
            Self::FragmentSpread => "FRAGMENT_SPREAD",
            Self::InlineFragment => "INLINE_FRAGMENT",
            Self::VariableDefinition => "VARIABLE_DEFINITION",
        }
    }

    /// Parses a directive location from a string.
    #[must_use]
    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "SCHEMA" => Some(Self::Schema),
            "SCALAR" => Some(Self::Scalar),
            "OBJECT" => Some(Self::Object),
            "FIELD_DEFINITION" => Some(Self::FieldDefinition),
            "ARGUMENT_DEFINITION" => Some(Self::ArgumentDefinition),
            "INTERFACE" => Some(Self::Interface),
            "UNION" => Some(Self::Union),
            "ENUM" => Some(Self::Enum),
            "ENUM_VALUE" => Some(Self::EnumValue),
            "INPUT_OBJECT" => Some(Self::InputObject),
            "INPUT_FIELD_DEFINITION" => Some(Self::InputFieldDefinition),
            "QUERY" => Some(Self::Query),
            "MUTATION" => Some(Self::Mutation),
            "SUBSCRIPTION" => Some(Self::Subscription),
            "FIELD" => Some(Self::Field),
            "FRAGMENT_DEFINITION" => Some(Self::FragmentDefinition),
            "FRAGMENT_SPREAD" => Some(Self::FragmentSpread),
            "INLINE_FRAGMENT" => Some(Self::InlineFragment),
            "VARIABLE_DEFINITION" => Some(Self::VariableDefinition),
            _ => None,
        }
    }
}

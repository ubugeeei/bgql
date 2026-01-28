; Keywords
[
  "type"
  "interface"
  "enum"
  "union"
  "input"
  "scalar"
  "opaque"
  "directive"
  "extend"
  "schema"
  "query"
  "mutation"
  "subscription"
  "fragment"
  "on"
  "implements"
  "repeatable"
] @keyword

; Type keywords
[
  "Option"
  "List"
] @keyword.type

; Boolean literals
[
  "true"
  "false"
] @constant.builtin.boolean

; Null literal
"null" @constant.builtin

; Operators and punctuation
[
  "="
  ":"
  "|"
  "&"
  "!"
  "@"
  "..."
] @operator

[
  "("
  ")"
  "["
  "]"
  "{"
  "}"
  "<"
  ">"
] @punctuation.bracket

"," @punctuation.delimiter

; Comments
(comment) @comment

; Strings
(string_value) @string
(description) @string.doc

; Numbers
(int_value) @number
(float_value) @number

; Types
(type_name) @type
(named_type) @type

; Fields and arguments
(field_definition
  name: (name) @property)

(argument_definition
  name: (name) @variable.parameter)

(input_value_definition
  name: (name) @variable.parameter)

; Directives
(directive
  "@" @attribute
  name: (name) @attribute)

(directive_definition
  "@" @attribute
  name: (name) @attribute)

; Enum values
(enum_value_definition
  name: (name) @constant)

; Variable references
(variable
  "$" @variable
  name: (name) @variable)

; Fragment references
(fragment_spread
  "..." @operator
  name: (name) @label)

; Operation names
(operation_definition
  name: (name) @function)

; Generic type parameters
(generic_type
  name: (name) @type
  arguments: (type_arguments
    (type) @type.parameter))

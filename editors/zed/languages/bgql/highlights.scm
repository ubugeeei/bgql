; Keywords
[
  "type"
  "interface"
  "union"
  "enum"
  "input"
  "scalar"
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

; Built-in types
((type_name) @type.builtin
  (#any-of? @type.builtin "Int" "Float" "String" "Boolean" "ID"))

; Type names
(type_name) @type

; Field names
(field_definition
  name: (name) @property)

(field
  name: (name) @property)

; Argument names
(argument
  name: (name) @variable.parameter)

(input_value_definition
  name: (name) @variable.parameter)

; Variable names
(variable) @variable

; Directive names
(directive) @attribute

; Enum values
(enum_value_definition
  name: (name) @constant)

(enum_value) @constant

; Fragment names
(fragment_definition
  name: (name) @function)

(fragment_spread
  name: (name) @function)

; Operation names
(operation_definition
  name: (name) @function)

; Comments
(comment) @comment

; Strings
(string_value) @string
(block_string_value) @string

; Numbers
(int_value) @number
(float_value) @number

; Booleans
(boolean_value) @constant.builtin

; Null
(null_value) @constant.builtin

; Operators
[
  "!"
  "="
  ":"
  "@"
  "$"
  "&"
  "|"
  "..."
] @operator

; Punctuation
[
  "{"
  "}"
  "["
  "]"
  "("
  ")"
] @punctuation.bracket

[
  ","
] @punctuation.delimiter

; Description strings
(description) @comment.documentation

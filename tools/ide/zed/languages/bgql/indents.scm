; Indent inside blocks
[
  (object_type_definition)
  (interface_type_definition)
  (enum_type_definition)
  (input_object_type_definition)
  (union_type_definition)
  (selection_set)
  (arguments)
  (arguments_definition)
  (directive_locations)
] @indent

; Outdent on closing brackets
[
  "}"
  ")"
  "]"
] @outdent

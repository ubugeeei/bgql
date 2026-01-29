; Indent after opening braces
[
  (object_type_definition)
  (interface_type_definition)
  (input_object_type_definition)
  (enum_type_definition)
  (union_type_definition)
  (selection_set)
  (arguments_definition)
  (arguments)
  (variable_definitions)
  (directive_locations)
  (fields_definition)
  (input_fields_definition)
  (enum_values_definition)
] @indent

; Outdent on closing braces
[
  "}"
  "]"
  ")"
] @outdent

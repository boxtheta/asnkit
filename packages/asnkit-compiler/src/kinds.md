<!-- A copy of the table of kinds in docs/file_formats.md, so that the
crate has it when published. The test same_as_the_specification in
compact.rs keeps the two the same. -->

<!-- kinds-begin -->

| No. | Kind | Fields |
|---|---|---|
| 1 | CompiledSpecification | modules |
| 2 | CompiledModule | name, types = [], object_classes = [] |
| 3 | TypeAssignment | name, type, location = null |
| 4 | Member | name, type, optional = false, default = null, location = null |
| 5 | Tag | number, tag_class = null, mode = null |
| 6 | Range | lower, upper |
| 7 | NamedNumber | name, number |
| 8 | ExtensionMarker | -- |
| 9 | ExtensionAdditionGroup | members |
| 10 | BuiltinType | name, *type fields* |
| 11 | ReferencedType | name, *type fields* |
| 12 | IntegerType | named_numbers = null, *type fields* |
| 13 | BitStringType | named_bits = null, *type fields* |
| 14 | EnumeratedType | values, *type fields* |
| 15 | SequenceType | members, *type fields* |
| 16 | SetType | members, *type fields* |
| 17 | ChoiceType | members, *type fields* |
| 18 | SequenceOfType | element, element_name = null, *type fields* |
| 19 | SetOfType | element, element_name = null, *type fields* |
| 20 | Min | -- |
| 21 | Max | -- |
| 22 | Identifier | name |
| 23 | HexString | digits |
| 24 | BinaryString | bits |
| 25 | Null | -- |
| 26 | PlusInfinity | -- |
| 27 | MinusInfinity | -- |
| 28 | NotANumber | -- |
| 29 | NamedValue | name, value |
| 30 | SequenceValue | members |
| 31 | UnsupportedValue | -- |
| 32 | Excluded | value |
| 33 | TableConstraint | object_set, component_ids = null |
| 34 | ComponentConstraint | name, presence = null, *constraint fields* |
| 35 | ElementConstraint | *constraint fields* |
| 36 | AnyDefinedByType | value, choices = [], *type fields* |
| 37 | AnyDefinedByChoice | value, type |
| 38 | ComponentsOf | type_name |
| 39 | ObjectClassAssignment | name, fields, syntax = null, location = null |
| 40 | ObjectClassField | name, type, location = null |
| 41 | SyntaxTree | modules |
| 42 | Module | name, tags = null, extensibility_implied = false, imports = [], types = [], values = [], objects = [], object_classes = [], object_sets = [], location = null |
| 43 | Import | module, symbols |
| 44 | ValueAssignment | name, type, value, location = null |
| 45 | ObjectAssignment | name, class_name, object, location = null |
| 46 | ObjectSetAssignment | name, class_name, members, location = null |
| 47 | Object | fields |
| 48 | ObjectField | name, setting |
| 49 | UnresolvedObject | source |
| 50 | ObjectSet | members |
| 51 | OptionalGroup | tokens |
| 52 | Parameter | name, governor = null |
| 53 | ElementSetSpecs | root, extensible = false, additions = null |
| 54 | Union | elements |
| 55 | Intersection | elements |
| 56 | Except | elements, exclusions |
| 57 | All | exclusions |
| 58 | SizeConstraint | constraint |
| 59 | PermittedAlphabet | constraint |
| 60 | ContainedSubtype | type, includes = false |
| 61 | PatternConstraint | value |
| 62 | InnerTypeConstraint | constraint |
| 63 | InnerTypeConstraints | partial = false, components |
| 64 | NamedConstraint | name, constraints = null, presence = null |
| 65 | ContentsConstraint | type = null, encoded_by = null |
| 66 | UserDefinedConstraint | -- |
| 67 | UnsupportedConstraint | -- |

<!-- kinds-end -->

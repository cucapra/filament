; Keywords
[
  "comp"
  "extern"
  "generate"
  "using"
  "import"
  "with"
  "where"
  "some"
  "opaque"
  "let"
  "new"
  "bundle"
  "interface"
  "for"
  "in"
  "if"
  "else"
  "assume"
  "assert"
  "not"
] @keyword

; Operators
[
  "="
  ":="
  "->"
  "+"
  "-"
  "*"
  "/"
  "%"
  ">"
  ">="
  "<"
  "<="
  "=="
  "=>"
  ".."
  "::"
] @operator

; Delimiters
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

[
  ","
  ";"
  ":"
  "."
] @punctuation.delimiter

; Literals
(bitwidth) @number
(float) @number.float
(string_literal) @string

; Comments
(comment) @comment

; Function names
(builtin_fn) @function.builtin

; Component and type names
(component
  (signature
    name: (identifier) @type))

(external
  (signature
    name: (identifier) @type))

(generate
  (signature
    name: (identifier) @type))

; Assignment component references and names
(assignment
  component: (identifier) @type)

(assignment
  name: (identifier) @variable)

; Parameter definitions
(param_bind
  name: (param_var
    (identifier) @parameter))

(sig_bind
  name: (param_var
    (identifier) @parameter))

(param_let
  name: (param_var
    (identifier) @parameter))

(for_loop
  var: (param_var
    (identifier) @parameter))

; Port definitions
(port_def
  name: (identifier) @property)

(bundle_def
  name: (identifier) @property)

; Port references
(port
  port: (identifier) @property)

(port
  instance: (identifier) @variable
  port: (identifier) @property)

; Events
(event
  (identifier) @constant)

; Parameter references
(param_var) @parameter

; Identifiers (general)
(identifier) @identifier

; String content in imports and externals
(import (string_literal) @string.special)
(external (string_literal) @string.special)
(generate (string_literal) @string.special)

; Attributes
(attr_bind
  (identifier) @attribute)
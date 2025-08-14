; Component definitions
(component
  (signature
    name: (identifier) @name)) @definition.class
    (#strip! @name)

; External component definitions
(external
  (signature
    name: (identifier) @name)) @definition.class
    (#strip! @name)

; Generate component definitions
(generate
  (signature
    name: (identifier) @name)) @definition.class
    (#strip! @name)

; Parameter definitions in component signatures
(param_bind
  name: (param_var
    (identifier) @name)) @definition.parameter
  (#strip! @name)

; Parameter definitions in with blocks
(sig_bind
  name: (param_var
    (identifier) @name)) @definition.parameter
  (#strip! @name)

; Parameter definitions in component body
(param_let
  name: (param_var
    (identifier) @name)) @definition.parameter
  (#strip! @name)

; Assignment definitions (instances, invocations, existential bindings)
(assignment
  name: (identifier) @name) @definition.variable
  (#strip! @name)

; Bundle definitions
(bundle_def
  name: (identifier) @name) @definition.property
  (#strip! @name)

; Port definitions
(port_def
  name: (identifier) @name) @definition.property
  (#strip! @name)

; Event definitions
(event_bind
  (event_with_delay
    (event
      (identifier) @name))) @definition.constant
  (#strip! @name)

; Component references in assignments
(assignment
  component: (identifier) @name) @reference.class
  (#strip! @name)

; Port references
(port
  port: (identifier) @name) @reference.property
  (#strip! @name)

; Instance references in port access
(port
  instance: (identifier) @name) @reference.variable
  (#strip! @name)

; Parameter references
(param_var) @reference.parameter

; Event references
(event
  (identifier) @reference.constant)
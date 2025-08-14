; Component scope
(component
  (signature) @local.scope)

; Parameter scopes
(params
  (param_bind
    name: (param_var
      (identifier) @local.definition.parameter)))

; Event scopes  
(abstract_var
  (event_bind
    (event_with_delay
      (event
        (identifier) @local.definition.constant))))

; With-block parameter definitions
(sig_bindings
  (sig_bind
    name: (param_var
      (identifier) @local.definition.parameter)))

; Body parameter definitions
(param_let
  name: (param_var
    (identifier) @local.definition.parameter))

; Assignment definitions (instances, invocations, existential bindings)
(assignment
  name: (identifier) @local.definition.variable)

; Bundle definitions
(bundle
  (bundle_def
    name: (identifier) @local.definition.property))

; Port definitions in signatures
(port_def
  name: (identifier) @local.definition.property)

; For loop variable scope
(for_loop
  var: (param_var
    (identifier) @local.definition.parameter)) @local.scope

; Parameter references
(param_var) @local.reference

; Port references
(port
  port: (identifier) @local.reference)

(port
  instance: (identifier) @local.reference)

; Event references
(event
  (identifier) @local.reference)
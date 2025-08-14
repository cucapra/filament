module.exports = grammar({
  name: 'filament',

  extras: $ => [
    /\s/,
    $.comment,
  ],

  conflicts: $ => [
    [$.expr, $.expr_base],
    [$.time, $.expr],
    [$.constraint, $.time],
  ],

  rules: {
    // Top level file structure
    source_file: $ => seq(
      optional($.imports),
      repeat($.comp_or_ext)
    ),

    // Comments
    comment: $ => choice(
      seq('//', /.*/),
      seq('/*', /[^*]*\*+([^/*][^*]*\*+)*/, '/')
    ),

    // Imports
    imports: $ => repeat1($.import),
    import: $ => seq(
      'import',
      $.string_literal,
      ';'
    ),

    // String literals
    string_literal: $ => seq(
      '"',
      repeat(choice(
        /[^"\\]/,
        /\\./
      )),
      '"'
    ),

    // Identifiers and basic types
    identifier: $ => /[a-zA-Z_][a-zA-Z0-9_]*/,
    param_var: $ => $.identifier,
    bitwidth: $ => /[0-9]+/,
    float: $ => /[0-9]+\.[0-9]+/,

    // Top level constructs
    comp_or_ext: $ => choice(
      $.component,
      $.external,
      $.generate
    ),

    // External definitions
    external: $ => seq(
      'extern',
      $.string_literal,
      '{',
      repeat(seq($.signature, ';')),
      '}'
    ),

    // Generate definitions
    generate: $ => seq(
      'generate',
      '(',
      $.identifier,
      ')',
      'using',
      $.string_literal,
      '{',
      repeat(seq($.signature, ';')),
      '}'
    ),

    // Component definition
    component: $ => seq(
      $.signature,
      '{',
      repeat($.command),
      '}'
    ),

    // Component signature
    signature: $ => seq(
      optional($.attributes),
      'comp',
      field('name', $.identifier),
      optional($.params),
      optional($.abstract_var),
      $.io,
      optional($.sig_bindings),
      optional($.constraints)
    ),

    // Attributes
    attributes: $ => seq(
      '#[',
      $.attr_bind,
      repeat(seq(',', $.attr_bind)),
      ']'
    ),

    attr_bind: $ => choice(
      seq('not', '(', $.identifier, ')'),
      seq($.identifier, '=', $.bitwidth),
      seq($.identifier, '=', $.float),
      $.identifier
    ),

    // Parameters in component signature
    params: $ => seq(
      '[',
      $.param_bind,
      repeat(seq(',', $.param_bind)),
      ']'
    ),

    param_bind: $ => choice(
      seq('?', field('name', $.param_var), '=', $.expr),
      field('name', $.param_var)
    ),

    // Abstract variables (events)
    abstract_var: $ => seq(
      '<',
      $.event_bind,
      repeat(seq(',', $.event_bind)),
      '>'
    ),

    event_bind: $ => choice(
      seq('?', $.event_with_delay, '=', $.time),
      $.event_with_delay
    ),

    event_with_delay: $ => seq(
      $.event,
      ':',
      $.delay
    ),

    event: $ => seq("'", $.identifier),

    delay: $ => choice(
      $.expr,
      seq($.time, '-', '(', $.time, ')')
    ),

    // Input/Output specification
    io: $ => seq(
      '(',
      optional($.ports),
      ')',
      '->',
      '(',
      optional($.ports),
      ')'
    ),

    ports: $ => seq(
      $.port_def,
      repeat(seq(',', $.port_def)),
      optional(',')
    ),

    port_def: $ => choice(
      seq(field('name', $.identifier), ':', $.bitwidth),
      $.bundle_def,
      seq(field('name', $.identifier), ':', $.interface)
    ),

    interface: $ => seq(
      'interface',
      '[',
      $.event,
      ']'
    ),

    // Signature bindings (with block)
    sig_bindings: $ => seq(
      'with',
      '{',
      repeat($.sig_bind),
      '}'
    ),

    sig_bind: $ => choice(
      seq('let', field('name', $.param_var), '=', $.expr, ';'),
      seq('some', field('name', $.param_var), optional($.constraints), ';'),
      seq('opaque', field('name', $.param_var), optional($.constraints), ';')
    ),

    // Constraints
    constraints: $ => seq(
      'where',
      $.constraint,
      repeat(seq(',', $.constraint))
    ),

    constraint: $ => choice(
      seq($.expr, $.order_op, $.expr),
      seq($.time, $.order_op, $.time)
    ),

    order_op: $ => choice('>', '>=', '<', '<=', '=='),

    // Time expressions
    time: $ => choice(
      seq($.event, '+', $.expr),
      seq($.expr, '+', $.event),
      $.event,
      $.expr
    ),

    // Expressions
    expr: $ => prec.left(1, choice(
      $.expr_base,
      prec.left(3, seq($.expr, choice('*', '/', '%'), $.expr)),
      prec.left(2, seq($.expr, choice('+', '-'), $.expr))
    )),

    expr_base: $ => choice(
      $.if_expr,
      seq($.builtin_fn, '(', $.expr, repeat(seq(',', $.expr)), ')'),
      seq('(', $.expr, ')'),
      $.bitwidth,
      seq($.identifier, '::', $.identifier),
      $.param_var
    ),

    if_expr: $ => seq(
      'if',
      $.expr_cmp,
      '{',
      $.expr,
      '}',
      'else',
      '{',
      $.expr,
      '}'
    ),

    expr_cmp: $ => seq(
      $.expr,
      $.order_op,
      $.expr
    ),

    builtin_fn: $ => choice(
      'pow2',
      'log2',
      'sin_bits',
      'cos_bits',
      'bit_rev'
    ),

    // Bundle definitions
    bundle_def: $ => seq(
      optional($.attributes),
      field('name', $.identifier),
      repeat(seq('[', $.expr, ']')),
      ':',
      $.bundle_typ
    ),

    bundle_typ: $ => seq(
      optional(seq('for', $.bundle_params)),
      $.interval_range,
      $.expr
    ),

    bundle_params: $ => seq(
      '<',
      $.param_var,
      repeat(seq(',', $.param_var)),
      '>'
    ),

    interval_range: $ => seq(
      '[',
      $.time,
      ',',
      $.time,
      ']'
    ),

    // Commands in component body
    command: $ => choice(
      $.bundle,
      $.assignment,
      $.connect,
      $.for_loop,
      $.if_stmt,
      $.fact,
      $.param_let
    ),

    // Unified assignment rule to handle instance, invocation, and existential assignments
    assignment: $ => seq(
      field('name', $.identifier),
      ':=',
      choice(
        // Instance creation: identifier := new Component[params]<events>(args) in lives;
        seq(
          'new',
          field('component', $.identifier),
          optional($.conc_params),
          optional($.invoke_args),
          optional($.inst_live),
          ';'
        ),
        // Invocation: identifier := Component<events>(args);
        seq(
          field('component', $.identifier),
          $.invoke_args,
          ';'
        ),
        // Existential binding: identifier := expr;
        seq(
          $.expr,
          ';'
        )
      )
    ),

    conc_params: $ => seq(
      '[',
      $.expr,
      repeat(seq(',', $.expr)),
      ']'
    ),

    inst_live: $ => seq(
      'in',
      $.interval_range,
      repeat(seq(',', $.interval_range))
    ),

    invoke_args: $ => seq(
      $.time_args,
      $.arguments
    ),

    time_args: $ => seq(
      '<',
      $.time,
      repeat(seq(',', $.time)),
      '>'
    ),

    arguments: $ => choice(
      seq('(', ')'),
      seq('(', $.port, repeat(seq(',', $.port)), ')')
    ),

    // Port references
    port: $ => choice(
      seq(field('instance', $.identifier), '.', field('port', $.identifier), repeat($.access)),
      seq(field('port', $.identifier), repeat($.access))
    ),

    access: $ => seq(
      '{',
      choice(
        seq($.expr, '..', $.expr),
        $.expr
      ),
      '}'
    ),

    // Connections
    connect: $ => seq(
      $.port,
      '=',
      $.port,
      ';'
    ),

    // Control flow
    for_loop: $ => seq(
      'for',
      field('var', $.param_var),
      'in',
      $.expr,
      '..',
      $.expr,
      '{',
      repeat($.command),
      '}'
    ),

    if_stmt: $ => seq(
      'if',
      $.expr_cmp,
      '{',
      repeat($.command),
      '}',
      optional(seq(
        'else',
        '{',
        repeat($.command),
        '}'
      ))
    ),

    // Facts (assumptions and assertions)
    fact: $ => seq(
      choice('assume', 'assert'),
      $.implication,
      ';'
    ),

    implication: $ => choice(
      seq($.expr_cmp, '=>', $.expr_cmp),
      $.expr_cmp
    ),

    // Parameter let binding
    param_let: $ => choice(
      seq('let', field('name', $.param_var), '=', $.expr, ';'),
      seq('let', field('name', $.param_var), '=', '?', ';')
    ),

    // Bundle command
    bundle: $ => seq(
      'bundle',
      $.bundle_def,
      ';'
    )
  }
});
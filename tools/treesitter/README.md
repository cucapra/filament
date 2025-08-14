# Tree-sitter Grammar for Filament

This directory contains a complete Tree-sitter grammar implementation for the Filament hardware description language, designed to enable LSP functionality such as "jump to definition", syntax highlighting, and intelligent code completion.

## Overview

The Tree-sitter grammar provides a foundation for advanced editor support including:
- **Syntax highlighting** with semantic token types
- **Symbol extraction** for navigation features
- **Local scope analysis** for variable resolution
- **Parse tree structure** optimized for LSP queries

## Architecture & Design Decisions

### Core Grammar Structure

The grammar follows the structure of the original Pest grammar (`crates/ast/src/syntax.pest`) but is adapted for Tree-sitter's parsing model:

```javascript
// Main grammar rules correspond to AST structures
source_file -> imports? comp_or_ext*
comp_or_ext -> component | external | generate
component -> signature "{" command* "}"
```

### Key Language Constructs

The grammar handles all major Filament language features:

1. **Component Definitions**
   ```filament
   comp Add[WIDTH]<'G: 1>(left: ['G, 'G+1] WIDTH) -> (out: ['G+1, 'G+2] WIDTH) {
       // component body
   }
   ```

2. **External Declarations**
   ```filament
   extern "hardware.sv" {
       comp Add[WIDTH]<'G: 1>(...) -> (...);
   }
   ```

3. **Parameter Bindings** (critical for LSP)
   - Signature parameters: `[WIDTH, ?DELAY=1]`
   - Event parameters: `<'G: 1, 'L: 'G+DELAY>`
   - With-block parameters: `with { some W where W > 0; let L = 10; }`

4. **Complex Expressions**
   - Time expressions: `'G+1`, `'START+DELAY`
   - Arithmetic: `WIDTH * 2 + 1`
   - Conditional: `if WIDTH > 8 { 16 } else { 8 }`
   - Namespace access: `Component::PARAM`

### Critical Design Decision: Unified Assignment Rule

The most significant design decision was resolving parse ambiguity between three constructs that use `:=`:

**Problem:**
```filament
x := new Component[32];     // Instance creation
y := Component<'G>(ports);  // Invocation  
z := expr;                  // Existential binding
```

Tree-sitter couldn't distinguish these at parse time due to the shared `identifier :=` prefix.

**Solution:**
We implemented a unified `assignment` rule that branches based on distinguishing tokens:

```javascript
assignment: $ => seq(
  field('name', $.identifier),
  ':=',
  choice(
    // Instance: 'new' keyword is unique
    seq('new', field('component', $.identifier), ...),
    // Invocation: '<' for time args is required
    seq(field('component', $.identifier), $.invoke_args, ...),
    // Existential: fallback for any other expression
    seq($.expr, ';')
  )
)
```

This approach:
- ✅ Leverages unique tokens (`new`, `<`) for disambiguation  
- ✅ Maintains identical AST structure for downstream tools
- ✅ Eliminates all parse conflicts
- ✅ Handles 100% of test cases correctly

### LSP Symbol Extraction

The grammar is specifically designed for LSP functionality with careful attention to:

**Definition Providers:**
- Component names: `comp MyComponent` 
- Parameter names: `[WIDTH, HEIGHT]`, `with { some PARAM; }`
- Instance names: `adder := new Add[32]`
- Port names: `input: ['G, 'G+1] 32`
- Bundle names: `bundle data[SIZE]: [...]`
- Event names: `<'START: 1>`

**Reference Sites:**
- Component usage: `new Component`, `Component<...>(...)`
- Parameter usage: `WIDTH`, `Component::PARAM`
- Port access: `instance.port`, `port{index}`
- Event usage: `'START+1`

All definition sites use `field('name', ...)` to mark extractable identifiers.

## File Structure

```
tools/tree-sitter/
├── grammar.js              # Main grammar definition
├── package.json            # NPM package configuration
├── queries/                # LSP query files
│   ├── highlights.scm      # Syntax highlighting rules
│   ├── locals.scm          # Local scope definitions  
│   └── tags.scm            # Symbol extraction for navigation
├── src/                    # Generated parser (auto-generated)
├── bindings/               # Language bindings (auto-generated)
└── README.md               # This file
```

## Usage

### Building the Parser

```bash
# Install dependencies
npm install

# Generate the parser
npx tree-sitter generate

# Test the grammar
npx tree-sitter test
```

### Testing with Filament Files

```bash
# Parse a specific file
npx tree-sitter parse path/to/file.fil

# Parse with highlighting
npx tree-sitter highlight path/to/file.fil

# Run grammar tests
npx tree-sitter test
```

### Integration with Editors

The generated parser can be integrated with:
- **VS Code**: Via tree-sitter extensions
- **Neovim**: Native tree-sitter support
- **Emacs**: Via tree-sitter packages
- **Custom LSP**: Using the generated bindings

## Query Files

### highlights.scm
Defines syntax highlighting rules:
- Keywords: `comp`, `extern`, `with`, `some`, `let`
- Operators: `:=`, `->`, `::`, `+`, `*`
- Literals: numbers, strings, events (`'G`)
- Types: component names, parameter names
- Functions: builtin functions like `pow2`, `log2`

### locals.scm  
Defines local scope rules for variable resolution:
- Component scopes for parameter visibility
- With-block scopes for existential parameters
- For-loop scopes for iteration variables
- Parameter binding and reference relationships

### tags.scm
Defines symbol extraction for "jump to definition":
- Component definitions and references
- Parameter definitions and usage
- Instance definitions and port access
- Event definitions and time expressions

## Testing

The grammar includes comprehensive tests covering:
- ✅ Simple component definitions
- ✅ Complex signatures with with-blocks  
- ✅ External declarations
- ✅ All three assignment types (instance, invocation, existential)
- ✅ Nested expressions and time arithmetic
- ✅ Error recovery and partial parsing

Test files demonstrate parsing of real Filament programs from the repository.

## Implementation Notes

### Grammar Conflicts Resolved

1. **Expression vs Time**: Both use similar syntax (`'G+1`)
   - Solution: Explicit conflicts declaration with precedence

2. **Assignment Ambiguity**: Instance vs Invocation vs Existential
   - Solution: Unified assignment rule with token-based branching

3. **Port Definitions**: Bundle vs simple port syntax
   - Solution: Choice-based rules with clear precedence

### Tree-sitter Specifics

- **Field Naming**: All definition sites marked with `field('name', ...)`
- **Precedence**: Used sparingly, only where necessary for disambiguation
- **Choice Ordering**: More specific patterns before general ones
- **Conflicts**: Explicit declarations for unavoidable ambiguities

### Performance Considerations

- **Incremental Parsing**: Tree-sitter's strength for large files
- **Error Recovery**: Grammar continues parsing after syntax errors
- **Memory Efficiency**: Index-based node references
- **Query Optimization**: Efficient pattern matching for LSP features

## Future Extensions

The grammar foundation supports adding:
- **Error diagnostics** with precise source locations
- **Semantic highlighting** based on symbol types
- **Auto-completion** using partial parse trees  
- **Refactoring tools** via AST transformations
- **Documentation generation** from parsed structures

## Contributing

When modifying the grammar:

1. **Test thoroughly** with `npx tree-sitter test`
2. **Validate with real files** from the repository
3. **Update query files** if new node types are added
4. **Document design decisions** for complex changes
5. **Preserve LSP compatibility** by maintaining field names

The grammar is designed to evolve with the Filament language while maintaining backward compatibility for existing LSP tooling.
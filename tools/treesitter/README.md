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

The parser build process involves several steps that transform the grammar definition into a usable shared library:

#### Automated Build (Recommended)
```bash
# Install dependencies and build parser in one step
npm install  # Runs postinstall hook automatically

# Or build explicitly
npm run build-parser
```

#### Manual Build Steps
```bash
# 1. Install tree-sitter CLI and dependencies
npm install

# 2. Generate C code from grammar.js
npx tree-sitter generate

# 3. Compile shared library for Neovim
gcc -o src/parser.so -shared -fPIC -I src src/parser.c

# 4. Test the grammar
npx tree-sitter test
```

#### What Each Step Does

1. **`npm install`**:
   - Installs `tree-sitter-cli` and Node.js dependencies
   - Runs `postinstall` hook → automatically builds parser
   - Creates `node_modules/` directory

2. **`tree-sitter generate`**:
   - Reads `grammar.js` (source grammar definition)
   - Generates `src/parser.c` (C implementation)
   - Generates `src/grammar.json` (grammar metadata)
   - Generates `src/node-types.json` (AST node types)
   - Creates `src/tree_sitter/parser.h` (header file)

3. **`gcc` compilation**:
   - Compiles `src/parser.c` to `src/parser.so`
   - Creates position-independent code (`-fPIC`)
   - Links as shared library (`-shared`)
   - Includes headers from `src/` directory (`-I src`)

4. **Result**: `src/parser.so` - ready for Neovim integration

#### Build Artifacts

**Generated files (git-ignored):**
```
src/
├── parser.c          # Generated C parser implementation
├── parser.so         # Compiled shared library (for Neovim)
├── grammar.json      # Grammar metadata
├── node-types.json   # AST node type definitions
└── tree_sitter/
    └── parser.h      # C header file
```

**Source files (version controlled):**
```
grammar.js            # Grammar definition (source of truth)
package.json          # Build configuration with npm scripts
queries/              # Syntax highlighting and query rules
├── highlights.scm
├── locals.scm
└── tags.scm
```

The `package.json` defines several build scripts for different purposes:

```json
{
  "scripts": {
    "build": "tree-sitter generate && node-gyp build",
    "build-parser": "tree-sitter generate && gcc -o src/parser.so -shared -fPIC -I src src/parser.c",
    "postinstall": "npm run build-parser",
    "test": "tree-sitter test"
  }
}
```

#### Script Breakdown

- **`npm run build`**: Full Node.js binding build
  - Generates parser C code
  - Builds Node.js native module using `node-gyp`
  - Creates `build/Release/tree_sitter_filament_binding.node`
  - Used for Node.js applications and tree-sitter CLI tools

- **`npm run build-parser`**: Neovim-specific build
  - Generates parser C code
  - Compiles directly to shared library (`parser.so`)
  - Optimized for Neovim tree-sitter integration
  - **This is what editor plugins use**

- **`postinstall`**: Automatic build hook
  - Runs after `npm install`
  - Calls `build-parser` to create `src/parser.so`
  - Enables zero-config installation for users

- **`npm test`**: Grammar validation
  - Runs tree-sitter test suite
  - Validates grammar against test files
  - Checks parsing correctness

#### Build Requirements

**System Dependencies:**
- **Node.js & npm**: For running build scripts
- **GCC**: For compiling C code to shared library
- **tree-sitter CLI**: Installed via npm dependencies

**Platform Notes:**
- **Linux/macOS**: Uses GCC directly
- **Windows**: May require MinGW or MSYS2 for GCC
- **Cross-platform**: Node.js parts work everywhere

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

## Build Troubleshooting

### Common Build Issues

#### Missing GCC Compiler
```
Error: gcc: command not found
```
**Solutions:**
- **macOS**: Install Xcode command line tools: `xcode-select --install`
- **Linux**: Install build-essential: `sudo apt install build-essential` (Ubuntu/Debian) or `sudo yum groupinstall "Development Tools"` (RHEL/CentOS)
- **Windows**: Install MinGW-w64 or use WSL with Linux tools

#### Node.js/NPM Missing
```
Error: npm: command not found
```
**Solutions:**
- Install Node.js from https://nodejs.org/
- Use package manager: `brew install node` (macOS), `sudo apt install nodejs npm` (Linux)
- Use version manager: `nvm install node`

#### Parser Generation Fails
```
Error during tree-sitter generate
```
**Solutions:**
- Check `grammar.js` syntax for JavaScript errors
- Verify tree-sitter CLI installation: `npx tree-sitter --version`
- Clean and rebuild: `rm -rf src/ && npm run build-parser`

#### Compilation Errors
```
Error: src/parser.c: No such file or directory
```
**Solutions:**
- Ensure `tree-sitter generate` ran successfully first
- Check that `src/parser.c` exists after generation
- Run full build: `npm install && npm run build-parser`

#### Permission Issues
```
Error: EACCES: permission denied
```
**Solutions:**
- Don't use `sudo` with npm (security risk)
- Fix npm permissions: https://docs.npmjs.com/resolving-eacces-permissions-errors-when-installing-packages-globally
- Use node version manager (nvm) instead of system Node.js

#### Windows-Specific Issues
```
Error: 'gcc' is not recognized as an internal or external command
```
**Solutions:**
- Install MinGW-w64: https://www.mingw-w64.org/
- Add GCC to PATH environment variable
- Use WSL (Windows Subsystem for Linux) with Ubuntu
- Consider using Visual Studio Build Tools with node-gyp

### Verification Steps

After successful build, verify:
```bash
# Check generated files exist
ls -la src/
# Should show: parser.c, parser.so, grammar.json, node-types.json

# Test the parser works
npx tree-sitter parse test_simple.fil

# Verify shared library
file src/parser.so
# Should show: shared library, dynamically linked
```

### Clean Rebuild

If experiencing persistent issues:
```bash
# Full clean and rebuild
rm -rf src/ node_modules/ build/
npm install
npm run build-parser
```

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

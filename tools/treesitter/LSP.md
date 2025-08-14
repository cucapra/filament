# Multi-file Support for Filament LSP

This document explains how Tree-sitter interacts with multi-file programs and what infrastructure is needed to support cross-file navigation, import resolution, and workspace-wide symbol indexing for Filament.

## Tree-sitter and Multi-file Programs

### Tree-sitter's Scope

Tree-sitter is fundamentally a **single-file parser**. It:
- Parses one file at a time into an AST
- Provides incremental parsing for that single file  
- Has no built-in awareness of other files or imports
- Cannot resolve symbols across file boundaries

### Current Import Handling

In our grammar, imports are parsed but not resolved:

```javascript
import: $ => seq(
  'import',
  $.string_literal,  // Just captures "primitives/core.fil" as a string
  ';'
)
```

This means Tree-sitter knows there's an import statement, but doesn't know:
- What components are defined in the imported file
- What symbols become available in the importing file
- How to navigate to definitions in imported files

## LSP Server Architecture for Multi-file Support

The Language Server Protocol (LSP) server handles multi-file awareness. Here's the typical architecture:

```
┌─────────────────────────────────────────────────────────┐
│                    LSP Server                           │
├─────────────────────────────────────────────────────────┤
│  Global Symbol Index                                    │
│  ┌─────────────────────────────────────────────────┐   │
│  │ Component: Add                                  │   │
│  │   File: primitives/comb.fil                     │   │
│  │   Line: 8, Column: 5                           │   │
│  │   Type: component_definition                    │   │
│  └─────────────────────────────────────────────────┘   │
│                                                         │
│  File Dependency Graph                                  │
│  ┌─────────────────────────────────────────────────┐   │
│  │ main.fil → primitives/core.fil                  │   │
│  │ core.fil → comb.fil, state.fil                  │   │
│  │ comb.fil → (no dependencies)                    │   │
│  └─────────────────────────────────────────────────┘   │
│                                                         │
│  Per-file Parse Trees (Tree-sitter)                    │
│  ┌─────────────────────────────────────────────────┐   │
│  │ main.fil: (source_file ...)                     │   │
│  │ core.fil: (source_file ...)                     │   │
│  │ comb.fil: (source_file ...)                     │   │
│  └─────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────┘
```

### Implementation Pattern

```typescript
class FilamentLSP {
  // Maps file paths to their Tree-sitter parse trees
  private parseTrees: Map<string, Tree> = new Map();
  
  // Global symbol table across all files
  private symbolTable: Map<string, SymbolInfo> = new Map();
  
  // File dependency graph
  private imports: Map<string, Set<string>> = new Map();
  
  async onFileOpen(uri: string) {
    // 1. Parse the file with Tree-sitter
    const tree = parser.parse(fileContent);
    this.parseTrees.set(uri, tree);
    
    // 2. Extract imports using Tree-sitter queries
    const imports = this.extractImports(tree);
    
    // 3. Parse imported files recursively
    for (const importPath of imports) {
      await this.parseIfNeeded(importPath);
    }
    
    // 4. Update global symbol table
    this.updateSymbolTable(uri, tree);
  }
  
  onDefinition(uri: string, position: Position) {
    // 1. Find symbol at position in current file
    const symbol = this.getSymbolAtPosition(uri, position);
    
    // 2. Check local definitions first
    let definition = this.findLocalDefinition(uri, symbol);
    
    // 3. If not found, check imported files
    if (!definition) {
      definition = this.symbolTable.get(symbol);
    }
    
    return definition;
  }
}
```

## Required Changes for Multi-file Support

### 1. Enhanced Import Grammar

Minimal changes to the Tree-sitter grammar to make imports more queryable:

```javascript
// Add field name for easier extraction
import: $ => seq(
  'import',
  field('path', $.string_literal),  // Field makes querying easier
  ';'
)
```

### 2. Import Extraction Queries

Create `queries/imports.scm` to extract import information:

```scheme
; Extract import paths for dependency resolution
(import
  path: (string_literal) @import.path)
  
; Track all imports in a file
(imports
  (import
    path: (string_literal) @import.file))
```

### 3. Symbol Export/Import Tracking

The LSP needs to track what symbols are available across files:

```typescript
interface FileSymbols {
  // Components/symbols defined in this file
  definitions: Map<string, Definition>;
  
  // Components made available through imports
  imports: Map<string, ImportedSymbol>;
  
  // Transitive closure of all available symbols
  availableSymbols: Map<string, Definition>;
}

interface Definition {
  name: string;
  type: 'component' | 'parameter' | 'instance' | 'port';
  file: string;
  position: Position;
  range: Range;
}
```

### 4. Filament-specific Import Resolution

Filament's import semantics are simpler than most languages:
- `import "primitives/core.fil"` imports **ALL** components from that file
- No selective imports (unlike JavaScript: `import { A, B } from "module"`)
- No namespacing (unlike Rust: `use module::Component`)
- No aliases or renaming

This simplifies the resolution algorithm:

```typescript
class FilamentSymbolResolver {
  resolveSymbol(currentFile: string, symbolName: string): Definition | null {
    // 1. Check if defined in current file
    const localDef = this.findInFile(currentFile, symbolName);
    if (localDef) return localDef;
    
    // 2. Check all imported files (including transitive imports)
    const imports = this.getTransitiveImports(currentFile);
    for (const importedFile of imports) {
      const def = this.findInFile(importedFile, symbolName);
      if (def) return def;
    }
    
    // 3. Symbol not found
    return null;
  }
  
  getTransitiveImports(file: string): Set<string> {
    const visited = new Set<string>();
    const result = new Set<string>();
    
    const visit = (currentFile: string) => {
      if (visited.has(currentFile)) return;
      visited.add(currentFile);
      
      const directImports = this.getDirectImports(currentFile);
      for (const imported of directImports) {
        result.add(imported);
        visit(imported);  // Recursive for transitive imports
      }
    };
    
    visit(file);
    return result;
  }
}
```

### 5. Workspace-level Indexing

For efficient operation on large projects:

```typescript
class WorkspaceIndexer {
  private symbolIndex: Map<string, Definition[]> = new Map();
  private fileGraph: Map<string, Set<string>> = new Map();
  
  async indexWorkspace(rootPath: string) {
    // 1. Find all .fil files
    const filamentFiles = await this.findFilamentFiles(rootPath);
    
    // 2. Parse all files with Tree-sitter
    for (const file of filamentFiles) {
      const tree = await this.parseFile(file);
      this.extractSymbols(file, tree);
      this.extractImports(file, tree);
    }
    
    // 3. Build transitive import graph
    this.buildDependencyGraph();
  }
  
  // Called when a file changes
  async onFileChanged(file: string) {
    // Re-parse the changed file
    const tree = await this.parseFile(file);
    
    // Update symbols for this file
    this.extractSymbols(file, tree);
    
    // Re-index files that import this one
    const dependents = this.findFilesThatImport(file);
    for (const dependent of dependents) {
      this.updateSymbolScope(dependent);
    }
  }
}
```

## Implementation Steps

### Phase 1: Grammar Updates (5 minutes)
1. Add `field('path', $.string_literal)` to import rule
2. Regenerate Tree-sitter parser
3. Create `queries/imports.scm` for import extraction

### Phase 2: Symbol Extraction (1-2 hours)
1. Create symbol extraction from Tree-sitter parse trees
2. Build file-local symbol tables
3. Implement component/parameter/port definition extraction

### Phase 3: Import Resolution (2-3 hours)
1. Implement import path resolution (relative to current file)
2. Build file dependency graph
3. Create transitive import resolution
4. Handle circular import detection

### Phase 4: LSP Integration (3-4 hours)
1. Integrate with LSP "Go to Definition" 
2. Implement "Find All References" across files
3. Add workspace symbol provider
4. Support rename refactoring across imports

### Phase 5: Performance & Caching (1-2 hours)
1. Cache parse trees and symbol tables
2. Implement incremental re-indexing
3. Add file watchers for change detection
4. Optimize for large workspaces

## Performance Considerations

### Lazy Loading
- Parse imported files only when needed for navigation
- Cache frequently accessed parse trees
- Use background threads for initial indexing

### Incremental Updates
- Re-parse only changed files and their dependents
- Maintain incremental symbol table updates
- Use file system watchers to detect changes

### Memory Management
- Limit number of cached parse trees
- Use weak references for infrequently accessed files
- Consider serializing symbol tables to disk for large workspaces

## Testing Strategy

### Unit Tests
```typescript
describe('Import Resolution', () => {
  it('should resolve components from direct imports', () => {
    // main.fil imports core.fil which defines Add
    expect(resolve('main.fil', 'Add')).toEqual({
      file: 'primitives/core.fil',
      position: { line: 8, character: 5 }
    });
  });
  
  it('should handle transitive imports', () => {
    // main.fil → core.fil → comb.fil
    expect(resolve('main.fil', 'Add')).toBeDefined();
  });
  
  it('should detect circular imports', () => {
    // a.fil → b.fil → a.fil
    expect(() => buildDependencyGraph()).not.toThrow();
  });
});
```

### Integration Tests
Test with real Filament project structure:
```
project/
├── main.fil              (imports primitives/core.fil)
├── primitives/
│   ├── core.fil          (imports comb.fil, state.fil)
│   ├── comb.fil          (defines Add, Mul, etc.)
│   └── state.fil         (defines Register, Memory, etc.)
└── examples/
    └── pipeline.fil      (imports main.fil)
```

### Example Navigation Scenarios
1. **Jump to Definition**: Click on `Add[32]` in main.fil → jump to definition in comb.fil
2. **Find All References**: Find all uses of `Register` across the entire workspace
3. **Workspace Symbols**: Search for all components named `*Mult*`
4. **Rename Refactoring**: Rename `Add` to `Adder` across all files

## Future Enhancements

### Advanced Features
- **Import auto-completion**: Suggest available files when typing import paths
- **Unused import detection**: Find imports that aren't used
- **Import organization**: Sort and group imports automatically
- **Cross-file refactoring**: Move components between files safely

### IDE Integration
- **VS Code extension**: Native Tree-sitter + LSP integration
- **Neovim support**: Tree-sitter highlighting + LSP navigation
- **Emacs package**: Complete Filament development environment

This multi-file support would provide Filament developers with modern IDE experiences including instant navigation, reliable refactoring, and comprehensive symbol search across large projects.
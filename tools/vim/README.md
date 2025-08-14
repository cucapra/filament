# Vim/Neovim Support for Filament

This directory contains vim/neovim configuration files for the Filament hardware description language, providing syntax highlighting and advanced editor features.

## Features

### Traditional Vim Support
- Syntax highlighting using vim's built-in syntax system
- File type detection for `.fil` files
- Compatible with all vim versions

### Neovim Tree-sitter Support (Enhanced)
- Advanced syntax highlighting with semantic tokens
- Local variable scoping and resolution
- Symbol extraction for navigation features
- Incremental parsing for better performance
- Error recovery during editing
- Foundation for LSP features

## Quick Installation

### One-Line Setup (Recommended)

**vim-plug:**
```vim
" Add to your init.vim/.vimrc and run :PlugInstall
Plug 'nvim-treesitter/nvim-treesitter', {'do': ':TSUpdate'}
Plug 'filament-hdl/filament', {'rtp': 'treesitter/tools/vim/filament', 'do': 'cd treesitter/tools/treesitter && npm install && npm run build-parser'}
```

**lazy.nvim:**
```lua
-- Add to your lazy configuration
{
  'filament-hdl/filament',
  rtp = 'treesitter/tools/vim/filament',
  build = 'cd treesitter/tools/treesitter && npm install && npm run build-parser',
  dependencies = { 'nvim-treesitter/nvim-treesitter' },
  config = function() require('filament').setup() end,
}
```

**That's it!** The `do`/`build` hook builds the parser during plugin installation, then syntax highlighting works immediately.

### Requirements
- **Neovim 0.8+** (for tree-sitter support)
- **Node.js & npm** (for building the parser)
- **GCC** (for compiling the shared library)

## Detailed Installation

### Method 1: Using Plugin Managers (Recommended)

#### vim-plug
Add to your `~/.config/nvim/init.vim` or `~/.vimrc`:
```vim
" Install the filament plugin with auto-build
Plug 'filament-hdl/filament', {'rtp': 'treesitter/tools/vim/filament', 'do': 'cd treesitter/tools/treesitter && npm install && npm run build-parser'}

" Install nvim-treesitter for enhanced features (Neovim only)
Plug 'nvim-treesitter/nvim-treesitter', {'do': ':TSUpdate'}
```

Then run `:PlugInstall` and restart.

#### lazy.nvim (Neovim)
Add to your lazy configuration:
```lua
{
  'filament-hdl/filament',
  rtp = 'treesitter/tools/vim/filament',
  build = 'cd treesitter/tools/treesitter && npm install && npm run build-parser',
  dependencies = { 'nvim-treesitter/nvim-treesitter' },
  config = function()
    require('filament').setup({
      -- Parser is built by 'build' hook above
    })
  end,
}
```

#### packer.nvim (Neovim)
Add to your packer configuration:
```lua
use {
  'filament-hdl/filament',
  rtp = 'treesitter/tools/vim/filament',
  requires = {'nvim-treesitter/nvim-treesitter'},
  run = 'cd treesitter/tools/treesitter && npm install && npm run build-parser',
  config = function()
    require('filament').setup({
      -- Parser is built by 'run' hook above
    })
  end
}
```

#### Vundle
Add to your `.vimrc`:
```vim
Plugin 'filament-hdl/filament'
Plugin 'nvim-treesitter/nvim-treesitter'
```

### Method 2: Manual Installation

#### For Traditional Vim Users
```bash
# Clone the repository or use the existing one
git clone https://github.com/filament-hdl/filament.git
cd filament/treesitter/tools/vim

# Copy vim configuration
cp -r filament ~/.vim/
```

#### For Neovim Users
```bash
# Clone the repository or use the existing one
git clone https://github.com/filament-hdl/filament.git
cd filament/treesitter/tools/vim

# Copy neovim configuration
cp -r filament ~/.config/nvim/
```

#### Using the Install Script
```bash
cd treesitter/tools/vim
./install.sh [vim|nvim]  # Specify editor type, or leave blank for auto-detection
```

## Tree-sitter Parser Installation

### Automatic Installation (Plugin Managers)

If you're using a plugin manager with the setup function:
```lua
require('filament').setup({
  auto_install = true,  -- Automatically install parser
  treesitter = {
    highlight = { enable = true },
    indent = { enable = true },
    fold = { enable = true },
  }
})
```

### Manual Installation

#### Method 1: Using TSInstall (if parser is registered)
```vim
:TSInstall filament
```

#### Method 2: Build from Source
```bash
# Navigate to the tree-sitter directory
cd filament/treesitter/tools/treesitter

# Install dependencies and build
npm install
npx tree-sitter generate

# For Neovim, copy the compiled parser
mkdir -p ~/.local/share/nvim/site/pack/packer/start/nvim-treesitter/parser
cp src/parser.so ~/.local/share/nvim/site/pack/packer/start/nvim-treesitter/parser/filament.so
```

## Configuration Examples

### Basic Configuration (init.lua)
```lua
-- Ensure nvim-treesitter is configured
require('nvim-treesitter.configs').setup {
  ensure_installed = { 'filament' }, -- Add other languages as needed
  highlight = {
    enable = true,
    additional_vim_regex_highlighting = false,
  },
  indent = {
    enable = true,
  },
}
```

### Advanced Configuration with Filament Plugin
```lua
require('filament').setup({
  -- Automatically install tree-sitter parser
  auto_install = true,
  
  -- Configure nvim-treesitter for filament
  treesitter = {
    highlight = {
      enable = true,
      additional_vim_regex_highlighting = false,
    },
    indent = {
      enable = true,
    },
    fold = {
      enable = true,
    },
    incremental_selection = {
      enable = true,
      keymaps = {
        init_selection = "gnn",
        node_incremental = "grn",
        scope_incremental = "grc",
        node_decremental = "grm",
      },
    },
  }
})

-- Optional: Set up folding for filament files
vim.api.nvim_create_autocmd("FileType", {
  pattern = "filament",
  callback = function()
    vim.wo.foldmethod = "expr"
    vim.wo.foldexpr = "nvim_treesitter#foldexpr()"
    vim.wo.foldenable = false -- Start with folds open
  end,
})
```

### Using with Different Plugin Managers

#### With vim-plug + custom config
```vim
" In init.vim or .vimrc
Plug 'filament-hdl/filament', {'rtp': 'treesitter/tools/vim/filament'}
Plug 'nvim-treesitter/nvim-treesitter', {'do': ':TSUpdate'}

" After plug#end(), add:
lua << EOF
require('filament').setup({
  auto_install = true,
})
EOF
```

#### With lazy.nvim (comprehensive)
```lua
{
  'filament-hdl/filament',
  rtp = 'treesitter/tools/vim/filament',
  event = 'BufRead *.fil',
  dependencies = {
    {
      'nvim-treesitter/nvim-treesitter',
      build = ':TSUpdate',
    }
  },
  config = function()
    require('filament').setup({
      auto_install = true,
      treesitter = {
        highlight = { enable = true },
        indent = { enable = true },
        fold = { enable = true },
      }
    })
  end,
}
```

## File Structure

```
vim/filament/
├── after/
│   └── ftplugin/
│       └── filament.lua          # Neovim tree-sitter auto-setup
├── ftdetect/
│   └── filament.vim             # File type detection
├── lua/
│   └── filament/
│       ├── init.lua             # Main plugin module
│       └── treesitter.lua       # Tree-sitter integration
├── plugin/
│   └── filament.vim             # Plugin initialization
├── queries/
│   └── filament/
│       ├── highlights.scm       # Syntax highlighting rules
│       ├── locals.scm          # Local scope definitions
│       └── tags.scm            # Symbol extraction rules
├── syntax/
│   └── filament.vim            # Traditional vim syntax
└── README.md                   # This file
```

## Plugin Manager Specific Notes

### vim-plug
- Use `'rtp': 'treesitter/tools/vim/filament'` to set the correct runtime path
- Tree-sitter parser installation may need to be done manually

### lazy.nvim
- Can use `event = 'BufRead *.fil'` for lazy loading
- Supports automatic parser installation through setup function

### packer.nvim
- Use `rtp = 'treesitter/tools/vim/filament'` for correct path
- Config function runs after plugin loads

### Vundle
- Basic installation only, manual tree-sitter setup required
- Best used with traditional vim syntax highlighting

## API Reference

### Lua API
```lua
local filament = require('filament')

-- Setup the plugin
filament.setup(opts)

-- Tree-sitter specific functions
local ts = require('filament.treesitter')

-- Check installation status
local status = ts.status()
-- Returns: { available = bool, parser_installed = bool, message = string }

-- Install parser manually
ts.install_parser()

-- Setup tree-sitter for current buffer
ts.setup()

-- Register parser configuration
ts.register_parser()
```

## Syntax Highlighting Features

The tree-sitter grammar provides comprehensive highlighting for:

### Keywords
- Component definitions: `comp`, `extern`, `generate`
- Control flow: `if`, `else`, `for`, `in`
- Declarations: `let`, `using`, `import`, `with`, `where`
- Modifiers: `some`, `opaque`, `bundle`, `interface`

### Language Constructs
- **Component signatures**: `comp Add[WIDTH]<'G: 1>(...) -> (...)`
- **Time expressions**: `'G+1`, `'START+DELAY`
- **Parameter bindings**: `[WIDTH, ?DELAY=1]`, `with { some W; }`
- **Port definitions**: `input: ['G, 'G+1] WIDTH`
- **Invocations**: `adder<'G>(left, right)`

### Literals and Types
- Numbers with bitwidth annotations: `32'd1024`
- String literals: `"hardware.sv"`
- Floating point: `3.14159`
- Component and type names
- Parameter and event references

## Troubleshooting

### Tree-sitter Parser Not Found
```
Error: no parser for 'filament' language
```
**Solutions**:
1. **Most likely**: Plugin manager didn't run the build hook. Reinstall with `:PlugClean` then `:PlugInstall`
2. **Manual build**: Run `:FilamentBuildParser` in Neovim
3. **Command line**: `cd treesitter/tools/treesitter && npm run build-parser`

### Plugin Not Loading
1. Check runtimepath: `:set rtp?` should include filament plugin path
2. For plugin managers, ensure correct `rtp` setting
3. Restart editor after installation

### Syntax Highlighting Not Working
1. Check file type: `:set filetype?` (should show `filetype=filament`)
2. Check tree-sitter: `:TSBufToggle highlight`
3. Verify parser: `:checkhealth nvim-treesitter`

### Performance Issues
1. Disable regex highlighting: `additional_vim_regex_highlighting = false`
2. Reduce highlight timeout: `highlight = { enable = true, timeout = 200 }`

## Development

### Testing Your Setup
```bash
# Test tree-sitter parsing
cd filament/treesitter/tools/treesitter
npx tree-sitter test

# Test highlighting
npx tree-sitter highlight test_simple.fil
```

### Contributing
1. Modify source query files in `treesitter/tools/treesitter/queries/`
2. Copy updated files to `vim/filament/queries/filament/`
3. Test with various plugin managers
4. Update documentation

## Related Files

- **Tree-sitter grammar**: `treesitter/tools/treesitter/grammar.js`
- **Query files**: `treesitter/tools/treesitter/queries/*.scm`
- **Test files**: `treesitter/tools/treesitter/test_*.fil`
- **VS Code extension**: `tools/vscode/`

## License

This vim configuration follows the same license as the Filament project.
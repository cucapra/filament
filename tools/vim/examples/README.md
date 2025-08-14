# Plugin Manager Configuration Examples

This directory contains example configurations for different Neovim/Vim plugin managers to help you get started with Filament language support.

## Available Examples

### [vim-plug.vim](vim-plug.vim)
Configuration for vim-plug, one of the most popular plugin managers.
- **Best for**: Users who want a simple, reliable plugin manager
- **Features**: Works with both Vim and Neovim
- **Setup**: Copy relevant sections to your `init.vim` or `.vimrc`

### [lazy.lua](lazy.lua)
Configuration for lazy.nvim, a modern plugin manager for Neovim.
- **Best for**: Neovim users who want fast startup times and lazy loading
- **Features**: Automatic lazy loading, dependency management
- **Setup**: Add to your lazy configuration directory

### [packer.lua](packer.lua)
Configuration for packer.nvim, a popular Lua-based plugin manager.
- **Best for**: Neovim users who prefer Lua configuration
- **Features**: Compiled configuration, dependency management
- **Setup**: Add to your packer configuration file

### [vundle.vim](vundle.vim)
Configuration for Vundle, a classic Vim plugin manager.
- **Best for**: Traditional vim users or simple setups
- **Features**: Basic plugin management, wide compatibility
- **Setup**: Copy to your `.vimrc`

## Quick Start

1. **Choose your plugin manager** from the examples above
2. **Copy the relevant configuration** to your vim/neovim config
3. **Install plugins** using your plugin manager's install command
4. **Install the tree-sitter parser** (see main README for details)
5. **Restart your editor** and open a `.fil` file

## Configuration Comparison

| Plugin Manager | Lazy Loading | Auto Parser Install | Complexity | Recommended For |
|---------------|--------------|-------------------|------------|-----------------|
| lazy.nvim     | ✅ Excellent  | ✅ Yes             | Medium     | Modern Neovim users |
| vim-plug      | ⚠️ Basic      | ✅ Yes             | Low        | General use |
| packer.nvim   | ✅ Good       | ✅ Yes             | Medium     | Lua enthusiasts |
| Vundle        | ❌ None       | ⚠️ Manual          | Low        | Traditional vim |

## Common Features Across All Configurations

All example configurations include:
- **File type detection** for `.fil` files
- **Syntax highlighting** using tree-sitter (Neovim) or vim syntax (fallback)
- **Basic editor settings** (comment string, indentation)
- **Tree-sitter integration** where supported

## Customization

Each example can be customized by:
- **Adding/removing languages** in `ensure_installed`
- **Enabling/disabling features** like folding, indent, incremental selection
- **Adding keybindings** for tree-sitter features
- **Configuring additional plugins** like textobjects or refactoring

## Troubleshooting

If you encounter issues:
1. **Check plugin installation**: Run your plugin manager's status command
2. **Verify tree-sitter parser**: Run `:TSInstall filament` in Neovim
3. **Check file type**: Run `:set filetype?` in a `.fil` file
4. **Review error messages**: Check `:messages` for any errors
5. **Consult the main README**: For detailed troubleshooting steps

## Contributing

To add a new plugin manager example:
1. Create a new file named `{manager-name}.{ext}`
2. Include comprehensive configuration with comments
3. Test with a clean installation
4. Update this README with the new example
5. Follow the existing format and style

## Plugin Manager Resources

- **vim-plug**: https://github.com/junegunn/vim-plug
- **lazy.nvim**: https://github.com/folke/lazy.nvim  
- **packer.nvim**: https://github.com/wbthomason/packer.nvim
- **Vundle**: https://github.com/VundleVim/Vundle.vim
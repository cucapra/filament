#!/bin/bash
set -euf -o pipefail

# Installation script for Filament vim/neovim support
# Usage: ./install.sh [vim|nvim]

EDITOR_TYPE="${1:-auto}"
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

echo "Filament Vim/Neovim Installation Script"
echo "========================================"

# Detect editor type if not specified
if [ "$EDITOR_TYPE" = "auto" ]; then
    if command -v nvim >/dev/null 2>&1; then
        EDITOR_TYPE="nvim"
        echo "Detected: Neovim"
    elif command -v vim >/dev/null 2>&1; then
        EDITOR_TYPE="vim"
        echo "Detected: Vim"
    else
        echo "Error: Neither vim nor nvim found in PATH"
        exit 1
    fi
fi

# Set installation paths
case "$EDITOR_TYPE" in
    nvim)
        if [ -n "${XDG_CONFIG_HOME:-}" ]; then
            INSTALL_DIR="$XDG_CONFIG_HOME/nvim"
        else
            INSTALL_DIR="$HOME/.config/nvim"
        fi
        echo "Installing for Neovim at: $INSTALL_DIR"
        ;;
    vim)
        INSTALL_DIR="$HOME/.vim"
        echo "Installing for Vim at: $INSTALL_DIR"
        ;;
    *)
        echo "Error: Invalid editor type '$EDITOR_TYPE'. Use 'vim' or 'nvim'"
        exit 1
        ;;
esac

# Create installation directory if it doesn't exist
mkdir -p "$INSTALL_DIR"

# Copy filament configuration
echo "Copying filament configuration..."
cp -r "$SCRIPT_DIR/filament" "$INSTALL_DIR/"

echo "✓ Filament configuration installed successfully!"

# Check for tree-sitter parser (neovim only)
if [ "$EDITOR_TYPE" = "nvim" ]; then
    echo ""
    echo "Tree-sitter Setup:"
    echo "=================="
    
    # Check if nvim-treesitter is installed
    if [ -d "$INSTALL_DIR/pack/"*"/start/nvim-treesitter" ] || [ -d "$HOME/.local/share/nvim/site/pack/"*"/start/nvim-treesitter" ]; then
        echo "✓ nvim-treesitter plugin detected"
        
        echo ""
        echo "Next steps:"
        echo "1. Open neovim and run: :TSInstall filament"
        echo "2. Or build the parser manually from the treesitter/ directory"
        echo "3. Open a .fil file to test syntax highlighting"
    else
        echo "⚠ nvim-treesitter plugin not detected"
        echo ""
        echo "To get full tree-sitter support, install nvim-treesitter:"
        echo "- Using packer: use 'nvim-treesitter/nvim-treesitter'"
        echo "- Using lazy: { 'nvim-treesitter/nvim-treesitter' }"
        echo "- Using vim-plug: Plug 'nvim-treesitter/nvim-treesitter'"
    fi
fi

echo ""
echo "Installation complete! 🎉"
echo ""
echo "File type detection is now enabled for .fil files."
echo "Open any Filament file to see syntax highlighting in action."
echo ""
echo "For plugin manager users:"
echo "- See examples/ directory for configuration examples"
echo "- Use 'filament-hdl/filament' with rtp = 'treesitter/tools/vim/filament'"
echo "- Add 'do'/'build'/'run' hook: 'cd treesitter/tools/treesitter && npm install && npm run build-parser'"
echo "- Call require('filament').setup() in your config"
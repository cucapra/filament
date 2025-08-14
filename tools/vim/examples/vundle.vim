" Example Vundle configuration for Filament support
" Add this to your ~/.vimrc

set nocompatible
filetype off

" Set the runtime path to include Vundle and initialize
set rtp+=~/.vim/bundle/Vundle.vim
call vundle#begin()

" Let Vundle manage Vundle, required
Plugin 'VundleVim/Vundle.vim'

" Essential for Neovim tree-sitter support
Plugin 'nvim-treesitter/nvim-treesitter'

" Filament language support
Plugin 'filament-hdl/filament'

" Optional: Additional functionality
Plugin 'nvim-treesitter/nvim-treesitter-textobjects'

call vundle#end()
filetype plugin indent on

" Configure plugins (Neovim only)
if has('nvim')
  lua << EOF
  -- Basic tree-sitter configuration
  require('nvim-treesitter.configs').setup {
    ensure_installed = { 'filament' },
    highlight = {
      enable = true,
      additional_vim_regex_highlighting = false,
    },
    indent = {
      enable = true,
    },
  }
  
  -- Setup filament support
  -- Note: Manual parser installation may be required with Vundle
  require('filament').setup({
    -- Don't auto-install parser with Vundle as it may not work reliably
    auto_install = false,
    treesitter = {
      highlight = { enable = true },
      indent = { enable = true },
    }
  })
EOF
endif

" Manual tree-sitter parser installation notice
if has('nvim')
  augroup FilamentSetup
    autocmd!
    autocmd FileType filament echohl WarningMsg | 
          \ echo "If syntax highlighting doesn't work, install the tree-sitter parser manually:" |
          \ echo ":TSInstall filament" |
          \ echohl None
  augroup END
endif

" Basic Filament file settings for all vim versions
augroup FilamentBasic
  autocmd!
  autocmd BufNewFile,BufRead *.fil setlocal commentstring=//\ %s
  autocmd BufNewFile,BufRead *.fil setlocal comments=://
augroup END
" Example vim-plug configuration for Filament support
" Add this to your ~/.config/nvim/init.vim or ~/.vimrc

call plug#begin()

" Essential plugins
Plug 'nvim-treesitter/nvim-treesitter', {'do': ':TSUpdate'}

" Filament language support with auto-build
Plug 'filament-hdl/filament', {'rtp': 'treesitter/tools/vim/filament', 'do': 'cd treesitter/tools/treesitter && npm install && npm run build-parser'}

" Optional: Additional tree-sitter plugins
Plug 'nvim-treesitter/nvim-treesitter-textobjects'
Plug 'nvim-treesitter/nvim-treesitter-refactor'

call plug#end()

" Configure tree-sitter after plugins load
lua << EOF
-- Basic nvim-treesitter setup
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

-- Setup filament with auto-install
require('filament').setup({
  auto_install = true,
  treesitter = {
    fold = { enable = true },
    incremental_selection = { enable = true },
  }
})
EOF

" Optional: Set up keybindings for tree-sitter features
nnoremap <silent> <leader>th :TSBufToggle highlight<CR>
nnoremap <silent> <leader>ti :TSBufToggle indent<CR>
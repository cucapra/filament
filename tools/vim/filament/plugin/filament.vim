" Filament language support for vim/neovim
" This file is loaded automatically by plugin managers

if exists('g:loaded_filament_plugin')
  finish
endif
let g:loaded_filament_plugin = 1

" Manual parser build function (for users who need it)
function! FilamentBuildParser()
  echo "Building Filament tree-sitter parser..."
  echo "This will build the parser from source."
  echo ""
  
  " Use the lua module which has better path handling
  if has('nvim-0.8')
    lua require('filament.treesitter').install_parser()
  else
    echohl ErrorMsg
    echo "✗ This command requires Neovim 0.8+ for tree-sitter support"
    echo "For manual build, run:"
    echo "  cd <filament-repo>/treesitter/tools/treesitter"
    echo "  npm install && npm run build-parser"
    echohl None
  endif
endfunction

" Command to manually build parser if needed
command! FilamentBuildParser call FilamentBuildParser()

" Auto-setup tree-sitter highlighting when opening filament files
augroup filament_treesitter_setup
  autocmd!
  if has('nvim-0.8')
    autocmd FileType filament lua require('filament.treesitter').setup()
  endif
augroup END
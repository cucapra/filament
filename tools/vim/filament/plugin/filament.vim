" Filament language support for vim/neovim
" This file is loaded automatically by plugin managers

if exists('g:loaded_filament_plugin')
  finish
endif
let g:loaded_filament_plugin = 1

" Build parser on plugin installation (for plugin managers with hooks)
function! s:BuildFilamentParser()
  let l:plugin_dir = expand('<sfile>:p:h:h')
  let l:treesitter_dir = fnamemodify(l:plugin_dir, ':h:h') . '/treesitter'
  
  if isdirectory(l:treesitter_dir)
    echo "Building Filament tree-sitter parser..."
    let l:result = system('cd "' . l:treesitter_dir . '" && npm install && npm run build-parser')
    if v:shell_error == 0
      echo "✓ Filament parser built successfully"
    else
      echohl ErrorMsg
      echo "✗ Failed to build Filament parser. Please run manually:"
      echo "  cd " . l:treesitter_dir . " && npm run build-parser"
      echohl None
    endif
  endif
endfunction

" Auto-build on first load (vim-plug post-update hook)
if !exists('g:filament_parser_built')
  let g:filament_parser_built = 1
  call s:BuildFilamentParser()
endif

" Auto-setup tree-sitter highlighting when opening filament files
augroup filament_treesitter_setup
  autocmd!
  if has('nvim-0.8')
    autocmd FileType filament lua require('filament.treesitter').setup()
  endif
augroup END
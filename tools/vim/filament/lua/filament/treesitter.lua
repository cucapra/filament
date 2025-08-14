-- Filament tree-sitter integration module
local M = {}

-- Check if tree-sitter is available
local function has_treesitter()
  return vim.fn.has('nvim-0.8') == 1 and pcall(require, 'nvim-treesitter')
end

-- Setup tree-sitter for filament buffers
function M.setup()
  if not has_treesitter() then
    return false
  end

  local ok, parsers = pcall(require, 'nvim-treesitter.parsers')
  if not ok then
    return false
  end

  -- Check if filament parser is available
  if not parsers.has_parser('filament') then
    vim.notify_once(
      'Tree-sitter parser for Filament not found. Install with :TSInstall filament',
      vim.log.levels.WARN
    )
    return false
  end

  -- Start tree-sitter for current buffer
  vim.treesitter.start(0, 'filament')

  -- Set up buffer-local options
  vim.bo.commentstring = '// %s'
  vim.bo.comments = '://'

  -- Enable folding if supported
  if vim.fn.has('nvim-0.9') == 1 then
    vim.wo.foldmethod = 'expr'
    vim.wo.foldexpr = 'nvim_treesitter#foldexpr()'
    vim.wo.foldenable = false
  end

  return true
end

-- Register filament parser using official Neovim API
function M.register_parser()
  if not has_treesitter() then
    return false
  end

  -- Get the path to our compiled parser
  local script_path = debug.getinfo(1, "S").source:sub(2)
  local plugin_dir = vim.fn.fnamemodify(script_path, ":h:h:h")  -- /path/to/vim/filament
  local treesitter_dir = vim.fn.fnamemodify(plugin_dir, ":h:h") .. "/treesitter"  -- /path/to/tools/treesitter
  local parser_path = treesitter_dir .. "/src/parser.so"

  -- Check if parser exists
  if vim.fn.filereadable(parser_path) == 0 then
    vim.notify_once(
      string.format(
        'Filament parser not found in `%s`. Build it with: :FilamentBuildParser',
        parser_path
      ), vim.log.levels.WARN)
    return false
  end

  -- Register the language using the official API
  vim.treesitter.language.add('filament', { path = parser_path })

  -- Register the filetype association
  vim.treesitter.language.register('filament', { 'fil' })

  return true
end

-- Manual installation helper
function M.install_parser()
  -- With the new approach, we build the parser from source
  local script_path = debug.getinfo(1, "S").source:sub(2)
  local plugin_dir = vim.fn.fnamemodify(script_path, ":h:h:h")  -- /path/to/vim/filament
  local treesitter_dir = vim.fn.fnamemodify(plugin_dir, ":h") .. "/treesitter"  -- /path/to/tools/treesitter

  vim.notify('Building Filament parser from source...', vim.log.levels.INFO)

  -- Run the build command
  local handle = io.popen('cd "' .. treesitter_dir .. '" && npm run build-parser 2>&1')
  local result = handle:read("*a")
  local success = handle:close()

  if success then
    vim.notify('✓ Filament parser built successfully', vim.log.levels.INFO)
    -- Register the parser
    return M.register_parser()
  else
    vim.notify('✗ Failed to build parser:\n' .. result, vim.log.levels.ERROR)
    return false
  end
end

-- Check installation status
function M.status()
  if not has_treesitter() then
    return {
      available = false,
      parser_installed = false,
      message = 'Tree-sitter not available (requires Neovim 0.8+)'
    }
  end

  -- Check if our local parser exists
  local script_path = debug.getinfo(1, "S").source:sub(2)
  local plugin_dir = vim.fn.fnamemodify(script_path, ":h:h:h")  -- /path/to/vim/filament
  local treesitter_dir = vim.fn.fnamemodify(plugin_dir, ":h") .. "/treesitter"  -- /path/to/tools/treesitter
  local parser_path = treesitter_dir .. "/src/parser.so"
  local parser_installed = vim.fn.filereadable(parser_path) == 1

  -- Also check if the language is registered with Neovim
  local lang_registered = false
  local ok = pcall(function()
    vim.treesitter.language.inspect('filament')
    lang_registered = true
  end)

  local message
  if parser_installed and lang_registered then
    message = 'Filament tree-sitter support ready'
  elseif parser_installed then
    message = 'Parser built but not registered (run setup)'
  else
    message = 'Parser not built (run install_parser)'
  end

  return {
    available = true,
    parser_installed = parser_installed,
    lang_registered = lang_registered,
    parser_path = parser_path,
    message = message
  }
end

return M

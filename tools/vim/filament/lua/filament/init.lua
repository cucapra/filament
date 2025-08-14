-- Filament language support main module
local M = {}

-- Setup function for plugin managers
function M.setup(opts)
  opts = opts or {}
  
  -- Register tree-sitter parser
  local treesitter = require('filament.treesitter')
  treesitter.register_parser()
  
  -- Configure nvim-treesitter if available
  local ok, ts_config = pcall(require, 'nvim-treesitter.configs')
  if ok and not opts.skip_treesitter_config then
    ts_config.setup(vim.tbl_deep_extend('force', {
      ensure_installed = {},
      highlight = { enable = true },
      indent = { enable = true },
    }, opts.treesitter or {}))
  end
  
  -- Auto-install parser if explicitly requested (not recommended)
  if opts.auto_install then
    vim.schedule(function()
      local status = treesitter.status()
      if status.available and not status.parser_installed then
        vim.notify('Auto-installing Filament parser...', vim.log.levels.INFO)
        treesitter.install_parser()
      end
    end)
  end
  
  return M
end

-- Export submodules
M.treesitter = require('filament.treesitter')

return M
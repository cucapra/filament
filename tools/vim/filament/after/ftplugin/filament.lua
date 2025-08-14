-- Neovim tree-sitter configuration for Filament
-- This file is loaded after the default ftplugin for filament files

-- Use the filament treesitter module for setup
local ok, filament = pcall(require, 'filament.treesitter')
if ok then
  filament.setup()
else
  -- Fallback to manual setup if module not available
  if vim.fn.has('nvim-0.8') == 1 and vim.treesitter then
    local parsers_ok, parsers = pcall(require, "nvim-treesitter.parsers")
    if parsers_ok and parsers.has_parser("filament") then
      vim.treesitter.start(0, "filament")
      vim.bo.commentstring = "// %s"
      vim.bo.comments = "://"
    end
  end
end
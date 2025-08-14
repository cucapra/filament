-- Example lazy.nvim configuration for Filament support
-- Add this to your ~/.config/nvim/lua/plugins/filament.lua
-- or include in your main lazy configuration

return {
  {
    'nvim-treesitter/nvim-treesitter',
    build = ':TSUpdate',
    config = function()
      require('nvim-treesitter.configs').setup {
        ensure_installed = {
          'filament',
          'lua', 'vim', 'vimdoc', -- Add other languages you use
        },
        auto_install = true,
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
    end,
  },
  {
    'filament-hdl/filament',
    rtp = 'treesitter/tools/vim/filament',
    ft = 'filament', -- Lazy load on filament files
    build = 'cd treesitter/tools/treesitter && npm install && npm run build-parser',
    dependencies = { 'nvim-treesitter/nvim-treesitter' },
    config = function()
      require('filament').setup({
        -- Parser built by 'build' hook above, no auto_install needed
        treesitter = {
          highlight = { enable = true },
          indent = { enable = true },
          fold = { enable = true },
        }
      })
      
      -- Optional: Set up filament-specific keybindings
      vim.api.nvim_create_autocmd("FileType", {
        pattern = "filament",
        callback = function()
          local opts = { buffer = true, silent = true }
          vim.keymap.set('n', '<leader>ts', function()
            local status = require('filament.treesitter').status()
            vim.notify(status.message, vim.log.levels.INFO)
          end, opts)
        end,
      })
    end,
  },
  
  -- Optional: Additional tree-sitter plugins
  {
    'nvim-treesitter/nvim-treesitter-textobjects',
    dependencies = { 'nvim-treesitter/nvim-treesitter' },
    config = function()
      require('nvim-treesitter.configs').setup {
        textobjects = {
          select = {
            enable = true,
            lookahead = true,
            keymaps = {
              ["af"] = "@function.outer",
              ["if"] = "@function.inner",
              ["ac"] = "@class.outer",
              ["ic"] = "@class.inner",
            },
          },
        },
      }
    end,
  },
}
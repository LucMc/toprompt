A simple rust cli tool to add mutliple files into a nice LLM prompt.
Simply add to `toprompt/target/release' to path and run:
```sh
toprompt file1.py file2.rs
```
etc with all the files you'd like formatted and appended to the prompt.

In addition to this, I have a neovim keymap for copying code encapsulated by markdown code formatting:

```lua
local function yank_with_codeblock()
  -- Get the visual selection range
  local start_row, start_col = unpack(vim.fn.getpos("'<"), 2, 3)
  local end_row, end_col = unpack(vim.fn.getpos("'>"), 2, 3)
  
  -- Get the lines in the visual selection
  local lines = vim.api.nvim_buf_get_lines(0, start_row - 1, end_row, false)
  
  -- Handle partial line selection for the first and last lines
  if #lines > 0 then
    -- Adjust first line
    if start_col > 1 then
      lines[1] = string.sub(lines[1], start_col)
    end
    
    -- Adjust last line
    if #lines > 1 and end_col < #lines[#lines] + 1 then
      lines[#lines] = string.sub(lines[#lines], 1, end_col)
    elseif #lines == 1 and end_col < #lines[1] + 1 then
      lines[1] = string.sub(lines[1], 1, end_col - start_col + 1)
    end
  end
  
  -- Get the current buffer's filetype
  local filetype = vim.bo.filetype
  
  -- If no filetype is set, try to use the file extension
  if filetype == "" then
    local filename = vim.fn.expand("%:t")
    local ext = vim.fn.fnamemodify(filename, ":e")
    if ext ~= "" then
      filetype = ext
    end
  end
  
  -- Create the code block
  local result = { "```" .. filetype }
  vim.list_extend(result, lines)
  table.insert(result, "```")
  
  -- Join with newlines and set to clipboard and default register
  local content = table.concat(result, "\n")
  vim.fn.setreg('"', content)
  vim.fn.setreg("+", content) -- Also copy to system clipboard
  
  -- Exit visual mode
  vim.api.nvim_feedkeys(vim.api.nvim_replace_termcodes("<Esc>", true, false, true), "n", false)
  
  -- Optional: Show a message
  vim.notify("Yanked " .. #lines .. " lines with " .. filetype .. " code block wrapper", vim.log.levels.INFO)
end

-- Create the keymap for visual mode
vim.keymap.set("v", "<leader>yp", yank_with_codeblock, { 
  desc = "Yank selection with code block wrapper",
  silent = true 
})

-- Alternative version using operator-pending mode (if you prefer)
-- This allows you to use motions like 'ypip' (yank paragraph with code block)
local function yank_with_codeblock_operator(type)
  local start_pos, end_pos
  
  if type == "line" then
    start_pos = vim.fn.getpos("'[")
    end_pos = vim.fn.getpos("']")
  else
    start_pos = vim.fn.getpos("'<")
    end_pos = vim.fn.getpos("'>")
  end
  
  local start_row, start_col = start_pos[2], start_pos[3]
  local end_row, end_col = end_pos[2], end_pos[3]
  
  local lines = vim.api.nvim_buf_get_lines(0, start_row - 1, end_row, false)
  
  if #lines > 0 and type ~= "line" then
    if start_col > 1 then
      lines[1] = string.sub(lines[1], start_col)
    end
    if end_col < #lines[#lines] + 1 then
      lines[#lines] = string.sub(lines[#lines], 1, end_col)
    end
  end
```

Roadmap:
 - Prompt templats for basic validation - I.e. 'Are there any logical errors in this code?' etc
 - Direct LLM integration to hit key and run


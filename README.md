A simple rust cli tool to add mutliple files into a nice LLM prompt.
Simply add to `toprompt/target/release' to path and run:
```sh
toprompt file1.py file2.rs
```
etc with all the files you'd like formatted and appended to the prompt.

In addition to this, I have a neovim keymap for copying code encapsulated by markdown code formatting:

```lua
local function yank_with_codeblock()
  -- Get the current buffer's filetype
  local filetype = vim.bo.filetype
  if filetype == "" then
    local ext = vim.fn.fnamemodify(vim.fn.expand("%:t"), ":e")
    if ext ~= "" then
      filetype = ext
    end
  end
  
  -- Yank the visual selection to a register
  vim.cmd('normal! "zy')
  
  -- Get the yanked content
  local content = vim.fn.getreg('z')
  
  -- Remove trailing newline if present
  content = content:gsub("\n$", "")
  
  -- Create the code block
  local wrapped = string.format("```%s\n%s\n```", filetype, content)
  
  -- Set to clipboard and default register
  vim.fn.setreg('"', wrapped)
  vim.fn.setreg("+", wrapped)
  
  -- Show a message
  local line_count = select(2, content:gsub('\n', '\n')) + 1
  vim.notify(string.format("Yanked %d line%s with %s code block", 
    line_count, 
    line_count == 1 and "" or "s",
    filetype ~= "" and filetype or "plain text"
  ), vim.log.levels.INFO)
end

vim.keymap.set("v", "<leader>yp", yank_with_codeblock, { 
  desc = "Yank selection with code block wrapper",
  silent = true 
})
```

Roadmap:
 - Prompt templats for basic validation - I.e. 'Are there any logical errors in this code?' etc
 - Direct LLM integration to hit key and run


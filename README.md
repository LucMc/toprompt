A simple rust cli tool to add mutliple files into a nice LLM prompt.

```sh
toprompt file1.py file2.rs
```
etc with all the files you'd like formatted and appended to the prompt.
The above command would copy the following to the system clipboard:
~~~
# file1.py
```python
print("hello world")
```
# file2.rs
```rust
fn main() {
    println!("Hello, world!");
}
```
~~~

## Advanced options
```sh
toprompt *.py # wildcards/ regex for specific files
toprompt . # Copy all files in current/specified folder
toprompt -r . # Copy all files in current/specified folder and subfolders recursively
toprompt -i . # Use .gitignore to not copy exclude specified files from copying
toprompt -ri . # Use .gitignore and recuse through subfolders
toprompt -i -R ".*\.py" . # Copy all python files in current/specified folder and subfolders recursively and use .gitignore
toprompt --xml "example.py" . # Copy files in XML format (best for Claude, see: https://docs.anthropic.com/en/docs/build-with-claude/prompt-engineering/use-xml-tags#why-use-xml-tags%3F)
```

# Installation
Install with Cargo (Recommended):
```sh cargo install toprompt```

Alternatively, install from source
```sh git clone https://github.com/LucMc/toprompt ```
Then add to `toprompt/target/release' to path.

# NeoVim Bonus
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
 - Prompt templates for basic validation - I.e. 'Are there any logical errors in this code?', generating tests, explaining, etc.
 - Direct LLM integration to hit key and run
 - Add `--tree` option, especially for nested folders


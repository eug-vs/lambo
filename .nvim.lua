vim.api.nvim_create_augroup("LamboSyntax", { clear = true })
vim.api.nvim_create_autocmd({ "BufRead", "BufNewFile" }, {
	pattern = "*.lambo",
	callback = function()
		vim.cmd("setfiletype ocaml")
		-- disable tree-sitter highlighting for this buffer
		if vim.fn.expand("%:e") == "lambo" then
			vim.cmd("TSBufDisable highlight")
		end
	end,
	group = "LamboSyntax",
})
vim.api.nvim_create_user_command("LamboRun", function(opts)
	-- Determine input: use args if provided, otherwise current file
	local input = opts.args ~= "" and opts.args or vim.fn.expand("%")
	local bufname = "/tmp/result.lambo"
	local bufnr = vim.fn.bufnr(bufname)

	if bufnr ~= -1 then
		-- Buffer exists somewhere, try to jump to it
		local win_found = false
		-- iterate all windows
		for _, win in ipairs(vim.api.nvim_list_wins()) do
			if vim.api.nvim_win_get_buf(win) == bufnr then
				vim.api.nvim_set_current_win(win) -- focus the window
				win_found = true
				break
			end
		end

		if not win_found then
			-- buffer exists but not displayed, open in current window
			vim.cmd("buffer " .. bufnr)
		end
	else
		-- buffer does not exist, open it
		vim.cmd("edit " .. bufname)
		bufnr = vim.fn.bufnr(bufname)
	end

	-- Clear buffer
	vim.cmd("%delete _")

	-- Read command output
	vim.cmd("read ! time cat " .. input .. ' | RUSTFLAGS="-Awarnings" cargo run --release --quiet ')

	-- Save output
	vim.cmd("write!")

	-- Optional: apply syntax highlighting for .lambo
	vim.cmd("setfiletype ocaml")
	vim.cmd("TSBufDisable highlight")
end, { nargs = "*" })

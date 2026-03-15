Output clean-1-setup.gif

Set Shell zsh
Set FontSize 16
Set Width 1200
Set Height 600
Set Padding 20
Set Theme "Catppuccin Mocha"
Set TypingSpeed 40ms

Hide
Type `export HOME=/tmp/demo-clean CLICOLOR_FORCE=1 && rm -rf $HOME && mkdir -p $HOME/.arbor`
Enter
Sleep 500ms
Type `printf 'worktree_dir = "~/.arbor/worktrees"\nrepos_dir = "~/.arbor/repos"\n' > $HOME/.arbor/config.toml`
Enter
Sleep 500ms
Type `mkdir -p /tmp/my-app && cd /tmp/my-app && rm -rf .git && git init -q -b main && git config user.email dev@example.com && git config user.name Dev && git commit -q --allow-empty -m "initial commit"`
Enter
Sleep 1s
Type `eval "$(arbor init zsh)"`
Enter
Sleep 500ms
Type "clear"
Enter
Sleep 500ms
Show

Sleep 500ms

Type "arbor add feature/auth"
Sleep 300ms
Enter
Sleep 1.5s

Type "arbor add fix/login-crash"
Sleep 300ms
Enter
Sleep 1.5s

Type "arbor add feat/dashboard"
Sleep 300ms
Enter
Sleep 1.5s

Type "arbor add chore/update-deps"
Sleep 300ms
Enter
Sleep 1.5s

Hide
Type `arbor switch feature/auth`
Enter
Sleep 1s
Type "clear"
Enter
Sleep 500ms
Show

Sleep 500ms

Type "arbor ls"
Sleep 300ms
Enter
Sleep 3s

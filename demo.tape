Output demo.gif

Set Shell zsh
Set FontSize 16
Set Width 1200
Set Height 500
Set Padding 20
Set Theme "Catppuccin Mocha"
Set TypingSpeed 40ms

Hide
Type `export HOME=/tmp/demo-arbor CLICOLOR_FORCE=1 && rm -rf $HOME && mkdir -p $HOME/.arbor`
Enter
Sleep 500ms
Type `printf 'worktree_dir = "~/.arbor/worktrees"\nrepos_dir = "~/.arbor/repos"\n' > $HOME/.arbor/config.toml`
Enter
Sleep 500ms
Type `mkdir -p /tmp/my-app && cd /tmp/my-app && rm -rf .git && git init -q && git config user.email test@test.com && git config user.name Test && git commit -q --allow-empty -m init`
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

Type "arbor add fix/typo"
Sleep 300ms
Enter
Sleep 1.5s

Type "arbor ls"
Sleep 300ms
Enter
Sleep 2s

Type "arbor status"
Sleep 300ms
Enter
Sleep 2s

Hide
Type "cd /tmp/my-app && clear"
Enter
Sleep 500ms
Show

Type "arbor rm fix/typo"
Sleep 300ms
Enter
Sleep 1.5s

Type "arbor ls"
Sleep 300ms
Enter
Sleep 2s

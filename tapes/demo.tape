Output demo.gif

Set Shell zsh
Set FontSize 20
Set Width 1200
Set Height 600
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
Type `mkdir -p /tmp/my-app && cd /tmp/my-app && rm -rf .git && git init -q && git config user.email dev@example.com && git config user.name Dev && echo "# my-app" > README.md && git add . && git commit -q -m "initial commit"`
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

# Start a feature — arbor creates a worktree and cd's into it
Type "arbor add feature/auth"
Sleep 300ms
Enter
Sleep 1.5s

# Do some work in the feature worktree
Type "echo 'TODO: implement OAuth flow' > auth.rs"
Sleep 300ms
Enter
Sleep 500ms

# Urgent bug — spin up another worktree, no stashing needed
Type "arbor add fix/login-crash"
Sleep 300ms
Enter
Sleep 1.5s

# See all worktrees at a glance
Type "arbor ls"
Sleep 300ms
Enter
Sleep 2.5s

# Bug fixed — switch back to the feature
Type "arbor switch feature/auth"
Sleep 300ms
Enter
Sleep 1.5s

# Your work is right where you left it
Type "cat auth.rs"
Sleep 300ms
Enter
Sleep 2s

# Clean up — remove the worktree and its branch
Type "arbor rm -d fix/login-crash"
Sleep 300ms
Enter
Sleep 2s

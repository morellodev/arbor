Output demo.gif

Set Shell zsh
Set FontSize 16
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
Type `mkdir -p /tmp/my-app && cd /tmp/my-app && rm -rf .git && git init -q && git config user.email dev@example.com && git config user.name Dev && git commit -q --allow-empty -m "initial commit"`
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
Type "echo 'TODO: implement OAuth' > auth.rs"
Sleep 300ms
Enter
Sleep 500ms

# Urgent bug report — spin up another worktree without stashing
Type "arbor add fix/login-crash"
Sleep 300ms
Enter
Sleep 1.5s

# See all worktrees at a glance
Type "arbor ls"
Sleep 300ms
Enter
Sleep 2s

# Check which worktrees are dirty or ahead/behind
Type "arbor ls --short"
Sleep 300ms
Enter
Sleep 2s

# Bug fixed — switch back to the feature
Type "arbor switch feature/auth"
Sleep 300ms
Enter
Sleep 1.5s

# Clean up the fix branch and its worktree
Type "arbor rm -d fix/login-crash"
Sleep 300ms
Enter
Sleep 1.5s

Type "arbor ls"
Sleep 300ms
Enter
Sleep 2s

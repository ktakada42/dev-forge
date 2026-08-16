# VHS tape for the README hero GIF.
#
# Regenerate with:
#   cargo build --release && vhs assets/demo.tape
#
# Without vhs installed, the official image records the same tape — it needs a
# Linux binary in target/release rather than the local build:
#   docker run --rm -v "$PWD":/vhs -w /vhs ghcr.io/charmbracelet/vhs assets/demo.tape
#
# Records the locally built binary, so the GIF always reflects the current tree
# rather than whatever `forge` is on PATH.

Output assets/demo.gif

Set Shell bash
Set FontSize 16
Set Width 900
Set Height 500
Set Padding 24
Set Theme "Catppuccin Mocha"
Set TypingSpeed 90ms

# Put the freshly built binary first on PATH and start from a clean screen.
Hide
Type "export PATH=$PWD/target/release:$PATH && PS1='$ ' && clear"
Enter
Sleep 1s
Show

Type "forge"
Enter

# The hammer falls for about a second, and the tool list comes up under it.
Sleep 2.5s

# Typing narrows the list, which is what the placeholder invites.
Type "b"
Sleep 1.2s
Enter
Sleep 1.2s

# `encode` is already under the cursor in the direction list.
Enter
Sleep 1.5s

Type "hello world"
Sleep 600ms
Enter

# Hold on the answer so it stays readable before the GIF loops.
Sleep 2.5s

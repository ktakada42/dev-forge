# VHS tape for the README hero GIF.
#
# Regenerate with:
#   cargo build --release && vhs assets/demo.tape
#
# Records the startup animation of the locally built binary, so the GIF always
# reflects the current tree rather than whatever `forge` is on PATH.

Output assets/demo.gif

Set Shell bash
Set FontSize 16
Set Width 900
Set Height 680
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

# Animation runs for ~1s; hold on the settled banner so it stays readable
# before the GIF loops. No `exit` — typing it only scrolls the banner away.
Sleep 3s

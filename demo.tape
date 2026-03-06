# VHS tape file for eulumdat-tui demo
# Install: brew install vhs
# Run:     vhs demo.tape

Output demo-glass-MEASURED_IESNA.gif
Output demo-glass-MEASURED_IESNA.mp4

Set Shell "zsh"
Set FontSize 14
Set Width 1400
Set Height 800
Set Theme "Catppuccin Mocha"
Set TypingSpeed 50ms
Set Padding 10

# Build first (off-screen)
Hide
Type "cargo build --release -p eulumdat-tui 2>/dev/null"
Enter
Sleep 15s
Show

# Launch with fluorescent luminaire
Type 'cargo run --release -p eulumdat-tui -- "crates/eulumdat-wasm/templates/S01.01.02.354_MAXWELL-8-T4 LUXEON 5050 Square with glass-MEASURED_IESNA.ies"'
Enter
Sleep 2s

# Show the default polar diagram
Sleep 3s

# Cycle C-planes with j/k
Type "j"
Sleep 1.5s
Type "j"
Sleep 1.5s
Type "j"
Sleep 1.5s
Type "k"
Sleep 1.5s

# Reset
Type "r"
Sleep 1s

# Zoom in
Type "+"
Sleep 0.5s
Type "+"
Sleep 0.5s
Type "+"
Sleep 1.5s

# Reset
Type "r"
Sleep 1s

# Switch to Cartesian
Type "v"
Sleep 3s

# Switch to Heatmap
Type "v"
Sleep 3s

# Switch to Cone
Type "v"
Sleep 3s

# Switch to Butterfly
Type "v"
Sleep 3s

# Back to Polar
Type "v"
Sleep 2s

# Focus sidebar, scroll info
Tab
Sleep 0.5s
Type "j"
Sleep 0.3s
Type "j"
Sleep 0.3s
Type "j"
Sleep 0.3s
Type "j"
Sleep 0.3s
Type "j"
Sleep 1.5s

# Back to diagram
Tab
Sleep 1s

# Quit
Type "q"
Sleep 1s

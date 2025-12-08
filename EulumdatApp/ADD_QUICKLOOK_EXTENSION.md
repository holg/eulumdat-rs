# Adding QuickLook Extension to EulumdatApp

I've created the QuickLook extension files in `EulumdatQuickLook/`. Now you need to add this extension to your Xcode project.

## Method 1: Add via Xcode (Recommended)

1. **Open Xcode project:**
   ```bash
   open /Users/htr/Documents/develeop/rust/eulumdat-rs/EulumdatApp/EulumdatApp.xcodeproj
   ```

2. **Add new QuickLook Extension target:**
   - File → New → Target
   - Select "Quick Look Preview Extension" (under macOS)
   - Product Name: `EulumdatQuickLook`
   - Click "Finish"
   - When prompted "Activate scheme?", click "Activate"

3. **Replace generated files:**
   - Delete the generated `PreviewProvider.swift` and `Info.plist` from the new target
   - In Xcode's Project Navigator, right-click `EulumdatQuickLook` group
   - Select "Add Files to EulumdatApp..."
   - Navigate to `EulumdatQuickLook/` folder and select:
     - `PreviewProvider.swift`
     - `Info.plist`
   - Make sure "Copy items if needed" is **unchecked**
   - Make sure `EulumdatQuickLook` target is selected
   - Click "Add"

4. **Link EulumdatKit framework:**
   - Select the `EulumdatQuickLook` target in Project Settings
   - Go to "General" tab
   - Under "Frameworks and Libraries", click "+"
   - Search for `EulumdatKit`
   - Add it with "Embed & Sign"

5. **Configure Bundle Identifier:**
   - Still in `EulumdatQuickLook` target settings
   - Under "General" → "Identity"
   - Set Bundle Identifier to: `com.yourcompany.EulumdatApp.QuickLook`
   - (Replace `com.yourcompany` with your actual bundle ID prefix)

6. **Build the extension:**
   ```bash
   cd /Users/htr/Documents/develeop/rust/eulumdat-rs/EulumdatApp
   xcodebuild -scheme EulumdatApp -configuration Release
   ```

## Method 2: Command-Line (Alternative)

If you prefer automation, I can create a Ruby script that uses `xcodeproj` gem to add the target programmatically. Let me know if you'd like this approach.

## Testing the QuickLook Preview

1. **Build and install the app:**
   ```bash
   cd /Users/htr/Documents/develeop/rust/eulumdat-rs/EulumdatApp
   xcodebuild -scheme EulumdatApp -configuration Release

   # Install to /Applications
   rm -rf /Applications/EulumdatApp.app
   cp -R build/Release/EulumdatApp.app /Applications/
   ```

2. **Reset QuickLook cache:**
   ```bash
   qlmanage -r
   qlmanage -r cache
   ```

3. **Test with a file:**
   ```bash
   # Test preview generation directly
   qlmanage -p EulumdatApp/Resources/Templates/road_luminaire.ldt

   # Or test in Finder:
   # Navigate to a folder with .ldt or .ies files
   # Select a file and press Space bar
   ```

4. **Debug if preview doesn't show:**
   ```bash
   # Check console for errors
   log stream --predicate 'subsystem contains "com.apple.QuickLook"' --level debug

   # Verify extension is embedded
   ls -la /Applications/EulumdatApp.app/Contents/PlugIns/

   # Should see: EulumdatQuickLook.appex
   ```

## Optional: Install rsvg-convert for Better SVG Rendering

The QuickLook extension uses `rsvg-convert` if available for better SVG rendering:

```bash
# Install via Homebrew
brew install librsvg

# Verify installation
which rsvg-convert
# Should output: /opt/homebrew/bin/rsvg-convert
```

Without `rsvg-convert`, it falls back to NSImage which has limited SVG support.

## Troubleshooting

**Extension not loading:**
- Verify bundle ID in extension matches app bundle ID + `.QuickLook`
- Check code signing: `codesign -dv /Applications/EulumdatApp.app/Contents/PlugIns/EulumdatQuickLook.appex`
- Ensure extension is copied to PlugIns folder during build

**Preview shows error:**
- Check Console.app for crash logs
- Test parsing manually: `swift run EulumdatApp` and open the file in the app
- Verify EulumdatKit is properly linked

**SVG not rendering:**
- Install `librsvg`: `brew install librsvg`
- Check if rsvg-convert is found: `ls -la /opt/homebrew/bin/rsvg-convert`

## Next Steps

After adding the extension:
1. Test previews in Finder with various .ldt and .ies files
2. Verify previews work for all diagram types
3. Test with edge cases (absolute photometry, TILT files, etc.)
4. Build archive for App Store submission

---

## What the QuickLook Extension Does

- **Parses** .ldt and .ies files using the EulumdatKit framework
- **Generates** polar candela diagram SVG (most useful preview)
- **Renders** SVG to PNG for display in QuickLook
- **Shows** in Finder when user presses Space on a file
- **Displays** thumbnails in Finder icon/column view (if thumbnail provider added)

The preview appears instantly when you select a photometric file in Finder and press Space bar!

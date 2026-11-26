# Player icon

## November 25, 2026

The initial five-layer Icon Composer design placed the playhead at x=581 on a track from x=265 to x=850. The light appearance uses blue controls on white. The catalog includes a dark appearance and a tintable stack. The welcome screen uses the separately exported clear foreground from the same design.

| Acceptance check | Result |
| --- | --- |
| `actool app/icons/AppIcon.icon --compile "$out" --output-partial-info-plist "$out/partial.plist" --app-icon AppIcon --include-all-app-icons --enable-on-demand-resources NO --target-device mac --minimum-deployment-target 14.0 --platform macosx` | Apple’s 27.2 compiler produced the checked-in `app/icons/Assets.car`. `assetutil --info` reported an AppIcon stack for Aqua, Dark Aqua and the system tint. |
| `./ui/node_modules/.bin/tauri bundle --bundles app --config tauri.bundle.conf.json --no-sign --no-binary-patching` in `app/` | An isolated bundle using a copy of the existing release binary contained byte-identical copies of the CAR and ICNS. Its plist named `AppIcon` and `icon.icns`. `NSWorkspace.iconForFile:` rendered the footprints, play control and playhead in the Mac’s selected green system tint. |
| `iconutil --convert iconset --output "$out/fallback.iconset" app/icons/icon.icns` | Decoded all ten standard and Retina representations from 16 to 1,024 pixels. |
| `python3 -c "from PIL import Image; print(sorted(Image.open('app/icons/icon.ico').ico.sizes()))"` | All seven Windows ICO representations decoded at 16, 24, 32, 48, 64, 128 and 256 pixels. The executable, installer and uninstaller reference this file. |
| `npm --prefix app/ui run build` and a 720 × 430 Chrome window | The favicon loaded. Changing among all four accents in Settings kept the foreground 48 × 48 CSS pixels and preserved transparent space between the motifs. |

For future artwork changes, recompile the document with the command above and copy `$out/Assets.car` into `app/icons`. For the initial triangle design, the static ICNS, ICO and PNGs were generated with the pinned Tauri CLI’s `icon` command from the approved sRGB white-and-blue 1,024 px render. The 32 px PNG and favicon use the separately rendered Dock size. The ICO includes this 32 px image and an additional 128 px representation. Normal package builds consume these checked-in outputs.

## In-app brand ink

The original theme picker and the Hen Local Translator palette specify blue `#0003FE`, orange `#FF5705`, pink `#FF0073` and green `#51F91B`. The welcome mark reuses the Icon Composer footprints, play, elapsed and playhead masks over the transparent glass export. This preserves the neutral playhead and remaining track while applying the selected ink to the other three layers. The Settings swatch and the decorative audio waveform also use this ink. Smaller foreground marks on dark glass have separate colors. Solid actions pair white text with blue and dark text with orange, pink and green.

`node /tmp/hen-brand-ink/check.cjs WebKit` and its Chrome counterpart sampled the rendered welcome mark for every theme at 2× scale. The same checks passed against the static production build. Each individual footprint, the play triangle and the elapsed track had pixels within two channel values of the palette, and the remaining track and playhead retained neutral highlights. The functional colors measured at least 4.66:1 against a `#51535A` gray surface, and text on the four solid actions measured at least 4.95:1.

## Combined play and pause mark

The transport occupies x=150–383 on the 1,024-unit canvas. Its two 36-unit pause bars share a baseline with the triangle. The shorter track begins at x=425 and ends at x=850, with its playhead at x=640. The footprint layer and its placement are identical across the two designs. The welcome screen uses the same control shapes in its accent mask and transparent glass foreground.

- Recompile the layered catalog with the `actool` command above. Use Icon Composer’s `ictool --export-image` on the default macOS appearance for individual raster sizes. Convert static fallback images to 8-bit RGBA sRGB before assembling the ten ICNS entries with `iconutil` and the seven ICO entries. The 16- and 24-pixel fallbacks give each pause bar a separate pixel column at x=5 and 7, or x=7 and 9, respectively. The favicon and Tauri’s 32-pixel image share one render.
- On the signed bundle, `NSWorkspace.icon(forFile:)` rendered the new symbol in the Mac’s selected green tint. Cross-sections through the glyph at 32, 48, 64 and 128 pixels resolved the triangle, both bars and the gaps. The 48-pixel glyph spans nine rendered pixels, compared with two for the old triangle.

## Compact play/pause mark

On the 1,024-unit canvas the combined glyph now spans 177 units and is 104 units tall. The previous combination was 233 × 126. The triangle tip and first pause bar are 5 units apart. The transport sits at y=840 and its playhead is halfway along the track. The original footprint layer and registration are preserved. The clear home artwork has 132 units of left, right and bottom inset and 134 units at the top when measured at alpha 64/255.

- `python3 scripts/export_icons.py` regenerates the native catalog, the seven-entry Windows ICO, the ten-entry Mac ICNS, fallback PNGs, the favicon and the welcome foreground. Run on a Mac with full Xcode selected, Pillow with LittleCMS, and librsvg’s `rsvg-convert`. Only the artwork export requires these tools. The welcome material references the five master Icon Composer layers. All static platform PNGs use 8-bit sRGB RGBA. The 16- and 24-pixel fallbacks align the pause bars to distinct pixel columns.
- `iconutil -c iconset -o /tmp/hen-compact-glyph/check.iconset app/icons/icon.icns`, Pillow’s `ico.sizes()`, and `assetutil --info app/icons/Assets.car` verified all raster sizes and the Aqua, dark Aqua and tintable native stacks. NSWorkspace renders at 16, 24, 32, 48, 64 and 128 px were inspected. At 48 px the thresholded foreground has 9, 9, 10 and 9 px insets on the left, top, right and bottom in the system green tint.
- `node /tmp/hen-compact-glyph/check-home.cjs WebKit` and the Chrome run verified saturated ink on the triangle, both bars and elapsed track for each of the four fixed accent choices. No colored pixel touched the edge of the welcome mark.

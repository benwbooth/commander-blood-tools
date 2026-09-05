# Runtime reference assets

`vga-bios-8x8.bin` is the 256-character, 8-row VGA BIOS font requested by
the original game with `INT 10h`, `AX=1130h`. The low 128 characters come
from `BH=3` and the high 128 characters from `BH=4`, matching the split ROM
tables exposed to DOS software.

The checked-in image was independently captured from DOSBox-X 2026.05.02 and
DOSBox Staging 0.82.2. Both produced this SHA-256 digest:

```text
75c79a7e7fa423dda67ec6d6d76cec86b63f85677726368750c75b0920ddf319
```

Assemble `capture-vga-bios-font.asm` as a DOS `.COM` program and run it inside
the reference emulator to reproduce the image.

## Application icon

`commander-blood.png` is the approved blue MANU3-style hand icon, generated
from a screenshot of the actual imported hand model. It is new application
artwork, not an extracted original game asset. The matching 256 by 256,
eight-bit RGBA pixels in `commander-blood.rgba` are embedded in the executable
so window creation needs neither an image codec nor an external command.
The PNG and matching desktop entry are installed by the Nix package.

To regenerate the embedded bytes after replacing the PNG:

```sh
nix develop -c magick crates/commander-blood-game/assets/commander-blood.png \
  -depth 8 rgba:crates/commander-blood-game/assets/commander-blood.rgba
```

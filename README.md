# palapply

Apply a Catppuccin palette to an image.

This tool is purpose-built for a specific workflow: recoloring images
exclusively with the Catppuccin color palette via OKLCH-space dithering. It is
**not** a general-purpose palette tool — no custom palettes, no color grading,
no LUT generation.

## Usage

```
palapply -i input.png -o output.png
```

Cached noise maps are stored alongside the output file in a `maps/` directory.

## Build

```
nix build
```

or

```
cargo build --release
```

## License

MIT

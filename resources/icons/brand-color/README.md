Full-color brand marks, rendered to 32 px PNG from the `-original` SVGs of
[devicon](https://github.com/devicons/devicon) 2.17.0 (MIT). The logos
themselves are trademarks of their owners. PNG rather than SVG because gpui's
`img` does not apply the display scale to an SVG.
They are rasterized straight from the vector at 32 px: gpui samples images
without mipmaps, so a larger PNG drawn at tree-icon size comes out jagged.

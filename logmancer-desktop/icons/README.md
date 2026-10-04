# Logmancer icons

`source.png` is the original book artwork; `source-small.png` is the simplified
version used for sizes up to 64 pixels. Both originals retain their transparency.
Generated icons are centered with a small transparent margin.

To regenerate the Desktop PNG/ICO/ICNS files, Web favicons, and `preview.png`:

```sh
python -m pip install Pillow
python logmancer-desktop/icons/generate.py
```

`preview.png` compares both sources at actual pixel sizes on dark and light
backgrounds. Desktop bundle configuration already references the generated icons.

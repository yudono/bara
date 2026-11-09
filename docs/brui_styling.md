# BRUI Styling (Compile-Time CSS)

Styling is resolved at transpile time, not at runtime. Every `.brx`
`<style>` block and every explicitly imported `styles.css` is parsed by
the CSS engine (`src/css_engine.rs`), and each widget's `class` list is mapped
to `egui` builder calls (`egui::Frame`, `egui::RichText`, spacing, scroll
limits).

## Shared stylesheet concept (explicit import)

A standalone `.css` file is only compiled in when a Bara file imports it:

```python
use "styles.css"
```

(`use styles.css` and shorthand `use styles` are equivalent.) So pages share
one sheet with no per-file `<style>` needed, but never by accident:

```
dashboard_gui/
  styles.css        # shared by shell.brx + all pages
  shell.brx         # no <style> block
  pages/auth/login.brx
  pages/dashboard/users.brx
```

Rules cascade: selectors are class sets (`.btn_filter.aktif` matches a widget
whose class list contains both), and every matching rule's properties merge,
later rules winning. `todo.brx` keeps its own inline `<style>` block; that is
equivalent — inline and shared rules merge into the same global sheet.

## Supported properties

| Property | Aliases | Effect |
|---|---|---|
| `background` | `background-color` | Frame fill (`Panel`/`Row`/`Col` with a matching class) or `RichText` color context; honors `opacity` |
| `color` | — | `RichText::color(...)` on `Text`/`Button` |
| `padding` | — | `Frame::inner_margin`, 1–4 values |
| `margin` | — | `Frame::outer_margin`, 1–4 values |
| `border-radius` | — | `Frame::rounding` |
| `border` | — | `Frame::stroke`, e.g. `1px solid #1f2937` |
| `border-width` + `border-color` | — | Same stroke when split across properties |
| `box-shadow` | — | Only `none` is honored (maps to `Shadow::NONE`) |
| `font-size` | — | `RichText::size(...)` |
| `font-weight` | — | `bold`/`700`–`900`/`bolder` → `RichText::strong()` |
| `font-style` | — | `italic`/`oblique` → `RichText::italics()` |
| `text-decoration` | — | `underline` → `.underline()`; `line-through` → `.strikethrough()` |
| `font-family` | — | `monospace`/`Courier`/`Consolas` → `.monospace()` |
| `gap` | `row-gap`, `column-gap` | Vertical (`Col`) / horizontal (`Row`) item spacing |
| `height` | `max-height` | `ScrollArea::max_height(...)` on `Scroll` |
| `width` / `max-width` | — | Parsed but not applied to a widget (no egui target) |
| `opacity` | — | `0..1` (or `0%..100%`); modulates fill/text alpha |
| `align-items`, `justify-content` | `align_items`, `justify_content` | Parsed; currently informational only |

## Values

- **Colors:** `#rgb`, `#rgba`, `#rrggbb`, `#rrggbbaa`, `rgb(r, g, b)`,
  `rgba(r, g, b, a)` (alpha as `0–1`, `0–255`, or `%`), and named colors:
  `transparent black white red green blue yellow cyan magenta purple gray grey
  lightgray darkgray slate zinc indigo emerald rose amber orange`.
- **Lengths:** `px`, `pt`, `rem`/`em` (×16px), bare numbers; `4px 12px`
  shorthand takes the first token where a single length is needed.
- **Shorthands:** `padding`/`margin` accept 1–4 values (CSS rules).

## Example

```css
.item_tugas {
    background: #111827;
    border: 1px solid #1f2937;
    padding: 12px;
    border-radius: 10px;
}
.teks_tugas {
    color: #e2e8f0;
    font-size: 14px;
}
```

generates roughly `egui::Frame::none().fill(...).inner_margin(...).rounding(...).stroke(...)`
for the container and `egui::RichText::new(...).color(...).size(14.0)` for the text.

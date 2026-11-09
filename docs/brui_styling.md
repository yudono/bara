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
  styles.css        # shared by dashboard.brx + all pages
  dashboard.brx      # no <style> block
  pages/auth/login.brx
  pages/dashboard/users.brx
```

Rules cascade: selectors are class sets (`.btn_filter.aktif` matches a widget
whose class list contains both), and every matching rule's properties merge,
later rules winning. `todo.brx` keeps its own inline `<style>` block; that is
equivalent — inline and shared rules merge into the same global sheet.

## Inline `style={{...}}` (React-style per-widget overrides)

Any widget accepts an inline style map, exactly like React:

```jsx
<Button class="btn" style={{color: "#ffffff", backgroundColor: "#0284c7"}} onClick={go()}>
    Go
</Button>
<Text class="t" style={{fontSize: 20}}>Hi</Text>
```

Rules:

- Both `kebab-case` and React `camelCase` keys work (`background-color` or
  `backgroundColor`, `font-size` or `fontSize`); keys normalize to the table
  above, unknown keys warn (`W0007`) and are ignored.
- Values must be literals (`"..."`, numbers, `true`/`false`); dynamic
  expressions are skipped — put dynamic styling in `class={...}` ternaries.
- Inline declarations beat the shared stylesheet: they merge as the final
  cascade layer, so `style={{color: ...}}` overrides any `color` from classes.
- Works on built-in widgets and on child components (a `style` on `<Tag/>`
  wraps the child render in a matching frame instead of becoming a prop).

## Preflight: the neutral base (no dark/light bias)

Bara ships no opinionated theme. Widgets with **no matching style at all**
render with plain egui constructors and inherit the active egui theme:

- Unstyled `Button`/`Text`/`Input` get **no** forced fill or color — they
  follow the active visuals (dark by default).
- Unstyled `Panel` renders a bare `CentralPanel::default()` with no frame.
- `color-scheme: light` on the root `Panel`'s class switches the whole app
  to `egui::Visuals::light()` (light neutral base). Without it, apps stay
  on the dark default. Either way, styled widgets look identical — only the
  *unstyled fallback* follows the scheme, so a design is never half-dark
  by accident.
- Invalid declarations (unknown units, garbage values) are ignored, never
  silently coerced: `Margin::parse`/`Stroke::parse` return `None` and the
  property is dropped (covered by `tests/css_props_comprehensive_test.rs`).

## Supported properties

| Property | Aliases | Effect |
|---|---|---|
| `background` | `background-color` | Frame fill (`Panel`/`Row`/`Col` with a matching class) or `RichText` color context; honors `opacity` |
| `color` | — | `RichText::color(...)` on `Text`/`Button`; `TextEdit::text_color(...)` on `Input`/`TextField` |
| `padding` | — | `Frame::inner_margin`, 1–4 values |
| `margin` | — | `Frame::outer_margin`, 1–4 values |
| `border-radius` | — | `Frame::rounding` |
| `border` | — | `Frame::stroke`, e.g. `1px solid #1f2937` |
| `border-width` + `border-color` | — | Same stroke when split across properties |
| `box-shadow` | — | Only `none` is honored (maps to `Shadow::NONE`) |
| `font-size` | — | `RichText::size(...)`; on `Input`/`TextField` maps to `FontId::proportional(...)` |
| `font-weight` | — | `bold`/`700`–`900`/`bolder` → `RichText::strong()` |
| `font-style` | — | `italic`/`oblique` → `RichText::italics()` |
| `text-decoration` | — | `underline` → `.underline()`; `line-through` → `.strikethrough()` |
| `font-family` | — | `monospace`/`Courier`/`Consolas` → `.monospace()` |
| `gap` | `row-gap`, `column-gap` | Item spacing: `item_spacing.y` on `Col`, `item_spacing.x` on `Row`, both axes on `Wrap` |
| `height` | `max-height` | Exact `height` → `set_height(...)` on `Panel`/`Col`/`Container`/`Row`; either `height` or `max-height` → `set_max_height(...)` (when both are set, exact `height` wins via `set_height` and `max-height` is ignored); on `Scroll` either one feeds `ScrollArea::max_height(...)` (default 300px); on `Button` pairs with `width` for `add_sized` else `min_size` on the height axis (same rule as the `width` row) |
| `width` / `max-width` | — | Exact `width` → `set_width(...)` on `Panel`/`Col`/`Container`/`Row`; `max-width` alone → `set_max_width(...)`; on `Wrap` and `Scroll` only `max-width` applies (`set_max_width`); on `Button` both dimensions set → `add_sized(vec2(w, h), ...)` while a single dimension → `min_size` on that axis; on `Spacer` `max-width`/`max-height` is the `add_space` length fallback |
| `align-items` | `align_items` | Cross-axis alignment: `center` → `vertical_centered` on `Panel`/`Col`/`Container`, `horizontal_centered` on `Row`; `flex-end`/`end`/`bottom`/`right`/`max` → `Row` renders `left_to_right(Align::Max)`; anything else is the default top/left layout |
| `justify-content` | `justify_content` | `space-between` on `Row` spreads children across the full row width (`available_width` + `allocate_ui_with_layout` with `main_justify(true)`); the cross axis still comes from `align-items` |
| `opacity` | — | `0..1` (or `0%..100%`); modulates fill/text alpha |

Frame properties (`background`, `padding`, `margin`, `border`,
`border-radius`) on an `Input`/`TextField` class wrap the editor in an
`egui::Frame::show(...)` block rather than styling the `TextEdit`
itself. `Input` ignores `width`/`height` — size the surrounding
container instead.

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

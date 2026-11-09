# CSS Concerns — Bara → egui Mapping Gaps

Scope: every CSS property in the test matrix that has **no egui mapping** (or only a
fallback). Verified against `src/css_engine.rs` getters + `src/codegen.rs` emit
(`egui`/`eframe 0.27`, immediate-mode). Test matrix:
`tests/css_box_model_test.rs`, `tests/css_flexbox_test.rs`,
`tests/css_typography_test.rs`, `tests/css_background_border_test.rs`,
`tests/css_effects_test.rs`, `tests/css_layout_position_test.rs`,
`tests/css_misc_test.rs` (+ existing `css_props_comprehensive_test.rs`).

Conventions below — **Requested**: what the CSS asks. **Actual**: what Bara does
(parse succeeds, no crash; ignored or nearest-equivalent). **Why**: concrete
technical reason, not a generic excuse.

---

## 1. Box model — fallbacks

| Property | Requested | Actual | Why |
|---|---|---|---|
| `min-width` | floor box width | Parsed by new `get_min_width()`; codegen ignores (no `set_min_width` emit). Transpile passes, no width emit. | `egui::Ui::set_width` / `set_max_width` are the only width knobs used; `egui 0.27` has no min-constraint API on `Ui`. Possible later via `set_min_width` + custom `Layout`, not wired. |
| `min-height` | floor box height | Parsed by `get_min_height()`; ignored. | Same: only `set_height` / `set_max_height` are emitted. No min-height target. |
| `width: 50%` / `height: auto` | relative / auto sizing | `parse_size` returns `None`; no emit. | Percentages have no absolute basis in immediate-mode (`available_width()` is only read for `justify-content: space-between` Row hack). `auto`/`none` explicitly unset. Skipped, not planned. |
| `margin-top/right/bottom/left` | per-side outer space | Parsed by `get_margin_top()` etc. + `get_effective_margin()` merge; `get_frame_code()` ignores (only `margin` shorthand → `outer_margin`). Longhand-only yields no frame. | `egui::Margin` supports per-side values, so this is **possible later** by switching `get_frame_code` to `get_effective_margin`. Currently shorthand-only by design. |
| `padding-top/right/bottom/left` | per-side inner space | Parsed by `get_padding_*()` + `get_effective_padding()`; ignored (only `padding` shorthand → `inner_margin`). | Same as margin: `egui::Margin` could carry it; wiring is a one-line change to `get_frame_code`. Parsed for future wiring. |
| `box-sizing` | `border-box` vs `content-box` | Parsed (`get_box_sizing()`); ignored. | egui `Frame` has no box-model switch — `inner_margin` + `stroke` are additive by construction. No target; skipped. |
| `aspect-ratio` | `16 / 9` constraint | Parsed (`get_aspect_ratio()` raw `16 / 9`); ignored. `width` still maps. | egui has no retained box solver to hold a ratio while content flows; would need manual `available_width()` math per container. Not planned. |

Mapped (not concerns): `width`, `max-width`, `height`, `max-height`, `margin` shorthand → `outer_margin`, `padding` shorthand → `inner_margin`.

## 2. Display / layout basics

| Property | Requested | Actual | Why |
|---|---|---|---|
| `display` (`block/flex/grid/none/inline-block`) | switch layout / hide | Parsed (`get_display()`); ignored. Tag selects layout (`Row`→`horizontal`, `Col`→`vertical`, `Wrap`→`horizontal_wrapped`, `Panel`→`CentralPanel`). `display:none` still emits. | Bara's tag is the layout primitive; there is no `display` → egui mapping. Hiding would need `if` / visibility gate in codegen, not wired. |
| `visibility` (`visible/hidden/collapse`) | hide without reflow | Parsed (`get_visibility()`); ignored — label still emitted. | egui has no per-widget `visible` flag in the emitted pattern (`Ui::set_visible` exists but is not emitted). Possible later via conditional emit; currently ignored. |
| `overflow`, `overflow-x`, `overflow-y` | clipping / scroll axis | Parsed (`get_overflow()` falls back `overflow` → `x` → `y`); ignored. `<Scroll>` always emits `ScrollArea::vertical().max_height(h or 300.0)`. | Only vertical scroll is wired; horizontal scroll / hidden clipping have no emit. Possible later via `ScrollArea::horizontal()` / `.both()`, not wired. |

## 3. Flexbox

| Property | Requested | Actual | Why |
|---|---|---|---|
| `flex` (`1 1 auto`), `flex-grow`, `flex-shrink`, `flex-basis`, `flex-flow`, `order` | grow/shrink/basis/order | Parsed raw (`get_raw_property`); ignored. No flex-factor emit; gap still works. | egui `Layout` is not a flex solver — children are placed top-down / left-to-right with fixed `item_spacing`. No flex-factor or `order` primitive. Skipped, not planned. |
| `flex-direction` (`row/column/reverse`) | axis direction | Parsed (`get_flex_direction()`); ignored. `Row` vs `Col` tag decides axis. | Direction is structural in Bara (choose `<Row>` vs `<Col>`), not a CSS switch. No codegen switch on the property. |
| `flex-wrap` (`wrap/nowrap/wrap-reverse`) | wrapping | Parsed (`get_flex_wrap()`); `<Wrap>` always emits `horizontal_wrapped` regardless of value. | Wrapping is structural (`<Wrap>` tag), not a CSS switch. `nowrap` on a `<Wrap>` still wraps — documented nearest-equivalent. |
| `justify-content: center/flex-start/flex-end/space-around/space-evenly/start/end` | main-axis distribution | Parsed (`get_justify_content()`); ignored (no `with_main_justify`). Only `space-between` on `Row` emits `allocate_ui_with_layout(...with_main_justify(true))`. | egui `Layout::left_to_right` supports `main_justify` only as a boolean spread; centered / space-around distributions have no one-line equivalent. Only the `space-between` spread was wired. |
| `align-items: stretch/flex-start/start/baseline` | cross-axis default | Parsed; maps to `Min` default (`horizontal()` / `vertical()`). No centering. | `align_cross_kind` maps `center→Center`, `flex-end/end/bottom/right/max→Max`, else `Min`. `stretch`/`baseline` have no egui `Align` equivalent in this layout scheme; default is correct fallback. |
| `align-content`, `align-self` | multi-line / per-item align | Parsed raw; ignored. `align-self: center` does **not** center the row. | No per-item cross-axis override in the emitted `horizontal()` / `vertical()` closures. Would need per-child `Layout::with_cross_align`; not wired. |
| `gap`, `row-gap`, `column-gap` | spacing | **Mapped** (`get_spacing()` → `item_spacing.x/y`). `gap` wins over `row-gap`. | — |

## 4. Grid

| Property | Requested | Actual | Why |
|---|---|---|---|
| `grid`, `grid-template-columns/rows/areas` | track sizing | Parsed raw; ignored. Falls back to `vertical()` container. | egui 0.27 is immediate-mode with no retained grid solver — track sizing / area placement have no declarative target. Skipped, not planned. Use `Row`/`Col` nesting. |
| `grid-auto-columns/rows/flow`, `grid-column/row/area` | auto tracks + placement | Parsed raw; ignored. | Same: no grid placement pass in codegen. Skipped. |
| `justify-items`, `justify-self` | grid-axis alignment | Parsed raw; ignored (does not trigger `vertical_centered`). | Grid-axis alignment ≠ `Row`/`Col` cross-axis (`Align::Center/Max`). No target; skipped. |

## 5. Positioning

| Property | Requested | Actual | Why |
|---|---|---|---|
| `position` (`static/relative/absolute/fixed/sticky`) | positioning scheme | Parsed (`get_position()`); ignored. | egui has no absolute box model in the emitted code (no `put`/`allocate_at`). All containers flow. Skipped, not planned. |
| `top/right/bottom/left`, `inset` | offsets | Parsed raw; ignored. | No `available_pos` / manual `Rect` placement emitted. Would need retained-layer `Area`/`Window` plumbing; not wired. |
| `z-index` | stacking | Parsed raw; ignored. | Immediate-mode draw order = emit order; no stacking context API in the emitted pattern. Skipped. |

## 6. Typography

Only `color`, `font-size`, `font-weight: bold/700/800/900/bolder`, `font-style: italic/oblique`, `text-decoration: underline/line-through`, `font-family: monospace/*Courier*/*Consolas*` are wired (→ `RichText::color/size/strong/italics/underline/strikethrough/monospace`, `Input` → `text_color` + `FontId::proportional`).

| Property | Requested | Actual | Why |
|---|---|---|---|
| `font` shorthand | `italic bold 16px/1.5 sans-serif` | Parsed raw; **not expanded** — no `size`/`strong` emit. | `get_rich_text_modifiers` reads discrete `font-size`/`font-weight`/`font-style` keys only; shorthand expansion not implemented. Possible later via shorthand parser. |
| `font-variant` | `small-caps` | Ignored. | `egui::RichText` has no small-caps variant. No target; skipped. |
| `line-height` | leading | Ignored. | `RichText` / `Label` expose no line-height knob in the emitted pattern. Skipped. |
| `text-align` | `left/center/right/justify` | Parsed (`get_text_align()`); ignored (no `text_align` emit). | Labels take container layout; egui label alignment would need `Layout::with_cross_align` per text, not wired. Possible later. |
| `text-decoration: overline` | overline | Ignored (only `underline`/`line-through` substrings map). | `RichText` has no overline modifier. No target. |
| `text-transform`, `text-indent`, `text-shadow`, `text-overflow` | case/indent/shadow/ellipsis | Ignored. | `RichText` has no transform/indent/shadow/ellipsis modifiers. Would need manual string transform (possible for `text-transform` only) — not wired. |
| `letter-spacing`, `word-spacing` | tracking | Ignored. | No tracking knob on `RichText`/`FontId`. No target. |
| `white-space`, `word-break`, `overflow-wrap` | wrapping | Ignored; label still plain `label(RichText::new(...))`. | egui wrapping is via `Wrap` / `ScrollArea`, not per-text CSS. No per-label wrap mode wired. |
| `writing-mode` | vertical text | Ignored. | egui text is horizontal-only in the emitted pattern. No target. |

## 7. Background

`background` / `background-color` → `fill(Color32)` (opacity-modulated) is wired.

| Property | Requested | Actual | Why |
|---|---|---|---|
| `background-image` | `url(...)` / gradient | Ignored; `get_fill_code`/`get_frame_code` return `None` for image-only. | `Frame::fill` takes a solid `Color32` only. Image brushes would need `TextureHandle` + `Image` plumbing; not wired. Possible later via `egui::Image`. |
| `background-position/size/repeat/attachment/blend-mode/clip` | placement / tiling | Ignored. | No-ops without an image layer; solid fill has no placement semantics. Skipped. |

## 8. Border / outline / shadow

Wired: `border` → `stroke`, `border-width` (+default black) → `stroke`, `border-color` (+1.0 default) → `stroke`, `border-radius` → `rounding`, `box-shadow: none` → `Shadow::NONE`.

| Property | Requested | Actual | Why |
|---|---|---|---|
| `border-top/right/bottom/left` | per-side border | Ignored (`get_frame_code`/`get_stroke_code` → `None`). | `egui::Stroke` on `Frame`/`Button` is uniform; per-side strokes would need a custom `Painter` rect pass. Not planned. |
| `border-style` alone (`dashed/solid`) | style | Ignored; style token dropped even when `border-width` supplies the stroke. | `Stroke` has no dash pattern; style has no target. Dropped by design. |
| `outline`, `outline-offset` | focus ring | Ignored. | No outline pass in `Frame`; focus rings would need extra painter stroke. Not wired. |
| `box-shadow: 0 1px 2px black` (non-`none`) | blur shadow | Ignored (frame keeps `fill`, no `shadow` call). | `egui::epaint::Shadow` supports only simple offset/blur via `Shadow` struct; CSS blur spreads were deliberately limited to `none`. Possible later via `Shadow { offset, blur, spread, color }`, not wired. |

## 9. Animation / transition / transform

| Property | Requested | Actual | Why |
|---|---|---|---|
| `transition`, `transition-property/duration/timing-function/delay` | timed property change | Ignored. | Immediate-mode has no retained property timeline; transitions would need per-frame `animate_value` state. Skipped, not planned. |
| `animation`, `animation-name/duration/timing-function/delay/iteration-count/direction` | keyframes | Ignored. | Same: no keyframe player in the emitted `update()`; would need `ctx.request_repaint` + time-based state. Not planned. |
| `transform`, `transform-origin` | `rotate/translate/scale` | Ignored. | egui `Ui` has no CSS-like transform stack in the emitted pattern (only `Layout` + spacing). Would need `Painter` matrix work per layer. Not planned. |

## 10. Effects / UI

Wired: `opacity` → `set_opacity` + alpha-modulated `fill`/`stroke`/`color`.

| Property | Requested | Actual | Why |
|---|---|---|---|
| `filter`, `backdrop-filter` | `blur/brightness/...` | Ignored. | No post-process filter pass on `Frame`/`Ui`. Would need custom `Painter`/shader; not planned. |
| `clip-path`, `mask`/`mask-image` | clipping | Ignored. | No clip-path plumbing (`Ui::clip_rect` exists but is not driven by CSS). Possible later, not wired. |
| `cursor`, `pointer-events` | hit-test / cursor | Ignored. | Emitted widgets use default egui hit-testing; no `cursor` / hit-test gate emitted. Possible later via `response.on_hover_cursor`, not wired. |
| `user-select`, `appearance`, `resize` | selection / native look / resizable | Ignored. | No egui knobs in the emitted pattern (`TextEdit` selection, native appearance, resizable frames are not driven by CSS). Skipped. |

## 11. List / table

| Property | Requested | Actual | Why |
|---|---|---|---|
| `list-style`, `list-style-type/position/image` | bullets | Ignored. | Bara has no `<ul>/<ol>` widget; lists are `Col` + `Text`. No bullet renderer. Skipped. |
| `border-collapse`, `border-spacing` | table grid | Ignored. | No table widget in codegen; `Row`/`Col` have no collapse model. Skipped. |

## 12. Media / scroll

Wired: `height`/`max-height` → `ScrollArea::vertical().max_height(h or 300.0)`; `max-width` → `set_max_width`.

| Property | Requested | Actual | Why |
|---|---|---|---|
| `content` | pseudo-element text | Ignored; children text used. | No pseudo-element pass; `content` would need synthetic `Text` injection. Not wired. |
| `object-fit`, `object-position` | image fitting | Ignored; `<Image>` renders placeholder `label("[Image: ...]")`. | Images are placeholder labels, not `egui::Image` widgets, so fit/position have no target. Possible later when `Image` becomes real. |
| `scroll-behavior`, `overscroll-behavior` | smooth / chaining | Ignored; vertical area still emitted. | `ScrollArea` smoothness/overscroll flags are not driven by CSS. Possible later. |
| `scroll-snap-type/align/stop` | snap points | Ignored. | No snap plumbing on `ScrollArea` in the emitted pattern. Skipped. |

---

## Engine / codegen already cover more than documented

Found while auditing (all covered by existing + new tests):

- `border-width` alone → `stroke(..., black)`; `border-color` alone → `stroke(1.0, color)` (not just the `border` shorthand).
- `gap` falls back to `row-gap` then `column-gap` (`get_spacing`); `Row` uses `item_spacing.x`, `Col`/`Panel` use `.y`, `Wrap` sets both.
- `opacity` alpha-modulates `fill`/`stroke`/`RichText::color`, not just `set_opacity`.
- `align-items` maps `flex-end`/`end`/`bottom`/`right`/`max` → `Align::Max` (`with_layout(left_to_right(Max))`), not just `center`.
- `Button` with only one of `width`/`height` emits `.min_size(vec2(w,0))`, not just `add_sized` for both.
- `<Scroll>` defaults to `max_height(300.0)` when no height is set; `<Spacer>` falls back to `max-width`/`max-height` then `8.0`.
- New additive getters (`get_min_width/height`, `get_margin_*`, `get_padding_*`, `get_effective_margin/padding`, `get_display/position/visibility/overflow/box-sizing/aspect-ratio/text-align/flex-direction/wrap`, `get_raw_property`) parse longhands/raw values for future wiring; codegen still ignores them (documented above).

# Bara Component Coverage (baseline: commit b8ab082)

Baseline: clean tree at `b8ab082`. Native tags available at this commit only:

`Panel, Col, Row, Scroll, Text, Button, Input, TextField, Checkbox, Slider,
Separator, Wrap, Container, Image, Progress, Spacer`

plus documented React features: `props`, `useEffect`, `onMount`/`onUpdate`,
child components via `<Tag/>`.

Composition tests: `tests/bara_composition_test.rs` (9 tests, all green).
Each composition below points at its test where applicable.

Legend — **Status**: `native` = built-in tag at this commit;
`composition` = idiomatic combination of native tags + props/state/style;
`skipped` = cannot be built from this baseline (reason given).

## Full requested set

| Component | Status | Notes / pattern / skip reason |
|---|---|---|
| View | `native` (`Panel`) | Root container. `Panel` → `egui::CentralPanel::default()`. Smoke-covered in `composition_basics_view_text_image_smoke`. |
| Text | `native` (`Text`) | `Text` → `ui.label(RichText::new(...))` + CSS modifiers. |
| Image | `native` (`Image`) | Placeholder binding at this commit: `<Image src="logo.png" />` → `label(RichText::new("[Image: logo.png]"))`. Smoke-covered. No decoded raster at this commit. |
| Icon | `composition` | No vector-icon tag. Pattern: `<Text class="ic">★</Text>` — vector icons = text glyphs/emoji. Covered in `composition_icon_text_glyph` (asserts `label` + `RichText`). |
| ScrollView | `native` (`Scroll`) | `Scroll` → `egui::ScrollArea::vertical().max_height(...)`. Default 300px when no height class. |
| FlatList | `composition` | Pattern: `Scroll` + `Col` + `{for item in items => Row(Text + Button)}` with `Vec` field + `retain` remove method; per-row delete uses deferred `__actions` pattern. Covered in `composition_flat_list_scroll_for_with_delete` (asserts `__actions` + `retain`). |
| SectionList | `composition` | Pattern: two groups, each header `Text` + `{for}` rows inside one `Scroll > Col`. Covered in `composition_section_list_two_groups` (asserts both headers `Fruits` + `Veggies`). |
| SafeAreaView | `composition` | Pattern: `Panel class="safe"` with `padding` → `inner_margin`. Covered in `composition_safe_area_view_panel_padding` (asserts `inner_margin`). There is no OS inset API on desktop; padding is the idiom. |
| KeyboardAvoidingView | `skipped` | Keyboard-avoidance is a mobile-OS concept; desktop egui has no occluding keyboard and no keyboard-visibility event in the egui 0.27 events used here. |
| Divider / Separator | `native` (`Separator`) | `<Separator />` → `ui.separator()`. |
| Stack | `skipped` (alias in progress) | Generic `Stack` alias does not exist at this commit. Use `Col`/`Row`/`Container` directly. See “in progress” note. |
| VStack | `composition` (alias in progress) | Idiom today: `Col` (≈ VStack, `ui.vertical`). A literal `<VStack>` alias does not exist at this commit. See “in progress” note. |
| HStack | `composition` (alias in progress) | Idiom today: `Row` (≈ HStack, `ui.horizontal`). A literal `<HStack>` alias does not exist at this commit. See “in progress” note. |
| ZStack | `skipped` | ZStack needs explicit paint-order layers with no declarative Bara syntax yet (egui `Area`/`LayerId` ordering is not bound in codegen). |
| Button | `native` (`Button`) | `<Button onClick={...}>` → `egui::Button::new(...)` + `.clicked()`. Supports `background`→`fill`, `border`→`stroke`, `border-radius`→`rounding`, `width`/`height`→`add_sized`. |
| TextInput | `native` (`Input` / `TextField`) | `<Input bind={x} placeholder="...">` → `TextEdit::singleline(&mut ...)`. Supports `color`→`text_color`, `font-size`, `background` frame. |
| Checkbox | `native` (`Checkbox`) | `<Checkbox bind={flag} />` → `ui.checkbox(&mut ...)`. |
| RadioButton | `skipped` (in progress) | No radio widget in the egui 0.27 API surface used here (only `checkbox`); exclusive-group selection needs a new widget + group state. If absent, compose with `Checkbox` + manual exclusivity, but true radio is skipped. |
| Switch / Toggle | `skipped` (in progress) | No switch/toggle widget bound in codegen at this commit. Needs custom toggle paint + drag/click state. |
| Slider | `native` (`Slider`) | `<Slider bind={v} />` → `egui::Slider::new(&mut v, 0..=100)`. |
| Picker / Dropdown / Select | `skipped` (in progress) | No combo-box/dropdown widget exposed in Bara codegen at this commit; egui `combo_box` exists upstream but there is no Bara `Picker` binding and no overlay-positioning syntax. |
| DatePicker / TimePicker | `skipped` | No calendar widget in egui 0.27 and no chrono/date crate vendored offline. Date/time picking cannot be built from `Text`/`Button`/`Slider` compositions. |
| Pressable / TouchableOpacity | `composition` | Pattern: `Button` with `onClick` assign/call. `Row`/`Col` do not consume `onClick` in codegen at this commit, so pressable containers must be `Button`. Opacity-on-press has no gesture state; use `opacity` class for static dimming only. |
| Swipeable | `skipped` | Gesture swipe has no desktop pointer equivalent in egui events used here (no touch-swipe stream, only click/drag); no swipe-action container primitive. |
| RefreshControl | `skipped` | Pull-to-refresh is a touch-scroll idiom; `egui::ScrollArea` as used here exposes no overscroll/refresh event, so there is nothing to bind a refresh callback to. |
| ActivityIndicator / Spinner | `skipped` (in progress) | No spinner widget bound in Bara codegen at this commit (upstream egui `Spinner` exists but is not emitted). Workaround is static `Text` (“Loading…”), not an animated indicator. |
| ProgressBar | `native` (`Progress`) | `<Progress bind={p} />` / `value` / `progress` → `egui::ProgressBar::new(...).show_percentage()`. |
| Modal / Dialog | `skipped` (in progress) | No overlay/window-stack primitive in Bara at this commit; modal needs a top-layer overlay + focus trap, neither of which has declarative syntax yet. |
| Toast / Snackbar | `skipped` | Needs a timed overlay + auto-dismiss queue; no timer/overlay primitive in Bara codegen at this commit (`onMount`/`useEffect` cannot spawn timed dismiss without a repaint-timer overlay layer). |
| Badge | `composition` | Pattern: `Row`/`Col` with `background` + `padding` + `border-radius` frame wrapping a count `Text`. Covered in `composition_badge_text_count` (asserts `RichText` + `fill` + `label`). |
| Tooltip | `skipped` (in progress) | Upstream egui has tooltip APIs but Bara exposes no tooltip prop/attribute at this commit, so there is no declarative binding. |
| BottomSheet | `skipped` (in progress) | Needs a sliding overlay panel + drag gesture; no overlay/drag-sheet primitive in codegen at this commit. |
| Header / NavigationBar / AppBar | `composition` | Pattern: `Row class="bar"` with title `Text` + action `Button`, `justify-content: space-between` → `allocate_ui_with_layout(... .with_main_justify(true))`. Covered in `composition_header_appbar_space_between`. |
| BottomTabBar | `composition` | Pattern: `Row` of `Button`s each `onClick={tab = "..."}` switching a `tab` field; content switches via `{if tab == ...}`. Covered in `composition_bottom_tab_bar_switching` (asserts `self.tab =`). |
| Drawer / SideMenu | `skipped` (in progress) | Needs an off-canvas overlay + scrim layer; no overlay primitive in codegen at this commit. |
| Card | `composition` | Pattern: `Col class="card"` with `background` + `border` + `padding` + `border-radius` → `Frame::none().fill(...).stroke(...).inner_margin(...).rounding(...)`. Covered in `composition_card_col_frame` (asserts `Frame::none` + `fill` + `rounding`). |
| WebView | `skipped` | Webview needs a native webview lib + separate windowing — out of scope (no web renderer crate vendored offline, separate OS window required). |
| Video | `skipped` | Video playback needs a media backend crate unavailable offline (no decoder/sink crate vendored; egui 0.27 used here has no video widget). |
| Canvas / SVG | `skipped` | No immediate-mode custom paint / SVG path API bound in Bara codegen at this commit (no `painter`/`Shape` emission, no SVG parser). |

## In-progress by design (parallel native-tag agent)

The following may be added by the parallel native-tag workstream:
`RadioButton`, `Picker`/`Select`, `Switch`, `Modal`, `Spinner`,
`BottomSheet`, `Drawer`, `Tooltip`, `Stack` aliases (`Stack`/`VStack`/`HStack`).

Status rule: if a tag from that list is absent when you read this file,
treat it as `skipped` with the reason in the table above. If it is present,
prefer the native tag over the composition workaround documented here.

## Composition test index (`tests/bara_composition_test.rs`)

- `composition_safe_area_view_panel_padding` → SafeAreaView
- `composition_flat_list_scroll_for_with_delete` → FlatList (`__actions` + `retain`)
- `composition_section_list_two_groups` → SectionList (both headers)
- `composition_card_col_frame` → Card (`Frame::none` + `fill` + `rounding`)
- `composition_header_appbar_space_between` → Header/AppBar (`with_main_justify`)
- `composition_bottom_tab_bar_switching` → BottomTabBar (`onClick` assigns)
- `composition_badge_text_count` → Badge (`RichText` + `fill`/`label`)
- `composition_icon_text_glyph` → Icon (`label`; vector icons = text glyphs)
- `composition_basics_view_text_image_smoke` → View/Text/Image (`CentralPanel`/`label`)

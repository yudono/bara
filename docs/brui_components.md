# BRUI Components

BRUI renders Bara views (`.brx` files and Brui XML layouts) to native
`egui`/`eframe` widgets at compile time.

## Single-file components (React-style `.brx`)

A `.brx` file is a complete React-style module: `type` (state + methods),
`view` (JSX), and an optional inline `<style>` block live together — no
mandatory `.br` split. `todo.brx`, `dashboard.brx`, and every `pages/**`
file follow this shape:

```jsx
use eframe
use egui

type Counter =
    count: int = 0
    bump() =>
        count = count + 1

view Counter =>
    <Panel class="screen">
        <Button class="btn" onClick={bump()}>Count: {count}</Button>
    </Panel>

<style>
    .btn { background: #0284c7; color: #ffffff; border-radius: 6px; }
</style>
```

`use` imports, `enum`/`type`/`fn` items, and `raw_rust { ... }` escape
hatches are all legal at the top level of a `.brx` file; the transpiler
merges every project file into one program before analysis, so order and
file boundaries never matter for resolution. The shared-`styles.css`
pattern (`use "styles.css"`) and inline `<style>` blocks merge into the
same global sheet — use shared files for multi-page apps and inline
blocks for single-file widgets.

A view binds to a declared type:

```jsx
use MyApp

view MyApp =>
    <Panel class="screen">
        <Text class="title">Hello</Text>
    </Panel>
```

The validator accepts 31 tags (`src/debugger.rs`), each with a dedicated
native codegen arm (`src/codegen.rs`): `Container` is an alias of `Col`,
`TextField` an alias of `Input`, and `Stack`/`VStack`/`HStack` alias the
column/row layouts. Anything else is treated as either a
React-style child component (when a matching `type` + `view` exists) or a
generic vertical container (a plain `ui.vertical(...)` wrapper) with an
`E0003` "Unknown Brui UI component" warning.

## Layout containers

| Tag | Attribute support | Native output |
|---|---|---|
| `Panel` | `class` | `egui::CentralPanel` + compile-time `egui::Frame` when the class sets frame props |
| `Col` | `class` | `ui.vertical(...)`, item spacing from `gap`, optional `Frame` wrapper |
| `Row` | `class` | `ui.horizontal(...)`, item spacing from `gap`, optional `Frame` wrapper |
| `Scroll` | `class` | `egui::ScrollArea::vertical()`, max height from `height`/`max-height` (default 300px) |
| `Separator` | — | `ui.separator()` |

`Wrap`, `Container`, `Spacer`, `Image`, `Progress` are accepted by the
validator and have dedicated native arms (`Wrap` → `horizontal_wrapped`,
`Image` → placeholder label, `Progress` → `egui::ProgressBar`,
`Spacer` → `add_space`).

## Content and input widgets

| Tag | Attributes | Native output |
|---|---|---|
| `Text` | `class`, children / `{expr}` | `ui.label(egui::RichText::new(...))` with style modifiers |
| `Button` | `class`, `onClick={...}`, children | `egui::Button` + `.clicked()` handler; label from children |
| `Input` | `bind={field}`, `placeholder="..."`, `class` | `egui::TextEdit::singleline(&mut field).hint_text(...)` |
| `TextField` | `bind={field}`, `placeholder="..."` | Accepted alias of `Input` (same single-line editor) |
| `Checkbox` | `bind={field}` | `ui.checkbox(&mut field, "")` |
| `Slider` | `bind={field}` | `egui::Slider::new(&mut field, 0..=100)` |

## Extended components

Beyond the base set, Brui ships React-Native-style widgets with native
egui codegen. All support `class` (frame/gap/opacity) unless noted:

| Tag | Attributes | Native output |
|---|---|---|
| `RadioButton` | `bind={field}`, `value="..."`, children label | `ui.radio_value(&mut field, value, label)` |
| `Picker` / `Dropdown` / `Select` | `bind={field}`, `<Option value="v">label</Option>` children | `egui::ComboBox` + `ui.selectable_value` rows |
| `Switch` / `Toggle` | `bind={field}`, children label | Button showing On/Off that flips the bool field |
| `Modal` / `Dialog` | `title="..."`, `open={flag}` (optional), children | `egui::Window` shown while the flag is true |
| `Spinner` / `ActivityIndicator` | `size="24"` (optional) | `egui::Spinner::new()` |
| `BottomSheet` | children | `egui::TopBottomPanel::bottom` sheet |
| `Drawer` / `SideMenu` | children | `egui::SidePanel::left` drawer |
| `Stack` / `VStack` | `class` | Same layout as `Col` |
| `HStack` | `class` | Same layout as `Row` |

`tooltip="..."` is supported on `Text` and `Button` (hover text via
`on_hover_text`). `ZStack` has no declarative equivalent in immediate-mode
egui and is intentionally unsupported (see `COMPONENT.md`).

## Attributes

| Attribute | Forms | Notes |
|---|---|---|
| `class` | `class="a b"`, `class={expr}`, `class={cond ? "a" : "b"}` | Literal class names feed the CSS engine; ternary branches are collected from both sides |
| `onClick` | `onClick={method()}`, `onClick={field = value}` | Any expression; runs when the button is clicked. Inside `{for}` loops the action is deferred so it can mutate `self` after the borrow ends |
| `bind` | `bind={field}` (`.brx`), `bind="field"` (XML) | Two-way binding to a view field; required on `Input`, `Checkbox`, `Slider` |
| `placeholder` | `placeholder="..."` (XML: also `hint="..."`) | Hint text for `Input`/`TextField` |

## Control flow inside views

```jsx
{for item in daftar_tugas =>
    {if filter == "Semua" =>
        <Row>
            <Text>{item.teks}</Text>
        </Row>
    }
}
```

- `{if cond => ...}` renders the child only when `cond` holds (optional `else` branch supported).
- `{for x in list => ...}` iterates `&mut list`; per-item click handlers are queued and applied after the loop.
- `{expr}` inside `Text`/`Button` children becomes `format!("{}", expr)` interpolation.
- `{/* comment */}` JSX comments are ignored.

## React-style components: props, state, hooks, lifecycle

A view whose tag names a `type` **with its own `view`** is a child component.
The parent embeds it with `<Tag .../>`; the compiler injects a persisted
instance (`self.__child_<tag_snake>_<n>`, e.g. `__child_login_form_0`) so
child state survives re-renders:

```jsx
view DashboardApp =>
    <Panel class="dashboard_screen">
        {if LoginForm.logged_in =>
            <OverviewPage />
        else =>
            <LoginForm />
        }
    </Panel>
```

- **Props**: plain attributes assign same-named child fields before each render
  (`<Clicker label="Team A" step="1" />`). `bind`/`onClick` are not supported
  on components (warning `W0006`); unknown attributes are rejected (`E0005`)
  with the valid prop list. A component needs a backing
  `type T = ...` (`E0004`) and cannot render itself (`E0007`).
- **State**: child fields are state. Parents read child state directly with
  `ChildType.field` / `ChildType.method()` — never write `__child_*`
  identifiers yourself; the analyzer rewrites `ChildType.x` to the embedded
  instance. Members not backed by an embedded component stay untouched, so
  typos still surface as undeclared-identifier errors.
- **`useEffect`**: `{useEffect(dep1, dep2) => effect_body()}` runs on mount
  and whenever the debug signature of `deps` changes. Effects are collected
  in tree order and executed at the end of `update()` (after paint, React
  semantics). Hooks must be top-level — nesting inside `{for}`/`{if}` is
  rejected (`E0006`), same rule as React.
- **Lifecycle — complete frame model**: every frame, egui calls the running
  view's `update()`, which Bara generates as:
  1. Render the JSX tree top-down (props are assigned to child instances
     just before each `<Child/>` renders).
  2. Run collected `useEffect` bodies in tree order — each only if this is
     the first frame or its dependency signature changed.
  3. Run `onUpdate()` on every frame *except* the first; run `onMount()`
     once instead on the first frame. The mount flag flips at the end.
  4. If the type defines `onUpdate`, request the next frame via
     `request_repaint_after(250ms)` so per-frame logic (timers, clocks)
     keeps ticking without input; types without `onUpdate` only re-render
     on interaction (event-driven, zero idle cost).
  5. `onUnmount()` runs from the generated `impl Drop` when the app (or an
     embedded child instance with the whole parent) is dropped.
  Embedded children (`<Tag/>` → `__render_as_child`) run the same
  render → effects → mount/update epilogue against their *own* persisted
  state, so child `onMount`/`onUpdate` also fire. Parent methods can drive
  children imperatively (`ChildType.method()`); parents observe child state
  declaratively (`ChildType.field`, including inside `useEffect` deps and
  `{if}` conditions). In headless/test mode the window is skipped but the
  same update path executes.
- **Parent-child tree**: parents compose `<OverviewPage/>`, `<UsersPage/>`,
  `<LoginForm/>`, `<RegisterForm/>` and switch them conditionally; each child
  keeps its own state (see `examples/dashboard_gui/` — sidebar shell with
  login-gated pages — and `examples/react_tree/`).

## Brui XML layout files

`layout.xml` files use the same tags with string attributes:

```xml
<view name="XmlDemoApp">
  <Panel class="screen">
    <Input bind="username" placeholder="Enter name..." />
    <Button onClick="login()">Login</Button>
  </Panel>
</view>
```

`on_click`/`onclick` normalize to `onClick`; `hint` normalizes to
`placeholder`. Pair an XML layout with a shared `styles.css` (see
`docs/brui_styling.md`) and a matching `.br` type definition.

# BRUI Components

BRUI renders Bara `view` blocks (`.brx` files and Brui XML layouts) to native
`egui`/`eframe` widgets at compile time. A view binds to a declared type:

```jsx
use MyApp

view MyApp =>
    <Panel class="screen">
        <Text class="title">Hello</Text>
    </Panel>
```

The validator accepts 16 tags (`src/debugger.rs`). Ten have dedicated native
codegen (`src/codegen.rs`); the rest fall through to a generic vertical
container (a plain `ui.vertical(...)` wrapper with no extra widget).

## Layout containers

| Tag | Attribute support | Native output |
|---|---|---|
| `Panel` | `class` | `egui::CentralPanel` + compile-time `egui::Frame` when the class sets frame props |
| `Col` | `class` | `ui.vertical(...)`, item spacing from `gap`, optional `Frame` wrapper |
| `Row` | `class` | `ui.horizontal(...)`, item spacing from `gap`, optional `Frame` wrapper |
| `Scroll` | `class` | `egui::ScrollArea::vertical()`, max height from `height`/`max-height` (default 300px) |
| `Separator` | — | `ui.separator()` |

`Wrap`, `Container`, `Spacer`, `Image`, `Progress` are accepted by the
validator but currently render through the generic vertical-container fallback.

## Content and input widgets

| Tag | Attributes | Native output |
|---|---|---|
| `Text` | `class`, children / `{expr}` | `ui.label(egui::RichText::new(...))` with style modifiers |
| `Button` | `class`, `onClick={...}`, children | `egui::Button` + `.clicked()` handler; label from children |
| `Input` | `bind={field}`, `placeholder="..."`, `class` | `egui::TextEdit::singleline(&mut field).hint_text(...)` |
| `TextField` | `bind={field}`, `placeholder="..."` | Accepted alias of `Input` (same single-line editor) |
| `Checkbox` | `bind={field}` | `ui.checkbox(&mut field, "")` |
| `Slider` | `bind={field}` | `egui::Slider::new(&mut field, 0..=100)` |

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
instance (`self.__child_<Tag>_<n>`) so child state survives re-renders:

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
- **Lifecycle**: `onMount()` runs once after the first render, `onUpdate()`
  runs at the end of later frames, `onUnmount()` runs via generated
  `impl Drop`. They work for the running app view and for embedded children
  (`__render_as_child`).
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

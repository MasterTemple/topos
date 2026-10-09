# Filters

A `Filter` decides, for each element, one `Action`:

- `Keep`: the element is rendered or written with its decoration (tag, class, template, brackets,
  line breaks) and its contents.
- `Unwrap`: the element itself is left out (no tag, class, template, span, label, or line break
  of its own) but its children are processed as if they were its parent's.
- `Drop`: the element and everything in it are left out.

Filters are used by `plain_text`, `Flattener`, and `HtmlRenderer` (not by `parse`, `verses`,
`slice`, or `refs`).

## Deciding

`action(element)`: the first of these that gives an action wins.

1. **Rules**: the custom rules added with `rule`, from the most recently added to the first. A rule
   returning `None` passes the element on to the next rule.
2. **Note types**: only when the element's `kind` is `Note` and it has a `note_type()` (its `type`
   attribute): the note-type entries added with `note_type`, most recent first; the first whose
   type equals the note's type ignoring ASCII case gives the action.
3. **Kinds**: the action set for the element's `kind` with `kind`, `keep`, `unwrap`, or `drop`.
   Setting a kind again replaces the earlier action.
4. Otherwise `Keep`.

A note without a `type` attribute is never matched by note-type entries. Note-type entries apply
whatever the kind entry for `Note` says (`Filter::text()` drops notes, but
`Filter::text().note_type("study", Action::Keep)` keeps study notes).

Because an element's action is decided on its own, a kept element inside a dropped one is never
reached, and an unwrapped element's children are each decided in turn.

## Constructors

| Constructor | Kinds set |
|---|---|
| `Filter::all()` (= `Filter::default()`) | none: everything is kept |
| `Filter::text()` | `Note`, `Title`, `Metadata` dropped |

`kind(k, a)`, `keep(k)`, `unwrap(k)`, `drop(k)`, `note_type(t, a)`, and `rule(f)` each return the
filter with the setting added (builder style; the receiver is consumed).

## Clone, Debug, threads

`Filter` is `Clone` (rules are shared between clones, not copied) and `Send + Sync` (rules must be
`Send + Sync + 'static`). Its `Debug` output shows the kinds and note types and the number of
rules (closures are not printed); its exact format is not part of the interface.

## Defaults elsewhere

| User | Default filter |
|---|---|
| `TextOptions::default()` | `Filter::text()` |
| `HtmlOptions::default()` | `Filter::all().drop(Kind::Metadata)` |

## Examples

```text
<title>The Word</title>In the beginning<note type="crossReference">Gen 1:1</note><note type="explanation">Or, origin</note>
```

| Filter | `plain_text` |
|---|---|
| `Filter::text()` | `In the beginning` |
| `Filter::text().keep(Kind::Note).note_type("crossReference", Action::Drop)` | `In the beginning [Or, origin]` |
| `Filter::all()` | `The Word` / `In the beginning [Gen 1:1] [Or, origin]` on two lines |

HTML with `Filter::all().drop(Kind::Title).unwrap(Kind::Word).note_type("crossReference",
Action::Drop)` and an empty class prefix, for
`<title type="psalm">A Psalm</title><w lemma="strong:H3068" morph="x">LORD</w><note type="crossReference" n="a">Ps 1:1</note><note type="explanation">Or, Jehovah</note>…`,
starts `LORD<span class="note explanation" data-note-type="explanation">`.

# Strong's numbers and morphology

## `Strongs::parse(token)`

One token, such as `strong:H07225`, `x-Strongs:G25`, `G25`, `h0430a`. Rules:

1. At most one prefix is removed, the first that matches exactly (case-sensitive) of: `strong:`
   (OSIS 2.1.1 manual, `lemma="strong:…"`), `x-Strongs:`, `Strongs:`. Any other prefix stays and
   makes the token fail (`lemma.TR:foo` is not a Strong's number).
2. The next character is the language: `G` or `g` is Greek, `H` or `h` is Hebrew. Anything else
   fails.
3. Then one or more ASCII digits, read as a `u32` (leading zeros allowed: `H0430` is 430). No
   digits, or a value too large for `u32`, fails.
4. Whatever follows is the suffix. It must be only ASCII letters, ASCII digits, and `!`; anything
   else fails. An empty suffix is `None`.

| Token | Result |
|---|---|
| `strong:H0430a` | Hebrew, 430, suffix `a` |
| `strong:H07225` | Hebrew, 7225 |
| `x-Strongs:G25`, `Strongs:G26`, `G25`, `g25` | Greek, 25 / 26 / 25 / 25 |
| `G0` | Greek, 0 |
| `strong:G12!` | Greek, 12, suffix `!` |
| `H1a2` | Hebrew, 1, suffix `a2` |
| `strong:Gx`, `G-1`, `Ga1`, `H`, `lemma.TR:x`, `strong:G99999999999` | none |
| `G12x-y` | none (the `-`) |
| `strong:strong:G1` | none (only one prefix is removed) |

`Strongs::parse_list(s)` splits `s` on whitespace and keeps the tokens that parse, in order
(duplicates kept): `strong:G1 x-Strongs:G2 lemma.TR:x` gives G1 and G2.

## On elements

`Element::strongs()`:

- `parse_list` of `attr("lemma")`, then
- the `strong` attribute (USX 3.0 `<char style="w" strong="…">`), split on spaces and commas
  (`' '` and `','` only), each piece parsed with `Strongs::parse` (empty pieces fail and are
  skipped).

So `<w lemma="strong:G1" strong="G2,G3 H4">` gives G1, G2, G3, H4. The element's kind is not
checked; `strongs_numbers` only asks `Word` elements.

`Element::morph()` is the `morph` attribute split on whitespace, with scheme prefixes kept:
`robinson:V-AAI-3S  strongsMorph:TH8804` gives `robinson:V-AAI-3S` and `strongsMorph:TH8804`.
Synthesised words (ThML `<sync type="morph">`, GBF `<WT…>`) store the morph as described in
parsing.md; repeated morphs are space-separated in one attribute, so `morph()` returns each.

## Display

`G` or `H`, the number in decimal without padding, then the suffix if any: `H430`, `G25`,
`H430a`, `G12!`, `G0`.

## Dictionary helpers

- `dictionary_key()`: the number zero-padded to at least five digits (`00430`, `07225`, `00000`,
  `123456`), the key form of SWORD's Strong's dictionary modules. The suffix and language are not
  included.
- `dictionary_module()`: `StrongsGreek` for Greek, `StrongsHebrew` for Hebrew.

## Equality and order

`Strongs` and `StrongsLang` derive `PartialEq`, `Eq`, `Hash`, `PartialOrd`, `Ord`. Order is by
language with `Hebrew` before `Greek` (declaration order), then number, then suffix (`None` before
any suffix, suffixes in string order).

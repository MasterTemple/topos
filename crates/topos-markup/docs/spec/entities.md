# Entity decoding

`decode_entities(s) -> String` replaces character and entity references in `s` and leaves
everything it does not recognise exactly as it was. It never fails and never panics. It is used
on XML text, attribute values, unreadable trailing input (parsing.md), and GBF text; plain text is
not decoded.

## Finding references

Scanning left to right, at each `&`:

1. The **reference body** is the text after the `&` up to the first of: `;`, `&`, a whitespace
   character, `<`, or the end of the string.
2. It is a candidate only if what stopped the body is a `;` and the body is at most 31 bytes long
   (so `&` + body + `;` is at most 33 bytes).
3. A candidate is decoded as below. If it decodes, the whole `&body;` is replaced and scanning
   continues after the `;`.
4. Otherwise the `&` is kept as a literal `&` and scanning continues with the character after it
   (so in `&&amp;`, the first `&` is kept and `&amp;` is decoded: `&&`).

There is a single pass: decoded text is never decoded again (`&amp;lt;` → `&lt;`).

## Numeric references

A body starting with `#` is numeric (XML 1.0 §4.1, HTML §13.5):

- `#x` or `#X` followed by hexadecimal digits: that code point.
- `#` followed by decimal digits: that code point.
- The digits are read as an unsigned 32-bit number in the way Rust's integer parsing does: an
  optional leading `+` is accepted (`&#+65;` and `&#x+41;` are `A`), a `-` is not, leading zeros
  are allowed (within the 31-byte limit), and overflow fails.
- The number must be a Unicode scalar value: surrogates (`&#xD800;`) and values above U+10FFFF
  (`&#x110000;`) fail. `&#0;` is U+0000 (it is a scalar value) and decodes.
- `&#;` and `&#x;` fail.

A reference that fails is kept literally.

## Named references

A body not starting with `#` is a name, matched case-sensitively against:

1. the table below, then
2. the Latin-1 accented letters: a name made of one base letter followed by an accent word:

| Accent word | Base letters (lowercase; uppercase gives the uppercase letter) |
|---|---|
| `grave` | a e i o u |
| `acute` | a e i o u y |
| `circ` | a e i o u |
| `tilde` | a o n |
| `uml` | a e i o u y |
| `ring` | a |
| `slash` | o |
| `cedil` | c |

So `&eacute;` → é, `&Eacute;` → É, `&Ouml;` → Ö, `&ccedil;` → ç, `&Ccedil;` → Ç, `&Oslash;` → Ø,
`&yuml;` → ÿ, and `&Yuml;` → Ÿ (U+0178, the uppercase of ÿ). Only the first letter's case may
vary: `&EACUTE;` and `&eAcute;` are not recognised. These are the HTML named character references
of those letters (HTML §13.5); `&Yacute;` (U+00DD) and `&Yuml;` (U+0178) are both in HTML too.

Names not found are kept literally (`&bogus;`, `&hearts;`, `&AMP;`, `&ymacr;`).

### The named-entity table

The five XML predefined entities (XML 1.0 §4.6) and the HTML ones (HTML §13.5) that ThML modules
use:

| Name | Character | Name | Character |
|---|---|---|---|
| `amp` | `&` U+0026 | `dagger` | † U+2020 |
| `lt` | `<` U+003C | `Dagger` | ‡ U+2021 |
| `gt` | `>` U+003E | `bull` | • U+2022 |
| `quot` | `"` U+0022 | `times` | × U+00D7 |
| `apos` | `'` U+0027 | `divide` | ÷ U+00F7 |
| `nbsp` | no-break space U+00A0 | `frac12` | ½ U+00BD |
| `mdash` | — U+2014 | `frac14` | ¼ U+00BC |
| `ndash` | – U+2013 | `frac34` | ¾ U+00BE |
| `hellip` | … U+2026 | `shy` | soft hyphen U+00AD |
| `lsquo` | ‘ U+2018 | `iexcl` | ¡ U+00A1 |
| `rsquo` | ’ U+2019 | `iquest` | ¿ U+00BF |
| `ldquo` | “ U+201C | `szlig` | ß U+00DF |
| `rdquo` | ” U+201D | `aelig` | æ U+00E6 |
| `laquo` | « U+00AB | `AElig` | Æ U+00C6 |
| `raquo` | » U+00BB | `oelig` | œ U+0153 |
| `para` | ¶ U+00B6 | `OElig` | Œ U+0152 |
| `sect` | § U+00A7 | `thinsp` | thin space U+2009 |
| `copy` | © U+00A9 | `ensp` | en space U+2002 |
| `reg` | ® U+00AE | `emsp` | em space U+2003 |
| `deg` | ° U+00B0 | `zwj` | zero-width joiner U+200D |
| `middot` | · U+00B7 | `zwnj` | zero-width non-joiner U+200C |
| | | `lrm` | left-to-right mark U+200E |
| | | `rlm` | right-to-left mark U+200F |

That is 44 names, plus 28 accented letters in each case (56).

## Required behaviour where the current crate is wrong

The current crate panics on a candidate whose name is empty (`&;`) or starts with a non-ASCII
character (`&é;`, `&éa;`). The required behaviour is the general rule: such a reference is not
recognised and is kept literally. `decode_entities("&;")` is `&;`, `decode_entities("a &é; b")` is
`a &é; b`, and parsing either in any XML-like markup or GBF gives that text.

## Examples

| Input | Output |
|---|---|
| `a &amp; b &lt;c&gt; &#8212; &#x41; &eacute;&Uuml; &bogus; R&D` | `a & b <c> — A éÜ &bogus; R&D` |
| `&nbsp;x` | U+00A0 then `x` |
| `&amp` | `&amp` (no `;`) |
| `&a&amp;` | `&a&` |
| `&a b;` | `&a b;` |
| `&#000000000000000000000000000065;` (30 digits; a 31-byte body) | `A` |
| `&#00000000000000000000000000000065;` (32 digits; a 33-byte body) | kept |
| `&#9;` | a tab |
| `&#x1F600;` | 😀 |

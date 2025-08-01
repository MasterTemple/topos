# τόπος CLI

## Examples

*Results are truncated and currently aligned manually*

### Default Search

Search for all Bible verses in current directory recursively (respecting `.gitignore`)

**Command**

```bash
topos
```

**Output**

```
| File                 | Line | Col | Verse              |
| ----                 | ---- | --- | -----              |
| ./Church 07-27-25.md | 24   | 12  | Colossians 2:1-3   |
| ./Church 07-27-25.md | 39   | 12  | Ephesians 6:18     |
| ./Church 07-20-25.md | 6    | 6   | Colossians 3:12-15 |
| ./Church 07-20-25.md | 27   | 12  | Colossians 3:12    |
| ./Church 07-20-25.md | 34   | 12  | Deuteronomy 7:6-8  |
```

### Search Specific File

**Command**

```bash
topos "Church 07-27-25.md"
```

**Output**

```
| File               | Line | Col | Verse            |
| ----               | ---- | --- | -----            |
| Church 07-27-25.md | 24   | 12  | Colossians 2:1-3 |
| Church 07-27-25.md | 39   | 12  | Ephesians 6:18   |
| Church 07-27-25.md | 46   | 12  | Colossians 4:12  |
| Church 07-27-25.md | 49   | 12  | Colossians 2:1   |
| Church 07-27-25.md | 56   | 12  | Romans 16:1-2    |
| Church 07-27-25.md | 61   | 12  | 3 John 5-8       |
..
```

### Filter by Testament

**Command**

```bash
topos -t new
```

**Output**

```
| File                 | Line | Col | Verse            |
| ----                 | ---- | --- | -----            |
| ./Church 07-27-25.md | 24   | 12  | Colossians 2:1-3 |
| ./Church 07-27-25.md | 39   | 12  | Ephesians 6:18   |
| ./Church 07-27-25.md | 46   | 12  | Colossians 4:12  |
| ./Church 07-27-25.md | 49   | 12  | Colossians 2:1   |
```

### Filter by Genre

**Command**

```bash
topos -g wisdom
```

**Output**

```
| File                 | Line | Col | Verse             |
| ----                 | ---- | --- | -----             |
| ./Church 07-20-25.md | 143  | 12  | Proverbs 16:32    |
| ./Church 07-13-25.md | 229  | 12  | Proverbs 4:24     |
| ./Church 03-02-25.md | 281  | 12  | Proverbs 20:27    |
| ./Church 03-02-25.md | 329  | 12  | Proverbs 28:9     |
| ./Church 06-15-25.md | 68   | 12  | Ecclesiastes 9:10 |
```

### Filter by Book

**Command**

```bash
topos -b Romans
```

**Output**

```
| File                 | Line | Col | Verse         |
| ----                 | ---- | --- | -----         |
| ./Church 07-27-25.md | 56   | 12  | Romans 16:1-2 |
| ./Church 07-20-25.md | 215  | 17  | Romans 15     |
| ./Church 07-20-25.md | 217  | 12  | Romans 15:11  |
| ./Church 07-20-25.md | 271  | 12  | Romans 12:1-2 |
| ./Church 07-13-25.md | 268  | 12  | Romans 12:9   |
| ./Church 07-06-25.md | 98   | 12  | Romans 10:9   |
| ./Church 06-29-25.md | 80   | 12  | Romans 11:29  |
| ./Church 06-22-25.md | 198  | 12  | Romans 12:2   |
| ./Church 02-23-25.md | 224  | 8   | Romans 8      |
```

### Filter Inside Passage

**Command**

```bash
topos -i "1 Peter 1:1-5, 4:11,14-16"
```

**Output**

```
| File                 | Line | Col | Verse         |
| ----                 | ---- | --- | -----         |
| ./Church 03-02-25.md | 118  | 12  | 1 Peter 1:2   |
| ./Church 06-01-25.md | 24   | 101 | 1 Peter 4     |
| ./Church 05-11-25.md | 334  | 12  | 1 Peter 4:15  |
| ./Church 02-02-25.md | 39   | 12  | 1 Peter 4:11  |
| ./Church 03-09-25.md | 241  | 12  | 1 Peter 1:3-4 |
```

### Exclude Testament/Genre/Book/Passage

Use just like above, but prefix full command with `exclude`

```bash
topos --exclude-testament new
```

```
| File                 | Line | Col | Verse             |
| ----                 | ---- | --- | -----             |
| ./Church 07-20-25.md | 34   | 12  | Deuteronomy 7:6-8 |
| ./Church 07-20-25.md | 108  | 93  | Numbers 12        |
| ./Church 07-20-25.md | 131  | 12  | Isaiah 53:7       |
| ./Church 07-20-25.md | 143  | 12  | Proverbs 16:32    |
| ./Church 07-20-25.md | 204  | 7   | Psalms 117        |
| ./Church 07-20-25.md | 208  | 12  | Psalms 117:1-2    |
| ./Church 07-20-25.md | 234  | 12  | Habakkuk 2:14     |
| ./Church 07-20-25.md | 255  | 12  | Psalms 117:2      |
```

## Rules

- By positively specifying a testament/genre/book, you will implicitly telling the program to exclude the remaining items in that category.
- You may choose to exclude a subset from a larger inclusion (ex: book from a genre), however this must be specified **after** the inclusion (or else it will be re-added)
- You may combine multiple filters, and they will be joined with a logical OR

## Usage

```bash
Usage: topos [OPTIONS] [INPUT]

Arguments:
  [INPUT]
          The input can be a directory path, a file path, text, or stdin.

Options:
  -t, --testament <TESTAMENTS>
          Include books from a specific testament (old/new)

      --exclude-testament <EXCLUDE_TESTAMENTS>
          Exclude books from a specific testament

  -g, --genre <GENRES>
          Include books of a specific genre (e.g. epistles, gospels)

      --exclude-genre <EXCLUDE_GENRES>
          Exclude books of a specific genre

  -b, --book <BOOKS>
          Include specific books (e.g. John)

      --exclude-book <EXCLUDE_BOOKS>
          Exclude specific books

  -i, --inside <INSIDE>
          Limit search to a verse range (e.g. John 1:2-3)

  -o, --outside <OUTSIDE>
          Forbid search from matching a verse range (e.g. John 3:4-5)

      --config <CONFIG>
          Use a custom configuration file

  -m, --mode <MODE>
          Specify output mode

          [default: table]

          Possible values:
          - count:    Count total matches
          - json:     Output matches as JSON
          - table:    Output matches as a table
          - quickfix: Output matches for the Neovim Quickfix List

  -v, --verbose
          Include more data about each match

  -c, --context <CONTEXT>
          Units of context

          [default: 1]

      --before <BEFORE_CONTEXT>
          Specify units of context before match to provide

      --after <AFTER_CONTEXT>
          Specify units of context after match to provide

  -h, --help
          Print help (see a summary with '-h')

  -V, --version
          Print version
```

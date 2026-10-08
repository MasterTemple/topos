# Multiple translations

How the Obsidian and Neovim plugins can work with several Bibles at once: a reference can name
its translation (`John 3:16 KJV`), hover and embeds can show it (or several translations next to
each other), and a book can be opened in any translation at the same verse.

topos only finds and parses references. A translation's **text** stays where it is now (USX
folders read by `topos-bible-markup`, `ToposBook` in the FFI, the LSP's `bible.rs`, SWORD modules
through sword-modules). The library's part is the **name** a reference gives and the
**versification** a translation uses.

## Where things stand

- Obsidian: every folder of USX files is a Bible named after the folder
  (`src/core/bibles.ts`). `settings.bible` picks one for links, hover, and embeds. The others can
  be opened, and `#v=3:16` locators already work in any translation of a book.
- Neovim and the LSP: one `bible` folder (`settings.rs`, `Bible::load`), named after the folder in
  URIs (`topos-bible://BSB/JHN`).
- sword-modules: SWORD Bibles with their own versification (`Versification::chapter_verses`
  already returns topos's `ChapterVerses`) and `VerseModule::passage(&Passage)`.
- topos-lib: one `BibleData` with one versification (`BookChapterVerses`). References never carry
  a translation, and the structs that store them (`Passage`, `MatchResult`, the index's
  `FileEntry`) stay that way: they are large or encoded compactly, and most references name no
  translation.

## Step 1: a translation after a reference (done)

`topos_bible::data::translations`:

- `Translations`: the names the caller knows (`Translations::new(["BSB", "KJV"])`), with aliases
  (`.alias("AV", "KJV")`), matched ignoring case. Only listed names count, so a word after a
  reference is never taken for a translation by accident. Multi-word names work
  (`New King James Version`), and the longest name wins (`NASB95` over `NASB`).
- `BibleData::parse_with_translation(input, &translations) -> Option<TranslatedPassage>`: parses
  like `BibleData::parse` and also returns the translation named right after the reference
  (`John 3:16 ESV`, `, ESV`, ` — ESV`, `(ESV)`, `[ESV]`), with its name as listed and its byte
  span. Without one, `translation` is `None`. A name is never read as part of the reference
  (`Ps 22, LXX` is Psalm 22 in the LXX, not Psalms 22 and 70).
- `TranslatedPassage { passage, translation: Option<Translation { name, span }> }` is new, so
  `Passage` is unchanged. `BibleData::parse` and search don't look for translations.

## Step 2: bindings

- FFI: `ToposTranslations` (names and aliases) and `Topos.parseWithTranslation(reference,
  translations, style) -> Option<TranslatedPassage>` with the translation's span in the caller's
  unit. This is what both plugins call. The OSIS and USFM forms that `Topos.parse` tries first
  could take a translation too: OSIS has one (`KJV:John.3.16`, a work prefix).
- The CLI doesn't need this until search does (step 3).

## Step 3: translations in search

- `BibleMatcher::search_with_translations(text, &Translations) -> Vec<TranslatedMatch>`: a new
  result with the `MatchResult` and an optional `Translation`. It runs search, then looks right
  after each match with the same rule as step 1 (`Translations::following`). `MatchResult` and
  `search` stay the same.
- The case `Ps 22, LXX` needs the matcher to end a reference at a translation name, as step 1
  does. Search reads segments in `FoundPassage::find`, so the cut goes there, and only when
  translations are given.
- Add cases to a new `tests/cases/translations.txt` (`input => Ref [KJV] | Ref`) next to
  `search.txt`, run by the library and the FFI.
- Completion: once a reference is complete, `complete_with_translations` offers the listed names
  after it (`John 3:16 K` → `KJV`). It's a new method so `complete` doesn't change, and its cases
  go in `complete.txt` with a translations header.
- Problems: a reference whose named translation lacks the verse (`3 John 1:15 KJV`, where the KJV
  ends at 14) is a problem only once step 4 gives each translation its own versification.

## Step 4: versification per translation

Translations number verses differently: Malachi 4 vs. 3:19–24, 3 John 1:14–15, psalm titles as
verse 1, Romans 16:25–27, and the deuterocanon.

- `Versifications`: named `BookChapterVerses` (`KJV`, `NRSV`, `Vulg`, `LXX`, …). The counts can
  come from sword-modules' 18 systems (generated into JSON so topos-lib stays dependency-free) or
  from a translation's own USX (each `<verse>` gives the counts).
- `TranslationInfo { name, aliases, versification }`, as serde input like `BibleDataInput`, so the
  CLI config, LSP settings, and plugins all describe translations the same way.
  `Translations::from_infos` builds the names.
- `BibleData::parse_with_translation` checks the reference against the named translation's
  versification when it has one (`3 John 1:15 KJV` is `None`), and against the default otherwise.
  `BibleData` itself stays one versification. A translation's `BibleData` is the default with
  its chapter counts swapped (`BibleData::with_versification`), cached by name.
- Mapping between versifications (Mal 4:1 KJV = Mal 3:19 MT) is a later, separate step: it needs
  mapping tables (no source chosen yet; sword-modules doesn't map either). Without it, a
  reference opens at the same numbers in another translation, which is right for almost every
  verse.

## Step 5: the LSP and the Neovim plugin

- Settings: `bibles = { BSB = "~/Bibles/BSB", KJV = "~/Bibles/KJV" }` and `default_bible = "BSB"`.
  The old `bible = "path"` still works as one Bible named after its folder. Each Bible loads
  lazily on first use (today's `Bible::load` per folder).
- Hover, completion documentation, and go-to-definition use the translation named after the
  reference (`parse_with_translation`/`search_with_translations`, built from the `bibles` names
  and their aliases), else `default_bible`. `hover_bibles = ["BSB", "KJV"]` shows several, one
  ```` ```topos-bible ```` block each with the translation in its heading.
- `topos-bible://KJV/JHN` already names the translation. Add a `topos/bibles` request (names and
  books) and a code action to open the same verse in another Bible.
- Insertions: a `{bible}` template variable, and a code action per Bible (`Insert John 3:16
  (KJV)`).
- Diagnostics: a reference to a verse its named translation lacks (with step 4).
- Neovim: `bible` accepts a table like `bibles`. `:ToposBible [name]` sets the default (a
  `workspace/didChangeConfiguration`), `:ToposParallel` opens the current `topos-bible://` book in
  another Bible in a split, scroll-bound by verse, and a Telescope picker lists the Bibles.
  `:checkhealth` checks each folder.

## Step 6: the Obsidian plugin

- The translation names are the Bible folders' names plus aliases from settings, and each
  reference resolves through `parseWithTranslation` in the WASM package.
- `[John 3:16 KJV]` links, `![John 3:16 KJV]` embeds, and hover use the named Bible, else
  `settings.bible`. Link templates get `{translation}` (Literal Word, Bible Gateway), so a
  translation the vault doesn't have still links somewhere.
- The Bible tab: a translation switcher that keeps the verse (`#v=` locators already work across
  translations), then a parallel view of two Bibles.
- Embeds and hover with several translations (`settings.compareBibles`).
- The index doesn't store translations. If the sidebar groups or filters by translation later,
  keep them in a side table keyed by entry and span, not in `FileEntry`'s codec.

## Step 7: SWORD modules (later)

sword-modules reads SWORD Bibles with their own versification. When the plugins or the LSP read
SWORD modules (native only; WASM would need the files in the vault), a module's name is its
translation name and its `Versification=` gives the counts for step 4. topos-lib still doesn't
depend on sword-modules; the front end lists the modules as `TranslationInfo`s.

## Order

Steps 1–2 make translation-tagged references usable in both plugins with one Bible each. Step 5
(LSP `bibles`) and step 6 (named Bibles in links and embeds) can then go in parallel. Step 3
(search) is what makes `KJV` after a reference in a note pick the Bible for hover. Step 4
(versification) is needed only once references hit the verses where translations differ.

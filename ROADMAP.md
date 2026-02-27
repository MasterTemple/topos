1. Fix issues related to matching
    - Parse older formats / Roman numerals / sub-verses: `Matth. x, 8` [#5](https://github.com/MasterTemple/topos/issues/5)
    - `cf` and `ff` [#10](https://github.com/MasterTemple/topos/issues/10)
    - Searching `John` only matches `1 John` [#8](https://github.com/MasterTemple/topos/issues/8)
    - Support dashes in book names for justified text [#11](https://github.com/MasterTemple/topos/issues/11)
2. Finish Auto-complete
3. Generalize match location [#9](https://github.com/MasterTemple/topos/issues/9)
    - PDF
    - SRT/VTT/..
    - JSON
    - XML
4. Validate match segments based on actual verse in the text
5. Reduce false positives (such as `"is"` for Isaiah) from searches, but keep for explicit parsing [#3](https://github.com/MasterTemple/topos/issues/3)
6. Contextual parsing: recognize verse headings when a document is about a particular book [#6](https://github.com/MasterTemple/topos/issues/6)
7. Provide user specified formatting options [#4](https://github.com/MasterTemple/topos/issues/4)
8. Complete/Improve CLI
    - Add grep-like commands (highlight, show context, ..)
9. Cache search results [#2](https://github.com/MasterTemple/topos/issues/2)
10. Add incremental parsing
11. Create a standard format for specifying segment ranges


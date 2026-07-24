# Afmt
To-Do:

- active static, prettier80, and comments fixture idempotency coverage is complete
- battle-test bulk write/check idempotency path is covered; retain fallback diagnostics
- to-do folder
- local bulk benchmark is available at tests/battle_test/benchmark_bulk.sh
- binary exp comments
- remove all the "pub" properties in struct/enum

## Big items:

- as I don't use precise group(), challenge: how to avoid some line-comment new Doc
  variant so line-comment doesn't need to calculate a newline anymore?

Other:
- should bodymember not handle 1-2 newline(), rather let the code or comment to
  handle it? (one place to handle it all, it's better?)

- check Dang's logic
- chain method comment, tests in to-do folder
- design newline handling that's not coupled?


## ToDo
- Doc::Text needs to check precedding space, fits() doesn't check the same.
  alternatives? change `b.txt(" ")` to `b.try_space()` to tell the conditional
  adding space
- field_access with super or this
- Check what Enum size too big?

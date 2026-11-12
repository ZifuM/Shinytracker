# Modern style (v0.5.0)

The Companion window's default look since 0.5. Clean and Classic are still
under Appearance → Style; the overlay is the same in every style.

## Rules (from the Tulip app-design guidance)

- **Standard layout everywhere.** Top bar (brand, where you are, the overlay's
  Start / Stop, window buttons), sidebar, page header, then cards. Same parts
  in the same places on every page, so nothing has to be relearned.
- **One colour per page.** Each page has an icon chip in its own colour, used
  in the sidebar, the top bar and its header, so pages are recognisable at a
  glance. Status colours mean the same thing everywhere: green = good / on,
  amber = attention, red = problem.
- **Primary action is the boldest thing on screen.** Only primary buttons use
  the accent (or gradient), with a soft glow. Secondary buttons are filled
  neutral surfaces. Start overlay is always in reach in the top bar.
- **Icons on buttons and tiles** give the action context.
- **Readable, consistent type.** System font; page title 24, section 16,
  body 13.5, nothing under 12.
- **Reusable components** (ui_kit): card, section, tab bar, segmented, pill,
  choice tile, stat tile, icon chip, page tiles, hero card.
- **Two-panel screens** for list + detail.
- **Responsive.** Tiles reflow by width; the sidebar folds to icons («).

## Surfaces

Dark: sidebar #0D0F16, page #12141D, card #1A1D29, raised #212534,
line #272B3B, text #ECEEF5, muted #9096AC. Cards: 14px corners, 1px line,
soft shadow. Light and semi-dark follow the same structure.

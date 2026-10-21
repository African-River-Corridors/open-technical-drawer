# ISO check list and house choices

This file belongs to the [Drawing Format Standard](drawing-format-standard.md).

The ISO texts (ISO 128, 5455, 5457, 7200, 3098 and 129) were not at hand when the standard was
written. Every value that the standard takes from an ISO part is a **proposal**. It is listed in
part A. Nobody may call a value "from ISO" until its item is closed.

To close an item: read the clause of the ISO part, compare the value, then write the clause number,
the date and the result in the "Closed" column. Do not copy ISO text into this repository.

The "Recall" column says how sure the author is of the proposed value. It is a guide to where to
look first. It is not evidence.

## A Pending ISO checks

| ID | ISO part | Clause | Proposed value | Used in | Recall | Closed |
|---|---|---|---|---|---|---|
| P-01 | ISO 5457 (A-series sizes are also in ISO 216) | to be confirmed | Trimmed sizes: A0 841 x 1189, A1 594 x 841, A2 420 x 594, A3 297 x 420, A4 210 x 297 mm | 6.1 | high | open |
| P-02 | ISO 5457 | to be confirmed | Border inset from the trimmed edge: left (filing margin) 20 mm on every size. Other three edges 20 mm on A0 and A1, 10 mm on A2, A3 and A4 | 6.3 | medium | open |
| P-03 | ISO 5457 | to be confirmed | Border line is continuous and at least 0.7 mm wide | 6.3.3 | medium | open |
| P-04 | ISO 5457 | to be confirmed | Four centring marks at the middle of each side. Each is 0.5 mm wide. Each runs from the trimmed edge to 5 mm inside the drawing space | 6.4 | low | open |
| P-05 | ISO 5457 | to be confirmed | Grid zones (long side x short side): A0 24 x 16, A1 16 x 12, A2 12 x 8, A3 8 x 6, A4 6 x 4. Equal division of the border side | 6.5 | medium (A0 is the least sure) | open |
| P-06 | ISO 5457 | to be confirmed | Zone numbers along the horizontal sides, zone letters along the vertical sides, letters I and O not used. Zone marks 0.35 mm wide | 6.5.2, 6.5.3 | medium | open |
| P-07 | ISO 5455 (check whether a later part has replaced it) | to be confirmed | Preferred ratios 1:1, 1:2, 1:5 and their powers of ten (the standard uses 1:1 to 1:100000) | 7.3.1 | high | open |
| P-08 | ISO 3098 (the clause for the height series) | to be confirmed | Standard letter heights in millimetres: 2.5, 3.5, 5, 7, 10, 14, 20. The standard also uses 2.5 mm for scale bar text and 3.5 mm for zone labels and the north label | 6.5.3, 7.9.1, 7.10.5, 11.2 | high | open |
| P-09 | ISO 128 (the part for line widths) | to be confirmed | Width series in mm: 0.13, 0.18, 0.25, 0.35, 0.5, 0.7, 1, 1.4, 2. The standard uses 0.18, 0.25, 0.35, 0.5, 0.7, 1.0 and 1.4 | 8.2 | high | open |
| P-10 | ISO 128 (the part for line widths) | to be confirmed | Widths of the three roles are in the ratio thin : thick : extra thick = 1 : 2 : 4 | 8.1.1 | high | open |
| P-11 | ISO 128 or ISO 5457 (to be found) | to be confirmed | Line group by sheet size: A4 and A3 group 0.35, A2 and A1 group 0.5, A0 group 0.7. ISO may leave the group free. If it does, this item moves to the house choices | 8.2 | low | open |
| P-12 | ISO 128 (the part for basic conventions for lines) | to be confirmed | Line types and numbers: continuous 01.1, dashed 02.1, long-dashed dotted ("chain") 04.1, long-dashed double-dotted ("chain double-dashed") 05.1. Pattern in units of line width d: dashed 12 dash, 3 gap; chain 24, 3, 0.5, 3; chain double-dashed 24, 3, 0.5, 3, 0.5, 3 | 8.4 | medium (the type numbers and the 0.5 dot are the least sure) | open |
| P-13 | ISO 3098 (the part for the alphabet and the clause for type B) | to be confirmed | Type B lettering: line thickness d = h / 10. Upright and inclined (75 degrees) forms exist. The standard uses the upright form | 11.1 | medium | open |
| P-14 | ISO 3098 (the part for type B lettering) | to be confirmed | Widths of the capital letters and digits of type B, in units of d = h / 10: A 7, B 6, C 6, D 6, E 5, F 5, G 6, H 6, I 1, J 5, K 6, L 5, M 7, N 6, O 6, P 6, Q 6, R 6, S 6, T 5, U 6, V 7, W 9, X 6, Y 7, Z 6, digits 6 (the digit 1: 4). Used only to compare candidate fonts | 11.6.3 | low (recalled, not read) | open |
| P-15 | ISO 129 (the part for general principles, and the part for arrowheads) | to be confirmed | A dimension is a dimension line, extension lines, arrowheads and text. Arrowheads are filled triangles that are about 15 degrees wide. The unit is stated in the notes and not on each dimension. Text is parallel to the dimension line and above it, readable from the bottom or the right | 12.1.1, 12.3 | medium | open |
| P-16 | ISO 129 (the part for extension lines) | to be confirmed | The extension line starts with a small gap from the object (about 1 mm) and goes a little (about 2 mm) beyond the dimension line | 12.3 | low | open |
| P-17 | ISO 7200 (the clause for data fields) | to be confirmed | Data fields of a title block: legal owner, identification (document) number, revision index, date of issue, title and supplementary title, document type, document status, language, responsible department, created by, approved by, sheet number and number of sheets. Mandatory: owner, identification number, title and date of issue. The standard marks owner, title and document number as required, and takes the date of issue from the latest revision | 14.3, 14.4 | low | open |

## B House choices

A house choice is a value or a rule that ISO does not set, or that this standard makes simpler on
purpose. It needs a reason. It does not need an ISO check.

| ID | Choice | Reason | Used in |
|---|---|---|---|
| H-01 | An unknown key in a document is an error | A silently ignored key can leave a drawing at a different scale from the one written | 4.3 |
| H-02 | The unit of a number comes from its member name ending (`_m`, `_mm`, `_deg`) | The two length units stay visible at every number | 4.5 |
| H-03 | Every finding names the clause it enforces | Tests and users can find the rule | 4.6 |
| H-04 | Paper origin top-left, +y down | SVG and PDF viewers use the same sense, so no output step flips a sign | 5.2 |
| H-05 | Precision: 6 places for model metres, 3 for paper millimetres, 3 for degrees | One micrometre is far below any drawing need and far above 64-bit rounding noise | 5.4 |
| H-06 | Half-even rounding on the exact binary value | One rule that is the same in every language | 5.5 |
| H-07 | Fit tolerance 1e-6 mm, on fit tests only | Absorbs binary rounding and nothing more | 5.7 |
| H-08 | Either orientation for every size | The standard sets no rule that ties a size to one orientation | 6.2 |
| H-09 | The border line is centred on the border rectangle | The simplest rule. The line is at most 1.4 mm wide | 6.3.3 |
| H-10 | Only the ISO preferred ratios, the four house ratios and `fit`. Any other ratio, and any enlargement, is an error | Two short lists of scales, so every scale can be set on a scale rule | 7.3.2 |
| H-11 | One scale per viewport, no exaggerated height | A wrong exaggeration is worse than none | 7.3.3 |
| H-12 | The scale applies to coordinates only, never to widths or text | A scale on the whole drawing would also scale every line width by 1/N | 7.4.1 |
| H-13 | An entity outside its viewport frame is an error | A stray point is a loud error, never a silent clip | 7.7 |
| H-14 | Scale bar constants: inset 6, bar height 3, tick 1.5, label baseline 5.0, ratio baseline 9.0, alternate segments solid | ISO sets no size for a scale bar. The baselines leave clear gaps for 2.5 mm text | 7.9.1, 7.9.5 |
| H-15 | Derived bar length: largest of 1, 2 or 5 times a power of ten that is at most a quarter of the frame width | A quarter keeps the bar small. A round length can be read against a scale rule | 7.9.4 |
| H-16 | Minimum frame sizes: 46 mm high with a scale bar, 38 mm in both directions with a north arrow | The bar and the arrow must fit and must not touch | 7.9.10, 7.10.7 |
| H-17 | `model_bearing_deg` is the compass bearing of the model +y axis | Matches the tool's tested output. The old README worded it the opposite way | 7.10.1 |
| H-18 | North arrow: 18 mm tall, inset 10 mm, head and shaft proportions as in 7.10.5 | ISO sets no size for a north arrow | 7.10.5 |
| H-19 | A `line_group` that differs from the group of the sheet size is an error | One rule gives one set of widths for one size. It keeps drawings on the same size alike | 8.2.2 |
| H-20 | Widths snap to the set of widths the DXF format can store: nearest, tie to the thinner, compared in whole micrometres | The three outputs must agree on one width. Whole numbers avoid binary tie errors | 8.3.1, 8.3.2 |
| H-21 | An entity on an undeclared layer is an error | A drawing with a layer that nobody declared hides a mistake | 8.5.1 |
| H-22 | The 19 standard layers, with their roles and line types | ISO sets no layer names. The names are those of the reference tool | 8.5.2 |
| H-23 | Custom layer names: A to Z, digits and underscore, up to 31 characters, starting with a letter | Valid in every CAD program, and the same in every case | 8.5.4 |
| H-24 | An entity may override the role or the line type of its layer, never the width | Layers name a role, not a width. A role override keeps that rule | 8.5.6 |
| H-25 | SVG and PDF print in pure black on white | An ISO drawing is black on white. Layer colour is a CAD aid | 8.6.1 |
| H-26 | One CAD colour number per standard layer, 19 different numbers, chosen to look different in the common CAD palette | A CAD user can tell layers apart. No claim is made about the hue | 8.6.2 |
| H-27 | Upright lettering only | One style keeps the width table to one table | 11.1 |
| H-28 | Text rotation is a paper angle. The viewport rotation does not turn text | A rotated map keeps its labels readable | 11.3.2 |
| H-29 | Until the width table exists, text containment is tested at the anchor point only | The text box needs the width table, and the width table needs the font | 11.5 |
| H-30 | A centre line is a line with default role thin and line type chain | The layer says what it belongs to. The type says how it looks | 9.7 |
| H-31 | Entity order has no effect on the normalised content | Clause 16 sorts the entities | 9.9 |
| H-32 | The hatch pattern names solid, lines, concrete, rockfill, soil, water | The materials the first examples need | 10.2 |
| H-33 | Each schema rule carries an `x-clause` annotation | A user who reads a schema error can find the rule in plain words | 18.6 |
| H-34 | Exact sheet furniture entities: border, centring marks, zone marks and labels | Two implementations must write the same entities. The values come from clause 6 | 16.6.1 |
| H-35 | Scale bar and north arrow entities, with thin lines | The lines match the thin lines of the drawing | 16.6.3, 16.6.4 |
| H-36 | The sort key of the entity list | A total order that needs no locale and no string form of a number | 16.8 |
| H-37 | House ratios 1:25, 1:250, 1:1250 and 1:2500, allowed without warning and used by `fit` | A barge of 80 m must fit an A3 sheet at 1:250. Site plans use 1:1250 and 1:2500 | 7.3.4 |
| H-38 | The font is Barlow Semi Condensed Regular: close to ISO 3098 lettering, not certified | An open-licence (SIL OFL 1.1) font that is nearest to ISO 3098 type B of those compared. osifont, which is drawn for ISO 3098, was not adopted (part C) | 11.6.3 |
| H-39 | Text width uses the advance widths only: no kerning, no ligatures, no spacing | A width that ignores kerning is the same in every implementation | 11.7.1 |

## C Font decision (ticket #8)

Decision of the maintainer, 2026-10-14: adopt an open-licence sans-serif under the SIL Open Font
License 1.1 as a house choice, "close to ISO 3098 lettering, not certified".

Candidates, all from the Google Fonts repository (OFL 1.1). Measured with the glyph outlines of the
font files. The width error is the mean difference, over the capitals and the digits, between the ink
width of the glyph (as a multiple of 0.1 of the capital height) and the type B widths of item P-14.
A smaller error is nearer. The stem is the width of a stem as a fraction of the capital height. ISO
3098 type B has 0.100.

| Font | Stem / capital height | Mean width error |
|---|---|---|
| **Barlow Semi Condensed Regular** | **0.101** | **0.48** |
| Barlow Semi Condensed Light | 0.076 | 0.62 |
| Barlow Regular | 0.101 | 0.74 |
| Barlow Light | 0.076 | 0.67 |
| IBM Plex Sans Condensed Regular, Light | not comparable (serifed `I`) | 0.73, 0.58 |
| Share Tech Regular | not comparable (serifed `I`) | 0.94 |

Barlow Semi Condensed Regular has the stem of type B and the smallest width error. Its licence text
was compared with the official text at openfontlicense.org and is the same. The font file allows
embedding (`fsType` 0). The comparison depends on P-14, which was recalled and not read from ISO 3098.

**osifont** (GPL or LGPL with font exception) is drawn for ISO 3098. It was not adopted because of
doubt about its licence in an Apache-2.0 tool (see ticket #8). It can be revisited after a legal view.
| H-40 | The hatch unit is 10 times the thin width of the line group | The pattern follows the line group, so it looks alike on every sheet size | 10.5.1 |
| H-41 | The hatch frame is fixed to the paper origin | Two hatches of one pattern and one angle line up. The result is the same in every implementation | 10.5.2 |
| H-42 | The marks of concrete, rockfill, soil and water as in the table of clause 10.5.7 | ISO sets no pattern for these materials in a form that can be computed. The shapes are plain, and the clearances keep marks off the boundary | 10.5.7 |
| H-43 | The DXF has a non-plotting layer `VIEWPORT` for the viewport frames | A viewport frame must not print | 15.4.2 |

## D Open checks that are not ISO

These are facts about CAD programs that the author could not check, because no CAD program was at
hand. They do not change the normalised content. The DXF can be read back without them.

| ID | Check | Where used |
|---|---|---|
| D-01 | Whether a CAD program turns the view the way that the twist angle of the viewport is written (clause 15.4.6): the twist is the viewport rotation, positive counter-clockwise. | 15.4.6 |
| D-02 | Whether `$PSLTSCALE` 1 with patterns in millimetres shows the patterns at paper size in every CAD program. | 15.4.1, 15.4.3 |
| D-03 | Whether the vertical alignment `middle` of a DXF text is at half the capital height, as clause 11.7.2 says. | 15.4.4 |
| H-44 | A dimension shows its unit through `unit` and `decimals`, with no unit sign. The text is the value rounded half-even to the stated places | The unit is stated in the notes. The rounding rule is the one of clause 5 | 12.1.1 |
| H-45 | Arrow length 2u, arrow width u/2, gap u/2, extension u, text offset u/2, where u is the hatch unit | The dimension follows the line group, as hatches do | 12.3 |
| H-46 | In a chain, the printed numbers of the parts must add to the printed overall | A reader adds the printed numbers. The numbers must agree | 12.8 |
| H-47 | In a symbol, lengths have no unit ending. `units` names the unit: `paper_mm` (constant size on the paper) or `model_m` (real size) | Equipment is a real size. A valve or a pump on a diagram is a sign | 13.2.1 |
| H-48 | The layout and the sizes of the shipped title block: 180 x 56 mm, cells as in the table of clause 14.3, a register of 8 rows of 5 mm | ISO 7200 sets the data and not the layout of a block | 14.3 |
| H-49 | Derived fields (revision, date of issue, created by, checked by, approved by, status, scale, sheet, size) are never typed. The approver is the `app` of the latest entry and nothing else | A value that can be derived cannot disagree. The approver is the signature of a person | 14.4.2 |
| H-50 | An overflow row stands first in a full register and says how many earlier revisions are not shown | A register that is cut is visible: nothing is dropped without a note | 14.7.2 |
| H-51 | The status `issued` needs a title block and a register whose latest entry has an approver and a valid seal | A drawing is issued by a person who signs. A flag that an agent can set is not a signature | 14.9.1 |
| H-52 | Strip sizes: heading 3.5 mm, items 2.5 mm, pitch 4.5 mm, edge space 2 mm; strips are cut from the strip area above the register | Simple, and the same in every implementation | 14.10.1 |
| H-53 | The status `draft` or `concept` is shown by one line of text of height 2.5 mm (`DRAFT – NOT FOR CONSTRUCTION`, `CONCEPT – NOT FOR CONSTRUCTION`), in the notes strip or in a small reserved region at the bottom-left of the drawing space. `issued` shows no line. There is no mark across the drawing | A mark across the drawing covers notes and dimensions. One small line states the status and covers nothing | 14.9 |

## E Visible changes for 1.0

Changes a reader of an earlier draft of the standard will see. One row for each change.

| Change | Before | Now | Clause |
|---|---|---|---|
| The status of a draft or concept drawing | A 20 mm diagonal watermark across the drawing space (`DRAFT`, `CONCEPT - NOT FOR CONSTRUCTION`, `ISSUED FOR CONSTRUCTION`), on the layer `WATERMARK` | One line of 2.5 mm text (`DRAFT – NOT FOR CONSTRUCTION`, `CONCEPT – NOT FOR CONSTRUCTION`) in the notes strip, or in a small reserved region at the bottom-left. `issued` shows no line. The layer `WATERMARK` and the origin `sheet.status_watermark` are gone; the origin is `sheet.status_line`. House choice H-53 | 14.9 |

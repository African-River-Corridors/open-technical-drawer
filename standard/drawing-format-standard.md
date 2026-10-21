# Drawing Format Standard

Version: 1.0 (draft, written under spec 0001, see `docs/specs/0001-drawing-format-standard.md`).

This is the standard for the **drawing document**: one JSON file that describes one whole sheet.
Any tool can read it. Any tool that follows this standard gives the same drawing from the same file.

## 0 How to read this standard

**0.1** Every rule has a number such as `7.4.2`. Tests, error messages and code cite these numbers.
A number never changes meaning inside one version of the standard.

**0.2** Rules marked **Basis** say where a value comes from. There are two kinds of basis.

- *Pending ISO check.* The value is taken from an ISO part, but the text of that part was not at
  hand when this clause was written. The value is a proposal. It is listed in
  [`iso-checklist.md`](iso-checklist.md) with a number `P-nn`. Until the checklist item is closed,
  nobody may call the value "from ISO".
- *House choice.* ISO does not set the value, or this standard chooses a simpler rule. The reason is
  given. The reason is listed in the checklist with a number `H-nn`.

**0.3** This standard does not quote ISO text. It states its own rules in its own words.

**0.4** Clauses 1 to 18 are complete in this draft. Each heading
names the ticket of spec 0001 that writes it.

## 1 Scope

**1.1** This standard defines the drawing document, its units and precision, the sheet, the
viewports and their scales, and (in later clauses) layers, lines, geometry, text, dimensions,
symbols, sheet furniture, the outputs, and the normalised content that two implementations must
agree on.

**1.2** This standard does not define what a drawing of a given kind must show. It does not define
geographic backdrops, build graphs, or any private title block.

## 2 Conformance and language

**2.1** The words MUST, MUST NOT, SHOULD and MAY have their usual standards meaning. MUST is a
requirement. SHOULD is a recommendation. MAY is a permission.

**2.2** A **conforming implementation** reads a drawing document, checks it, and produces the
**normalised content** of clause 16. It conforms when it gives the expected normalised content, or
the expected error, for every case of the conformance suite (clause 17).

**2.3** SVG, PDF and DXF bytes MAY differ between implementations. Only the normalised content and
the findings are compared.

## 3 Terms

**3.1 Drawing document.** One JSON file that describes one sheet and follows this standard.

**3.2 Model space.** The space where the real object is described. The unit is the metre.

**3.3 Paper space.** The space of the printed sheet. The unit is the millimetre.

**3.4 Trimmed sheet.** The sheet after it is cut to its final size. All sheet sizes in this standard
are trimmed sizes.

**3.5 Drawing space.** The part of the trimmed sheet inside the border.

**3.6 Viewport.** A rectangle of the drawing space that shows a window of model space at one scale.

**3.7 Finding.** A message from a check. A finding is an **error** or a **warning** (clause 4.6).

## 4 The drawing document

**4.1** A drawing document is one JSON text in UTF-8. The root is an object.

**4.2** The root object has these members. The JSON Schema of this standard (clause 18) is the
final word on member names. This clause says what each member means.

| Member | Meaning | Clause |
|---|---|---|
| `standard_version` | The version of this standard that the file follows. | 18 |
| `sheet` | Size, orientation and line group. | 6, 8 |
| `title_block` | The title block template and its field values. | 14 |
| `revisions` | The revision register. | 14 |
| `status` | The drawing status: draft, concept or issued. | 14 |
| `layers` | The layers in use. | 8 |
| `symbol_libraries` | The symbol libraries that the document uses: files, or libraries written in the document. | 13 |
| `viewports` | One or more viewports, each with its entities. | 7, 9 to 13 |
| `notes`, `references` | The notes table and the references table. | 14 |

**4.3** A reader MUST reject a root with a member that this standard does not define. This applies
to every object in the document. A mistyped key is an error. It is never ignored.
*Basis: house choice H-01. A key that is silently ignored can leave a drawing at a different scale
from the one its author wrote.*

**4.4** A reader MUST reject a document that holds a repeated key in one object.

**4.5** Every number in a document is a finite JSON number. The unit of a number comes from the end
of its member name:

| Ending | Unit |
|---|---|
| `_m` | metres in model space |
| `_mm` | millimetres on paper |
| `_deg` | degrees |

A number whose member name has none of these endings has no unit. Examples are counts and the scale
denominator. The schema states which members are whole numbers.
*Basis: house choice H-02. The rule keeps the two length units visible at every number.*

**4.6** A check gives **findings**. An **error** stops the build: no output is written. A
**warning** is reported and the build goes on. Every finding carries the number of the clause it
enforces. Example: error `7.4.3`.
*Basis: house choice H-03. A clause number lets tests and users find the rule.*

**4.7** Array order is kept as written. Order does not change the normalised content, because
clause 16 sorts it. The only exception is any array that this standard says is ordered.

## 5 Units and precision

**5.1 Model space.** Model coordinates are in metres. The axes are right-handed: +x to the right,
+y up. Version 1.0 draws in two dimensions. There is no z.

**5.2 Paper space.** Paper coordinates are in millimetres. The origin is the top-left corner of the
trimmed sheet. +x runs to the right. +y runs **down**.
*Basis: house choice H-04. SVG and PDF viewers use the same sense, so no output step flips a sign.
Only the map from model to paper (clause 7.4) turns +y up into +y down.*

**5.3 Angles.** Angles are in degrees.

- A model angle is measured counter-clockwise from +x, with +y up.
- A rotation of a viewport is positive counter-clockwise (clause 7.4).
- A compass bearing and a paper direction are measured clockwise (clause 7.10).

**5.4 Precision.** Each quantity has a fixed number of decimal places. A value is rounded to that
number of places once, at the end, as clause 5.5 says. Never round a value that will be used in a
later calculation.

| Quantity | Decimal places | Smallest step |
|---|---|---|
| Model length, in metres | 6 | 1 micrometre |
| Paper length, in millimetres | 3 | 1 micrometre |
| Angle, in degrees | 3 | 0.001 degree |
| Scale denominator | 0 (whole number) | 1 |

*Basis: house choice H-05. One micrometre is far below any drawing need and far above the
rounding noise of 64-bit arithmetic.*

**5.5 Rounding.** Rounding uses the **half-even** rule, applied to the exact value of the number:

1. Take the exact decimal value of the 64-bit binary number. Do not first convert it to text.
2. Find the two numbers with the required decimal places that are on each side of it.
3. Pick the nearer one.
4. If the value is exactly half way between them, pick the one whose last digit is even.
5. If the result is a negative zero, write it as zero.

*Basis: house choice H-06. One rule that is the same in every language.*

**5.5.1** Worked numbers. The first five rows are exact ties in 64-bit binary. The last three rows
show why step 1 says "exact value".

| Value | Places | Result | Why |
|---|---|---|---|
| 0.0625 | 3 | 0.062 | half way between 0.062 and 0.063; 2 is even |
| 0.1875 | 3 | 0.188 | half way between 0.187 and 0.188; 8 is even |
| -0.0625 | 3 | -0.062 | the sign does not change the rule |
| 2.5 | 0 | 2 | half way between 2 and 3 |
| 3.5 | 0 | 4 | half way between 3 and 4 |
| -0.0004 | 3 | 0 | the result is negative zero, so it is written as 0 |
| 1.0005 | 3 | 1.000 | the 64-bit number nearest 1.0005 is a little below it, so it is not a tie |
| 0.0005 | 3 | 0.001 | the 64-bit number nearest 0.0005 is a little above it, so it is not a tie |

**5.6 Arithmetic.** All arithmetic is in 64-bit binary floating point (IEEE 754 binary64). Two
implementations MAY differ in the last bit of a result. The conformance suite is built so that this
never changes a rounded value (clause 17).

**5.6.1** The sine and cosine of a multiple of 90 degrees MUST be exact: 0, 1 or -1. An
implementation MUST NOT use a library value such as 6.1e-17 for the cosine of 90 degrees.

**5.7 Fit tolerance.** A test that asks "does this fit" accepts a value that is larger than the limit
by not more than **0.000001 mm** (1e-6 mm). No other comparison in this standard has a tolerance.
*Basis: house choice H-07. The tolerance absorbs binary rounding and nothing more. A larger one
would hide a real overflow.*

## 6 Sheets

**6.1 Sizes.** A sheet has one of five trimmed sizes. A sheet of any other size is an error (6.1.3).

| Size | Short edge, mm | Long edge, mm |
|---|---|---|
| A0 | 841 | 1189 |
| A1 | 594 | 841 |
| A2 | 420 | 594 |
| A3 | 297 | 420 |
| A4 | 210 | 297 |

*Basis: pending ISO check, ISO 5457 (clause to be confirmed), checklist P-01.*

**6.1.1** Only trimmed sizes are used. The standard has no rule for untrimmed sheets.

**6.1.2** The standard does not define sizes larger than A0 or smaller than A4.

**6.1.3** An unsupported size is error `6.1.3`.

**6.2 Orientation.** The orientation is `landscape` or `portrait`. In landscape the long edge is
horizontal. In portrait the short edge is horizontal. The default is `landscape`. Either
orientation is allowed for every size.
*Basis: house choice H-08. The standard sets no rule that ties a size to one orientation.*

**6.2.1** Any other orientation value is error `6.2.1`.

**6.3 Border and drawing space.** The **border** is a rectangle inset from the trimmed edge. The
space inside it is the drawing space. The insets are:

| Size | Left (filing margin) | Top | Right | Bottom |
|---|---|---|---|---|
| A0 | 20 mm | 20 mm | 20 mm | 20 mm |
| A1 | 20 mm | 20 mm | 20 mm | 20 mm |
| A2 | 20 mm | 10 mm | 10 mm | 10 mm |
| A3 | 20 mm | 10 mm | 10 mm | 10 mm |
| A4 | 20 mm | 10 mm | 10 mm | 10 mm |

*Basis: pending ISO check, ISO 5457 (clause to be confirmed), checklist P-02.*

**6.3.1** "Left" is the left edge as the sheet is viewed. It does not change with orientation. The
insets are the same in portrait and in landscape.

**6.3.2** Worked numbers. The drawing space is the trimmed sheet less the insets.

| Sheet | Drawing space, width x height | Left x, top y of drawing space |
|---|---|---|
| A3 landscape | 390 x 277 mm | 20, 10 |
| A3 portrait | 267 x 400 mm | 20, 10 |
| A4 portrait | 180 x 277 mm | 20, 10 |
| A0 landscape | 1149 x 801 mm | 20, 20 |

**6.3.3** The border line is a continuous line of the extra thick role (clause 8). It is centred on
the border rectangle. Its width is never less than 0.7 mm.
*Basis for the minimum width: pending ISO check, ISO 5457 (clause to be confirmed), checklist P-03.
Basis for centring: house choice H-09. It is the simplest rule, and the line is at most 1.4 mm wide.*

**6.4 Centring marks.** There are four centring marks. One is at the middle of each side of the
trimmed sheet. Each mark is a straight line 0.5 mm wide. It runs at a right angle to its side, from
the trimmed edge to a point 5 mm inside the drawing space.
*Basis: pending ISO check, ISO 5457 (clause to be confirmed), checklist P-04. The width, the
position and the length of the mark are all unconfirmed.*

**6.5 Grid reference.** Each side of the drawing space is divided into equal **zones**. The number
of zones along the long side and along the short side comes from the size:

| Size | Zones along the long side | Zones along the short side |
|---|---|---|
| A0 | 24 | 16 |
| A1 | 16 | 12 |
| A2 | 12 | 8 |
| A3 | 8 | 6 |
| A4 | 6 | 4 |

*Basis: pending ISO check, ISO 5457 (clause to be confirmed), checklist P-05.*

**6.5.1** A zone is one equal part of the border side. Its length is the side length divided by the
number of zones. In landscape, the long side is horizontal. In portrait, the counts swap.

**6.5.2** Zones along the horizontal sides are numbered 1, 2, 3 and so on, from left to right.
Zones along the vertical sides are lettered from top to bottom: A, B, C, D, E, F, G, H, J, K, L, M,
N, P, Q, R. The letters I and O are never used, because they look like 1 and 0.
*Basis: pending ISO check, ISO 5457 (clause to be confirmed), checklist P-06.*

**6.5.3** A **zone mark** is a straight line, 0.35 mm wide, that crosses the margin strip between
the border and the trimmed edge, at each boundary between two zones. A zone **label** is the number
or the letter of the zone. It is 3.5 mm high. It is centred on the zone, and centred across the
margin strip. Labels appear on both horizontal sides and on both vertical sides.
*Basis: pending ISO check. The mark width and the label place are from ISO 5457 (clause to be
confirmed), checklist P-06. The text height is a standard ISO 3098 height (clause to be confirmed),
checklist P-08.*

**6.5.4** Worked numbers. A3 landscape has a drawing space 390 mm wide, 8 zones across. Each zone is
48.75 mm wide. The boundaries are at x = 20, 68.75, 117.5, 166.25, 215, 263.75, 312.5, 361.25 and 410.
It has 6 zones down a height of 277 mm. Each zone is 46.1666... mm, which rounds to 46.167 mm (clause
5.5). The label of the first zone down is `A`. The label of the sixth is `F`.

**6.6 Reserved regions.** A **reserved region** is a rectangle inside the drawing space, with its
position and size in paper millimetres. The title block, the revision register, and the strips for the
notes and the references are reserved regions (clause 14.5). So is the region of the status line, when the document has one (clause 14.9).

**6.6.1** The drawing space minus the reserved regions is the space where viewports may sit
(clause 7.2).

## 7 Viewports and scales

**7.1 What a viewport is.**

**7.1.1** A viewport is a rectangle on the paper (its **frame**) that shows a
window of model space at one scale. A sheet has one or more viewports. A sheet with none is error
`7.1.1`.

A viewport has these members. The schema is final for names.

| Member | Meaning |
|---|---|
| `id` | A name for the viewport. It is unique in the document (clause 9.8). |
| `frame` | The frame on paper: `x_mm`, `y_mm` (the top-left corner), `width_mm`, `height_mm`. |
| `window` | The model window: `xmin_m`, `ymin_m`, `xmax_m`, `ymax_m`. |
| `scale` | A preferred or house ratio, or `fit` (clause 7.3 and 7.5). |
| `rotation_deg` | How far the model turns on the paper, in degrees (clause 7.4). Default 0. |
| `scale_bar` | An optional scale bar (clause 7.9). |
| `north` | An optional north arrow (clause 7.10). |
| `entities` | The entities of the viewport (clause 9). The list may be empty. |

**7.2 Placement.**

**7.2.1** The width and the height of a frame MUST be more than zero. Error `7.2.1`.

**7.2.2** A frame MUST lie inside the drawing space (clause 6.3). Error `7.2.2`.

**7.2.3** Two frames MUST NOT overlap. Frames that only touch along an edge do not overlap. Error
`7.2.3`.

**7.2.4** A frame MUST NOT overlap a reserved region (clause 6.6). Error `7.2.4`.

**7.2.5** The tolerance of clause 5.7 applies to the tests in 7.2.2 to 7.2.4.

**7.3 Scale.** A scale is a ratio 1:N. N is a whole number of 1 or more. One metre of model space
is 1000 / N millimetres on paper. The value 1000 / N is called `mm_per_m`.

**7.3.1** The **preferred ratios** are those where N is one of:

> 1, 2, 5, 10, 20, 50, 100, 200, 500, 1000, 2000, 5000, 10000, 20000, 50000, 100000

*Basis: pending ISO check, ISO 5455 (clause to be confirmed), checklist P-07. ISO 5455 may have
been replaced by a later part. The check must find the part that now holds the list of scales. The
extension up to 1:100000 follows the rule that the list may be extended by powers of ten.*

**7.3.2** A scale that is not in the list of clause 7.3.1, not in the list of clause 7.3.4, and not
`fit`, is error `7.3.2`. This includes 1:300. Enlargement ratios such as 2:1 are not part of
version 1.0.
*Basis: house choice H-10. The standard has two short lists of scales, so every scale on a sheet can
be set on a scale rule.*

**7.3.3** A viewport has one scale. Both axes use it. A view with exaggerated height is not
possible in version 1.0.
*Basis: house choice H-11. A wrong exaggeration is worse than none.*

**7.3.4 House ratios.** The **house ratios** are those where N is one of:

> 25, 250, 1250, 2500

They are allowed in the same way as the preferred ratios, and give no warning.
*Basis: house choice H-37. A barge of 80 m must fit an A3 sheet at 1:250, and site plans use 1:1250
and 1:2500. These four ratios are not in the ISO list.*

**7.4 The map from model to paper.** Let the window be `xmin, ymin, xmax, ymax`, the frame be
`x, y, w, h`, the scale denominator be N and the rotation be theta.

1. The model centre is `cx = (xmin + xmax) / 2` and `cy = (ymin + ymax) / 2`.
2. The frame centre is `fx = x + w / 2` and `fy = y + h / 2`.
3. For a model point `(px, py)`, let `dx = px - cx` and `dy = py - cy`.
4. Turn the offset by theta, counter-clockwise, with +y up:
   `rx = dx cos(theta) - dy sin(theta)` and `ry = dx sin(theta) + dy cos(theta)`.
5. The paper point is `(fx + rx * k, fy - ry * k)`, where `k = 1000 / N`.

**7.4.1** The map acts on coordinates only. A line width, a dash length and a text height are
paper values. They are never multiplied by `k`.
*Basis: house choice H-12. A scale applied to the whole drawing would also scale every line width
by 1/N.*

**7.4.2** A rotation is taken modulo 360 degrees. A rotation of -10 degrees is 350 degrees.

**7.4.3** Worked numbers. Frame `20, 10, 390, 277` (centre 215, 148.5). Window `0, 0, 100, 50`
(centre 50, 25). Scale 1:500, so `k = 2`.

| Model point | Rotation | Offset (dx, dy) | Paper point |
|---|---|---|---|
| (100, 50) | 0 | (50, 25) | (315, 98.5) |
| (100, 50) | 90 | (50, 25) turns to (-25, 50) | (165, 48.5) |
| (50, 25) | any | (0, 0) | (215, 148.5) |
| (0, 0) | 0 | (-50, -25) | (115, 198.5) |

**7.4.4** Model +y maps to paper "up" (-y) when the rotation is 0. When the rotation is 90 degrees,
model +y maps to paper left.

**7.5 Fit.**

**7.5.1** The window MUST fit the frame at the scale. The window is turned by theta. Its
bounding box has the paper size:

- width: `(|w_m cos(theta)| + |h_m sin(theta)|) * k`
- height: `(|w_m sin(theta)| + |h_m cos(theta)|) * k`

where `w_m` and `h_m` are the window width and height in metres. The window **fits** when both
numbers are not larger than the frame width and height (clause 5.7). If it does not fit, that is
error `7.5.1`. The message MUST name the largest scale of clauses 7.3.1 and 7.3.4 that does fit, if there is one.

**7.5.2** Declared fit. When `scale` is `fit`, the scale is the first ratio, from 1:1 upward, in the
lists of clauses 7.3.1 and 7.3.4 taken together, at which the window fits. The first ratio is the
largest scale. If no ratio fits, that is error `7.5.2`. The resolved ratio, not the word `fit`, is the
scale of the viewport in every later rule and every output.

**7.5.3** Worked numbers. Frame 390 x 277 mm.

| Window (w_m x h_m) | Rotation | Scale | Paper size needed | Result |
|---|---|---|---|---|
| 200 x 100 | 0 | 1:500 | 400 x 200 mm | error 7.5.1; the message names 1:1000 |
| 200 x 100 | 0 | 1:1000 | 200 x 100 mm | fits |
| 200 x 100 | 0 | fit | | 1:1000 |
| 100 x 300 | 90 | 1:500 | 600 x 200 mm | error 7.5.1 |
| 100 x 300 | 90 | fit | 300 x 100 mm at 1:1000 | 1:1000 |
| 80 x 20 | 0 | 1:200 | 400 x 100 mm | error 7.5.1; the message names 1:250 |
| 80 x 20 | 0 | fit | 320 x 80 mm at 1:250 | 1:250 |
| 390 x 277 | 0 | 1:1000 | 390 x 277 mm | fits exactly |

**7.6 Window.**

**7.6.1** `xmax` MUST be more than `xmin`. `ymax` MUST be more than `ymin`. Error `7.6.1`.

**7.7 Entities stay in the frame.**

**7.7.1** After the map of clause 7.4, every point of every entity of a
viewport MUST lie inside the frame (clause 5.7). Error `7.7.1`.
*Basis: house choice H-13. A stray point is a loud error. It is never clipped without a word.*

**7.7.2** The test uses these points, after the map and before rounding:

- a `line`, `centre_line` or `polyline`: its points;
- a `circle`: the four extreme points, centre plus and minus the paper radius on each axis;
- an `arc`: its two end points, and each of the paper angles 0, 90, 180 and 270 degrees at which the
  arc has a point, as the extreme point at that angle;
- a `hatch`: the points of its boundary;
- a `text`: the four corners of its text box (clause 11.7.2).

**7.8 Scale and the sheet.** The viewport scale is the only scale a rule refers to. The title block
scale field is derived from the viewport scales (clause 14). It is never typed.

**7.9 Scale bar.** A viewport MAY have a scale bar. The bar is derived from the scale. Its length,
its ticks, its labels and its place are never typed. The only inputs are `length_m`, `divisions` and
`unit`, all optional.

**7.9.1 Constants.** All values are paper millimetres.

| Name | Value | Use |
|---|---|---|
| inset | 6 | distance from the frame edge to the bar block |
| bar height | 3 | height of the bar |
| tick length | 1.5 | the tick below the bar |
| tick label baseline | 5.0 | below the bottom of the bar |
| ratio baseline | 9.0 | below the bottom of the bar |
| text height | 2.5 | the tick labels and the ratio text |

*Basis: house choice H-14 for every row. ISO does not set the size of a scale bar. The text height
is a standard ISO 3098 height, pending ISO check, checklist P-08. The baselines leave 1 mm between
the tick and the label, and 1.5 mm between the two lines of text.*

**7.9.2 Place.** The bar is fixed to the **frame**, never to a model point. Its left end is at
`x + 6`. The bottom of the bar is `15` mm above the bottom of the frame. The top of the bar is
therefore 18 mm above the bottom of the frame. The ratio baseline is 6 mm above the bottom of the
frame.

**7.9.3 Available width.** The paper length of the bar MUST NOT be more than the frame width less
twice the inset: `available = w - 12`. A bar that touches the border has no clearance and cannot be
printed as drawn. If the bar is longer, that is error `7.9.3`. The message MUST give both the
required and the available length in millimetres.

**7.9.4 Length.** If `length_m` is given, the bar has that length, and the rule of 7.9.3 applies. If
it is not given, the length is the largest number in the series `{1, 2, 5} x 10^n` metres, with n a
whole number from -3 to 6, that meets both of these:

1. It is not more than the **target**. The target is the model length that is a quarter of the frame
   width on paper: `target = (w / 4) / k` metres.
2. Its paper length meets clause 7.9.3.

If no number meets both, that is error `7.9.4`.
*Basis: house choice H-15. A quarter of the frame keeps the bar small. The series 1, 2, 5 gives a
round length that can be read against a scale rule.*

**7.9.5 Division.** `divisions` is a whole number of 1 or more. The default is 5. The bar is split
into that many segments of equal paper length. The bar outline is a rectangle with the full
length and the bar height. A division line crosses the bar at each inner boundary. Segments 0, 2, 4
and so on (counting from the left) are solid black. The others are not filled.
*Basis: house choice H-14. Solid and empty segments read in black print.*

**7.9.6 Ticks and labels.** A tick is at every boundary, including both ends. A tick runs down from
the bottom of the bar by the tick length. Tick `i` runs from 0 to `divisions`. Its label is centred
on the tick, on the tick label baseline. The label text is the value `length * i / divisions`.

**7.9.7 Label text.** The value is in the bar unit (`m`, the default, or `km`). For `km`, divide the
value by 1000. Round it to 6 decimal places (clause 5.5). Write it with no trailing zeros and no
trailing decimal point. The last label has the unit after it, with one space. Below the bar, centred
on it, on the ratio baseline, the text is the scale, written `1:N`.

**7.9.8 The bar does not turn.** The rotation of the viewport never changes the bar. A map has one
scale in every direction.

**7.9.9 Worked numbers.** Frame width 390 mm. The available width is 378 mm.

| Scale | Target | Derived length | Bar on paper | Segments (5) | Labels |
|---|---|---|---|---|---|
| 1:100 | 9.75 m | 5 m | 50 mm | 10 mm | 0, 1, 2, 3, 4, 5 m |
| 1:200 | 19.5 m | 10 m | 50 mm | 10 mm | 0, 2, 4, 6, 8, 10 m |
| 1:500 | 48.75 m | 20 m | 40 mm | 8 mm | 0, 4, 8, 12, 16, 20 m |
| 1:1000 | 97.5 m | 50 m | 50 mm | 10 mm | 0, 10, 20, 30, 40, 50 m |
| 1:2000 | 195 m | 100 m | 50 mm | 10 mm | 0, 20, 40, 60, 80, 100 m |

The `m` follows the last label only. For a 2 m bar at 5 divisions the labels are 0, 0.4, 0.8, 1.2,
1.6 and 2 m. For a bar of 2000 m in km at 4 divisions they are 0, 0.5, 1, 1.5 and 2 km.

With `length_m` given, at 1:1000 in a 390 mm frame: 378 m needs 378 mm, which fits exactly. 379 m
needs 379 mm and is error 7.9.3. 390 m needs 390 mm and is error 7.9.3. The error for 390 m states
"required 390 mm, available 378 mm".

**7.9.10** A viewport that has a scale bar MUST have a frame at least 46 mm high (so that the bar and
the north arrow of clause 7.10 do not touch), and wide enough for clause 7.9.3. Error `7.9.10`.
*Basis: house choice H-16.*

**7.10 North arrow.** A viewport MAY have a north arrow. The arrow direction is derived. It is never
typed. The inputs are `model_bearing_deg` (default 0) and `label` (default `N`).

**7.10.1 The bearing input.** `model_bearing_deg` is the compass bearing, measured clockwise from
north, of the model +y axis. When it is 0, model +y points north.
*Basis: house choice H-17. The README of the earlier tool worded this the opposite way round. The
tool's own tests and its output follow the rule given here.*

**7.10.2 Direction.** The paper direction of north, measured clockwise from paper "up" (the
direction of -y), is:

`paper_deg = (-rotation_deg - model_bearing_deg) mod 360`

The result is in the range 0 up to, but not including, 360. It is rounded as clause 5.4 says.

**7.10.3** The standard does not turn grid north into true north. A projected grid can differ from
true north by a few degrees. The input `model_bearing_deg` states which north is meant.

**7.10.4 Worked numbers.**

| rotation_deg | model_bearing_deg | paper_deg | North points |
|---|---|---|---|
| 0 | 0 | 0 | up |
| 90 | 0 | 270 | left |
| 90 | 90 | 180 | down |
| 40.25 | 0 | 319.75 | up and a little left |
| 0 | 12.5 | 347.5 | up and a little left |
| -10 | 0 | 10 | up and a little right |

The row for 90 degrees agrees with clause 7.4.4. The viewport puts model +y at paper left, and the
arrow points left.

**7.10.5 Size and place.** The arrow is 18 mm tall. Its centre `(cx, cy)` is at `x + 10 + 9` and
`y + 10 + 9`, where `x, y` is the top-left corner of the frame. Drawn with north at the top, the
parts are:

- the **head**: a closed polygon with four points: the tip at `(cx, cy - 9)`, then
  `(cx - 4.14, cy + 5.76)`, then `(cx, cy + 0.9)`, then `(cx + 4.14, cy + 5.76)`;
- the **shaft**: a line from `(cx, cy + 0.9)` to `(cx, cy + 9)`;
- the **label**: text 3.5 mm high, centred at `(cx, cy - 11.5)` on its baseline.

Then the head, the shaft and the label position all turn clockwise by `paper_deg` about `(cx, cy)`.
The label text itself does not turn. It stays upright.
*Basis: house choice H-18 for the size, the inset and the proportions (the head width is 0.23 of the
height each side, the head base is 0.32 of the height below the centre, and the inner point is 0.05
of the height below it). The label height is a standard ISO 3098 height, pending ISO check,
checklist P-08.*

**7.10.6** Worked numbers. Frame `20, 10, 390, 277`. The centre is `(39, 29)`. With `paper_deg = 0` the
tip is at `(39, 20)`. With `paper_deg = 90` the tip turns to `(48, 29)`. With `paper_deg = 270` the
tip turns to `(30, 29)`.

**7.10.7** The frame MUST be at least 38 mm wide and 38 mm high, so that the arrow and its inset
fit. Error `7.10.7`.
*Basis: house choice H-16.*

**7.11** In version 1.0 the standard does not check whether the scale bar or the north arrow
overlaps the entities of the viewport. The author places the window so that they are clear.

## 8 Layers, line types, line groups and colour

The machine-readable form of this clause is `docs/standard/data/line-styles.json`. The text and the
file MUST agree. A test checks it.

**8.1 Line roles.** A line has one of three **roles**: `extra_thick`, `thick` or `thin`. A role is a
relative weight. It is not a width. The sheet turns a role into a width (clause 8.2).

**8.1.1** In one line group, the thick width is twice the thin width, and the extra thick width is
twice the thick width, each taken to the nearest value of the width series. The widths are given in
the table in clause 8.2. They are not computed.
*Basis: pending ISO check, ISO 128 (clause to be confirmed), checklist P-10.*

**8.2 Line groups.** A **line group** is a set of three widths, one for each role. The group is
named by its thick width. The sheet size sets the group. All widths are paper millimetres.

| Line group | Sheet sizes | Thin | Thick | Extra thick |
|---|---|---|---|---|
| 0.35 | A4, A3 | 0.18 | 0.35 | 0.7 |
| 0.5 | A2, A1 | 0.25 | 0.5 | 1.0 |
| 0.7 | A0 | 0.35 | 0.7 | 1.4 |

*Basis: the widths are from the ISO 128 series of line widths, pending ISO check (clause to be
confirmed), checklist P-09. The choice of group by sheet size is pending ISO check (clause to be
confirmed), checklist P-11.*

**8.2.1** The `sheet` object MAY name the line group in the member `line_group`. The value is the
group name: `0.35`, `0.5` or `0.7`. If it is missing, the group is the one in the table for the sheet
size.

**8.2.2** A `line_group` that is not the group of the sheet size is error `8.2.2`.
*Basis: house choice H-19. One rule gives one set of widths for one size. It keeps drawings on the
same size alike.*

**8.2.3** Worked numbers. Half of 0.35 is 0.175, which is not in the width series. The thin width
of the group is therefore the table value 0.18. This is why the table is used and the widths are not
computed.

**8.3 Line widths.** The width of a line is the width of its role in the line group of the sheet.
It is in paper millimetres. It is never scaled (clause 7.4.1).

**8.3.1 CAD line widths.** A CAD file can hold only a fixed set of line widths. The set, in
millimetres, is:

> 0.05, 0.09, 0.13, 0.15, 0.18, 0.20, 0.25, 0.30, 0.35, 0.40, 0.50, 0.53, 0.60, 0.70, 0.80, 0.90,
> 1.00, 1.06, 1.20, 1.40, 1.58, 2.00, 2.11

Every width in the table of clause 8.2 is in the set.
*Basis: house choice H-20. This is the set of line widths that the DXF format can store. It is a
format rule, not an ISO value.*

**8.3.2 Snap.** A width that does not come from the table of clause 8.2 (for example a width in a
symbol library) is **snapped** to the set of clause 8.3.1 before it enters the normalised content.
The rule is:

1. Round the width to 3 decimal places (clause 5.5). Write it as a whole number of micrometres.
2. If the result is 0, or more than 2110, that is error `8.3.2`.
3. Pick the member of the set that is nearest. Compare whole micrometres.
4. If two members are equally near, pick the **thinner** one.

*Basis: house choice H-20. The comparison is on whole numbers, so every implementation picks the
same member. A comparison on binary fractions can pick a different member at a tie.*

**8.3.3** Worked numbers.

| Width, mm | Micrometres | Nearest members | Result, mm | Why |
|---|---|---|---|---|
| 0.36 | 360 | 350 (10 away), 400 (40 away) | 0.35 | nearest |
| 0.12 | 120 | 90 (30 away), 130 (10 away) | 0.13 | nearest |
| 0.175 | 175 | 150 (25 away), 180 (5 away) | 0.18 | nearest |
| 0.55 | 550 | 530 (20 away), 600 (50 away) | 0.53 | nearest |
| 0.165 | 165 | 150 and 180, each 15 away | 0.15 | tie, so the thinner |
| 0.275 | 275 | 250 and 300, each 25 away | 0.25 | tie, so the thinner |
| 0.45 | 450 | 400 and 500, each 50 away | 0.40 | tie, so the thinner |
| 0.7 | 700 | 700 (0 away) | 0.7 | already in the set |
| 0.02 | 20 | 50 (30 away) | 0.05 | nearest |
| 2.2 | 2200 | none | error 8.3.2 | more than 2110 |

The width 0.275 mm is a tie that binary arithmetic can get wrong: 0.275 times 100 is
27.500000000000004 in 64-bit binary, which is nearer to 30 than to 25. Rule step 1 prevents this.

**8.3.4** A width in a CAD file is stored in hundredths of a millimetre. The width 0.35 mm is 35.

**8.4 Line types.** There are four line types. Each has an identifier, an ISO 128 type number, and
a pattern of dashes and gaps. A pattern is a list of numbers, in units of `d`, the width of the line
(clause 8.3). A positive number is a dash that is drawn. A negative number is a gap. The pattern
repeats along the line.

| Identifier | ISO 128 type | Pattern, in units of d |
|---|---|---|
| `continuous` | 01.1 | no pattern |
| `dashed` | 02.1 | 12, -3 |
| `chain` | 04.1 | 24, -3, 0.5, -3 |
| `chain_double_dashed` | 05.1 | 24, -3, 0.5, -3, 0.5, -3 |

*Basis: pending ISO check, ISO 128 (clause to be confirmed), checklist P-12.*

**8.4.1** The **length in millimetres** of one element is the number in the table times `d`, rounded
to 3 decimal places (clause 5.5). The pattern depends on the width of the line, so thin lines have
shorter dashes than thick lines.

**8.4.2** A pattern starts with a dash at the first point of the line. It does not change when the
sheet is scaled, because it is in paper millimetres (clause 7.4.1).

**8.4.3** The normalised content (clause 16) holds the identifier of the line type. It does not hold
the pattern. The pattern is a rule for the output step (clause 15).

**8.4.4** Worked numbers. All lengths are paper millimetres.

| Line type | d = 0.18 | d = 0.35 | d = 0.5 |
|---|---|---|---|
| `dashed` | 2.16, -0.54 | 4.2, -1.05 | 6, -1.5 |
| `chain` | 4.32, -0.54, 0.09, -0.54 | 8.4, -1.05, 0.175, -1.05 | 12, -1.5, 0.25, -1.5 |
| `chain_double_dashed` | 4.32, -0.54, 0.09, -0.54, 0.09, -0.54 | 8.4, -1.05, 0.175, -1.05, 0.175, -1.05 | 12, -1.5, 0.25, -1.5, 0.25, -1.5 |

**8.5 Layers.** Every entity is on one layer. A layer has a name, a role, a line type, a CAD colour
number and a use.

**8.5.1 Declaration.** The document lists the layers it uses in the member `layers`. An entity that
names a layer that is not listed is error `8.5.1`. The standard never creates a layer without a
declaration.
*Basis: house choice H-21. A drawing with a layer that nobody declared hides a mistake.*

**8.5.2 Standard layers.** A document declares a standard layer by its name alone. The role, the
line type and the colour then come from this table.

| Layer | Role | Line type | CAD colour | Use |
|---|---|---|---|---|
| OUTLINE | thick | continuous | 7 | Primary section or plan outline |
| OBJECT | thin | continuous | 8 | Secondary object lines |
| HIDDEN | thin | dashed | 244 | Edges hidden in this view |
| CENTRE | thin | chain | 6 | Centre lines and axes of symmetry |
| DIMENSIONS | thin | continuous | 3 | Dimension, extension and projection lines |
| ANNO | thin | continuous | 174 | Leaders, tags and callouts (not text) |
| TEXT | thin | continuous | 30 | Annotation text |
| HATCH | thin | continuous | 9 | Hatch and fill line work |
| GRID | thin | chain | 194 | Setting-out grid |
| TITLEBLOCK | thick | continuous | 54 | Border, title block, revision register and other sheet furniture |
| FDN | thick | continuous | 34 | Foundations, slabs and plinths |
| EQUIP | thick | continuous | 5 | Equipment outlines |
| NOZZLE | thin | continuous | 4 | Nozzles and connection points |
| PIPE | extra thick | continuous | 150 | Pipework |
| ELEC | thin | continuous | 214 | Electrical equipment and cable routes |
| WATER | thin | continuous | 134 | Water bodies, ponds and channels |
| ZONE | thin | dashed | 70 | Keep-clear and access zones (not physical) |
| ACCESS | thin | dashed | 94 | Access routes and hardstanding edges |

*Basis: house choice H-22. ISO sets no layer names. The names are those of the reference tool. The
roles follow clause 8.1. The line types follow clause 8.4.*

**8.5.3 Redeclaration.** A document that repeats a standard layer name and gives a role, a line type
or a colour that differs from the table is error `8.5.3`.

**8.5.4 Custom layers.** A document MAY declare a layer that is not in the table. It gives the name,
the role, the line type and the CAD colour number. The name MUST start with a capital letter A to Z
and MUST hold only capital letters A to Z, digits and the underscore, up to 31 characters in all.
Any other name is error `8.5.4`.
*Basis: house choice H-23. The rule keeps names valid in every CAD program, and the same in every
case.*

**8.5.5 Duplicates.** Two declarations of one layer name in one document are error `8.5.5`, even
when they are alike.

**8.5.6 Entity overrides.** An entity MAY name a role, or a line type, or both. The name then
replaces the one of its layer for that entity. This is how the border of the sheet is extra thick on
a layer whose role is thick. The entity never names a width.
*Basis: house choice H-24. Layers name a role, not a width. A role override keeps that rule.*

**8.6 Colour.**

**8.6.1 Print colour.** The SVG and the PDF are drawn in black (RGB 0, 0, 0), on the unprinted
white of the paper. There are no grey or coloured lines, text or fills in them. Fills, where this
standard asks for a solid fill, are black.
*Basis: house choice H-25. An ISO drawing is black on white. Layer colour is a CAD aid and not a
print colour.*

**8.6.2 CAD colour.** Each layer has one CAD colour number, in the range 1 to 255. The table of
clause 8.5.2 gives it. In a DXF, an entity takes the colour of its layer. No two standard layers
have the same number. The number 7 is shown by CAD programs as black or white, whichever is seen
against the background.
*Basis: house choice H-26. The numbers are 18 different numbers chosen so that the layers differ in
the common CAD palette. The standard makes no other claim about the hue.*

**8.6.3** A custom layer MUST NOT have the same colour number as any other layer declared in the
same document. Error `8.6.3`.

**8.7 What an entity gets.** For an entity on a declared layer, in this order:

1. The role is the role of the entity if it has one, else the default of its entity type (clause
   9.7), else the role of its layer.
2. The line type is the line type of the entity if it has one, else the default of its entity type
   (clause 9.7), else the line type of its layer.
3. The width is the width of the role in the line group of the sheet (clause 8.2), snapped if it is
   not from the table (clause 8.3.2).
4. A text entity has no line width. A hatch entity follows clause 10.4.

**8.7.1** Worked numbers. On an A3 sheet (group 0.35), a line on `OUTLINE` has width 0.35 and line
type `continuous`. A line on `HIDDEN` has width 0.18 and line type `dashed`, so its dash is 2.16 mm
and its gap is 0.54 mm. A line on `PIPE` has width 0.7. On an A0 sheet (group 0.7) the same three
lines have widths 0.7, 0.35 and 1.4. The sheet border (extra thick, clause 6.3.3) has width 0.7 on A3
and 1.4 on A0.

## 9 Geometry

This clause defines the entities of the core schema: line, polyline, arc, circle, centre line, and
text (clause 11). Hatches are in clause 10. Dimensions and leaders are in clause 12. Symbols are in
clause 13.

**9.1 Entities.** Each viewport holds a list `entities`. Every entity is an object with a member
`type`. The types are `line`, `polyline`, `arc`, `circle`, `centre_line`, `hatch`, `text`,
`dimension`, `leader` and `symbol`. Any other type is error `9.1`.

**9.1.1 Common members.** Every entity has `type` and `layer`. The layer name MUST be declared
(clause 8.5.1). An entity MAY have `id` (clause 9.8). Every entity except `text` MAY have `role`
(clause 8.5.6). Every entity except `text`, `hatch`, `dimension`, `leader` and `symbol` MAY have `line_type`.

**9.2 Points.** A point is a list of two numbers: x then y. A point in model space is in metres. The
member name ends in `_m`.

**9.3 Line.** A `line` has `from_m` and `to_m`, two points. The two points MUST differ. Error `9.3`.

**9.4 Polyline.** A `polyline` has `points_m`, a list of two or more points, and `closed`, true or
false. The default is false. A closed polyline joins its last point to its first.

**9.4.1** Two points that follow each other MUST differ. Error `9.4.1`. For a closed polyline, the
last point MUST also differ from the first.

**9.5 Arc.** An `arc` has `centre_m`, `radius_m`, `start_deg` and `end_deg`. The radius is more than
zero. The arc runs counter-clockwise (clause 5.3), in model space, from the start angle to the end
angle. Its sweep is `(end_deg - start_deg) mod 360`, in the range 0 up to, but not including, 360.

**9.5.1** A sweep of exactly 0 is error `9.5.1`. A full circle is a `circle`.

**9.5.2** Worked numbers. Start 0 and end 90: sweep 90. Start 270 and end 90: sweep 180, through
angle 0. Start 90 and end 0: sweep 270. Start 45 and end 405: sweep 0, which is error 9.5.1.

**9.6 Circle.** A `circle` has `centre_m` and `radius_m`. The radius is more than zero.

**9.7 Centre line.** A `centre_line` has `from_m` and `to_m`, like a line (clause 9.3). Its default
role is `thin` and its default line type is `chain`. The entity MAY name another role or line type.
The layer is still declared (clause 8.5.1), and is usually `CENTRE`. In the normalised content
(clause 16) a centre line is a `line`.
*Basis: house choice H-30. A centre line is a line with a conventional style. The layer says what it
belongs to. The type says how it looks.*

**9.8 Identifiers.** An `id` is 1 to 64 characters: a letter, then letters, digits, `_`, `-` or `.`.
Two entities, or an entity and a viewport, MUST NOT share an `id` in one document. Error `9.8`.

**9.9 Order.** The order of entities in the list has no effect on the normalised content.
*Basis: house choice H-31. Clause 16 sorts the entities.*

## 10 Hatches

This clause defines the hatch entity, the patterns, and the marks that each pattern draws. The
marks are part of the normalised content (clause 16).

**10.1 The hatch entity.** A `hatch` has `boundary_m`, a list of three or more points, and a
`pattern`. It MAY have `holes_m`, a list of lists of three or more points, and `angle_deg`
(default 0, a model angle). The boundary is closed: the last point joins the first.

**10.2 Patterns.** The pattern is one of: `solid`, `lines`, `concrete`, `rockfill`, `soil`, `water`.
Any other pattern is error `10.2`. The marks of each pattern are defined in clause 10.5.
*Basis: house choice H-32. The names are the materials the first examples need.*

**10.3 Validity.** A boundary and every hole MUST have a non-zero area, MUST NOT cross itself, and
MUST NOT repeat a point that follows another. Each hole MUST lie inside the boundary, and holes MUST
NOT overlap. A touch counts as a crossing or an overlap. Error `10.3`.

**10.4 Lines of a hatch.** The lines of a hatch are continuous. Their width is the width of the role
of the entity, or of its layer (clause 8.7). A hatch entity has no `line_type`.

**10.5 Marks.** The marks of a hatch are generated on the paper, after the map of clause 7.4. The
data of the patterns is `docs/standard/data/hatch-patterns.json`. The text and the file MUST agree.

**10.5.1 Unit.** Let `d` be the thin width of the line group of the sheet (clause 8.2). The **hatch
unit** is `u = 10 * d` mm. It is 1.8 mm for group 0.35, 2.5 mm for group 0.5 and 3.5 mm for group 0.7.
All spacings and sizes below are multiples of `u`.
*Basis: house choice H-40. The pattern follows the line group, so it looks alike on every sheet size.*

**10.5.2 Frame.** The marks are made from the normalised hatch entity: its boundary, its holes and
its `angle_deg`, all already rounded (clause 16.4). A reader can therefore make the marks from the
normalised content alone. Let `alpha` be the `angle_deg` of the normalised hatch (the model angle plus
the viewport rotation, clause 16.6.2). On the paper let `dir = (cos(alpha), -sin(alpha))` and
`nor = (sin(alpha), cos(alpha))`. For a point `p`, let `t = dir . p` (along) and `s = nor . p`
(across). The two axes are at right angles. The frame is fixed to the paper origin `(0, 0)`, not to
the boundary, so that two hatches of one pattern and one angle line up. Clause 5.6.1 applies to
sine and cosine. A point with the values `t` and `s` is `t * dir + s * nor`.
*Basis: house choice H-41. Fixing the pattern to the sheet makes the result the same in every
implementation.*

**10.5.3 The region.** The **region** of a hatch is its boundary with its holes cut out, by the
even-odd rule. A line `s = c` crosses an edge from `a` to `b` when `s(a) <= c < s(b)` or
`s(b) <= c < s(a)`. An edge with `s(a) = s(b)` is never crossed. A point is inside the region when a
ray from it in the direction of +`t` crosses an odd number of edges, by this same rule, counting
the edges of the boundary and of every hole.

**10.5.4 Line patterns.** The patterns `lines`, `soil` and `water` are **line patterns**. Each has a
step, a multiple of `u` (table in clause 10.5.7), and a line type. For every whole number `k` such
that `c = k * step` lies between the smallest and the largest `s` of the boundary points (both
included), do this:

1. Find the `t` value where the line `s = c` crosses each edge of the boundary and of the holes
   (clause 10.5.3). For an edge from `a` to `b`, it is
   `t(a) + (c - s(a)) / (s(b) - s(a)) * (t(b) - t(a))`.
2. Sort the `t` values, smallest first. Pair them: first with second, third with fourth, and so on.
3. Each pair `(t1, t2)` with `t2 > t1` gives a `line` entity from `t1 * dir + c * nor` to
   `t2 * dir + c * nor`.

**10.5.5 Lattice patterns.** The patterns `concrete` and `rockfill` are **lattice patterns**. The
lattice has a column step `A` and a row step `B`, multiples of `u`. The lattice point of row `r`
and column `q` (whole numbers, possibly negative) has `s = r * B` and `t = (q + o) * A`, where `o` is
0.5 when `r` is odd and 0 when `r` is even. Each lattice point has a **mark** chosen by the parity of
`r` (table in clause 10.5.7). A mark has a **clearance** `m * u`. The mark is drawn only when its
lattice point is inside the region (clause 10.5.3) and is at least `m * u` away from every edge of
the boundary and of every hole (the distance to the edge as a segment). Only lattice points whose
`s` and `t` lie within the range of the boundary points are tried.

**10.5.6 Mark shapes.** A `dot` is a `circle` entity with its centre at the lattice point and a
radius of `radius_u * u`. A `polygon` is a closed `polyline` entity. Each vertex `(x, y)` in units of
`u` is placed at `lattice point + x * u * dir + y * u * nor`.

**10.5.7 Table.** In this table the unit is `u`.

| Pattern | Kind | Step or lattice | Line type | Marks |
|---|---|---|---|---|
| `solid` | solid | none | none | The region is filled. No marks. |
| `lines` | lines | step 1 | `continuous` | none |
| `soil` | lines | step 1 | `dashed` | none |
| `water` | lines | step 2 | `continuous` | none |
| `concrete` | lattice | `A` 2, `B` 2 | `continuous` | even rows: a dot of radius 0.1, clearance 0.1. Odd rows: a triangle with vertices (-0.4, -0.3), (0.4, -0.3), (0, 0.4), clearance 0.5 |
| `rockfill` | lattice | `A` 3, `B` 2 | `continuous` | every row: a pentagon with vertices (-1, -0.5), (0, -0.8), (1, -0.4), (0.8, 0.6), (-0.6, 0.7), clearance 1.2 |

*Basis: house choice H-42. ISO sets no pattern for these materials in a form that can be computed. The
shapes are plain, and the clearances keep marks off the boundary.*

**10.5.8 Entities.** The marks of a hatch are entities of the normalised content: `line`, `circle`
and `polyline`. Each has the layer of the hatch, the origin `viewport.<id>.hatch`, the line type
of the pattern (`continuous` for lattice marks), and the width of clause 10.4. A hatch with the
pattern `solid` has no marks. A hatch with another pattern draws nothing but its marks: the boundary
is not drawn by the hatch.

**10.5.9 Worked numbers.** On A3 (group 0.35) `u = 1.8` mm. The pattern `lines` at angle 0 has `dir`
= (1, 0) and `nor` = (0, 1). It draws a horizontal line at every `y = k * 1.8`. For the boundary
(165, 168.5), (265, 168.5), (265, 188.5), (165, 188.5): the values of `k` are 94 to 104, because
94 * 1.8 = 169.2 is the first multiple of 1.8 that is not below 168.5, and 104 * 1.8 = 187.2 is the
last that is not above 188.5. The first line runs from (165, 169.2) to (265, 169.2). The pattern
`water` at angle 0 has step 3.6 and `k` from 47 to 52.

## 11 Lettering and font

This clause defines the text entity, the font, its width table and the text box.

**11.1 Style.** Text is drawn in upright ISO 3098 lettering of type B. In type B the line thickness
of a letter is one tenth of the letter height. Inclined lettering is not part of version 1.0.
*Basis: pending ISO check, ISO 3098 (clause to be confirmed), checklist P-13. Basis for upright
only: house choice H-27. One style keeps the text width table to one table.*

**11.2 Heights.** The **height** `h` of text is the height of a capital letter, in paper
millimetres. It is one of:

> 2.5, 3.5, 5, 7, 10, 14, 20

Any other height is an error `11.2`.
*Basis: pending ISO check, ISO 3098 (clause to be confirmed), checklist P-08.*

**11.3 The text entity.** A text entity has one line of text, an anchor point in model space, a
height (clause 11.2), a rotation, a horizontal alignment and a vertical alignment.

**11.3.1 Alignment.** The horizontal alignment is `left`, `centre` or `right`. The default is `left`.
The vertical alignment is `baseline`, `middle` or `top`. The default is `baseline`. The anchor is the
point of the text box that the alignment names. `middle` is at half the capital height above the
baseline. `top` is at the full capital height above the baseline.

**11.3.2 Rotation.** The rotation is in degrees on the paper, counter-clockwise from the paper
horizontal. The default is 0. It is rounded as clause 5.4 says. Text with rotation 0 reads left to
right on the sheet, whatever the rotation of the viewport.
*Basis: house choice H-28. A rotated map keeps its labels readable.*

**11.3.3** The text is one line. It holds no line break. It is not empty. Every character of it is in
the character set of clause 11.6.2. Any other text is error `11.3.3`.

**11.4 Scale.** The height of text is a paper value. The viewport scale never changes it
(clause 7.4.1).

**11.5 Containment.** The test of clause 7.7.1 uses the four corners of the text box (clause 11.7.2).

**11.6 Font.** The standard has one font.

**11.6.1** The font is **Barlow Semi Condensed Regular**, version 1.408, designed by Jeremy Tribby.
The file is `src/technical_drawings_for_agents/data/fonts/BarlowSemiCondensed-Regular.ttf`. Its
SHA-256 is `e8c7242f612b13d026e18a94b63b0fd079e91a3faf009d1b6981dbb5f16d1153`. Its licence is the SIL
Open Font License 1.1. The licence text is in `OFL.txt` in the same directory. The source and the
check of the licence text are in `SOURCE.txt` there. The licence allows embedding the font in a PDF
and shipping it with this tool. The font file permits embedding (its OS/2 `fsType` is 0).

**11.6.2 Character set.** Text may use these characters: U+0020 to U+007E, and `°` (U+00B0), `±`
(U+00B1), `×` (U+00D7), `µ` (U+00B5), `²` (U+00B2), `³` (U+00B3), `·` (U+00B7), `–` (U+2013) and
`Ø` (U+00D8). `Ø` is the diameter sign.

**11.6.3** The font is **close to ISO 3098 lettering. It is not certified as ISO 3098 lettering.**
*Basis: house choice H-38. The font has a uniform stroke of 0.101 of the capital height, where ISO
3098 type B has 0.1. Its capital widths are the nearest of the open fonts compared. The comparison is
in the checklist, part C. It uses the type B widths of checklist item P-14, which are pending ISO
check.*

**11.6.4** The font is embedded in every PDF. It is named in the DXF (clause 15).

**11.7 Width table and text box.**

**11.7.1 Width table.** The width table is `docs/standard/data/font-widths.json`. It gives the
**advance** of each character of clause 11.6.2, in font units (1000 to the em). The cap height of the
font is 700 font units. The width of a text of height `h` mm is the sum of the advances of its
characters, times `h`, divided by 700, rounded as clause 5.4 says. The font is used with no kerning,
no ligatures and no extra spacing.
*Basis: house choice H-39. A width that ignores kerning is the same in every implementation.*

**11.7.2 Text box.** The **text box** is a rectangle with the width `W` of clause 11.7.1 and the
height `h`. The box stands on the baseline: descenders are not part of it. Let the anchor be `(ax,
ay)` on the paper. Before rotation, the left edge is at `ax` for `left`, at `ax - W/2` for `centre`,
and at `ax - W` for `right`. The baseline is at `ay` for `baseline`, at `ay + h/2` for `middle`, and at
`ay + h` for `top`. The corners, in this order, are `(x0, y0)`, `(x0 + W, y0)`, `(x0 + W, y0 - h)` and
`(x0, y0 - h)`, where `x0` is the left edge and `y0` is the baseline.
*Basis: house choice H-29. The box is the capital box. It is simple, and it is the same in every
implementation.*


**11.7.3 Rotation.** The corners then turn counter-clockwise on the sheet by the rotation `rho` about
the anchor: for an offset `(dx, dy)` from the anchor the new offset is
`(dx cos(rho) + dy sin(rho), -dx sin(rho) + dy cos(rho))`. Clause 5.6.1 applies to sine and cosine.

**11.7.4** Worked numbers. The advances of `A` and `B` are 508 and 538 font units. The text `AB` at
height 7 mm has the width `(508 + 538) * 7 / 700 = 10.46` mm. At anchor `(100, 50)`, `left`,
`baseline`, rotation 0, the corners are `(100, 50)`, `(110.46, 50)`, `(110.46, 43)` and `(100, 43)`.
With `centre` the left edge is at 94.77. With `right` it is at 89.54. With `top` the baseline is at 57.

## 12 Dimensions and leaders

A **dimension** is an entity that measures geometry. Its text is **computed** from the geometry that it
points at. It is never typed. A **leader** names a part with a label.

**12.1 The dimension entity.** A `dimension` has `type`, `layer`, a `kind`, and the members of its kind
(clauses 12.3 to 12.5). It MAY have `id`, `role`, `unit`, `decimals`, `text_height_mm`, `chain` and
`chain_role`.

| Kind | Members | What it measures |
|---|---|---|
| `linear` | `orientation`, `from`, `to`, `offset_mm` | The distance between two points along a model axis |
| `aligned` | `from`, `to`, `offset_mm` | The distance between two points |
| `radius` | `of`, `direction_deg` (optional) | The radius of a circle or an arc |
| `diameter` | `of`, `direction_deg` (optional) | The diameter of a circle or an arc |
| `angle` | `vertex`, `from`, `to`, `arc_radius_mm` | The angle at a vertex between two rays |

**12.1.1 Unit and precision.** `unit` is `m` (the default) or `mm`. `decimals` is a whole number from 0
to 3. Its default is 2 for `m` and 0 for `mm`, and 1 for an angle. The text of a dimension is its value,
in the unit, rounded half-even (clause 5.5) to `decimals` places, written with exactly that number of
places. An angle is in degrees and its text ends with `°`. A radius has `R` before it. A diameter has
`Ø` before it. The text has no unit sign.
*Basis: house choice H-44. The unit is stated in the notes of a drawing, as ISO 129 allows (pending
ISO check, checklist P-15).*

**12.1.2 Text height.** `text_height_mm` is a height of clause 11.2. The default is 3.5.

**12.2 Points and references.** A **point spec** is an object. It is either `{"ref": id, "at": place}`
or `{"point_m": [x, y]}`. A `ref` names an entity of the same viewport. The places are:

| Entity | `at` |
|---|---|
| `line`, `centre_line` | `start`, `end` |
| `polyline` | `vertex:i`, where `i` is a whole number from 0 up to one less than the number of points |
| `circle` | `centre` |
| `arc` | `centre`, `start`, `end` (the points at the start and end angles) |

An unknown `id`, an entity of another type, or a place that the entity does not have, is error `12.2`.
The reference is to the model geometry, so the dimension follows the geometry when it is edited.

**12.2.1** Below, `P` means a point spec made into a model point, and `p` means that point on the paper
after the map of clause 7.4. The parts of a dimension are drawn on the paper. A dimension that measures
is not scaled: its text is the model value.

**12.3 Constants.** Let `u` be the hatch unit of the line group (clause 10.5.1). All lengths are paper
millimetres. The **arrow length** is `a = 2 * u`. The **arrow width** is `u / 2`. The **gap** between a
point and the start of its extension line is `u / 2`. An extension line goes `u` past the dimension
line. The text stands `u / 2` from the dimension line. The lines of a dimension are `continuous` and
have the width of the role of the entity, or of its layer (default `thin`).
*Basis: pending ISO check, ISO 129 (clause to be confirmed), checklist P-15 for the form of the
arrowhead, P-16 for the gap and the extension. House choice H-45 for the use of `u`: the dimension
follows the line group, as hatches do.*

**12.3.1 Side.** For a paper vector `D = (dx, dy)` let its **left normal** be `L = (dy, -dx)`. On the
sheet (+y down) this turns `D` counter-clockwise by 90 degrees. `D = (1, 0)` has `L = (0, -1)`, which
is up.

**12.3.2 Arrowhead.** An arrowhead with the tip `T` pointing along the unit vector `e` (from the base
towards the tip), and the left normal `n` of `e`, is the solid triangle `T`, `T - a*e + (u/4)*n`,
`T - a*e - (u/4)*n`. It is a `hatch` with the pattern `solid`.

**12.3.3 Reading direction.** For a paper vector at the visual angle `theta` (counter-clockwise from
the paper horizontal, in the range 0 up to 360), the **reading rotation** is `theta` when
`theta <= 90` or `theta > 270`, and `theta - 180` (taken modulo 360) otherwise. The text then never reads upside down. The
text of a dimension has this rotation, `centre` and `baseline` alignment, and its anchor is the middle of
the dimension line moved by `u / 2` along the up direction of the text, `(-sin(rho), -cos(rho))`, where
`rho` is the reading rotation.

**12.4 Linear and aligned.** For `linear`, the axis is the model x axis for `horizontal` and the model
y axis for `vertical`. The value is the distance of the two model points along that axis. For
`aligned`, the value is the model distance of the two points.

**12.4.1** A value of 0 is error `12.4.1`.

**12.4.2** Let `p1` and `p2` be the paper points, and `r` the rotation of the viewport. The paper
direction of the model x axis is `(cos(r), -sin(r))`. The paper direction of the model y axis is
`(-sin(r), -cos(r))`. The **direction** `D` of a `linear` dimension is the direction of its axis. The
direction `D` of an `aligned` dimension is the unit vector of `p2 - p1`. In both cases `D` is turned by
180 degrees if needed, so that `(p2 - p1) . D >= 0`. Let `L` be the left normal of `D`. The **reference
point** is `R = (p1 + p2) / 2 + offset_mm * L`. A positive `offset_mm` puts the dimension line on the left
of `D`. For each `i`, `q_i = p_i + ((R - p_i) . L) * L` is the foot of the point on the dimension line.
The entities are:

1. Two **extension lines**, one for each `i`. Let `s_i = (R - p_i) . L` and `v = sign(s_i) * L`. The line
   runs from `p_i + (u/2) * v` to `q_i + u * v`.
2. The **dimension line**, a `line` from `q1` to `q2`.
3. Two arrowheads (clause 12.3.2): tip `q1` with `e = -D`, and tip `q2` with `e = D`.
4. The text (clause 12.3.3), with the vector `D`.

**12.4.3** If `|s_i| <= u / 2` for an `i`, or `|q2 - q1| < 2 * a`, that is error `12.4.3`. (The extension
line would have no length, or the arrowheads would not fit.)

**12.5 Radius, diameter and angle.**

**12.5.1** `of` names a `circle` or an `arc`. Any other entity is error `12.2`. The optional
`direction_deg` is a model angle, counter-clockwise from the x axis. Its default is 45. For an `arc`, the
direction MUST lie within the sweep of the arc, ends included. Otherwise that is error `12.5.1`.
Let `c` be the paper centre, `R` the paper radius, `phi = direction_deg + r` and
`e = (cos(phi), -sin(phi))`. The value is the model radius (`radius`) or twice the model radius
(`diameter`).

**12.5.2 Radius.** The dimension line is a `line` from `c` to `c + R*e`. One arrowhead has its tip at
`c + R*e` and `e` as direction. The text has the text `R` and the value, and the vector `e`. If `R < 2*a`,
that is error `12.4.3`.

**12.5.3 Diameter.** The dimension line is a `line` from `c - R*e` to `c + R*e`. Two arrowheads have
their tips at the ends, with `-e` at `c - R*e` and `e` at `c + R*e`. The text has the text `Ø` and the
value, and the vector `e`. If `R < a`, that is error `12.4.3`.

**12.5.4 Angle.** `vertex`, `from` and `to` are point specs. Let `V`, `A` and `B` be the model points.
The rays are `V -> A` and `V -> B`. The model angles of the rays are `ta` and `tb`. The value is
`(tb - ta) mod 360` degrees, counter-clockwise from the first ray to the second. A zero ray (`A = V` or
`B = V`) or a value of 0 is error `12.5.4`. `arc_radius_mm` is the paper radius of the dimension arc. Let
`v` be the paper point of `V`, `alpha_a = ta + r` and `alpha_b = tb + r`. The entities are:

1. Two extension lines, one for each ray. For angle `alpha`, the line runs from
   `v + (u/2) * w` to `v + (arc_radius_mm + u) * w`, where `w = (cos(alpha), -sin(alpha))`.
2. The **dimension arc**, an `arc` with the centre `v`, the radius `arc_radius_mm`, the start angle
   `alpha_a` and the sweep equal to the value.
3. Two arrowheads at the ends of the arc. At the start, the tip is `v + arc_radius_mm * wa` and the
   direction `e` is `-tangent(alpha_a)`, where `tangent(alpha) = (-sin(alpha), -cos(alpha))`. At the end,
   the tip is `v + arc_radius_mm * wb` and `e` is `tangent(alpha_b)`.
4. The text `value°`, with rotation 0, `centre` and `middle` alignment, at the anchor
   `v + (arc_radius_mm + text_height_mm) * wm`, where `wm` is `w` at the angle `alpha_a + value / 2`.

If `arc_radius_mm < 2 * a`, that is error `12.4.3`.

**12.6 Leaders.** A `leader` has `from` (a point spec: the part that it names), `label_at_m` (a model
point), and `text` (one line of text, clause 11.3.3). It MAY have `text_height_mm`. Let `t` be the paper
point of `from` and `l` the paper point of `label_at_m`. The entities are:

1. A `line` from `l` to `t`.
2. An arrowhead with the tip `t` and the direction `e = unit(t - l)`.
3. A `text` with the text, rotation 0, `middle` vertical alignment, at the anchor `l` moved by
   `(u/2, 0)` with `left` alignment when `l.x >= t.x`, and by `(-u/2, 0)` with `right` alignment
   otherwise.

**12.6.1** A leader whose two points are equal is error `12.6.1`.

**12.7 Entities.** The parts of a dimension or a leader are entities of the normalised content: `line`,
`arc`, `hatch` and `text`. Their layer is the layer of the dimension. Their origin is
`viewport.<id>.dimension` or `viewport.<id>.leader`. Lines and arcs have the type `continuous`. A
line, an arc and an arrowhead have the width of the role of the entity, or of its layer (clause 8.7).
Every part of a dimension MUST lie inside the frame of its viewport (clause 7.7.1).

**12.8 Chains.** Dimensions with the same `chain` form a chain. Each has `chain_role`, `part` or
`overall`. A chain MUST have exactly one `overall`, and at least two `part`. Let `unit` and `decimals`
be those of each dimension: they MUST be the same in a chain. Compare the texts of the dimensions as
exact decimal numbers: the sum of the texts of the parts MUST equal the text of the overall. Any other
chain is error `12.8`.

**12.8.1** Worked numbers. Parts 1.25, 1.25 and 1.25, overall 3.75: equal, so valid. Parts of the
model lengths 1.254, 1.254 and 1.254 with `decimals` 2 have texts 1.25, 1.25 and 1.25. The overall length
3.762 has the text 3.76. The sum 3.75 is not 3.76, so that is error 12.8, even though the true sum is
3.762. The author then states the overall as the sum of the parts, or changes `decimals`.
*Basis: house choice H-46. A reader of the drawing adds the printed numbers. The numbers MUST agree.*


## 13 Symbols

A **symbol** is a piece of drawing that is made once and placed many times. Symbols are kept in
**libraries**. A drawing places a symbol with a `symbol` entity, which gives the place, the turn and the
size. The drawing stays short.

**13.1 Libraries.** A **library** is a set of symbols. It is either a **library file**, or it is written in
the document. The library file is JSON. Its schema is
`docs/standard/schema/symbol-library-1.0.schema.json`. A library that does not follow its schema, or
that breaks a rule of clause 13.2, is error `13.1`. A library file that cannot be read is also error
`13.1`.

**13.2 The symbol.** A library holds `symbols`, an object. Each member is the **identifier** of a symbol
(clause 9.8 form), and its value is a symbol with these members:

| Member | Meaning |
|---|---|
| `units` | `paper_mm` or `model_m` (clause 13.2.1). |
| `insertion` | The point `[x, y]` of the symbol that is put at the place of the insert. Default `[0, 0]`. |
| `geometry` | A list of parts: `line`, `polyline`, `arc`, `circle` and `hatch` (clause 13.2.2). |
| `ports` | Connection points: a list of `{id, at, direction_deg}` (clause 13.2.3). Optional. |
| `label_slot` | Where the label goes (clause 13.5). Optional. |

**13.2.1 Units.** In a symbol, every length has the unit that `units` names. Its names carry no unit
ending. This is the one place where a number does not name its unit (clause 4.5). A symbol with
`paper_mm` is drawn at the same size on the paper at every scale. A symbol with `model_m` is a real
size, and is drawn at the scale of the viewport. The axes are those of model space: +x to the right, +y
up (clause 5.1).
*Basis: house choice H-47. Equipment is a real size. A valve or a pump on a diagram is a sign.*

**13.2.2 Parts.** A part has the members of the entity of the same name (clause 9), with these names for
the numbers, and no `layer`, no `id`:

| Part | Members |
|---|---|
| `line` | `from`, `to` |
| `polyline` | `points`, `closed` |
| `arc` | `centre`, `radius`, `start_deg`, `end_deg` |
| `circle` | `centre`, `radius` |
| `hatch` | `boundary`, `pattern`. The pattern MUST be `solid`. No holes. |

A part MAY have `role` and, except for a `hatch`, `line_type`. A part with none takes the role and the
line type of the layer of the insert (clause 13.3.2). The rules of clauses 9.3 to 9.6 and 10.3 apply to
the parts. A symbol has at least one part.

**13.2.3 Ports.** A **port** has an `id` (unique in the symbol), a point `at` and a direction
`direction_deg` (a model angle, counter-clockwise from +x). A port is a place that other entities may
point at (clause 13.6). Its direction is for tools that connect lines, and does not change the drawing.

**13.3 Using symbols.**

**13.3.1 Libraries in the document.** The member `symbol_libraries` is a list. Each item has an `id`
and either `path` (the library file, a path that is relative to the folder of the document) or `symbols`
(a library written in the document). An item with both, or with neither, is error `13.3.1`. Two items
with the same `id` are error `13.3.1`.

**13.3.2 The insert.** A `symbol` entity has `type`, `layer`, `library` (an `id` of clause 13.3.1),
`symbol` (an identifier of the library), and `position_m` (a model point). It MAY have `id`,
`rotation_deg` (a model angle, counter-clockwise, default 0), `scale` (a number more than 0, default
1), `role` and `label`. An unknown library or symbol is error `13.3.2`. The layer MUST be declared
(clause 8.5.1). The role of a part with none is the `role` of the entity, if it has one, else the role of
the layer. The line type of a part with none is the line type of the layer.

**13.4 Place, turn and size.** Let `q` be a point of the symbol, `q0` its insertion point, `P` the
position, `rho` the rotation, `s` the scale and `N` the denominator of the scale of the viewport. The
**size factor** is `f = s` for `model_m`, and `f = s * N / 1000` for `paper_mm`. Let `d = (q - q0) * f`.
The model point is `P + (d.x cos(rho) - d.y sin(rho), d.x sin(rho) + d.y cos(rho))`. It is then mapped
to the paper by clause 7.4. A radius is multiplied by `f`. The start and the end angle of an arc have
`rho` added. The paper angle of an arc is as in clause 16.6.2.

**13.4.1** Worked numbers. A `paper_mm` symbol with the circle of radius 2 at `(0, 0)`, inserted at
`(5, 4)` with scale 1 in a viewport at 1:100, has `f = 100 / 1000 = 0.1` and a model radius of 0.2 m. On
the paper its radius is `0.2 * 1000 / 100 = 2` mm. The same symbol at 1:200 has the model radius 0.4 m
and the paper radius 2 mm. A `model_m` circle of radius 2 inserted at the same places has the paper radius
20 mm at 1:100 and 10 mm at 1:200.

**13.5 Label.** `label_slot` has `at` (a point), `align_h` (default `centre`), `align_v` (default
`middle`) and `height_mm` (a height of clause 11.2, default 3.5). When the insert has a `label`, the
label is a `text` entity with that text, the height, the alignments, the rotation 0 on the paper, and the
anchor at the paper point of `at` (by clause 13.4). The label text follows clause 11.3.3.

**13.5.1** A `label` on a symbol with no `label_slot` is error `13.5.1`.

**13.6 Ports as references.** The place `port:<id>` of a point spec (clause 12.2) is the point of the
port of the symbol that is placed by the insert with that `id`, by clause 13.4. A symbol insert has only
this place. An unknown port is error `12.2`.

**13.7 Entities.** The parts of a symbol insert, and its label, are entities of the normalised content:
`line`, `polyline`, `arc`, `circle`, `hatch` (solid) and `text`. Their layer is the layer of the insert,
their origin is `viewport.<id>.symbol`, and their widths follow clause 8.7. A `hatch` of a symbol has the
angle 0. Every one of them MUST lie
inside the frame of its viewport (clause 7.7.1).

**13.8 DXF.** A symbol is written to the DXF as a block, and an insert as an INSERT (clause 15.4.10).


## 14 Sheet furniture

This clause defines the title block, the revision register, the status and its status line, and the notes
and references strips. The border, the zone marks, the scale bar and the north arrow are in clauses 6.3 to
6.5, 7.9 and 7.10.

All of this furniture is **opt-in**. A document with no `title_block`, `revisions`, `status`, `strips`,
`notes` or `references` has none of these parts, and its normalised content is the same as before.

**14.1 Members.** The root members of this clause are:

| Member | Meaning |
|---|---|
| `title_block` | `template` (the identifier of a shipped template) or `template_path` (a template file), and `fields`, the given values (clause 14.4). |
| `revisions` | `scheme` (`alpha` or `numeric`, default `alpha`) and `entries`, oldest first (clause 14.7). |
| `status` | `draft`, `concept` or `issued` (clause 14.9). |
| `strips` | The strips for the notes and the references (clause 14.10). |
| `notes` | A list of notes (one line each). |
| `references` | A list of `{document_number, revision, title}`. |

**14.2 Title block template.** A **title block template** is a JSON file. Its schema is
`docs/standard/schema/title-block-template-1.0.schema.json`. It describes one title block, in paper
millimetres, with its origin at its top-left corner, +x to the right and +y down. It has:

| Member | Meaning |
|---|---|
| `id` | The identifier of the template. |
| `width_mm`, `height_mm` | The size of the block. |
| `pad_mm` | The space between a cell edge and its text. Default 1. |
| `lines` | Lines: `from_mm`, `to_mm` and `role` (clause 8.1). |
| `cells` | Cells of fields (clause 14.2.2). |
| `register` | The revision register (clause 14.7.1). |

A template that cannot be read, that does not follow its schema, that is named by an unknown identifier,
or whose `standard_version` has another major number, is error `14.2`.

**14.2.1** A template MUST also follow these rules, or that is error `14.2.1`: every line and every cell
lies inside the block; no two cells overlap; no two cells have the same `field`; a cell is high enough
for its label and its value, that is `label_height_mm + text_height_mm + 3 * pad_mm <= height_mm`; the
label of a cell fits its width (clause 14.6); the widths of the columns of the register add up to
`width_mm`; the row height of the register is at least `text_height_mm + 2 * pad_mm`.

**14.2.2 Cells.** A cell has `field` (a field of clause 14.4), `label`, `x_mm`, `y_mm`, `width_mm`,
`height_mm` and `text_height_mm` (a height of clause 11.2). It MAY have `label_height_mm` (default 2.5),
`align_h` (`left`, `centre` or `right`, default `left`) and `required` (true or false, default false).

**14.2.3** A template is the property of its owner. A template of the owner of a drawing MAY be a private
file. The standard ships one template (clause 14.3). A private template is passed by `template_path`,
and never has to be published.

**14.3 The ISO 7200 template.** The shipped template has the identifier `iso7200`. It is the file
`docs/standard/data/title-block-iso7200.json`. Its block is 180 mm wide and 56 mm high. Its cells are:

| Field | x | y | Width | Height | Text height | Required |
|---|---|---|---|---|---|---|
| `title` | 0 | 0 | 110 | 20 | 5 | yes |
| `supplementary_title` | 0 | 20 | 110 | 12 | 3.5 | no |
| `owner` | 0 | 32 | 110 | 12 | 3.5 | yes |
| `created_by` | 0 | 44 | 36 | 12 | 3.5 | no |
| `checked_by` | 36 | 44 | 37 | 12 | 3.5 | no |
| `approved_by` | 73 | 44 | 37 | 12 | 3.5 | no |
| `document_number` | 110 | 0 | 70 | 14 | 5 | yes |
| `revision` | 110 | 14 | 20 | 14 | 3.5 | no |
| `date_of_issue` | 130 | 14 | 50 | 14 | 3.5 | no |
| `status` | 110 | 28 | 35 | 14 | 3.5 | no |
| `scale` | 145 | 28 | 35 | 14 | 3.5 | no |
| `sheet` | 110 | 42 | 25 | 14 | 3.5 | no |
| `sheet_size` | 135 | 42 | 20 | 14 | 3.5 | no |
| `document_type` | 155 | 42 | 25 | 14 | 3.5 | no |

The lines are the outer rectangle and the line `x = 110` (role `thick`), and the other cell borders (role
`thin`). The register has 8 rows of 5 mm, text height 2.5, and the columns `rev` 12, `date` 24,
`description` 100, `by` 14, `chk` 14, `app` 16.
*Basis: pending ISO check, ISO 7200 (clause to be confirmed), checklist P-17 for the list of fields. House
choice H-48 for the layout and the sizes: ISO 7200 sets the data and not the layout of a block.*

**14.4 Content.** The content of a cell is one line of text. A field is **given** or **derived**.

**14.4.1 Given fields.** These are given in `title_block.fields`: `owner`, `title`,
`supplementary_title`, `document_number`, `document_type`, `language`, `department`, `sheet_number` and
`sheet_count` (whole numbers of 1 or more). A text value follows clause 11.3.3.

**14.4.2 Derived fields.** These are never given. They come from the document:

| Field | Value |
|---|---|
| `revision` | The `rev` of the last entry of `revisions`, or empty. |
| `date_of_issue` | The `date` of that entry. |
| `created_by` | The `by` of that entry. |
| `checked_by` | The `chk` of that entry, or empty. |
| `approved_by` | The `app` of that entry, or empty. It is **never** defaulted, and never taken from another entry or from another field. |
| `status` | The status in capitals, or empty. |
| `scale` | `1:N` when all viewports have the scale 1:N, else `AS SHOWN`. |
| `sheet` | `n/m`, from `sheet_number` and `sheet_count`, when both are given, else empty. |
| `sheet_size` | The size of the sheet, for example `A3`. |

*Basis: house choice H-49. A value that can be derived is not typed, so it cannot disagree. The
approver is the signature of a person: it comes only from the latest revision entry.*

**14.4.3** A cell with `required` true and an empty text, and a given field that the template has no
cell for, are error `14.4.3`. An empty cell draws no text.

**14.5 Placement.** Let `(x, y, w, h)` be the drawing space. The block of width `W` and height `H`
stands in its bottom-right corner: its top-left corner is `(x + w - W, y + h - H)`. The register stands
on top of the block, with the same width and the same left edge (clause 14.7.3). The strips are at the
edges of the **strip area** (clause 14.10). The block, the register and the strips are the **reserved
regions** of clause 6.6. A region MUST lie inside the drawing space, and two regions MUST NOT overlap
(clause 5.7). (A strip is cut from the strip area, so it never overlaps the block or the register.) Otherwise that is error `14.5`. A frame of a viewport MUST NOT overlap a region
(clause 7.2.4). The region of the status line (clause 14.9) is a region of this clause.

**14.6 Fit.** The text of a cell, of the register, and of a strip MUST fit. The width of a text is
that of clause 11.7.1. It fits a cell when its width is not more than the width of the cell less
`2 * pad_mm` (clause 5.7). A text is **never** cut, shortened or made smaller to fit. Text that does not
fit is error `14.6`.

**14.7 Revision register.** `revisions.entries` is a list with the oldest entry first. An entry has `rev`,
`date`, `description` and `by`. It MAY have `chk`, `app` and `seal`. The entry `app` is written by a
person. No tool supplies it.

**14.7.1 Register layout.** The template `register` has `row_height_mm`, `max_rows`,
`text_height_mm` and `columns`, a list of `{field, label, width_mm}` with the fields `rev`, `date`,
`description`, `by`, `chk` and `app`.

**14.7.2 Rows.** Let `n` be the number of entries and `m` the `max_rows`. When `n <= m` the rows are the
entries, oldest first. When `n > m` there are `m` rows: the first is the **overflow row**, then the newest
`m - 1` entries. In the overflow row, the `description` cell is `K EARLIER REVISIONS NOT SHOWN`, where `K`
is `n - (m - 1)`, and the other cells are empty. The register has one header row (the labels) above the
rows. The rows stand from top to bottom, so the newest entry is next to the title block.
*Basis: house choice H-50. A register that is cut is visible: nothing is dropped without a note.*

**14.7.3 Geometry.** The register has the width of the block. Its height is `(1 + rows) * row_height_mm`.
Its bottom edge is the top edge of the block.

**14.7.4** The `rev` of each entry has the form of the scheme (`alpha`: capital letters A to Z, for
example `A`, `B`, `AA`; `numeric`: whole numbers without a sign). The tokens are unique and rise from
entry to entry, in the order of the scheme (`A` < `B` < ... < `Z` < `AA`). Otherwise that is error `14.7.4`.

**14.7.5** Each `date` is a date of the form `YYYY-MM-DD` that exists. The dates never go back from one
entry to the next. Otherwise that is error `14.7.5`.

**14.7.6** An entry with a `seal` that is not the seal of clause 14.8 is error `14.7.6`. A sealed entry
cannot be changed, moved, deleted or put between others without that error.

**14.8 Seal.** The **seal** of entry `i` (counting from 0) is `sha256:` followed by the 64 lowercase
hexadecimal digits of the SHA-256 of the **canonical text** of the entry. The canonical text is the
JSON text, in UTF-8, of an object with the members `app`, `by`, `chk`, `date`, `description`, `index`,
`prior_digest` and `rev`, in this order (the order of their names), with no spaces, and with the
characters outside ASCII written as they are. `app` and `chk` are `null` when the entry has none.
`index` is `i`. `prior_digest` is the seal of entry `i - 1`, and the empty text for `i = 0`. The seal of
an entry is therefore a function of all the entries before it. A reader works out the seals from the
entries, and does not read the stored seals to do so.

**14.8.1** Worked numbers. A register of one entry: `rev` `A`, `date` `2026-01-05`, `description`
`FIRST ISSUE`, `by` `AB`. The canonical text is
`{"app":null,"by":"AB","chk":null,"date":"2026-01-05","description":"FIRST ISSUE","index":0,"prior_digest":"","rev":"A"}`.
Its SHA-256 (worked with `shasum -a 256`, not with this tool) is
`ed44c9a928dd581e3690279a741e60bb1f6e24bebd2cccc91e74350f6b4b08e2`, so the seal of entry 0 is
`sha256:ed44c9a928dd581e3690279a741e60bb1f6e24bebd2cccc91e74350f6b4b08e2`. A second entry: `rev` `B`,
`date` `2026-02-10`, `description` `ADDED SECTION B`, `by` `AB`, `chk` `CD`, `app` `EF`. Its canonical text is
`{"app":"EF","by":"AB","chk":"CD","date":"2026-02-10","description":"ADDED SECTION B","index":1,"prior_digest":"sha256:ed44c9a928dd581e3690279a741e60bb1f6e24bebd2cccc91e74350f6b4b08e2","rev":"B"}`.
Its seal is `sha256:9c7d9dbafc4b64d2a751dd8dbdc019805c4c0bea682349f5d1bb0ff3cb788404`.

**14.9 Status.** The member `status` is `draft`, `concept` or `issued`. A document with the status `draft` or `concept` has one **status line**: a
`text` of the height 2.5 (the normal text of clause 11.2), on the layer `TITLEBLOCK`, `left` and
`baseline`, with no rotation. Its text is `DRAFT – NOT FOR CONSTRUCTION` or
`CONCEPT – NOT FOR CONSTRUCTION`, with the en dash `–` (U+2013) between two spaces. A document with the
status `issued` has no status line. There is no mark across the drawing. The status line is the last
entity of the sheet furniture, and its origin is `sheet.status_line`. Its place is fixed:

- **With a strip for the notes** (clause 14.10): in that strip, at `(x + 2, y + h - 2)`, where `(x, y, w, h)`
  is the strip. The text MUST fit the width `w - 2 * 2` (clause 14.6), or that is error `14.6`. The last
  line of the strip is the status line, so the notes stop one pitch short of it (clause 14.10.1).
- **Otherwise**: just inside the border, above the bottom-left corner of the drawing space `(x, y, w, h)`,
  at `(x + 2, y + h - 2)`. The status line then has a **reserved region** of its own (clause 6.6),
  `(x, y + h - 6.5, t + 4, 6.5)`, where `t` is the width of the text (clause 11.7.1). The region follows
  clauses 14.5 and 7.2.4: it MUST lie inside the drawing space and overlap no other region (error `14.5`),
  and no frame of a viewport may overlap it (error `7.2.4`). With a title block the status cell of the
  block still shows the status (clause 14.4.2), and the status line is also drawn.

Because the line lies in a strip or in a reserved region, it never crosses a viewport, the text of a strip,
the title block or the revision register.
*Basis: house choice H-53. A mark across the drawing covers notes and dimensions. One small line states
the status and covers nothing. The size 2.5 mm is the normal text height, and the place is the same in
every implementation.*

**14.9.1 The gate.** A document with the status `issued` MUST have a `title_block`, and `revisions` with
at least one entry, and the last entry MUST have a non-empty `app` and a valid `seal`. Otherwise that
is error `14.9`. The gate only adds a requirement. No tool writes an `app` or a `seal` into a document.
*Basis: house choice H-51. A drawing is issued by a person who signs. A flag that an agent can set is not
a signature.*

**14.10 Strips.** `strips` is a list of `{id, edge, size_mm, content}`. `edge` is `left`, `right`, `top`
or `bottom`. `content` is `notes` or `references`. The **strip area** is the part of the drawing space above the top edge of the register (or of the
title block, when there is no register). With no title block it is the whole drawing space. Each strip is
cut off its edge of the free rectangle, in the order of the list, starting from the strip area. A strip that leaves no room, two strips with the same `id`
or the same `content`, or a `notes` or a `references` member without a strip of that content, is error
`14.10`.

**14.10.1 Content.** A strip has a heading (`NOTES` or `REFERENCES`, height 3.5) at `(x + 2, y + 2 + 3.5)`,
left, baseline. Item `i`, counting from 1, has height 2.5 and is at `(x + 2, y + 2 + 3.5 + 4.5 * i)`, left,
baseline. A note is `i. text`. A reference is the non-empty parts of `document_number`, `revision` and
`title`, joined by two spaces. Every item MUST fit (clause 14.6, with `2 * 2` in place of `2 * pad_mm`),
and the last baseline MUST be at most `y + h - 2`, or at most `y + h - 2 - 4.5` in a notes strip of a document with a status line (clause 14.9), and the heading too in that case. Otherwise that is error `14.6`.
*Basis: house choice H-52 for these sizes.*

**14.11 Entities.** The parts of the furniture are entities of the normalised content, on the layer
`TITLEBLOCK`:

- `sheet.title_block`: a `line` for each line of the template, with the width of its role (clause 8.2),
  `continuous`; for each cell a `text` for its label, at `(x + pad, y + pad + label_height)`, left,
  baseline, and a `text` for its value if it is not empty, at `(x + pad, y + h - pad)` (for `centre`
  `x + w / 2`, for `right` `x + w - pad`), baseline, with the alignment of the cell.
- `sheet.revision_register`: a closed `polyline` of its outer edge (thick), a `line` (thin) at each boundary
  of the rows, a `line` (thin) at each boundary of the columns, and a `text`, left and baseline, at
  `(column x + pad, row bottom - pad)` for each non-empty cell of the header and of the rows.
- `sheet.notes` and `sheet.references`: a closed `polyline` of the strip (thin) and the texts of clause
  14.10.1.
- `sheet.status_line`: the status line of clause 14.9.


## 15 Output mapping

**15.1 General.** One drawing document gives one normalised content (clause 16). The SVG, the PDF and
the DXF are all made from it. The DXF is made from the document and its normalised content together,
because model space holds the model coordinates of the document. An output that is made from a
document with an error is not written (clause 16.1.1).

**15.2 SVG.** One SVG user unit is one millimetre of paper. The root has the width and height of the
trimmed sheet in `mm` and a `viewBox` of the same size. Every entity of the normalised content is
drawn, in the order of the list, in black (clause 8.6.1), with its width and its dash pattern
(clause 8.4.1) in paper millimetres. A `text` is placed at the first corner of its text box
(clause 11.7), with the font size `h / 0.7` and the width of clause 11.7.1. A hatch with the pattern
`solid` is filled by the even-odd rule. A hatch with another pattern draws nothing, because its marks
are entities of their own (clause 10.5.8). The font MAY be embedded in the SVG.

**15.3 PDF.** The PDF has one page, of the size of the trimmed sheet. It is drawn in black. The font
of clause 11.6 is embedded (clause 11.6.4). A text is placed and rotated as in clause 15.2. Two builds
of one document give the same bytes.

**15.4 DXF.**

**15.4.1 Units.** Model space is in metres, with +y up. The header variable `$INSUNITS` is 6.
`$PSLTSCALE` is 1, so that a line pattern in a viewport keeps its paper size. Paper space is in
millimetres. The DXF paper space has +y up, so a paper point `(x, y)` of the normalised content is
written `(x, H - y)`, where `H` is the height of the sheet.

**15.4.2 Layers.** The DXF has a layer for every layer of the document and for `TITLEBLOCK`. It has one
more layer, `VIEWPORT`, which does not plot. Each layer has the colour number of clause 8.5.2, the line
weight of its role in the line group of the sheet (clause 8.3.4), and the line type of clause 15.4.3.
*Basis: house choice H-43. A viewport frame must not print.*

**15.4.3 Line types.** Every entity names its line type and its line weight, so that a role override
or a layer change does not move it. The line type `continuous` is `CONTINUOUS`. Any other line type
with the line width `w` mm is `TDFA_<TYPE>_<w in hundredths>`, for example `TDFA_DASHED_018`. Its
pattern is the pattern of clause 8.4.1, in millimetres.

**15.4.4 Model space.** The entities of every viewport are written in model space, in metres:
`line` and `centre_line` as LINE; `polyline` as LWPOLYLINE; `circle` as CIRCLE; `arc` as ARC, with the
start and end angles of the document; `hatch` as HATCH with its boundary and holes, filled solid. A
hatch whose pattern is not `solid` is written invisible (its marks are in paper space, clause 15.4.5). A
`text` is written as TEXT in the style `TDFA`, with the height `h * N / 1000` metres, so that it plots
at `h` mm in a viewport of the scale 1:N, and with the rotation `rho - rotation` degrees (clause 11.3.2).
The horizontal and vertical alignments are the DXF alignments of the same names.

**15.4.5 Paper space.** The layout is named `SHEET`. Its paper size is the trimmed sheet. It holds the
entities of the normalised content that are not entities of a viewport: the sheet furniture, the
furniture of each viewport, and the marks of the hatches. A `line` is a LINE, a `polyline` a
LWPOLYLINE, a `circle` a CIRCLE, a `hatch` a HATCH (solid), a `text` a TEXT.

**15.4.6 Viewports.** Each viewport of the document is a DXF VIEWPORT in paper space, on the layer
`VIEWPORT`. Its centre and size are those of its frame. Its view centre is the centre of the model
window. Its view height is `frame height * N / 1000` metres, so that it plots at 1:N. Its twist angle
is the rotation of the viewport in degrees.

**15.4.7 Style and font.** The text style `TDFA` names the font file
`BarlowSemiCondensed-Regular.ttf` (clause 11.6.1).

**15.4.8 Extended data.** Every entity and every viewport carries extended data of the application
`TDFA`, as strings `key=value`. Paper-space entities have `origin` (clause 16.5) and `type`.
Model-space entities have `vp` (the viewport) and `type`. Both may have `id`. A hatch has `pattern`
and `angle`. A viewport has `id` and `scale`.

**15.4.9 Overlapping windows.** The viewports of one DXF share one model space. If the model windows of
two viewports overlap, each viewport shows the content of the other in a CAD program. The writer MUST
report a warning that names clause 15.4.9. The drawing is still written.

**15.4.10 Symbols.** Each symbol that a document uses, with each layer that it is inserted on, is written
as a BLOCK named `TDFA_<library>_<symbol>@<layer>`. The block holds the parts, in the units of the symbol,
on the layer of the insert, with the line type and weight of each part, and has the insertion point as
its base point. Each insert is an INSERT in model space, with the position, the rotation, and the scale
`f` (clause 13.4) in both directions. The label is a TEXT entity, as clause 15.4.4 says, with the
extended data `part=symbol`. A symbol with `paper_mm` has `f = s * N / 1000`, so that it plots at its
size on the paper.

**15.4.11 Read back.** The normalised content can be made again from the DXF. A paper-space entity is
read as it is, with the paper point `(x, H - y)`. A model-space entity is mapped with clause 7.4 and
the view centre, scale and twist of the viewport named by its `vp`. A text height is `height * 1000 / N`.
The conformance of the DXF output is tested in this way, with a reader that does not use the library
that wrote the file.

## 16 Normalised content

**16.1 What it is.** The **normalised content** of a drawing document is a JSON object. It holds the
sheet, the viewports as resolved, and a sorted list of **entities** in paper millimetres. It is what
a conforming implementation produces from a document (clause 2.2). It is the thing that two
implementations compare. SVG, PDF and DXF are all made from it (clause 15).

**16.1.1** A document that has an error (clause 4.6) has no normalised content. Its result is the
list of errors.

**16.2 Members.** The normalised content has these members:

| Member | Content |
|---|---|
| `standard_version` | The version string of the document. |
| `sheet` | `size`, `orientation`, `line_group` (the group name, always present), `width_mm`, `height_mm` of the trimmed sheet, and `drawing_space` with `x_mm`, `y_mm`, `width_mm`, `height_mm`. |
| `viewports` | One object for each viewport, sorted by `id` (compared byte by byte). Each has `id`, `scale` (the resolved denominator, a whole number) and `frame` (`x_mm`, `y_mm`, `width_mm`, `height_mm`). |
| `entities` | The sorted list of entities (clause 16.8). |

**16.3 Entities: common members.** Every entity has `type`, `origin` and `layer`. It has `id` only if
the document gave one. The types are `line`, `polyline`, `arc`, `circle`, `hatch` and `text`. A
`dimension`, a `leader` and a `symbol` do not appear: they are written as these types (clauses 12.7
and 13.7).
`origin` says where the entity came from (clause 16.5).

**16.4 Entities: members by type.** All lengths are paper millimetres, rounded to 3 places. All
angles are degrees on the paper, rounded to 3 places, counter-clockwise as seen on the sheet, and in
the range 0 up to, but not including, 360. A rounded 360 is written 0. A point is a list `[x, y]`.

| Type | Members besides the common ones |
|---|---|
| `line` | `from_mm`, `to_mm`, `line_type`, `width_mm` |
| `polyline` | `points_mm`, `closed` (true or false), `line_type`, `width_mm` |
| `arc` | `centre_mm`, `radius_mm`, `start_deg`, `sweep_deg`, `line_type`, `width_mm`. The arc runs counter-clockwise on the paper from `start_deg` through `sweep_deg`. |
| `circle` | `centre_mm`, `radius_mm`, `line_type`, `width_mm` |
| `hatch` | `boundary_mm`, `holes_mm` (always present, a list that may be empty), `pattern`, `angle_deg`, `width_mm` |
| `text` | `anchor_mm`, `text`, `height_mm`, `rotation_deg`, `align_h`, `align_v` |

A `text` has no `line_type` and no `width_mm`. A `hatch` has no `line_type`. The width is the final
width of clause 8.7, in paper millimetres. The line type is an identifier of clause 8.4.

**16.5 Origins.** The `origin` of an entity is one of:

- `sheet.border`, `sheet.centring_mark`, `sheet.zone_mark`, `sheet.zone_label`;
- `sheet.title_block`, `sheet.revision_register`, `sheet.notes`, `sheet.references` and `sheet.status_line` (clause 14.11);
- `viewport.<id>`, for an entity written in the viewport `<id>`;
- `viewport.<id>.hatch`, for the marks of the hatches of that viewport (clause 10.5.8);
- `viewport.<id>.dimension` and `viewport.<id>.leader`, for the parts of the dimensions and leaders of that viewport (clause 12.7);
- `viewport.<id>.symbol`, for the parts and labels of the symbols of that viewport (clause 13.7);
- `viewport.<id>.scale_bar` and `viewport.<id>.north_arrow`, for the furniture of that viewport.

**16.6 How each source becomes entities.** Rounding (clause 5.5) is applied at the end, to each
number that is written. Nothing is rounded before.

**16.6.1 Sheet furniture.** All sheet furniture is on the layer `TITLEBLOCK`. That layer is declared
without the document declaring it. A document MAY also declare it. Let `W` and `H` be the width and
height of the trimmed sheet (clause 6.2), `L, T, R, B` the left, top, right and bottom insets (clause
6.3), and `g` the line group of the sheet (clause 8.2). All lines are `continuous`.

1. **Border.** One `polyline`, origin `sheet.border`, closed. The points are `(L, T)`, `(W-R, T)`,
   `(W-R, H-B)` and `(L, H-B)`, in that order. The width is the extra thick width of `g`.
2. **Centring marks.** Four `line` entities, origin `sheet.centring_mark`, width 0.5. They run from
   the trimmed edge inwards:
   `(W/2, 0)` to `(W/2, T+5)`; `(W/2, H)` to `(W/2, H-B-5)`; `(0, H/2)` to `(L+5, H/2)`;
   `(W, H/2)` to `(W-R-5, H/2)`.
3. **Zones.** Let `nh` be the number of zones along the horizontal sides and `nv` the number along
   the vertical sides (clause 6.5: in landscape `nh` is the long-side count, in portrait it is the
   short-side count). Let `zw = (W-L-R) / nh` and `zh = (H-T-B) / nv`.
4. **Zone marks.** `line` entities, origin `sheet.zone_mark`, width 0.35. For `k` from 1 to `nh - 1`,
   with `x = L + k * zw`: from `(x, 0)` to `(x, T)`, and from `(x, H)` to `(x, H-B)`. For `j` from 1
   to `nv - 1`, with `y = T + j * zh`: from `(0, y)` to `(L, y)`, and from `(W, y)` to `(W-R, y)`.
5. **Zone labels.** `text` entities, origin `sheet.zone_label`, height 3.5, rotation 0, `align_h`
   `centre`, `align_v` `middle`. For `k` from 1 to `nh`, the text is the number `k`, at
   `(L + (k - 0.5) * zw, T / 2)` and at `(L + (k - 0.5) * zw, H - B / 2)`. For `j` from 1 to `nv`,
   the text is the `j`-th letter of `ABCDEFGHJKLMNPQR`, at `(L / 2, T + (j - 0.5) * zh)` and at
   `(W - R / 2, T + (j - 0.5) * zh)`.

*Basis: house choice H-34. The standard of clause 6 gives the values. This clause fixes the exact
entities, so that two implementations write the same ones. The ends of each side need no zone mark,
because the border is there.*

**16.6.2 Viewport entities.** Each entity of a viewport is mapped with clause 7.4 (let `k` be the
paper millimetres per metre and `rotation` the viewport rotation). Its origin is `viewport.<id>`.
The layer is the layer of the entity. The role, line type and width follow clause 8.7.

- `line` and `centre_line` become a `line`, with the two points mapped. `from_mm` is the mapped
  `from_m`.
- `polyline` becomes a `polyline` with every point mapped, and the same `closed`.
- `circle`: the centre is mapped. `radius_mm` is `radius_m * k`.
- `arc`: the centre is mapped. `radius_mm` is `radius_m * k`. `start_deg` is
  `(start_deg + rotation) mod 360`. `sweep_deg` is the sweep of clause 9.5.
- `hatch`: the boundary and each hole are mapped. `angle_deg` is `(angle_deg + rotation) mod 360`.
  The width is the width of the role (clause 10.4). Its marks are generated as clause 10.5 says.
- `text`: the anchor is mapped. The height, the rotation (a paper angle, clause 11.3.2) and the
  alignments are copied. The rotation is taken modulo 360.

**16.6.3 Scale bar.** For a viewport with a scale bar (clause 7.9), let the frame be `x, y, w, h`,
the bar length `Lm` metres, the divisions `n`, `total = Lm * k`, `seg = total / n`, `x0 = x + 6`,
`y0 = y + h - 18` and `yb = y0 + 3`. The origin is `viewport.<id>.scale_bar`. The layer is
`TITLEBLOCK`. Every line is `continuous` and has the thin width of `g`. The entities are:

1. One closed `polyline`: `(x0, y0)`, `(x0+total, y0)`, `(x0+total, yb)`, `(x0, yb)`.
2. For `i` from 1 to `n - 1`: a `line` from `(x0+i*seg, y0)` to `(x0+i*seg, yb)`.
3. For each even `i` from 0 to `n - 1`: a `hatch`, pattern `solid`, angle 0, no holes, with the
   boundary `(x0+i*seg, y0)`, `(x0+(i+1)*seg, y0)`, `(x0+(i+1)*seg, yb)`, `(x0+i*seg, yb)`. The
   width is the thin width of `g`.
4. For `i` from 0 to `n`: a tick `line` from `(x0+i*seg, yb)` to `(x0+i*seg, yb+1.5)`.
5. For `i` from 0 to `n`: a `text`, the label of clause 7.9.7, at `(x0+i*seg, yb+5)`, height 2.5,
   rotation 0, `centre`, `baseline`.
6. One `text`, `1:N`, at `(x0+total/2, yb+9)`, height 2.5, rotation 0, `centre`, `baseline`.

*Basis: house choice H-35. The lines of the bar are thin so that they match the thin lines of the
drawing.*

**16.6.4 North arrow.** For a viewport with a north arrow (clause 7.10), let `c = (x+19, y+19)` and
`phi = paper_deg` (not rounded). **Turning** a point `(px, py)` about `c` by `phi` means:
`dx = px - cx`, `dy = py - cy`, then `(cx + dx*cos(phi) - dy*sin(phi), cy + dx*sin(phi) + dy*cos(phi))`.
This turns clockwise on the sheet. Clause 5.6.1 applies to the sine and cosine. The origin is
`viewport.<id>.north_arrow`. The layer is `TITLEBLOCK`. The entities are:

1. A `hatch`, pattern `solid`, angle 0, no holes, with the boundary `(cx, cy-9)`,
   `(cx-4.14, cy+5.76)`, `(cx, cy+0.9)`, `(cx+4.14, cy+5.76)`, each turned. The width is the thin
   width of `g`.
2. A `line` from `(cx, cy+0.9)` to `(cx, cy+9)`, both turned, `continuous`, thin width of `g`.
3. A `text` with the label, at `(cx, cy-11.5)` turned, height 3.5, rotation 0, `centre`,
   `baseline`. The text itself is not turned.

*Basis: house choice H-35.*

**16.7 Rounding.** A number is written after rounding to the places of clause 5.4. A length or angle
that rounds to a negative zero is written 0 (clause 5.5, step 5).

**16.8 Order.** The list `entities` is sorted by this key. Compare the parts in order. The first
part that differs decides.

1. `origin`, `type`, `layer`: strings, compared byte by byte on their UTF-8 form. A string that is
   the start of a longer one comes first.
2. `id`: a string, compared the same way. A missing `id` counts as the empty string.
3. `line_type`: a string, as above. A missing one counts as the empty string.
4. `width_mm`: a number. A missing one counts as 0.
5. The **geometry list** of the type, compared number by number, then by length (a shorter list that
   is the start of a longer one comes first):
   - `line`: `from_mm` x, y, then `to_mm` x, y.
   - `polyline`: every point's x, y in order, then 0 if open or 1 if closed.
   - `arc`: centre x, y, `radius_mm`, `start_deg`, `sweep_deg`.
   - `circle`: centre x, y, `radius_mm`.
   - `hatch`: every boundary point's x, y; then the number of holes; then every hole point's x, y
     in order; then `angle_deg`.
   - `text`: anchor x, y, `height_mm`, `rotation_deg`.
6. A final string: for a `text` the `text`; for a `hatch` the `pattern`; else the empty string.
   Then `align_h`, then `align_v` (empty for other types).

Entities that are equal in every part stay, as many as there are.
*Basis: house choice H-36. The key is a total order that needs no locale and no string form of a
number.*

**16.9 Numbers in files.** The members are JSON. Two files are equal when their parsed values are
equal. The spelling of a number is not compared. A whole number MAY be written with or without a
decimal point.

## 17 Conformance

**17.1 What the suite is.** The **conformance suite** is the directory `docs/standard/conformance/`.
It holds cases. A case is data. It needs no program in any language. An implementation proves that it
follows this standard by passing every case.

**17.2 Layout.**

| Path | Content |
|---|---|
| `suite.json` | The list of all cases: `id`, `kind`, `file` or `dir`, the clauses the case tests, and a one-line description. |
| `documents/<id>/input.json` | A drawing document. |
| `documents/<id>/expected.json` | The expected result (clause 17.4). |
| `units/<id>.json` | A list of unit checks (clause 17.5). |
| `result.schema.json` | The JSON Schema of a result and of an expected file. |

**17.3 The runner contract.** A **runner** is a program of the implementation. It is given the path
of one drawing document. It writes one result, as JSON, to its standard output. It MUST NOT read
anything except that file, `docs/standard/schema/drawing-document-1.0.schema.json`,
`docs/standard/schema/symbol-library-1.0.schema.json`, `docs/standard/data/line-styles.json`,
`docs/standard/data/font-widths.json`, `docs/standard/data/hatch-patterns.json` and the symbol library files that the document names (clause 13.3.1). It MUST NOT use the network or the clock. It MUST give the same
result each time.

- If the document has no errors, the result is
  `{"result": "ok", "normalised": <the normalised content of clause 16>}`.
- If it has errors, the result is `{"result": "error", "errors": [{"clause": "<clause number>"}, ...]}`.
  The list has one entry for each different clause number, sorted by clause number as text. For a
  schema error, the clause number is the `x-clause` value of the rule that failed (clause 18.6).

**17.4 The expected file.** An expected file is a JSON object.

- `result`: `ok` or `error`.
- For `error`: `errors`, a list of `{"clause": ...}`. The result of the runner MUST have the same set.
- For `ok`: any of the members `sheet`, `viewports` and `entities`, as in clause 16.2. The result
  MUST hold each member that the expected file has, with an equal value.
- For `ok`: an optional `origin_prefixes`, a list of strings. When it is present, the runner result
  is first reduced: only the entities whose `origin` starts with one of the strings are kept, in
  their order. The reduced list is compared with `entities`.
- Members that the expected file does not have are not compared.

**17.5 Unit checks.** A unit file is a JSON object with `function` (a name below) and `checks`, a
list of `{"id", "clause", "input", "expected"}`. A runner of unit checks reads the file, applies the
function to each `input`, and the result MUST equal `expected`. A result that is an error is written
`{"error": "<clause number>"}`.

| Function | Input members | Result |
|---|---|---|
| `round_half_even` | `value`, `places` | a number (clause 5.5) |
| `snap_width` | `width_mm` | a number, or an error `8.3.2` (clause 8.3.2) |
| `drawing_space` | `size`, `orientation` | `x_mm`, `y_mm`, `width_mm`, `height_mm` (clause 6.3) |
| `map_point` | `frame`, `window`, `scale`, `rotation_deg`, `point_m` | `[x_mm, y_mm]`, rounded (clause 7.4) |
| `window_fits` | `frame`, `window`, `scale`, `rotation_deg` | true or false (clause 7.5) |
| `fit_scale` | `frame`, `window`, `rotation_deg` | a denominator, or `null` if none fits (clause 7.5.2) |
| `bar_length` | `frame_width_mm`, `scale` | metres, or an error `7.9.4` (clause 7.9.4) |
| `bar_labels` | `length_m`, `divisions`, `unit` | a list of strings (clause 7.9.7) |
| `north_deg` | `rotation_deg`, `model_bearing_deg` | a number (clause 7.10.2) |
| `line_pattern` | `line_type`, `width_mm` | a list of numbers (clause 8.4.1) |
| `arc_sweep` | `start_deg`, `end_deg` | a number, or an error `9.5.1` (clause 9.5) |
| `text_width` | `text`, `height_mm` | the width in mm, rounded (clause 11.7.1) |
| `dimension_text` | `value_m`, `unit`, `decimals` | the text of a length dimension (clause 12.1.1) |
| `reading_rotation` | `theta_deg` | the rotation of dimension text, in 0 up to 360 (clause 12.3.3) |
| `chain_ok` | `parts`, `overall` (texts) | true or false (clause 12.8) |
| `symbol_point` | `units`, `insertion`, `position_m`, `rotation_deg`, `scale`, `denominator`, `q` | the model point `[x_m, y_m]`, rounded to 6 places (clause 13.4) |
| `revision_seals` | `entries` | the list of seals of clause 14.8 |
| `register_layout` | `entries`, `max_rows` | `{rows, omitted}`: the rows shown and the earlier entries not shown (clause 14.7.2) |
| `text_box` | `anchor_mm`, `text`, `height_mm`, `rotation_deg`, `align_h`, `align_v` | the four corners, each `[x_mm, y_mm]`, rounded (clauses 11.7.2 and 11.7.3) |

`frame`, `window` and the other objects have the members of the drawing document (clause 7.1).

**17.6 Rules for writing cases.**

1. The expected value of a case is written by hand, or worked out by one method and then checked by
   hand against another. It is **never** copied from the output of an implementation. If an
   implementation disagrees with a case, the case is checked first. A case changes only with a
   written reason.
2. An error case breaks exactly one rule.
3. A case avoids a value that lies within 1e-7 of a rounding tie, unless the value is exact in
   64-bit binary and the case is there to test the tie (clause 5.6).
4. A case holds no private data. Its numbers are made up.
5. Every clause that gives worked numbers has a case that uses them.

**17.7 Passing.** An implementation passes when it passes every case of `suite.json`. The suite
version is the version of the standard.

## 18 Versions and the schema

**18.1 Version.** The standard has a version of the form `MAJOR.MINOR`. A drawing document names the
version it follows in `standard_version`. A missing or wrongly formed value is error `18.1`.
This draft is version `1.0`.

**18.2 Reading rule.** A reader of version `A.B` MUST accept a document of version `A.C` when `C` is
not more than `B`. It MUST reject a document with another major number, or a larger minor number,
with error `18.2`. A change of the minor number only adds members or values. A change of the major
number may change meaning.

**18.3 The schema.** The JSON Schema of this version is the file
`docs/standard/schema/drawing-document-1.0.schema.json`. It follows JSON Schema draft 2020-12. The
standard and the schema have the same version. A change to one without the other is a defect.

**18.4 What the schema checks.** The schema checks the structure of a document: the members, their
types, their units (by name), the allowed values, and the unknown keys (clause 4.3).

**18.5 What the schema cannot check.** A reader MUST also check these rules, and report them with
the clause number: 4.4 (repeated keys); 6.1 to 6.6 (sheet and drawing space); 7.2 to 7.10 (viewports,
fit, furniture); 8.2.2, 8.3.2, 8.5.1, 8.5.3 to 8.5.5 and 8.6.3 (line group, widths, layers); 9.3 to
9.5 and 9.8 (geometry and identifiers); 10.3 (hatch validity); 12.2 to 12.8 (dimensions and
leaders); 18.2 (version reading).

**18.6 Annotation.** Each place in the schema that gives a rule has an `x-clause` annotation. It
names the clause of this standard that the place enforces. A reader that reports a schema error
SHOULD report the instance path, the rule that failed, and the `x-clause` value.
*Basis: house choice H-33. A user who reads a schema error can find the rule in plain words.*

**18.7 Members.** The schema defines every root member of clause 4.2, and `$schema`. A document that holds
any other root member has an unknown key (clause 4.3). The other schema files of this version are
`symbol-library-1.0.schema.json` (clause 13.1) and `title-block-template-1.0.schema.json` (clause 14.2).

**18.8 Numbers.** The schema states the unit of a number by the member name (clause 4.5). The schema
does not state precision. A reader rounds as clause 5 says.

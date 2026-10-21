"""The one embargo list, used by the release guard.

The repo is public, so the list must not publish the names it protects. It holds the SHA-256 of
each lowercased term instead. A term is one to three words; a multi-word term is its words joined
by one space ("north channel").

Matching: each line is lowercased and split into words (runs of a-z and 0-9; everything else,
including `_`, `-`, `.` and `/`, separates words). Every run of 1-3 consecutive words is hashed
and looked up. So matching is whole-word: an embargoed word inside a longer word is not a hit.

A hit is reported by its term id: the first 12 hex digits of the term's hash. Path rules (machine-
local paths) are not secret and are plain patterns.

Maintain the list from the private plaintext copy, which never enters the repo:

    python tools/embargo.py --hash < terms.txt     # prints the new TERM_HASHES body
    python tools/embargo.py --id "some term"       # the id the guard prints for a term

Standard library only.
"""
from __future__ import annotations

import hashlib
import re
import sys

MAX_WORDS = 3
ID_LEN = 12

TERM_HASHES = frozenset({
    "044107f7bb73598fa43d39f24faf463bdbf80abe45c74ed0cdfeb9a982d615f6",
    "074d16fa5c252378744584808b063adf480a2f5ca2d0d69860c3da91672580e3",
    "0a26f6b7ee15e2f149d16d022a1d9efa5240d096ba5c44f4d3f1210cc8ea8f52",
    "123c616e6b6b932ed34058711dd9aac12fdf897b791f139a7d7b8eba155eedaf",
    "15934f9d71aa63f18ceef8000aca86d534d7adf5b6eb7be066f6472357b3ab6b",
    "1f65896b357bf188f17eb9c90f118f4a8506bf0ed568fc182f92a012890225c4",
    "218ee6a4d0804794ee33f7d565c92ab389d53f7c893ec9fd06a263cc0f83c1a6",
    "23717b246edb53a4226c66a7dc36953cfa3cc7636af9ceba2d73d317d0074e71",
    "23ac3253b0cfa87c246fddbdd910f873692198d6bfaa4c1f5ae1133fa03fe035",
    "23ee1dde40578f36e1affb1151485b93fbf0474457ae3728fb055ba6c9cdf857",
    "2bcdcadeba2e4c3f8e8af00e9c94966d7180a4568c30c72733b2c257f1ea0e92",
    "2fa6522f8155681f770bba8992acfbf2dff07add42282526fdfeed1dc2d3cc01",
    "3347d0bdd97e6df866abddb2c3de496bbf1ad77d74745b688e70bc73bba31d8c",
    "376919680209f00db2464e22fc58d0862410cb043b1fe2d45ba9bd8d803a2f9e",
    "3be59856d7929f9f99a88a4bb26c7696bc71ba60e8c0e3bd55501f0514fdf148",
    "40dde54db87b2cbf6c5f1cfb91c6967056f44e9f60a6eb1050ff4b2a4c245af3",
    "43e74f2eb7c7633840b50940f48c560e2047801b4c5d14be424a6ae618d03934",
    "483a2540f0d53504744ccc66b8c99ba5e437921c3a23d32934ba33f186e153f0",
    "6248b9187f994056e0ce39a36567a12b5e60d8e64d1f82a7db8ccf4a4d361bbf",
    "7025acdab45004d56e4084bf1ae739bb851e029b79dfe497aefe78835854b77c",
    "74518b220b9a4146b7a9a0649cbb6e1afe45e5ffa8fe7ea26d72f544b1a1177f",
    "74dd17a579debfa10466572e52ac96bf8e704f93a158c8a1883eac6bb572bab9",
    "7a83cf6380133a968052be298a4a0a2f889f3696ffaf863b13d0799f0ceb0a37",
    "7ca005b2ce689dcd82af5202e378d1f6fea1e7a148a8c4e87eec907cd8163be8",
    "8137e03521a27c87f45971f7459250cf32b85dab738151d5947980f99d0dd99f",
    "8495c057c5cb49abecd2f72d5fd35cabced5b8d2731fbbf9181f1ea4453f87bc",
    "89aa35570f6ab9677e429e9aae80652004f26a4c7b2c4595e3a21f50290f796f",
    "8ce1249e693f6a1c59fc727e9389c1eb39d936bcec08f0e1eca84f619d5ea13f",
    "9dbe5a5decc4802c7f69782b527a99bba31b40543185a1da7b6225ece6c9fed6",
    "a0dd2580dbb3577da4ffce9fc0c1454ddc69db2357f13001c3b4a355d663abee",
    "a1b8536d7f98f6cb3c83f0abd1ff2c73eaf3e155eff6a78705f1d67a254d7fcb",
    "a587dbc0a2ea880db82e45b549a8f0564a42ebf901d87852b388cb91af77f5e6",
    "b5881603bab7a8556a1803cd05db2a69a7bd7be260fb107d0010449ed2123d3a",
    "b9329e2f61f1accd8db024af7f673f3a2b127c56f430f222ccf1985c9c08394d",
    "b9627fc2244cc9ed8a33dae1ee7fd4c4fe261a3a65464fa40fe4b9e0ae7bfe53",
    "bb29dd2f1c24e0fc17d7e579bc64e27bd09bc7715cb6e7dd1982afac2bd0abb6",
    "c2ebcbbc42847e0b0411af7334e49fbbaeafa7c58bd03d56ee944bd8f1cb675e",
    "c6c43dad9bb5474670afdf38c6ea916b173b68f014ef3435dbbbb92820fadf61",
    "c94b56e02b7e9b04e57fe718fa86026ad90ce209a0e231e5b55717f2671f1f0c",
    "cdafc6dd7eaf85cdd6560a701ab4b374055894ab904e4feb1901b88d6b59a847",
    "cf0c7caa1cc55779e7ef489de20ba3c79aceda4fb91dd6d4ad042b23f91d2f71",
    "e19ef6cf903ebce323462c0fc016c648f45d09dda554beb71a87a864b35e167a",
    "e3d040587cd096e5752fc30429e689c1200586e45b98c2027fb2ca6cb58ed69a",
    "e491c28d24694b437c8fc31c65531c6ca8fa3f98c0fb267b707181d6f2fa21c2",
    "eaa3265cf7a32305e984eeb981c06c912b4582439dfaa36bd05c4465ca981fd0",
    "eb3b61687299145d89541b57180d246647e85a7e400e19ced9010cef12e0383d",
    "f6018825e847b1c53c2784901db7d6af744418424d25190607dca3ee946c283f",
    "fe55b75dd9b2be54fae8571172bd3d70d65bc4ca9840c1bcda81d358299360ff",
    "ff31fe74a5dfc35e546a1ab31bf501471f1506fa5ee25dde3d5cbef683ec9dab",
})

# Path rules: not secret, so plain patterns. name -> pattern.
PATH_RULES = {
    "/" "Users/": re.compile(r"/[u]sers/", re.IGNORECASE),
}

_WORD = re.compile(r"[a-z0-9]+")


def term_hash(term: str) -> str:
    """SHA-256 of a term, normalised the way lines are: lowercase words joined by one space."""
    return hashlib.sha256(" ".join(_WORD.findall(term.lower())).encode()).hexdigest()


def term_id(term: str) -> str:
    return term_hash(term)[:ID_LEN]


def find(line: str, hashes: frozenset[str] | None = None) -> list[tuple[str, str]]:
    """Every embargo hit in one line, as (term id, matched text). Ordered, de-duplicated."""
    hashes = TERM_HASHES if hashes is None else hashes
    words = _WORD.findall(line.lower())
    out: list[tuple[str, str]] = []
    seen = set()
    for i in range(len(words)):
        for n in range(1, MAX_WORDS + 1):
            if i + n > len(words):
                break
            gram = " ".join(words[i:i + n])
            h = hashlib.sha256(gram.encode()).hexdigest()
            if h in hashes and h[:ID_LEN] not in seen:
                seen.add(h[:ID_LEN])
                out.append((h[:ID_LEN], gram))
    for name, pat in PATH_RULES.items():
        m = pat.search(line)
        if m and name not in seen:
            seen.add(name)
            out.append((name, m.group(0)))
    return out


def find_lines(lines, hashes: frozenset[str] | None = None) -> list[tuple[int, str, str]]:
    """Like `find`, but a term may wrap over line ends. `lines` is (line number, text) pairs.
    Words run on across lines whose numbers are consecutive; a gap in the numbers (a diff hunk
    boundary) breaks the run. A hit is reported at the line of its first word, as
    (line number, term id, matched words)."""
    hashes = TERM_HASHES if hashes is None else hashes
    out: list[tuple[int, str, str]] = []
    seen: set[tuple[int, str]] = set()

    def add(n: int, tid: str, gram: str) -> None:
        if (n, tid) not in seen:
            seen.add((n, tid))
            out.append((n, tid, gram))

    run: list[tuple[int, str]] = []             # (line number, word) since the last gap

    def flush() -> None:
        for i in range(len(run)):
            for k in range(1, MAX_WORDS + 1):
                if i + k > len(run):
                    break
                gram = " ".join(w for _, w in run[i:i + k])
                h = hashlib.sha256(gram.encode()).hexdigest()
                if h in hashes:
                    add(run[i][0], h[:ID_LEN], gram)
        run.clear()

    last = None
    for n, text in lines:
        if last is not None and n != last + 1:
            flush()
        last = n
        run.extend((n, w) for w in _WORD.findall(text.lower()))
        for name, pat in PATH_RULES.items():
            m = pat.search(text)
            if m:
                add(n, name, m.group(0))
    flush()
    return sorted(out, key=lambda t: t[0])


def contains(text: str, hashes: frozenset[str] | None = None) -> list[str]:
    """Sorted term ids found anywhere in a text (any line)."""
    return sorted({tid for line in text.splitlines() for tid, _ in find(line, hashes)})


def known_id(term: str) -> str | None:
    """Resolve an allowlist term: '*', a path-rule name, a term id (hex prefix), or the term
    itself. Returns the id, or None if it names nothing on the list."""
    t = term.strip()
    if t == "*":
        return "*"
    for name in PATH_RULES:
        if t.lower() == name.lower():
            return name
    if re.fullmatch(r"[0-9a-f]{%d,64}" % ID_LEN, t.lower()):
        pre = t.lower()
        if any(h.startswith(pre) for h in TERM_HASHES):
            return pre[:ID_LEN]
    h = term_hash(t)
    return h[:ID_LEN] if h in TERM_HASHES else None


def _main(argv: list[str]) -> int:
    if argv[:1] == ["--hash"]:
        terms = [ln.strip() for ln in sys.stdin if ln.strip() and not ln.lstrip().startswith("#")]
        for h in sorted({term_hash(t) for t in terms}):
            print(f'    "{h}",')
        return 0
    if argv[:1] == ["--id"] and len(argv) == 2:
        print(term_id(argv[1]))
        return 0
    print(__doc__)
    return 2


if __name__ == "__main__":
    sys.exit(_main(sys.argv[1:]))

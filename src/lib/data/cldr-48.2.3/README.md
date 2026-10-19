# Unicode CLDR English display names

These files are unchanged copies from the Unicode CLDR JSON **48.2.3** release.
They provide every English language, territory and script display-name entry;
they are label data, not a list of languages supported by DLsite.

Source: https://github.com/unicode-org/cldr-json/tree/48.2.3/cldr-json/cldr-localenames-full/main/en

License: the accompanying `LICENSE` is the unchanged Unicode License v3 from
https://github.com/unicode-org/cldr-json/blob/48.2.3/LICENSE. Settings includes
the full notice in its About view so the bundled application's data retains it.

| File | Entries | SHA-256 |
| --- | ---: | --- |
| languages.json | 693 | a5960d2371bc7d982da7fcfd9a0e237429d3467a5cc76f680bc3af5bcef9fb4e |
| territories.json | 316 | 158c1d575308f7e46912edbeda435c8fe2ef5dad280798231f3a432e406b1807 |
| scripts.json | 220 | 48ce7e6cf85ca719ec04850786e778bb93fb7c83067a417f56f02d2fa882448b |
| LICENSE | — | 220ba0e1c43b99530d2d5bdb892a99dca0989414f51ab695ecd90163eaa1ec3b |

To update, choose a stable release, replace all three original files and its
license, compare complete files/maps against that pinned source, then update
the version, hashes, counts and corresponding tests together. Do not curate
individual entries or fetch display-name data while the app is running.

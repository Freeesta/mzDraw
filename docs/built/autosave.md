# Autosave, built

the page writes the document to the browser's private file system (OPFS, file `mzdraw-autosave.json`) one second after the last change and when the tab is hidden. On the next visit a bar offers it (what it holds, Restore / Discard); what is worth offering is decided in Rust (`io::saved_summary`), and Restore opens the same bytes with an empty history (`Editor::open`). While the offer waits nothing is written. An empty canvas is not offered. Without OPFS there is no autosave and the page works as before. It is a page-only feature: the CLI reads and writes files.

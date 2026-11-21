# Fidelity reference `fidelity/1`, built (tests and tools only; no reader or writer changed)

`tests/data/fidelity/set.smi` is 200 synthetic structures (`tools/fidelity/gen.py`, fixed seed, standard library only). `tools/fidelity/run.py --replay <replay example>` writes each one in molfile V2000 and V3000, SDF, SMILES, KET, CDXML and CML with the native `replay` example, plus 20 drawn reactions as RXN; RDKit and Open Babel (a separate program, never linked) read them and the InChIKey of what they read goes next to the key of the input. The reverse direction (RDKit and Open Babel write molfile, SDF and SMILES, we read them) is in the same file. Output: `tests/data/fidelity/reference.json` (per structure and format `inchikey_in`, `inchikey_out_rdkit`, `inchikey_out_obabel`, `lost`, and a hash of our file; the program versions, the date and the success rates with a Wilson 95% interval). `crates/mzdraw/tests/fidelity.rs` needs no Python: it replays the 200 structures and the 20 reaction requests and fails, naming the structure and format, when any of our files is not byte-identical to the one the reference was made from (FNV-1a 64 hash). The table of rates is in `docs/mzdraw-formats.md` §1b.

Not built:
- `inchi-1` (the IUPAC binary) is used only when it is on PATH or in `INCHI_1`; it was not on the machine and was not fetched, so InChI 1.07.5 is **not measured** (RDKit's bundled 1.07.3 is the one used).
- RDKit and Open Babel have no KET reader (KET rows carry a hash only); RDKit has no CML reader.
- The reverse direction is not byte-checked by the Rust test (the files are written by the programs and are not committed): a reader change is seen only by rerunning the tool.
- Open Babel makes no InChI of a structure with an R group: those 14 are not compared with it.
- The public-corpus option (`--corpus`, PubChem ids listed in the script) prints rates only; nothing from it is committed.
- No reader or writer was fixed here: the failures are listed in the PR.

# Privacy Policy — FileZ

**Last updated: 22 September 2026 (version 1.0.0).**

FileZ is a file organiser for Windows. It watches folders you choose and moves new files
into category folders on your own computer. Because it works with your files, you deserve a
straight answer about what it does with them.

**FileZ has no network code. It cannot send anything anywhere.** There are no accounts, no
analytics, no crash reporting, no update checks. The app never opens a network connection.
Everything in this document concerns files kept on your own computer, which only you can read.

## What FileZ reads

- **The names of files** in the folders you asked it to watch (Downloads by default). Only the
  top level of each folder; it never looks inside subfolders.
- **The website a download came from.** Windows attaches a small hidden note (the
  "Zone.Identifier" stream) to every file a browser downloads, recording the address it came
  from. FileZ reads the site name from that note so a rule like "files from moodle go to
  University" can work. It does not read the file's contents, and it does not keep the address.
- **Whether a file is still in use** by another program, so it never moves a file that is
  being written or is open.

FileZ never opens or reads the contents of your files.

## What FileZ writes

All of it lives in `%APPDATA%\com.spacez.filez\` on your computer:

- `config.json` — your settings: watched folders, destination, rules, theme.
- `history.jsonl` — the list of moves it made (from, to, when), so **Put back** can reverse
  them. Up to 5,000 entries are kept.
- `ignored.json` — files you put back, so they are left alone afterwards.
- `empty-seen.json` — when each category folder was last seen empty, for the optional
  empty-folder cleanup.
- `tidy.log` — a plain-text log of moves and errors.

Delete that folder and FileZ forgets everything. Uninstalling the app leaves your files and
the category folders exactly where they are; nothing of yours is removed.

## What FileZ never does

- Delete a file. It moves files, and every move can be undone. The only thing it ever removes
  is an **empty** category folder that it created itself, after the number of days you chose,
  and you can turn that off.
- Touch files older than the date you chose in setup, if you picked "only tidy new files".
- Read file contents, upload anything, or talk to any server.

## Store version

The Microsoft Store build is the same program. The Store handles installation and updates;
FileZ itself still has no network code. Start-with-Windows in the Store version is managed
by Windows (Settings > Apps > Startup).

## Questions

Open an issue at <https://github.com/nur-arpon/FileZ/issues>.

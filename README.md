# lyric_viewer

displays the current synced lyric of mpd 

## Usage

### Requirements

- [MPD](https://musicpd.org)
- Store all [synced lyric files](https://en.wikipedia.org/wiki/LRC_\(file_format\)) (only basic supported as of yet) in a directory (default `$XDG_DATA_HOME/lyrics (~/.local/share/lyrics)`) as `Artist - Title.lrc`

### Options

#### Modes
- `--now` | `-n` => Show lyrics for this point in time and exit.
- `--stream` | `-f` => Show lyrics in a stream.
- `--help` | `-h` | `-?` => Print a help message.

#### Configuration
- `--url <url>` | `-u <url>` => Set the `url` for MPD (default: localhost:6600)
- `--dir <dir>` | `-d <dir>` => Set the directory `dir` to look for lyric files (default: $XDG_DATA_HOME/lyrics)
- `--blanklines` => Print blank lines (off by default)

*[lyric no longer looks like a real word](https://en.wikipedia.org/wiki/Semantic_satiation)*
%%%
title = "An Example Presentation"

[page_break]
type = "thematic_break"

[commands.initialize]
binary = "./scripts/handle"
arguments = ["initialize", "${presentation.path}", "${presentation.title}"]

[commands.update]
binary = "./scripts/handle"
arguments = ["update", "${presentation.path}", "${page.current}"]

[commands.finalize]
binary = "./scripts/handle"
arguments = ["finalize", "${presentation.path}"]
%%%

# Let's Make a Presentation!

This guide will show you an example of how to generate a multi-modal
presentation.

The presentation will run in a terminal as well as an external PDF viewer.

---

## Basic setup

To manage the external PDF display, we add hooks in the `initialize`, `update`
and `finalize` scripts:

```toml
[commands.initialize]
binary = "./handle"
arguments = ["initialize", "${presentation.path}"]

[commands.update]
binary = "./handle"
arguments = ["update", "${presentation.path}", "${page.current}"]

[commands.finalize]
binary = "./handle"
arguments = ["finalize", "${presentation.path}"]
```

---

## Prepeare the Presentation for _Pandoc_

_Rupert_ supports inline configuration delimited by `"%%%"`, but _pandoc_ does
not, so we first need to strip the configuration from the _markdown_ file:

```shell
awk '\
    BEGIN { meta = 0 } \
    { if (meta > 1) print } \
    /^%%%$/ { meta += 1 }'
```

This script relies on the fact that _rupert_ only considers configuration at
the start of the document.

---

## Manage Page Breaks

Once we have removed the configuration from the presentation, we must also
ensure that _pandoc_ inserts page breaks like _rupert_ does. The example script
found in `page-break.lua` transforms thematic breaks to page breaks:

```lua
--- Converts thematic breaks to page breaks.
function RawBlock (el)
    if el.text:match '^----*$' then
        return pandoc.RawBlock('tex', pagebreak.latex)
    else
        return nil
    end
end
```

---

## Generate the PDF

To finally generate the presentation, we use the following `Makefile`:

```makefile
.runtime/presentation.pdf: presentation.md
    @awk '\
        BEGIN { meta = 0 } \
        { if (meta > 1) print } \
        /^%%%$$/ { meta += 1 }' \
        < "$<" \
    | pandoc \
        --lua-filter=scripts/page-break.lua \
        --from gfm \
        --to beamer \
        --output="$@"
```

Note that you need to have _pandoc_ and several other components installed to
run these commands. On _Ubuntu_, the following packages are required:

* `pandoc`
* `texlive-latex-extra`
* `texlive-latex-recommended`

---

## Display the PDF

To finally display the presentation, we use the script `pdf-viewer` found in
this directory.

Note that you need to have _Python 3_ and several other components installed to
run this command. On _Ubuntu_, the following packages are required:

* `libpoppler-glib-dev`
* `python3-gi-cairo`

The PDF viewer listens to a named pipe that we can use to synchronise the page
displayed by _rupert_.

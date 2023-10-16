--- Converts thematic breaks to page breaks.
function RawBlock (el)
    if el.text:match '^----*$' then
        return pandoc.RawBlock('tex', pagebreak.latex)
    else
        return nil
    end
end

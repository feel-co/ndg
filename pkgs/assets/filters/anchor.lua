-- Adds anchor links to headings with IDs.
-- Based on the header anchor links Lua filter from Pandoc's website
-- https://github.com/jgm/pandoc-website/blob/160240324036ae620395836ff8092f724fb8f3f2/tools/anchor-links.lua
function Header(h)
    if h.identifier and h.identifier ~= '' then
        local anchor_link = pandoc.Link(
            '',                  -- empty content
            '#' .. h.identifier, -- href
            '',                  -- title
            { class = 'anchor', ['aria-hidden'] = 'true' }
        )
        h.content:insert(#h.content + 1, anchor_link)
    end
    return h
end

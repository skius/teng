- incremental game mechanics?

- renderer test if we keep prev display state and only render the positions that diff? with explicit moveto only 
where needed, eg at end of a contiguous (x-oriented) sequence of changed pixels

- default bg color for all pixels, then support fading animations by slowly fading fg color towards background color?

- important: on resize, we need to move any entities with custom collision handling
like the player! it will store a potentially too large y value and will try to index.
I suppose some simple y bounds checks should be fine here.
- incremental game mechanics?

- renderer test if we keep prev display state and only render the positions that diff? with explicit moveto only 
where needed, eg at end of a contiguous (x-oriented) sequence of changed pixels

- default bg color for all pixels, then support fading animations by slowly fading fg color towards background color?

- important: on resize, we need to move any entities with custom collision handling
like the player! it will store a potentially too large y value and will try to index.
I suppose some simple y bounds checks should be fine here.

- collision board new: for the world just store blocks by their world coords,
but for collision treat each corner of each pixel as a collision node,
that way a 1x1 pixel would have 4 points (x y, x+1 y, x y+1,x+1 y+1) but
each corner can be >= comparison instead of any +1 depending on left or right


I could turn this into a rogue lite? Rogue lites (or maybe -likes) have
strategy that can count as mastery, so I'm not promoting dark patterns which
is essentially what an incremental game would be all about... except
novelty, I think that could exist in incremental games. maybe that's what
my game could be about? incremental game where you unlock more and more things
that are novel? it still feels kind of dark pattern-y...
so, rogue like could be worth a try


need to fix the depths, have some way to override them, maybe a place to define some globals?






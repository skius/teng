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


ooh! could have the main player be an actual main character, and then upgrades
are placing 'recordings' of the player around the world, then it would be nice to find a big cliff
to start a recording and jump off. 
the recording would have a quick countdown like "starting recording in 3.. 2.. 1.." and then the
player becomes a different color (maybe by activating a different component by switching a phase?)
the slingshot would not be overpowered in that instance, if:
 - the player itself past some point (maybe the point at which you have enough money for the slingshot?)
   would not generate 'significant' money anymore, or maybe slingshots disable
   money generation until the player touches the ground again?
   could make the player a different color and invincible during slingshotting.
 - and if recordings can only be started while standing on the ground, and during
   a recording the player can only do actions that the recorded version can do as well.
BUT WHERE IS THE SKILL??
 - there could be some hurdles to overcome to reach the point of recordings in the first place,
   like falling meteors that kill you without giving you a lot of money or something
- to make different recordings be placed at different locations (instead of multiple recordings at the best location over and over)
  we could have different recording lengths per recording, so only long enough recordings can fall from the highest mountains, etc

Minimap of the world?
- scrolls (due to infinite world size)
- only shows the y range where the ground level can be
- halfblock display
- size dynamically adjusts in top right corner according to screen size
- toggleable with m

Add seed passing to commandline for worldgen.

TODO: if I want to really do rasterization, should use nalgebra and properly define scene structures.

Microsoft Terminal waits until it renders above 60fps: https://github.com/microsoft/terminal/blob/f28f65870a9caeb629498c83efc4ab6992c93bad/src/renderer/base/RenderEngineBase.cpp#L91
nope fixed in preview

TODO: test sixels again? https://github.com/zellij-org/sixel-image

ah let's not. sixels need color palettes

TODO: before release make sure it works on windows, so only run on ::Press .

Some kind of event recorder that records all events and timestamps that happen?
Then serializes them and allows replaying them for tests?
Would also need to store display size, since some events are relative to display size.
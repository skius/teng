This directory contains components that were removed from the game at some point. They are here for reference only.

Note that some old git commits may have even more sample components, unfortunately this archive system has
not existed back then.

`git log -p | grep "impl Component for"` is a good way to find old components.

or `git grep '<regex>' $(git rev-list --all)`

The commits might also contain more code related to the archived components, like how they
were instantiated, etc.
# teng 📟 
A minimal, cross-platform game engine for the terminal with a focus on performance

## Getting Started
teng uses components as the building blocks. Every frame, each component (optionally):
- Handles received events (mouse, keyboard, resizes, etc.)
- Updates the game state
- Renders its core concept (if any) to the screen

## Is teng an ECS?
Not really. teng's "Components" are quite similar to "Systems" in an ECS, but there is no built-in notion of entities or components in the ECS sense.
However, you can build an ECS inside teng quite easily, see [`examples/ecs`](examples/ecs/main.rs) for an example.
# teng 📟 
A minimal, cross-platform game engine for the terminal with a focus on performance

## Getting Started
teng uses components as the building blocks. Every frame, each component (optionally):
- Handles received events (mouse, keyboard, resizes, etc.)
- Updates the game state
- Renders its core concept (if any) to the screen

## Is teng an ECS?
No, not really. While teng's "Components" are like "Systems" in an ECS, the similarities end there.
You can build an ECS inside teng, but teng itself is not an ECS and does not have any concept of attaching components to entities.
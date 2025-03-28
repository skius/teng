# Useful resources:
WGSL struct alignments (needs Chrome): https://webgpufundamentals.org/webgpu/lessons/resources/wgsl-offset-computer.html#x=5d000001009707000000000000003d888b0237284ce7dce121b384fd72bd9a1ff9370b2b26abd86f8993068d7492e69e133cc27b4028edd4baaa7e98c3a8788233427d19b342b387620065eb6a2e5db9b619bf0d97606103b85ddf24e6f944c79f603e9506ee3037750155c4aaeef11ed8c9998346e5c57565f28b0c6b50459aacfa1c749eeb8c16abae86164950116b2bfa6155c8135fe56adaf5af26a558d268b048a172c1c1a503fff1cf722873c143eaa3c3484ed5f9309020e4b0f73695d5fe582e610cd36ec21239cd05d50165d1c17a04959bbfe533a210d72e5639e8996acb94c383043b3ae816c65f2354e006ed8e57b8b78d421853dd2c45802e843bfe6f33ba5a8a2476d170978f9e7d946b693291ad0b1adeed48b18f5bbf013299c992de91babbf2ed911482c56ed2f2c8992c212886e3a02228e2ff679e3d5c8a99d73b693b5f95e4bda21698669540e466166506e18229560aa5e9d50e04768353c53c16d8e03b36a145711f178aaa099fac7010b5e3748744b492741d2e2021c98e9179fab92b4801ed68d740be3cb48fd58064c8f3d5538e52802d6ec1b17d029288a54a2cf1f8064912988d420edf22f33854b75c4df3aaad7c1bd2a78f9ebdd299680798baf5055de6d7f3097992331ce786841c34aab0e4edb01a856928e18e1880f4bfe624e6e7e68bc32dbe9d9ac82705219b32b696068db62610b8b45ac9f04bf9c42fcb8c7533e223a7e5b6b09ff38532d4c0494ad4898a20754abd78c10f4c410b93bebd017985a740ff38e02ce7a3806427f6638b50e652f820e0306a2f62be17f1c36fdb6051c954a0d54ebb775100885633011213e91d94121b354050e3ede1adf151e886efefc50db7a61f0f81b18f72afffb3e01c664010b6c4bba4060c15b4faec0a80efcf731e3cd7d36027c4c3228fdc48361260cd90a657551cd67526f81259069c04c7664e265a3c4a49090acffd101876a6070f3c32766a0f01103afea066510ecba846487bc9380bc21374fa7121469cfff1d25924

"Learn WGPU": https://sotrh.github.io/learn-wgpu/

when using the perspective projection camera, clip_position coords get x y in [0, screen_width] and [0, screen_height] I think.
hmm. why does the tutorial say the camera computes normalized coordinates? 
TODO look into how coords get passed from vertex to fragment shaders

Binding groups and how they can be optimized across pipelines: https://developer.nvidia.com/vulkan-shader-resource-binding


# TODO:
- [x] Switch from the weird VertexAttr descs to the wgpu macro wgpu::vertex_attr_array!
  - Done. But needed to use const { }. Maybe look around for easier wgpu wrappers?
- [x] Use a depth buffer.
- [ ] Create an instance for a sprite incl width, height, position, (texture?)
  - [ ] allow defining a view into the source texture by offset(xy) and size(wh), which then computes tex coords.
  maybe by giving the instance a top-left and bottom-right uv coord, and then the vs shader
    can determine the correct uv coord by deciding which corner it is based on its model index.
- [ ] Make sprite position center based like in CPU version
- [ ] Use a single sprite texture atlas and run animations based on some passed 
frame index and then sampling the correct texture location.
  - [ ] For the frame index, I suppose it would make sense to put that into a separate buffer? if we put it into a separate buffer, we only need to update that if it has changed.
     otherwise we can just keep it. Same actually for size and tex atlas offset - those should be constant and we don't want to resend them
     So what do we need to resend? Position and frame index. Both are updated based on different conditions though...
    - And how do we even use the frame index? just offset to the right based on sprite size? that would be an option.
    - Just in general: I *think* it makes sense to split up buffers based on how frequently we need to change them? but it probably doesn't matter that much for our small simulation.
  - [ ] We have the atlas ability now. How do we actually generate the atlas though?
    - This seems to be enough for our purposes: https://umesh-kc.itch.io/free-online-texture-packer-alternative
  - [ ] Skip transparencies by just keeping track of where the original center of the sprite is. See image in notes app.
- [ ] Test normal maps!
  - Keep in mind the weirdness with normal maps and the fact that the normal map is in tangent space.
  - oh! WGSL has builtin dpdx/dpdy... could be useful!
  - Normal maps for sprites work now assuming we don't rotate the sprite.
- [ ] Need to add lights!
  - Have something that seems to work.
- [ ] It's probably really a good idea to have instances/sprites actually store their 'world' positions, maybe even with positive y,
  since when we move the camera, we would have to move _every instance_ in order to get the moving camera effect.
  Instead, if we compute the screen space coords _inside the shader_ this is much faster since it's parallel.
  - [ ] This should also make "zooming out" easier, since if we pass in a 'wrong' camera size that does not correspond to our screen,
    everything should just be handled for us.
- [ ] Restructure the code to look better. In particular, split up bind groups, have everything be typed correctly (looking especially at bytemuck::cast_slice arguments!!!!),
  and also pass in some general arguments to the shaders like frame count, time, etc.
- [ ] Figure out screen tearing that's happening when debug info is not showing.
  - For some weird reason, as soon as I press 'i' the screen tearing goes away. Maybe it's something about
    a buggy position being kept track in the DisplayRenderer? DebugInfo renders its first position very early since
    it basically always has a diff, so that could be a reason. Test this!
  - ok no. It seems to be HBD? when I just have a full block render thing moving around there's no screen tearing.
    but if I have (even something behind the full block block, so occluded) a hbd will cause screen tearing when it's moving around
    ??? no idea why.
- [ ] Post processing: Use rendered texture as input to new draw call where we just have one screen-sized quad and a fragment
  shader that does some cool effect?
  - For that pass, optimize using just one triangle: https://wallisc.github.io/rendering/2021/04/18/Fullscreen-Pass.html

# Ideas:
First pass: render everything to a texture.
Second pass: in the beginning, just display this texture over the entire screen
it should be entirely opaque that there is a second texture being rendered.
Then there should be a "wow" effect once we start moving around that
second texture by eg putting it on many triangles and simulating
some "screen shatter" effect.
Even more crazy would be if behind the screen shatter effect there is a proper 3d world being rendered.
Or maybe instead of screen shatter effect, just "zoom out" and you see some virtual 3d model computer screen (old CRT) (maybe this model: https://www.artstation.com/artwork/rRQKbm)
on which you can see the previous texture, but then you can now look around in the 3d world. Maybe behind you there's a
guy saying "Are you winning son?".
Or maybe there's just a flickering lamp somewhere. The screen would have its own illumination, maybe now it's also flickering a bit?
Maybe the way to enter the 3D world is to choose between a blue and red pill, and if you pick blue pill the screen kind of blurs a bit and you restart the game
If you pick the red pill however, everything goes black. Then, slowly, you're in the 3d world but at first only the CRT screen starts to light up.
Then the lamp flickers on and you fully see that you're in the 3d world.


maybe in 2d world theres a waiting different looking player somewhere and blocking a path or something, and the twist is 
that thats a separate Computer in 3d and you have to swap over there to unblock, metroidvania style (maybe you see the blockage early,
but cant do anything until later when u get red pill. the scene should be immediately recognizable on the 2nd screen so its an aha moment)
maybe in the beginning you can select "Your Character" (between different options, hair color, body types), and "Your Friend" - the other player that you see.
Hmm. Then the message would be something like "you don't have friends, you just play with yourself" which I think is kind of funny.

for "moving" in the 3d world with the computer screen example, I would maybe suggest having freedom of looking around, but to switch to the other computer
(which could be next to your computer, perhaps), you just press the right button (wasd in the direction of the screen) and you get LERPd to the other computer.

Hmm. what about after you play the other character and return to your screen, you suddenly see the other character moving around on its own
there's mulitple things to do with this, either something like Schachnovelle where you have a split personality and in truth you're playing
with yourself, i.e., if you were to take the red pill again you don't see anyone in front of the other screen.
the other way to approach this is to say okay now you found a friend after a while, or you reminisce about times where you did have a friend to play with,
and when you take the red pill again you do see someone sitting in front of the other screen.

if we do the option where there is no one sitting in front of the other screen (honestly sounds like the easier option from a modelling perspective)
then it could also have a bit of a horror vibe, where your friend in the game moves around on their own but "IRL" (in 3D) you don't see anyone
controlling the character, and if you switch to their monitor you also don't see the player moving anymore.
you could do things where if you "enter" 2D again as the friend maybe there is some "presence" trying to get back control over the
character, maybe some screenshake or a darkness creeping in from the edge, but im unsure how to proceed from there.


To first get a proper screen size, we could have an override component that disables everything else if a too small screen size is detected,
and only if the use says "yes" (or the screen resizes large enough), then the screen disappears and the game starts.

# More interesting terminal ideas:
How can we use the fact that we're inside a terminal? Right now, with half block pixels, it's really just a "worse" screen.
Running it in the terminal is good for:
- The wow effect
- Being able to run it over eg SSH
  - multiplayer could potentially be implemented by many people connecting over ssh, and inter-process-communication instead of network things.
- Depending on inputs, it can run on mobile termux (oh actually, wgpu probably makes this difficult)
- A simple run command, "hey do you want some time off from sysadmin? play this game right in the terminal"
- You can render 'text' on top of the pixels, but since normal pixel art games also use fonts that are higher-res than their sprites, this is not
that special.
- Perhaps the ability to run any other command inside the game somehow, but I don't really want to do that I think.
- Need a better way to do audio! it would be cool if frame rendering would not be blocked due to audio signals.
- Inputs are currently also a bit weird. would really love keyup/keydown events.
- 


# Texture Atlas Tool:
Consumes all sprites. Trims alpha on all images.
For strip animations, splits them up and trims alpha for each animation frame.
Stores strip animations in a separate json hash map.

Animations on CPU-side would then just be sprite atlas indices and the information about removed transparency.
but, instead of *just* passing frame index like it could be done with strips that have transparency, we have to pass
those things, so that's a downside. I.e., for every frame instead of updating a single index, we need to update atlas source, and transparency source info.
Can we perhaps have an intermediate buffer that the instance can look into? one that maps animation idx + frame idx to atlas source and transparency info?
That buffer would be constant, and an instance would also constantly hold the animation idx, and every frame we would again just need to update frame idx.
Yes, uniform buffers for up to 64KiB seems fine, if we need more, up to 128MiB, then storage buffer: https://webgpufundamentals.org/webgpu/lessons/webgpu-storage-buffers.html

Hmm, how to handle composite animations? We want to render them in a specific order, so do we just slightly adjust the z index and have different instances?
or do we try to do everything with one instance? if we did everything with one instance, our VertexOutput would have to be multiple uv coords that the fragment shader would have to overlay correctly.
In particular, even if an instance only uses one animation (nothing composited), we still need to have multiple uv coords since the same shader is run for every instance.

We might want to precompute composited animations and store those in the texture atlas instead? Then recompute every time we want to change the composition?
IMO it's fine for now to do compositing. This step would be done easiest in the texture atlas generator.
maybe a source json can indicate different composite groups?
{
"Characters_Human_HURT_composited": [
  "Characters_Human_HURT_base_hurt_strip8.png", ...
]
}
# Useful resources:
WGSL struct alignments (needs Chrome): https://webgpufundamentals.org/webgpu/lessons/resources/wgsl-offset-computer.html#x=5d000001009707000000000000003d888b0237284ce7dce121b384fd72bd9a1ff9370b2b26abd86f8993068d7492e69e133cc27b4028edd4baaa7e98c3a8788233427d19b342b387620065eb6a2e5db9b619bf0d97606103b85ddf24e6f944c79f603e9506ee3037750155c4aaeef11ed8c9998346e5c57565f28b0c6b50459aacfa1c749eeb8c16abae86164950116b2bfa6155c8135fe56adaf5af26a558d268b048a172c1c1a503fff1cf722873c143eaa3c3484ed5f9309020e4b0f73695d5fe582e610cd36ec21239cd05d50165d1c17a04959bbfe533a210d72e5639e8996acb94c383043b3ae816c65f2354e006ed8e57b8b78d421853dd2c45802e843bfe6f33ba5a8a2476d170978f9e7d946b693291ad0b1adeed48b18f5bbf013299c992de91babbf2ed911482c56ed2f2c8992c212886e3a02228e2ff679e3d5c8a99d73b693b5f95e4bda21698669540e466166506e18229560aa5e9d50e04768353c53c16d8e03b36a145711f178aaa099fac7010b5e3748744b492741d2e2021c98e9179fab92b4801ed68d740be3cb48fd58064c8f3d5538e52802d6ec1b17d029288a54a2cf1f8064912988d420edf22f33854b75c4df3aaad7c1bd2a78f9ebdd299680798baf5055de6d7f3097992331ce786841c34aab0e4edb01a856928e18e1880f4bfe624e6e7e68bc32dbe9d9ac82705219b32b696068db62610b8b45ac9f04bf9c42fcb8c7533e223a7e5b6b09ff38532d4c0494ad4898a20754abd78c10f4c410b93bebd017985a740ff38e02ce7a3806427f6638b50e652f820e0306a2f62be17f1c36fdb6051c954a0d54ebb775100885633011213e91d94121b354050e3ede1adf151e886efefc50db7a61f0f81b18f72afffb3e01c664010b6c4bba4060c15b4faec0a80efcf731e3cd7d36027c4c3228fdc48361260cd90a657551cd67526f81259069c04c7664e265a3c4a49090acffd101876a6070f3c32766a0f01103afea066510ecba846487bc9380bc21374fa7121469cfff1d25924

"Learn WGPU": https://sotrh.github.io/learn-wgpu/

when using the perspective projection camera, clip_position coords get x y in [0, screen_width] and [0, screen_height] I think.
hmm. why does the tutorial say the camera computes normalized coordinates? 
TODO look into how coords get passed from vertex to fragment shaders




# TODO:
- [x] Switch from the weird VertexAttr descs to the wgpu macro wgpu::vertex_attr_array!
  - Done. But needed to use const { }. Maybe look around for easier wgpu wrappers?
- [x] Use a depth buffer.
- [ ] Create an instance for a sprite incl width, height, position, (texture?)
  - [ ] allow defining a view into the source texture by offset(xy) and size(wh), which then computes tex coords.
  maybe by giving the instance a top-left and bottom-right uv coord, and then the vs shader
    can determine the correct uv coord by deciding which corner it is based on its model index.
- [ ] Use a single sprite texture atlas and run animations based on some passed 
frame index and then sampling the correct texture location.
- [ ] Test normal maps!
  - Keep in mind the weirdness with normal maps and the fact that the normal map is in tangent space.
- [ ] Need to add lights!

# Ideas:
First pass: render everything to a texture.
Second pass: in the beginning, just display this texture over the entire screen
it should be entirely opaque that there is a second texture being rendered.
Then there should be a "wow" effect once we start moving around that
second texture by eg putting it on many triangles and simulating
some "screen shatter" effect.
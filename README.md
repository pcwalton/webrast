# webrast

`webrast` is an experimental, high-performance, GPU-based rasterizer for common Web content. It is not a general-purpose vector graphics library. The code is written in the Rust language for reasons of security, performance, and embeddability.

Traditionally, Web browser engines have used immediate mode 2D vector graphics libraries for rasterization, such as Skia, Cairo, and Core Graphics/Quartz 2D. These APIs were designed for CPU rendering of arbitrary, SVG-like vector content and, for that setting, work quite well. However, when rasterizing ordinary Web content in GPU-based environment, it's potentially worth rethinking these assumptions. The idea behind `webrast` is to take a radically different approach to the traditional one to achieve better browser graphics performance.

This project is in very early research stages, and large changes to the design are probable.

## Design principles

* *Use a retained-mode display list instead of an immediate-mode API.* The most important aspect of rendering on the GPU is knowing which assets need to be retained and which assets can be discarded. The display lists in existing browser engines are a natural choice for this, but existing vector graphics libraries know nothing of the browser's display list and so have to guess via caching heuristics which resources to retain.

  For this to work, it is important for the display lists to contain enough high-level information to allow for effective incremental update. Regenerating the entire display list (or large parts of the display list) on each update nullifies the potential gains from this approach. Gecko's display list (which Servo adopts and refines) offers a proven method of incremental update, via display-list based invalidation (DLBI).

* *Focus on common Web content.* Existing vector graphics libraries are great at handling the full generality of (for example) canvas and SVG. There's no need to reinvent the wheel for rendering those. We want a high-performance engine focused on the most common Web content—say, the (non-SVG) CSS properties used by 5% or more of Web sites.

* *Batch aggressively.* It is very important, especially on mobile, to avoid excessive state changes and draw calls. We should not be dogmatic about this—state changes exist for a reason—but pathological cases such as changing state to draw one rectangle should be avoided at all costs, because these are likely to result in our vector rendering being CPU-bound. As a rule of thumb, we should strive for a half-dozen batches per page at most, especially since the higher-level tiled rendering in a browser compositor means that every batch we produce is likely to be repeated multiple times to handle each tile. It's quite possible that many pages can be fully drawn in *one* batch.
  
  At present, `webrast` uses a technique involving separate stencil functions to enable changing clipping state without issuing separate draw calls. Tricks like these to cut down on draw calls should be encouraged if they are observed to help performance.

* *Rasterize assets as early in the browser pipeline as possible.* We should pipeline requests so that assets like glyphs, border corners, and images are ideally already available on the GPU as soon as painting begins. Generally, which assets are likely to be needed can be determined as soon as style recalculation is complete; since layout and display list construction are often relatively slow (hundreds of milliseconds), this should allow for a good deal of pipelining.

* *Rasterize CSS features, not Bézier paths.* Outside of canvas, SVG, and little-used CSS features, the Web's vector graphics support is built on *text* and *borders*. Both of these are much more restricted than arbitrary Bézier paths and can be handled directly. Generalized tessellation and Bézier rasterization algorithms are not needed to handle most of the Web.

* *Rasterize assets in parallel.* We should use all hardware CPUs to perform all CPU rasterization of assets on a thread pool. In the future, we may be able to use Vulkan and similar APIs to insert drawing commands directly into command queues from these threads.

* *Allow smooth rerasterization at different resolutions without reuploading to the GPU.* The idea here is that, on mobile, during a pinch zoom we should present a sharp (non-blurry) picture and never reupload to the GPU while the user's fingers are down. We hope to aggressively use distance fields for text and borders for this.

* *Focus on OpenGL ES 2.0.* This gives us the most well-tested, broadly-supported graphics API stack.

* *Keep it simple.* When possible, do the obvious thing. Use nine-patch images instead of fancy shaders for blurs. Adjust texture and vertex coordinates instead of tricky clipping techniques where possible. Use plain old vertex coordinates for gradients. Keep shaders small and simple, and don't be afraid to hand-code them as opposed to using a full-blown shader combiner. Avoid general tessellation algorithms. And so forth.

  `webrast` should be a lean and mean, complementary fast path for Skia-GL/Ganesh, not a replacement for it in its full generality.

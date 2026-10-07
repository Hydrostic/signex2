# signex_resource

`ThreadedResourceServer` owns logical resource identities, resolved paths, decoded CPU data, and the last successful G00 cache. `ResourceHandle` owns one reference; moving it between threads does not change the reference count, and dropping it releases that reference. Repeated `open` calls give independent handles for the same current `ResourceRef`.

Only one server can run in a process. Constructing another while the first server or any of its handles keeps the worker alive returns `ResourceError::AlreadyRunning`. Once the last handle and server owner have been dropped and the workers have exited, a new server can start.

Create a `ResourceContext` with an absolute resource root, an ordered append list relative to that root, and the current append index. Relative roots are rejected. Names search from the current append onward. For each append, pictures try `.g00`, `.bmp`, `.png`, `.jpg`, then `.dds`. `open` stores the selected absolute path; `reload` creates a new revision, while `evict` discards a decoded record so `get_data` can use the last G00 cache or read its stored path again. The last G00 remains available through the single-entry cache until another G00 replaces it or its final handle is dropped.

Media names use the same append order. `Sound` searches `wav/` for WAV, NWA, OGG, and OWP; `Voice` derives WAV, NWA, and OVK paths from its numeric ID; `ObjectMovie` searches `mov/` for OMV; `SystemMovie` searches `mov/` for WMV, MPG, and AVI. The built-in decoder does not decode these media formats yet.

`Font` searches `dat/` beneath each append. Save thumbnails and ordinary thumbnails use numeric names padded to four and ten digits, respectively, and search image formats directly under the separate absolute `save_dir` supplied through `ResourceContext::with_save_dir`. `Other` uses its relative name under each append because it has no fixed Siglus directory rule.

`get_data` returns the sole shared `Arc<DecodedResource>` layer. A successfully delivered non-G00 resource is removed from the worker's decoded cache; a later request decodes the saved path again. Callers retain any `Arc` they already received. G00 data uses only the last-G00 cache.

The built-in `AssetDecoder` decodes G00 content with `signex_asset` for picture, atlas, and mask resources, regardless of the selected file's extension. Other content returns `DecodeFailed` with the resolved path. Hosts can pass a `ResourceDecoder` to `with_decoder` to support additional formats; decoded values can be returned through `DecodedResource::Other`.

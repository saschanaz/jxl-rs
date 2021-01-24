# kagamijxl

Opinionated JPEG XL decoder/encoder library.

## API

See the [documentation](https://docs.rs/kagamijxl).

### Easiest use

```rust
let result = kagamijxl::decode_memory(vec);
result.frames[0].data
```

### Advanced

```rust
let mut decoder = kagamijxl::Decoder::default();
decoder.need_color_profile = true;
let result = decoder.decode(vec);
(result.color_profile, result.frames[0].data)
```

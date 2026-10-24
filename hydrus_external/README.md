# hydrus_external

Rust port of `hydrus_python/hydrus/external`.

The crate contains standalone parsers and helpers that do not depend on a
client or server controller.

## Modules

| Module | Python source | Purpose |
| --- | --- | --- |
| `blurhash` | `blurhash.py` | BlurHash encoding/decoding and colour helpers |
| `hexagonitswfheader` | `hexagonitswfheader.py` | SWF header parsing |
| `logic_expression_query_parser` | `LogicExpressionQueryParser.py` | Boolean tag query parser |
| `system_predicate_parser` | `SystemPredicateParser.py` | `system:` predicate parser |

`blurhash` and `hexagonitswfheader` reuse `hydrus_core`; the parsers are native
Rust ports.

## Examples

### Boolean tag query

```rust
use hydrus_external::logic_expression_query_parser::parse_logic_expression_query;

let clauses = parse_logic_expression_query("(a || b) and not c")?;
// [{"a", "b"}, {"-c"}]
```

### System predicate

```rust
use hydrus_external::system_predicate_parser::{
    parse_system_predicate, Predicate, ValueData,
};

let predicate = parse_system_predicate("system:width >= 500 pixels")?;

assert_eq!(predicate.predicate, Predicate::Width);
assert_eq!(predicate.value, Some(ValueData::Natural(500)));
```

### BlurHash

```rust
use hydrus_external::blurhash::{blurhash_encode, blurhash_decode};

let hash = blurhash_encode(&image, 4, 4, false)?;
let pixels = blurhash_decode(&hash, 32, 32, 1.0, false)?;
```

### SWF header

```rust
use hydrus_external::hexagonitswfheader::parse_path;

let header = parse_path("movie.swf")?;
println!("{}x{}", header.width, header.height);
```

## Notes

- The Python `SystemPredicateParser` uses `dateparser` for arbitrary natural
  language dates. The Rust port supports explicit intervals (`3 days`,
  `1 hour 30 mins`), `YYYY-MM-DD` dates and `now`.
- Rating-advanced service names are returned in `ServiceSpecifier::service_names`
  because this library has no client service manager to resolve them to keys.
- All source files are kept below 200 lines.

## Tests

```text
cargo test -p hydrus_external
```

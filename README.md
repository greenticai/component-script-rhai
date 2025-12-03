# component-script-rhai

Rhai-driven Greentic component that executes user-provided scripts over a generic invocation envelope
(`config`, `msg`, `payload`, `state`, and `connections`) and returns a structured component result.

## Requirements

- Rust 1.89+
- `wasm32-wasip2` target (`rustup target add wasm32-wasip2`)

## Configuration

`schemas/component.schema.json` documents the config contract:

- `script` (string, required): Rhai source to execute.
- `result_mode` (`wrap` | `raw`, default `wrap`): Wrap script return value in `{output: ...}` or return it directly.
- `on_error` (`fail` | `continue`, default `fail`): Whether to fail the component on script error.

Scripts receive `msg`, `payload`, `state`, and `connections` in scope. Returning a map with `__greentic`
allows setting payload/control directly:

```rhai
return #{
    __greentic: {
        payload: #{ confirmed: true },
        out: ["next_node"],
        err: ["error_node"]
    }
};
```

State mutations persist even if the script errors.

## Running locally

```bash
cargo build --target wasm32-wasip2
cargo test
ci/local_check.sh
```

The manifest references the release artifact at
`target/wasm32-wasip2/release/component_script_rhai.wasm`. Update its hash via
`greentic-component inspect --json target/wasm32-wasip2/release/component_script_rhai.wasm`.

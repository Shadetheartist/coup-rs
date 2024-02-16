
### Compile release
choose whether to include debug symbols in `Cargo.toml`

`cargo build --release`

### Profile
compile, then

`valgrind --tool=callgrind --dump-instr=yes --collect-jumps=yes --simulate-cache=yes ./target/release/coup-rs`

### Benchmark
`cargo bench`

then open the results at 

`./target/criterion/complete game/report/index.html`

### Debug Print Actions
find the constant `PRINT_ACTIONS` and set it to true

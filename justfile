build-npm:
    cd front_end && npm run build

build-editor:
    cd editor && trunk build --public-url /editor/

run: 
    cargo run --bin back_end

run-npm: build-npm run

run-editor: build-editor run

full-run: build-npm build-editor run

editor:
    cd editor && cargo run --target x86_64-pc-windows-gnu --features native

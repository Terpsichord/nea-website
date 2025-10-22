build-npm:
    cd front_end && npm run build

build-editor:
    cd editor && trunk build --public-url /editor/

run: 
    cargo run --bin nea-website

full-run: build-npm build-editor run

editor:
    cd editor && cargo run --target x86_64-pc-windows-gnu

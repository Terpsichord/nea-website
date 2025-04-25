build-npm:
    cd front_end && npm run build

build-editor:
    cd editor && trunk build --public-url /editor/

run: build-npm build-editor
    cargo run --bin nea-website
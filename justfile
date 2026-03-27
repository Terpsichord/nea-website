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
    cd editor && cargo run --target x86_64-pc-windows-gnu --features native --release

docker-build:
    for lang in c cpp cs java js py rs sh ts; do \
        docker build -t nea/$lang -f back_end/languages/$lang/$lang.dockerfile back_end/languages/$lang & \
    done
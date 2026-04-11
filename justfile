build:
  cargo build --release -p nucld -p nuclctl -p nuclstart
install:
  sudo rm -rf /usr/local/bin/nucl*
  cargo build --release -p nucld -p nuclctl -p nuclstart
  sudo cp target/release/nucld /usr/local/bin/
  sudo cp target/release/nuclstart /usr/local/bin/
  sudo cp target/release/nuclctl /usr/local/bin/
build-debug:
  cargo build -p nucld -p nuclctl -p nuclstart
install-debug:
  sudo rm -rf /usr/local/bin/nucl*
  cargo build -p nucld -p nuclctl -p nuclstart
  sudo cp target/debug/nucld /usr/local/bin/
  sudo cp target/debug/nuclstart /usr/local/bin/
  sudo cp target/debug/nuclctl /usr/local/bin/
clean:
  cargo clean
uninstall:
  sudo rm -rf /usr/local/bin/nucl*
run:
 sudo nucld

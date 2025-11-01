# Maintainer: Your Name <your.email@example.com>

pkgname=igrok
pkgver=1.0.0
pkgrel=1
pkgdesc="Terminal-based music player with YouTube download and real-time audio visualization"
arch=('x86_64')
url="https://github.com/mrwildfoxgamer/Igrok"
license=('MIT')
depends=('yt-dlp' 'mpv' 'cava')
makedepends=('rust' 'cargo')
source=("$pkgname-$pkgver.tar.gz::$url/archive/v$pkgver.tar.gz")
sha256sums=('SKIP')

prepare() {
  cd "Igrok-$pkgver" # ✅ Changed from "$pkgname-$pkgver"
  export RUSTUP_TOOLCHAIN=stable
  cargo fetch --locked --target "$CARCH-unknown-linux-gnu"
}

build() {
  cd "Igrok-$pkgver" # ✅ Changed
  export RUSTUP_TOOLCHAIN=stable
  export CARGO_TARGET_DIR=target
  cargo build --frozen --release --all-features
}

check() {
  cd "Igrok-$pkgver" # ✅ Changed
  export RUSTUP_TOOLCHAIN=stable
  cargo test --frozen --all-features
}

package() {
  cd "Igrok-$pkgver" # ✅ Changed
  install -Dm755 "target/release/$pkgname" "$pkgdir/usr/bin/$pkgname"
  install -Dm644 LICENSE "$pkgdir/usr/share/licenses/$pkgname/LICENSE"
  install -Dm644 README.md "$pkgdir/usr/share/doc/$pkgname/README.md"
}

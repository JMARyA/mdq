# Maintainer: JMARyA <jmarya@hydrar.de>
pkgname=mdq
pkgver=main
pkgrel=1
pkgdesc="query markdown documents which have yaml frontmatter"
arch=('x86_64' 'aarch64')
url="https://git.hydrar.de/mdtools/mdq"
license=("MIT")
depends=()
makedepends=("rustup" "git")
source=("${pkgname}::git+https://git.hydrar.de/mdtools/mdq.git")
sha256sums=("SKIP")

pkgver() {
    cd "$srcdir/$pkgname"
   	echo "$(date +%Y.%m.%d)_$(git rev-parse --short HEAD)"
}

prepare() {
    cd "$srcdir/$pkgname"
    rustup default nightly
    cargo fetch
}

build() {
    cd "$srcdir/$pkgname"
    cargo build --release
}

check() {
    cd "$srcdir/$pkgname"
    cargo test --release
}

package() {
    cd "$srcdir/$pkgname"
    install -Dm755 "target/release/mdq" "$pkgdir/usr/bin/mdq"
}

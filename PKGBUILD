# Maintainer: Gaurav Atreya <allmanpride@gmail.com>
pkgname=clipcap
pkgver=0.1.1
pkgrel=1
pkgdesc="Tool to capture clipboard in terminal"
arch=('x86_64')
license=('GPL3')
depends=('gcc-libs')
makedepends=('rust' 'cargo')

build() {
	cargo build --release
}

package() {
    cd "$srcdir"
    mkdir -p "$pkgdir/usr/bin"
    cp "../target/release/${pkgname}" "$pkgdir/usr/bin/${pkgname}"
    chmod u+s "$pkgdir/usr/bin/${pkgname}"
}

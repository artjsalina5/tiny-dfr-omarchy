# Maintainer: Arturo Salinas-Aguayo <artjsalina5@gmail.com>

pkgname=tiny-dfr-omarchy
pkgver=0.6.5
pkgrel=1
pkgdesc='Touch Bar daemon for Apple T2 and Apple Silicon Macs with Omarchy integration'
arch=('x86_64' 'aarch64')
url='https://github.com/artjsalina5/tiny-dfr-omarchy'
license=('MIT' 'Apache-2.0')
depends=(
  'bash'
  'cairo'
  'fontconfig'
  'freetype2'
  'gcc-libs'
  'glibc'
  'libinput'
  'librsvg'
)
makedepends=('cargo')
optdepends=(
  'hyprland: window-context integration'
)
provides=("tiny-dfr=${pkgver}")
conflicts=('tiny-dfr')
install="${pkgname}.install"
source=(
  "${pkgname}-${pkgver}.tar.gz::${url}/archive/refs/tags/v${pkgver}.tar.gz"
)
sha256sums=('REPLACE_WITH_REAL_SUM')

prepare() {
  cd "${srcdir}/${pkgname}-${pkgver}"

  export RUSTUP_TOOLCHAIN=stable
  cargo fetch --locked --target "${CARCH}-unknown-linux-gnu"

  # Preserve the Omarchy-flavored default from the installer.
  sed -i \
    's/^MediaLayerDefault = false$/MediaLayerDefault = true/' \
    share/tiny-dfr/config.toml
}

build() {
  cd "${srcdir}/${pkgname}-${pkgver}"

  export RUSTUP_TOOLCHAIN=stable
  export CARGO_TARGET_DIR=target
  cargo build --frozen --release --bin tiny-dfr
}

# Uncomment this if the repo has a reliable non-hardware test suite.
# check() {
#   cd "${srcdir}/${pkgname}-${pkgver}"
#
#   export RUSTUP_TOOLCHAIN=stable
#   export CARGO_TARGET_DIR=target
#   cargo test --frozen
# }

package() {
  cd "${srcdir}/${pkgname}-${pkgver}"

  install -Dm0755 target/release/tiny-dfr \
    "${pkgdir}/usr/bin/${pkgname}"
  ln -s "${pkgname}" "${pkgdir}/usr/bin/tiny-dfr"
  ln -s "${pkgname}" \
    "${pkgdir}/usr/bin/omarchy-dynamic-function-row-daemon"

  install -Dm0755 bin/tiny-dfr-terminal-exec \
    "${pkgdir}/usr/bin/tiny-dfr-terminal-exec"
  install -Dm0755 bin/wait-for-device.sh \
    "${pkgdir}/usr/bin/wait-for-device.sh"
  install -Dm0755 bin/tiny-dfr-kbd-backlight \
    "${pkgdir}/usr/bin/tiny-dfr-kbd-backlight"
  install -Dm0755 bin/omarchy-touchbar-status \
    "${pkgdir}/usr/bin/omarchy-touchbar-status"
  install -Dm0755 bin/omarchy-touchbar-restart \
    "${pkgdir}/usr/bin/omarchy-touchbar-restart"
  install -Dm0755 bin/omarchy-touchbar-debug \
    "${pkgdir}/usr/bin/omarchy-touchbar-debug"

  cp -a share/tiny-dfr "${pkgdir}/usr/share/"

  install -Dm0755 etc/omarchy/hooks/theme-set \
    "${pkgdir}/usr/share/doc/${pkgname}/examples/theme-set"

  install -Dm0644 etc/systemd/system/tiny-dfr-omarchy.service \
    "${pkgdir}/usr/lib/systemd/system/tiny-dfr-omarchy.service"
  ln -s tiny-dfr-omarchy.service \
    "${pkgdir}/usr/lib/systemd/system/tiny-dfr.service"
  ln -s tiny-dfr-omarchy.service \
    "${pkgdir}/usr/lib/systemd/system/omarchy-dynamic-function-row-daemon.service"

  install -Dm0644 etc/systemd/system/suspend-fix-t2.service \
    "${pkgdir}/usr/lib/systemd/system/suspend-fix-t2.service"

  install -Dm0644 etc/udev/rules.d/99-touchbar-seat.rules \
    "${pkgdir}/usr/lib/udev/rules.d/99-touchbar-seat.rules"
  install -Dm0644 etc/udev/rules.d/99-touchbar-tiny-dfr.rules \
    "${pkgdir}/usr/lib/udev/rules.d/99-touchbar-tiny-dfr.rules"

  if [[ -f LICENSE ]]; then
    install -Dm0644 LICENSE \
      "${pkgdir}/usr/share/licenses/${pkgname}/LICENSE"
  fi

  if [[ -f LICENSE-MIT ]]; then
    install -Dm0644 LICENSE-MIT \
      "${pkgdir}/usr/share/licenses/${pkgname}/LICENSE-MIT"
  fi

  if [[ -f LICENSE-APACHE ]]; then
    install -Dm0644 LICENSE-APACHE \
      "${pkgdir}/usr/share/licenses/${pkgname}/LICENSE-APACHE"
  fi

  install -Dm0644 README.md \
    "${pkgdir}/usr/share/doc/${pkgname}/README.md"
}

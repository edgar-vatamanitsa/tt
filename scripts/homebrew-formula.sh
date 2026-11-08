#!/bin/sh
# Prints a Homebrew formula for a published GitHub release of tt.
#
#   scripts/homebrew-formula.sh v0.2.0 > ../homebrew-tap/Formula/tt.rb
#
# The release must already exist: the formula pins the checksums of its
# prebuilt archives. The GitHub repo is taken from the `origin` remote;
# override it with REPO=owner/name (or BASE_URL for testing against local files).
set -eu

tag=${1:?usage: $0 <tag, e.g. v0.2.0>}
version=${tag#v}
repo=${REPO:-$(git remote get-url origin | sed -E 's#(git@github\.com:|https://github\.com/)##; s#\.git$##')}
base=${BASE_URL:-https://github.com/$repo/releases/download/$tag}

sha256() {
    curl -fsSL "$base/tt-$tag-$1.tar.gz.sha256" | cut -d' ' -f1
}

mac_arm=$(sha256 aarch64-apple-darwin)
mac_intel=$(sha256 x86_64-apple-darwin)
linux_intel=$(sha256 x86_64-unknown-linux-musl)

cat <<EOF
class Tt < Formula
  desc "Plain-text task tracker that lives in the tasks/ folder of your repo"
  homepage "https://github.com/$repo"
  version "$version"
  license "GPL-2.0-or-later"

  on_macos do
    on_arm do
      url "$base/tt-$tag-aarch64-apple-darwin.tar.gz"
      sha256 "$mac_arm"
    end
    on_intel do
      url "$base/tt-$tag-x86_64-apple-darwin.tar.gz"
      sha256 "$mac_intel"
    end
  end

  on_linux do
    on_intel do
      url "$base/tt-$tag-x86_64-unknown-linux-musl.tar.gz"
      sha256 "$linux_intel"
    end
  end

  def install
    bin.install "tt"
  end

  test do
    assert_match version.to_s, shell_output("#{bin}/tt version")
    system bin/"tt", "init"
    system bin/"tt", "new", "-t", "test", "Brew test task"
    assert_match "Brew test task", shell_output("#{bin}/tt ls :test")
  end
end
EOF

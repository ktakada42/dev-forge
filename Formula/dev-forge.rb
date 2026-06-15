class DevForge < Formula
  desc "A developer's workshop for everyday transformations"
  homepage "https://github.com/ktakada42/dev-forge"
  version "0.1.0"
  license "MIT"

  on_macos do
    on_arm do
      url "https://github.com/ktakada42/dev-forge/releases/download/v#{version}/forge-v#{version}-aarch64-apple-darwin.tar.gz"
      sha256 "ede0439b41e305be91608202c15da60bb0c1e8e2b2ee4ff6cb1c5c438d6115c1"
    end

    on_intel do
      url "https://github.com/ktakada42/dev-forge/releases/download/v#{version}/forge-v#{version}-x86_64-apple-darwin.tar.gz"
      sha256 "553003525bcc5befff6b0675cd083c4fb7b7708cae97026f1f945b870799e495"
    end
  end

  def install
    bin.install "forge"
  end

  test do
    assert_match version.to_s, shell_output("#{bin}/forge --version")
  end
end

class Sniff < Formula
  desc "AI misalignment pattern detection for code quality assurance"
  homepage "https://github.com/conikeec/sniff"
  url "https://github.com/conikeec/sniff/releases/download/v0.1.0/sniff-v0.1.0-x86_64-apple-darwin.tar.gz"
  sha256 "PLACEHOLDER_SHA256"
  license "MIT"
  version "0.1.0"

  depends_on "git" => :optional

  def install
    bin.install "sniff"
    
    # Install shell completions if available
    if (buildpath/"completions").exist?
      bash_completion.install "completions/sniff.bash" => "sniff"
      zsh_completion.install "completions/_sniff"
      fish_completion.install "completions/sniff.fish"
    end
    
    # Install man page if available
    if (buildpath/"sniff.1").exist?
      man1.install "sniff.1"
    end
  end

  test do
    # Test basic functionality
    assert_match "sniff", shell_output("#{bin}/sniff --version")
    
    # Test help command
    assert_match "AI misalignment pattern detection", shell_output("#{bin}/sniff --help")
    
    # Create a test file with a misalignment pattern
    (testpath/"test.rs").write <<~EOS
      fn main() {
          // TODO: implement this properly
          println!("Hello, world!");
      }
    EOS
    
    # Test file analysis (should detect the TODO pattern)
    output = shell_output("#{bin}/sniff analyze-files #{testpath}/test.rs", 1)
    assert_match "misalignment", output.downcase
  end
end
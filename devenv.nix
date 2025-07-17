{pkgs, ...}: {
  dotenv.enable = true;

  packages = [pkgs.cocogitto pkgs.git pkgs.sqlx-cli];

  languages = {
    rust.enable = true;
  };

  services.postgres = {
    enable = true;
    listen_addresses = "127.0.0.1";
  };

  git-hooks.hooks = {
    # Nix

    alejandra.enable = true;

    # Git

    cocogitto = {
      enable = true;
      entry = "cog verify --file .git/COMMIT_EDITMSG";
      stages = ["commit-msg"];
      pass_filenames = false;
    };

    # Rust

    cargo-check.enable = true;
    rustfmt.enable = true;
    clippy.enable = true;

    test = {
      enable = true;
      entry = "cargo test --all-features";
      pass_filenames = false;
      stages = ["pre-push"];
    };
  };
}

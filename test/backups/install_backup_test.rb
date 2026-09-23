require "test_helper"
require "tmpdir"
require "open3"
require "fileutils"

# Contract tests for deploy/backups/install-backup.sh. Each test runs the real
# installer rootless: INSTALL_*_DIR overrides redirect every write into a temp
# directory, and stubs for id, systemctl, apt-get and gcloud stand in for the
# host boundary. The installed payloads are the real files from deploy/backups.
class InstallBackupTest < ActiveSupport::TestCase
  SCRIPT = Rails.root.join("deploy/backups/install-backup.sh").freeze

  test "installs the scripts, units and a fresh config" do
    with_install_env do |env, dirs|
      out, status = run_install(env)
      assert status.success?, "installer failed: #{out}"

      %w[campfire-backup.sh restore-check.sh lifecycle.json].each do |file|
        assert File.file?(dirs[:dest].join(file)), "missing installed #{file}"
      end
      assert File.executable?(dirs[:dest].join("campfire-backup.sh"))
      assert dirs[:systemd].join("campfire-backup.service").file?
      assert dirs[:systemd].join("campfire-backup.timer").file?

      config = dirs[:config].join("backup.env")
      assert config.file?, "installer did not create backup.env"
      assert_equal 0o600, File.stat(config).mode & 0o777
      assert_equal 0o700, File.stat(dirs[:config]).mode & 0o777
      assert_includes File.read(config), "BACKUP_BUCKET="

      log = stub_log(dirs)
      assert log.any? { |line| line.start_with?("systemctl daemon-reload") }, log.join("\n")
      assert log.any? { |line| line.start_with?("systemctl enable --now campfire-backup.timer") }, log.join("\n")
    end
  end

  test "installs age through apt when the configured encryption needs it" do
    with_install_env do |env, dirs|
      out, status = run_install(env)
      assert status.success?, "installer failed: #{out}"
      log = stub_log(dirs)
      assert log.any? { |line| line.start_with?("apt-get update") }, "apt index was not refreshed:\n#{log.join("\n")}"
      assert log.any? { |line| line.start_with?("apt-get install -y age") }, "age was not installed:\n#{log.join("\n")}"
    end
  end

  test "does not call apt when age is already present" do
    with_install_env do |env, dirs|
      age = dirs[:bin].join("age")
      age.write("#!/usr/bin/env bash\necho age-test\n")
      FileUtils.chmod 0o755, age
      out, status = run_install(env)
      assert status.success?, "installer failed: #{out}"
      refute stub_log(dirs).any? { |line| line.start_with?("apt-get") }, "apt must not run when age exists"
    end
  end

  test "installs the README the unit files point at" do
    with_install_env do |env, dirs|
      out, status = run_install(env)
      assert status.success?, "installer failed: #{out}"
      assert dirs[:dest].join("README.md").file?,
        "unit Documentation=file:///opt/campfire-backups/README.md would 404"
    end
  end

  test "keeps an existing backup.env across re-installs" do
    with_install_env do |env, dirs|
      _out, first = run_install(env)
      assert first.success?
      config = dirs[:config].join("backup.env")
      config.write("BACKUP_BUCKET=lead-configured\nBACKUP_ENCRYPTION=age\nBACKUP_AGE_RECIPIENT=age1x\n")

      out, second = run_install(env)
      assert second.success?, "re-install failed: #{out}"
      assert_equal "BACKUP_BUCKET=lead-configured\nBACKUP_ENCRYPTION=age\nBACKUP_AGE_RECIPIENT=age1x\n",
        File.read(config)
    end
  end

  private

    def with_install_env
      Dir.mktmpdir("smartfire-install-test") do |dir|
        root = Pathname.new(dir)
        dirs = {
          root: root,
          bin: root.join("bin"),
          dest: root.join("opt"),
          config: root.join("etc"),
          systemd: root.join("systemd")
        }
        dirs.each_value(&:mkpath)
        write_stubs(dirs)
        # NB: PATH is stubbin + the core dirs only, so the real age (if any)
        # is invisible and the installer must provide it.
        env = {
          "PATH" => "#{dirs[:bin]}:/usr/bin:/bin",
          "STUB_LOG" => root.join("calls.log").to_s,
          "STUB_BIN" => dirs[:bin].to_s,
          "INSTALL_DEST_DIR" => dirs[:dest].to_s,
          "INSTALL_ENV_DIR" => dirs[:config].to_s,
          "INSTALL_SYSTEMD_DIR" => dirs[:systemd].to_s
        }
        yield env, dirs
      end
    end

    def run_install(env)
      Open3.capture2e(env, SCRIPT.to_s)
    end

    def stub_log(dirs)
      path = dirs[:root].join("calls.log")
      path.file? ? File.read(path).lines.map(&:strip) : []
    end

    def write_stubs(dirs)
      stub(dirs, "id", <<~'SH')
        #!/usr/bin/env bash
        echo "id $*" >> "$STUB_LOG"
        echo 0
      SH
      stub(dirs, "install", <<~'SH')
        #!/usr/bin/env bash
        # Faithful enough for the installer's subset: -d, -m, one source.
        # Ownership flags are recorded but not applied: chown to root is
        # impossible unprivileged, and the VM's real install sets it.
        echo "install $*" >> "$STUB_LOG"
        mode=""
        makedir=0
        args=()
        while [ "$#" -gt 0 ]; do
          case "$1" in
            -d) makedir=1; shift ;;
            -m) mode="$2"; shift 2 ;;
            -o|-g) shift 2 ;;
            *) args+=("$1"); shift ;;
          esac
        done
        if [ "$makedir" = "1" ]; then
          mkdir -p "${args[@]}"
          [ -n "$mode" ] && chmod "$mode" "${args[@]}"
        else
          last=$((${#args[@]} - 1))
          dest="${args[$last]}"
          unset "args[$last]"
          for src in ${args[@]+"${args[@]}"}; do cp "$src" "$dest"; done
          [ -n "$mode" ] && chmod "$mode" "$dest"
        fi
        exit 0
      SH
      stub(dirs, "systemctl", <<~'SH')
        #!/usr/bin/env bash
        echo "systemctl $*" >> "$STUB_LOG"
        exit 0
      SH
      stub(dirs, "gcloud", <<~'SH')
        #!/usr/bin/env bash
        echo "gcloud $*" >> "$STUB_LOG"
        exit 0
      SH
      stub(dirs, "apt-get", <<~'SH')
        #!/usr/bin/env bash
        echo "apt-get $*" >> "$STUB_LOG"
        if [ "$1" = "install" ]; then
          # Pretend the package manager provides age from here on.
          printf '#!/usr/bin/env bash\necho "age from fake apt"\n' > "$STUB_BIN/age"
          chmod 0755 "$STUB_BIN/age"
        fi
        exit 0
      SH
    end

    def stub(dirs, name, body)
      path = dirs[:bin].join(name)
      path.write(body)
      FileUtils.chmod 0o755, path
    end
end

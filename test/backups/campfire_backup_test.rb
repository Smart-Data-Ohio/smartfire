require "test_helper"
require "sqlite3"
require "tmpdir"
require "open3"
require "fileutils"
require "json"
require "date"

# Contract tests for deploy/backups/campfire-backup.sh. Each test builds a
# fake ONCE storage volume (a temp SQLite database plus an uploads tree) and
# runs the real script against it in BACKUP_VOLUME_DIR mode, producing the
# encrypted archive in a temp output directory. Only the host/cloud boundary
# is stubbed: a poison `gcloud` proves the script never shells out to the
# cloud, and a stubbed `df` drives the free-space refusal. sqlite, tar, gzip,
# age and gpg are the real binaries.
#
# The gpg path always runs (gpg ships on CI runners). The age path runs when
# an `age` binary is on PATH and skips otherwise; pass extra directories via
# BACKUP_TEST_EXTRA_PATH (colon-separated) to exercise it locally.
class CampfireBackupTest < ActiveSupport::TestCase
  SCRIPT = Rails.root.join("deploy/backups/campfire-backup.sh").freeze
  DB_SENTINEL = "sentinel-db-row-7f3a91"
  FILE_SENTINEL = "sentinel-file-data-9c2e44"

  test "backs up a live database while a writer keeps writing" do
    with_backup_env do |env, dirs|
      seed = seed_volume(dirs[:volume], messages: 50)
      stop = false
      warmed_up = Queue.new
      writer = Thread.new do
        db = SQLite3::Database.new(seed)
        db.busy_timeout = 5_000
        db.execute("INSERT INTO messages(body) VALUES ('writer-warmup')")
        warmed_up << true
        until stop
          db.execute("INSERT INTO messages(body) VALUES ('writer-row')")
          sleep 0.002
        end
        db.close
      end
      warmed_up.pop

      begin
        _out, status = run_backup(env, dirs, "BACKUP_DATETIME" => "20260923-090000")
        assert status.success?, "backup failed while the writer was running"
      ensure
        stop = true
        writer.join
      end

      live_count = sqlite_count(seed, "messages")
      snapshot = extract_snapshot(dirs, "20260923-090000", age: false)
      assert_equal "ok", sqlite_query(snapshot[:db], "PRAGMA integrity_check;").strip
      snapshot_count = sqlite_count(snapshot[:db], "messages")
      assert snapshot_count >= 51, "snapshot lost committed rows: #{snapshot_count}"
      assert snapshot_count <= live_count, "snapshot has rows from the future"
      assert_equal FILE_SENTINEL, File.read(snapshot[:file]).strip
    end
  end

  test "output bytes are encrypted and round-trip through decryption" do
    with_backup_env do |env, dirs|
      seed_volume(dirs[:volume], messages: 5)
      _out, status = run_backup(env, dirs, "BACKUP_DATETIME" => "20260923-090000")
      assert status.success?

      blob = output_blob(dirs, "20260923-090000", "gpg")
      refute_includes blob, DB_SENTINEL, "database contents leaked into the output"
      refute_includes blob, FILE_SENTINEL, "file contents leaked into the output"
      _out, gzip_status = Open3.capture2e("gzip", "-t", output_path(dirs, "20260923-090000", "gpg"))
      refute gzip_status.success?, "output is plain gzip, not encrypted"

      snapshot = extract_snapshot(dirs, "20260923-090000", age: false)
      assert_includes File.binread(snapshot[:db]), DB_SENTINEL
    end
  end

  test "age encryption round-trips when age is available" do
    skip "age is not on PATH" unless age_available?

    with_backup_env(encryption: :age) do |env, dirs|
      seed_volume(dirs[:volume], messages: 5)
      _out, status = run_backup(env, dirs, "BACKUP_DATETIME" => "20260923-090000")
      assert status.success?

      blob = output_blob(dirs, "20260923-090000", "age")
      assert blob.start_with?("age-encryption.org/"), "missing age header"
      refute_includes blob, DB_SENTINEL
      refute_includes blob, FILE_SENTINEL

      snapshot = extract_snapshot(dirs, "20260923-090000", age: true)
      assert_equal "ok", sqlite_query(snapshot[:db], "PRAGMA integrity_check;").strip
      manifest = JSON.parse(File.read(snapshot[:manifest]))
      assert_equal "20260923-090000", manifest["stamp"]
      assert_equal Digest::SHA256.file(snapshot[:db]).hexdigest, manifest.dig("database", "sha256")
    end
  end

  test "manifest describes the backup and the database checksum matches" do
    with_backup_env do |env, dirs|
      seed_volume(dirs[:volume], messages: 3)
      _out, status = run_backup(env, dirs, "BACKUP_DATETIME" => "20260923-090000")
      assert status.success?

      snapshot = extract_snapshot(dirs, "20260923-090000", age: false)
      manifest = JSON.parse(File.read(snapshot[:manifest]))
      assert_equal "20260923-090000", manifest["stamp"]
      assert_equal "gpg", manifest["encryption"]
      assert_equal 1, manifest.dig("files", "count")
      assert_equal Digest::SHA256.file(snapshot[:db]).hexdigest, manifest.dig("database", "sha256")
    end
  end

  test "output-dir mode produces only encrypted output and prints its path and checksum" do
    with_backup_env do |env, dirs|
      seed_volume(dirs[:volume], messages: 2)
      out, status = run_backup(env, dirs, "BACKUP_DATETIME" => "20260923-090000")
      assert status.success?, "backup failed: #{out}"

      expected = output_path(dirs, "20260923-090000", "gpg")
      assert_equal [ expected ], Dir.glob(dirs[:out].join("*").to_s).sort,
        "the output directory must hold exactly the encrypted archive"
      assert_equal expected, out[/^BACKUP_FILE=(.*)$/, 1]&.strip,
        "BACKUP_FILE line missing or wrong:\n#{out}"
      assert_equal Digest::SHA256.file(expected).hexdigest, out[/^BACKUP_SHA256=(.*)$/, 1]&.strip,
        "BACKUP_SHA256 line missing or wrong:\n#{out}"

      # No plaintext anywhere: the unencrypted tarball is removed and the
      # work directory goes with the EXIT trap.
      assert_empty Dir.glob(dirs[:root].join("**/*.tar.gz").to_s),
        "an unencrypted tarball survived the run"
      assert_empty Dir.glob(dirs[:state].join("campfire-backup.*").to_s),
        "the work directory survived the run"
      assert dirs[:state].join("campfire-nightly-last.json").file?,
        "the run summary was not written"

      # The script never uploads: a poison gcloud fails the run if invoked.
      assert_empty poison_log(dirs), "the script shelled out to gcloud"
    end
  end

  test "accepts the output directory from BACKUP_OUTPUT_DIR" do
    with_backup_env do |env, dirs|
      seed_volume(dirs[:volume], messages: 1)
      out, status = run_backup(env, dirs,
        { "BACKUP_DATETIME" => "20260923-090000", "BACKUP_OUTPUT_DIR" => dirs[:out].to_s },
        [])
      assert status.success?, "backup failed: #{out}"
      assert File.file?(output_path(dirs, "20260923-090000", "gpg"))
    end
  end

  test "refuses an output directory inside its own work directory" do
    with_backup_env do |env, dirs|
      seed_volume(dirs[:volume], messages: 1)
      inside = dirs[:state].join("campfire-backup.20260923-090000", "sub").to_s
      _out, status = run_backup(env, dirs,
        { "BACKUP_DATETIME" => "20260923-090000" }, [ "--output-dir", inside ])
      refute status.success?, "an output dir inside the work dir would be deleted by the trap"
    end
  end

  test "exits 75 when a release holds the release lock" do
    with_backup_env do |env, dirs|
      seed_volume(dirs[:volume], messages: 1)
      lock_path = env.fetch("BACKUP_RELEASE_LOCK_FILE")
      File.open(lock_path, "w") do |held|
        assert held.flock(File::LOCK_EX | File::LOCK_NB), "could not take the test lock"
        out, status = run_backup(env, dirs, "BACKUP_DATETIME" => "20260923-090000")
        assert_equal 75, status.exitstatus, "expected EX_TEMPFAIL, got #{status.exitstatus}:\n#{out}"
        assert_includes out, "a release holds"
      end
    end
  end

  test "refuses to start without enough free space" do
    with_backup_env do |env, dirs|
      seed_volume(dirs[:volume], messages: 5)
      write_df_stub(dirs, avail_bytes: 1024)
      out, status = run_backup(env, dirs, "BACKUP_DATETIME" => "20260923-090000")
      refute status.success?, "the backup must not start on a nearly-full disk"
      assert_includes out, "bytes free"
      assert_empty Dir.glob(dirs[:state].join("campfire-backup.*").to_s),
        "a refused run must not stage anything"
    end
  end

  test "prunes stale work directories at start and keeps the rest" do
    with_backup_env do |env, dirs|
      seed_volume(dirs[:volume], messages: 1)
      stale = dirs[:state].join("campfire-backup.20200101-000000")
      fresh = dirs[:state].join("campfire-backup.fresh")
      other = dirs[:state].join("unrelated-old-dir")
      [ stale, fresh, other ].each do |dir|
        dir.mkpath
        dir.join("leftover").write("x")
      end
      old = Time.now - (3 * 24 * 3600)
      File.utime(old, old, stale, stale.join("leftover"), other, other.join("leftover"))

      _out, status = run_backup(env, dirs, "BACKUP_DATETIME" => "20260923-090000")
      assert status.success?

      refute stale.exist?, "a work dir older than a day must be pruned"
      assert fresh.directory?, "a fresh work dir must be kept"
      assert other.directory?, "a non-matching dir must be kept"
    end
  end

  test "missing database exits non-zero" do
    with_backup_env do |env, dirs|
      out, status = run_backup(env, dirs, "BACKUP_DATETIME" => "20260923-090000")
      refute status.success?
      assert_includes out, "database file not found"
    end
  end

  test "bad recipient exits non-zero" do
    with_backup_env do |env, dirs|
      seed_volume(dirs[:volume], messages: 1)
      _out, status = run_backup(env, dirs,
        "BACKUP_DATETIME" => "20260923-090000",
        "BACKUP_GPG_RECIPIENT" => "nobody-knows-this-key@example.com")
      refute status.success?
    end
  end

  test "malformed stamp exits non-zero" do
    with_backup_env do |env, dirs|
      seed_volume(dirs[:volume], messages: 1)
      _out, status = run_backup(env, dirs, "BACKUP_DATETIME" => "yesterday-teatime")
      refute status.success?
    end
  end

  test "example placeholder age recipient exits non-zero" do
    skip "age is not on PATH" unless age_available?

    with_backup_env(encryption: :age) do |env, dirs|
      seed_volume(dirs[:volume], messages: 1)
      out, status = run_backup(env, dirs,
        "BACKUP_DATETIME" => "20260923-090000",
        "BACKUP_AGE_RECIPIENT" => "REPLACE-age1-public-recipient")
      refute status.success?
      assert_includes out, "placeholder"
    end
  end

  private

    def with_backup_env(encryption: :gpg)
      assert_path "sqlite3"
      Dir.mktmpdir("campfire-backup-test") do |dir|
        root = Pathname.new(dir)
        dirs = {
          root: root,
          volume: root.join("vol"),
          state: root.join("state"),
          out: root.join("out"),
          bin: root.join("bin")
        }
        dirs.each_value(&:mkpath)
        write_gcloud_poison(dirs)

        env = {
          "BACKUP_ENCRYPTION" => encryption.to_s,
          "BACKUP_VOLUME_DIR" => dirs[:volume].to_s,
          "BACKUP_STATE_ROOT" => dirs[:state].to_s,
          "BACKUP_LOCK_FILE" => root.join("backup.lock").to_s,
          "BACKUP_RELEASE_LOCK_FILE" => root.join("release.lock").to_s,
          "STUB_LOG" => root.join("calls.log").to_s
        }
        if encryption == :age
          env["BACKUP_AGE_RECIPIENT"] = age_recipient(root)
          @age_identity = root.join("age-key.txt")
        else
          home = root.join("gnupg")
          home.mkpath
          FileUtils.chmod 0o700, home
          generate_gpg_key(home)
          env["BACKUP_GPG_HOME"] = home.to_s
          env["BACKUP_GPG_RECIPIENT"] = "backup-test@example.com"
          @gpg_home = home
        end

        yield env, dirs
      end
    end

    def run_backup(env, dirs, overrides, args = nil)
      args ||= [ "--output-dir", dirs[:out].to_s ]
      full = stub_path_env(dirs).merge(env).merge(overrides.compact)
      overrides.each_key { |key| full.delete(key) if overrides[key].nil? }
      Open3.capture2e(full, SCRIPT.to_s, *args)
    end

    def stub_path_env(dirs = {})
      extra = [ dirs[:bin]&.to_s, ENV["BACKUP_TEST_EXTRA_PATH"], ENV["PATH"] ].compact.join(":")
      { "PATH" => extra }
    end

    # The script must never invoke gcloud: the workflow uploads, not the VM.
    # Any call fails loudly and is recorded for the assertion.
    def write_gcloud_poison(dirs)
      stub = dirs[:bin].join("gcloud")
      stub.write(<<~SH)
        #!/usr/bin/env bash
        echo "gcloud $*" >> "$STUB_LOG"
        echo "poison gcloud must never be invoked" >&2
        exit 9
      SH
      FileUtils.chmod 0o755, stub
    end

    def write_df_stub(dirs, avail_bytes:)
      stub = dirs[:bin].join("df")
      stub.write(<<~SH)
        #!/usr/bin/env bash
        printf 'Avail\\n#{avail_bytes}\\n'
      SH
      FileUtils.chmod 0o755, stub
    end

    def poison_log(dirs)
      log = dirs[:root].join("calls.log")
      return [] unless log.file?

      File.read(log).lines.map(&:strip)
    end

    def seed_volume(volume, messages:)
      db_dir = volume.join("db")
      files_dir = volume.join("files/ab")
      db_dir.mkpath
      files_dir.mkpath
      db = db_dir.join("production.sqlite3").to_s
      sqlite_query(db, "PRAGMA journal_mode=WAL;")
      sqlite_query(db, <<~SQL)
        CREATE TABLE users(id INTEGER PRIMARY KEY, name TEXT);
        CREATE TABLE rooms(id INTEGER PRIMARY KEY, name TEXT);
        CREATE TABLE messages(id INTEGER PRIMARY KEY, body TEXT);
        INSERT INTO users(name) VALUES ('#{DB_SENTINEL}');
        INSERT INTO rooms(name) VALUES ('general');
      SQL
      messages.times { |n| sqlite_query(db, "INSERT INTO messages(body) VALUES ('seed-#{n}');") }
      files_dir.join("blob1").write(FILE_SENTINEL)
      db
    end

    def output_path(dirs, datetime, ext)
      dirs[:out].join("smartfire-backup-#{datetime}.tar.gz.#{ext}").to_s
    end

    def output_blob(dirs, datetime, ext)
      File.binread(output_path(dirs, datetime, ext))
    end

    # Decrypts the output archive and extracts it, mirroring the documented
    # restore path (decrypt -> tar -> SHA256SUMS), and returns the paths.
    def extract_snapshot(dirs, datetime, age:)
      work = dirs[:root].join("extracted")
      work.mkpath
      plain = work.join("backup.tar.gz").to_s
      if age
        _out, status = Open3.capture2e(stub_path_env(dirs),
          "age", "--decrypt", "--identity", @age_identity.to_s,
          "--output", plain, output_path(dirs, datetime, "age"))
        assert status.success?, "age decryption failed"
      else
        _out, status = Open3.capture2e(
          "gpg", "--batch", "--yes", "--homedir", @gpg_home.to_s,
          "--decrypt", "--output", plain, output_path(dirs, datetime, "gpg"))
        assert status.success?, "gpg decryption failed"
      end
      _out, status = Open3.capture2e("tar", "-xzf", plain, "-C", work.to_s)
      assert status.success?, "tar extraction failed"
      stage = work.join("smartfire-backup-#{datetime}")
      sums, status = Open3.capture2e("sha256sum", "-c", "SHA256SUMS", chdir: stage.to_s)
      assert status.success?, "SHA256SUMS failed:\n#{sums}"
      { db: stage.join("production.sqlite3").to_s,
        file: stage.join("files/ab/blob1").to_s,
        manifest: stage.join("manifest.json").to_s }
    end

    def sqlite_query(db, sql)
      out, status = Open3.capture2e("sqlite3", db, sql)
      raise "sqlite3 failed: #{out}" unless status.success?
      out
    end

    def sqlite_count(db, table)
      sqlite_query(db, "SELECT count(*) FROM \"#{table}\";").strip.to_i
    end

    def generate_gpg_key(home)
      _out, status = Open3.capture2e("gpg", "--batch", "--pinentry-mode", "loopback",
        "--passphrase", "", "--homedir", home.to_s,
        "--quick-generate-key", "backup-test@example.com", "rsa2048", "encr", "never")
      raise "gpg key generation failed" unless status.success?
    end

    def age_recipient(root)
      key_file = root.join("age-key.txt").to_s
      _out, status = Open3.capture2e(stub_path_env, "age-keygen", "-o", key_file)
      raise "age-keygen failed" unless status.success?
      File.read(key_file)[/^# public key: (age1[0-9a-z]+)/, 1] or raise "no age recipient"
    end

    def age_available?
      _out, status = Open3.capture2(stub_path_env, "which", "age")
      status.success?
    end

    def assert_path(tool, prepend: nil)
      path = [ prepend, ENV["BACKUP_TEST_EXTRA_PATH"], ENV["PATH"] ].compact.join(":")
      _out, status = Open3.capture2({ "PATH" => path }, "sh", "-c", "command -v #{tool}")
      assert status.success?, "#{tool} is not on PATH (needed by the backup test)"
    end
end

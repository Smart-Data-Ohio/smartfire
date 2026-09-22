require "minitest/autorun"
require "fileutils"
require "open3"
require "tmpdir"

class PostRestoreHookTest < Minitest::Test
  SCRIPT = File.expand_path("../../hooks/post-restore", __dir__)

  def setup
    @storage = Dir.mktmpdir
    FileUtils.mkdir_p(File.join(@storage, "backups"))
    FileUtils.mkdir_p(File.join(@storage, "appendonlydir"))
    File.write(File.join(@storage, "backups", "test.sqlite3"), "snapshot-db")
    File.write(File.join(@storage, "appendonlydir", "appendonly.aof.manifest"), "stale-queue")
    File.write(File.join(@storage, "appendonly.aof"), "stale-queue")
  end

  def teardown
    FileUtils.remove_entry(@storage)
  end

  def test_post_restore_restores_the_database_and_clears_the_redis_queue
    output, status = Open3.capture2e(
      { "CAMPFIRE_STORAGE" => @storage, "RAILS_ENV" => "test" }, SCRIPT
    )

    assert status.success?, output
    assert_equal "snapshot-db", File.read(File.join(@storage, "db", "test.sqlite3"))
    refute File.exist?(File.join(@storage, "appendonlydir")),
      "expected the Redis 7 appendonlydir to be removed"
    assert_empty Dir[File.join(@storage, "appendonly.aof*")],
      "expected legacy appendonly.aof files to be removed"
  end
end

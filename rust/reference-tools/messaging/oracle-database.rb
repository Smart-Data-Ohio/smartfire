require 'fileutils'

# HTTP transaction behavior must be measured without an enclosing test transaction.
# The runner has no job worker; disconnect its pools before restoring the private seed.
module MessagingOracleDatabase
  def self.scenarios(output)
    database = ActiveRecord::Base.connection_db_config.database
    baseline = File.join(File.dirname(output), "#{File.basename(output, '.json')}-baseline.sqlite3")
    FileUtils.rm_f(baseline)
    ActiveRecord::Base.connection.execute("VACUUM INTO #{ActiveRecord::Base.connection.quote(baseline)}")
    ->(&block) do
      begin
        block.call
      ensure
        ActiveRecord::Base.connection_handler.clear_all_connections!
        FileUtils.rm_f(["#{database}-wal", "#{database}-shm"])
        FileUtils.cp(baseline, database)
        ActiveRecord::Base.clear_query_caches_for_current_thread
      end
    end
  end
end

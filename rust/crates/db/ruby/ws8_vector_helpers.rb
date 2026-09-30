require_relative "load_fixtures"
require "json"
ActiveJob::Base.queue_adapter = :test
C = ActiveRecord::Base.connection
TABLES = C.tables.reject { |t| t.start_with?("search_index", "sqlite_") || %w[schema_migrations ar_internal_metadata background_jobs].include?(t) }
def dump
  TABLES.to_h { |t| [t, C.select_all("SELECT * FROM #{C.quote_table_name(t)}").to_a.to_h { |r| [r["id"], r] }] }
end
def delta(before)
  dump.flat_map do |table, rows|
    rows.filter_map do |id, row|
      next if before.fetch(table)[id] == row
      cols = row.keys.map { |k| C.quote_column_name(k) }.join(",")
      vals = row.values.map { |v| C.quote(v) }.join(",")
      "INSERT OR REPLACE INTO #{C.quote_table_name(table)} (#{cols}) VALUES (#{vals});"
    end
  end.join("\n")
end
def check(sql)
  { sql: sql, rows: C.select_rows(sql) }
end

# Lossless expected-state encoding. Every changed row retains every column;
# unchanged rows come from the recorded baseline, never from candidate output.
module MessagingOracleState
  def self.changes(baseline, actual)
    actual.filter_map do |table, rows|
      before = baseline.fetch(table)
      next if rows == before
      old = before.index_by { |row| row.fetch('id') }
      current = rows.index_by { |row| row.fetch('id') }
      [table, {rows: rows.reject { |row| old[row.fetch('id')] == row }, deleted_ids: old.keys - current.keys}]
    end.to_h
  end
end

module Agents
  # Uniform outcome for the agent domain services shared by the REST agent
  # API and the MCP tools. Controllers translate failures into their own
  # error shape (a REST status vs an MCP isError result); the message text
  # stays identical so agents see one vocabulary on both surfaces.
  class ServiceResult < Data.define(:payload, :error, :status)
    def self.ok(payload = nil, status: :ok)
      new(payload: payload, error: nil, status: status)
    end

    def self.fail(error, status: :unprocessable_entity, payload: nil)
      new(payload: payload, error: error, status: status)
    end

    def ok?
      error.nil?
    end

    # The JSON body a REST controller renders for a failure: an explicit
    # payload when the endpoint has its own error shape (record errors),
    # otherwise the standard { error } body.
    def failure_body
      payload || { error: error }
    end
  end
end

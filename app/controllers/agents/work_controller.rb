class Agents::WorkController < ApplicationController
  include AgentApiThrottle

  allow_agent_access only: %i[ index show update result handoff ]

  before_action :ensure_agent_token, only: %i[ index show update result handoff ]
  throttle_agent_api limit: 60, only: :handoff

  # GET /agents/work (Bearer-only, JSON). Lists the threads the agent
  # currently owns, newest first, max 100, filtered to rooms the agent's
  # user still belongs to and where the agent holds read_messages. The
  # lookup lives in Agents::WorkThreads, shared with the MCP tools.
  def index
    no_store_response!

    render_work_result Agents::WorkThreads.list(agent: Current.agent)
  end

  # GET /agents/work/:id (Bearer-only, JSON). Returns one owned thread.
  # Anything the agent does not own, or whose room it cannot read, is 404.
  def show
    no_store_response!

    render_work_result Agents::WorkThreads.show(agent: Current.agent, id: params[:id])
  end

  # PATCH /agents/work/:id (Bearer-only, JSON). Updates the status, tags,
  # and run_url of a thread the agent owns, with an optional plain-text
  # note (max 500) recorded in the WorkThreadEvent. Each field updates
  # only when its key is given; tags takes an array or a comma-separated
  # string and run_url must be https, with blank clearing either. Anything
  # the agent does not own is 404; a missing manage_threads grant in the
  # thread's room is 403. Agents cannot reassign, convert, or stop tracking.
  def update
    no_store_response!

    render_work_result Agents::WorkThreads.update(
      agent: Current.agent, id: params[:id],
      work_status: agent_work_field(:work_status),
      note: params[:note].presence || params.dig(:work, :note),
      tags: agent_work_field(:tags),
      run_url: agent_work_field(:run_url)
    )
  end

  # PUT /agents/work/:id/result (Bearer-only, JSON). Replaces the pinned
  # result of a thread the agent owns: markdown is required (max 20,000
  # characters), blank clears, an unchanged value writes nothing, and
  # every write records a result_updated event. Anything the agent does
  # not own is 404; a missing manage_threads grant in the thread's room
  # is 403.
  def result
    no_store_response!

    render_work_result Agents::WorkThreads.set_result(
      agent: Current.agent, id: params[:id],
      markdown: params[:markdown], markdown_given: params.key?(:markdown)
    )
  end

  # POST /agents/work/:id/handoff (Bearer-only, JSON). Hands a thread
  # the agent owns to another agent with a context package: summary
  # (required, max 2000 characters), links (up to 10 http(s) URLs), and
  # open_questions (up to 10, max 500 characters each). Ownership
  # transfers, Work history records the handoff, the receiver gets a
  # work_handed_off event, and the audit log records work.handoff.
  # Anything the agent does not own is 404; a missing manage_threads
  # grant in the thread's room is 403; an ineligible receiver is 422.
  # Throttled at 60/minute per credential, shared with the MCP
  # handoff_work tool.
  def handoff
    no_store_response!

    result = Agents::WorkHandoffs.create(
      agent: Current.agent, id: params[:id],
      receiver_agent_id: params[:receiver_agent_id],
      summary: params[:summary].to_s,
      links: params[:links],
      open_questions: params[:open_questions]
    )

    if result.ok?
      payload = work_thread_payload(result.payload[:thread]).merge(handoff: result.payload[:handoff].payload)
      render json: payload, status: :created
    elsif result.status == :not_found
      head :not_found
    else
      render json: result.failure_body, status: result.status
    end
  end

  private
    def ensure_agent_token
      unless authenticated_by.agent_token? && Current.agent
        render json: { error: "Forbidden: Bearer agent token required" }, status: :forbidden
      end
    end

    def render_work_result(result)
      if result.ok?
        payload = result.payload.is_a?(Array) ? result.payload.map { |thread| work_thread_payload(thread) } : work_thread_payload(result.payload)
        render json: payload, status: result.status
      elsif result.status == :not_found
        head :not_found
      else
        render json: result.failure_body, status: result.status
      end
    end

    def work_thread_payload(thread)
      Agents::WorkPayload.for(thread, agent: Current.agent)
    end

    # Reads an updatable work field from the top level or the nested work
    # object, the top level winning when both carry the key. Returns the
    # unset sentinel when neither does, so the model can tell an omitted
    # field from an explicit blank.
    def agent_work_field(key)
      return params[key] if params.key?(key)

      nested = params[:work]
      return ChannelThread::UNSET_WORK_VALUE unless nested.respond_to?(:key?) && nested.key?(key)

      nested[key]
    end
end

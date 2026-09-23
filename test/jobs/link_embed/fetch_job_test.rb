require "test_helper"

class LinkEmbed::FetchJobTest < ActiveJob::TestCase
  test "fetches the embed and broadcasts its card container" do
    WebMock.stub_request(:get, "https://example.com/job")
      .to_return(status: 200, headers: { content_type: "text/html" },
        body: '<html><head><meta property="og:title" content="Job Page">' \
          '<meta property="og:description" content="Fetched by the job."></head></html>')

    message = rooms(:designers).messages.create!(
      creator: users(:david), client_message_id: "embed-job-fetch",
      markdown_source: "read https://example.com/job"
    )
    embed = message.link_embeds.first
    assert_not_nil embed

    assert_turbo_stream_broadcasts [ message.room, :messages ], count: 1 do
      LinkEmbed::FetchJob.perform_now(embed)
    end

    assert_equal "Job Page", embed.reload.title
    assert_nil embed.fetch_error
  end
end

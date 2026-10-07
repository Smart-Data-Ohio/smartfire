import { useParams } from "@tanstack/react-router";
import { AgentProfilePage } from "./agent-profile-page.tsx";

/** `/app/agents/$agentId`: the profile of the agent in the URL. */
export function AgentProfileRoute() {
  const { agentId } = useParams({ from: "/shell/agents/$agentId" });

  return <AgentProfilePage key={agentId} agentId={agentId} />;
}

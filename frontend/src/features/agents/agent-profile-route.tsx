import { Outlet, useMatchRoute, useParams } from "@tanstack/react-router";
import { Suspense } from "react";
import { AgentProfilePage, type AgentSection } from "./agent-profile-page.tsx";

/** Which section the URL shows. */
function useSection(): AgentSection {
  const matchRoute = useMatchRoute();

  if (matchRoute({ to: "/agents/$agentId/approvals" }) !== false) {
    return "approvals";
  }

  return matchRoute({ to: "/agents/$agentId/events" }) === false ? "overview" : "events";
}

/**
 * `/app/agents/$agentId` and its sections: the profile of the agent in the URL, with the section
 * (overview, approvals or activity) in its outlet. A section's chunk loads inside the panel, so
 * the profile (and its loaded state) stays up while a tab is opened for the first time.
 */
export function AgentProfileRoute() {
  const { agentId } = useParams({ from: "/shell/agents/$agentId" });
  const section = useSection();

  return (
    <AgentProfilePage key={agentId} agentId={agentId} section={section}>
      <Suspense fallback={null}>
        <Outlet />
      </Suspense>
    </AgentProfilePage>
  );
}

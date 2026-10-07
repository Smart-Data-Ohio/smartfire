import { useNavigate } from "@tanstack/react-router";
import type { ReactNode } from "react";
import { Menu, MenuItem, MenuSeparator } from "../../ui/menu.tsx";

/**
 * Leaves the new UI: `POST /app/ui_preference` with `ui=classic`, as the classic profile's form
 * sends it (the session's token in `authenticity_token`). The server records the choice and
 * redirects to the classic page for where the person is (`return_to`, mapped from this SPA path).
 */
export function switchToClassic(
  location: Pick<Location, "pathname" | "search"> = window.location,
): void {
  const form = document.createElement("form");

  form.method = "post";
  form.action = "/app/ui_preference";
  form.hidden = true;

  const fields = new Map([
    ["ui", "classic"],
    ["return_to", `${location.pathname}${location.search}`],
  ]);

  const param = document.querySelector('meta[name="csrf-param"]')?.getAttribute("content");
  const token = document.querySelector('meta[name="csrf-token"]')?.getAttribute("content");

  if (param && token) {
    fields.set(param, token);
  }

  for (const [name, value] of fields) {
    const input = document.createElement("input");

    input.type = "hidden";
    input.name = name;
    input.value = value;
    form.append(input);
  }

  document.body.append(form);
  form.submit();
}

/**
 * The signed-in person's menu, opened from their panel at the foot of the sidebar (`children` is
 * the trigger's content): their profile and settings, the workspace (its people, and for
 * administrators the rest of the classic account pages), and the way back to the classic UI.
 */
export function UserMenu({ children }: { readonly children: ReactNode }) {
  const navigate = useNavigate();

  return (
    <Menu
      label="Your account"
      placement="top-start"
      trigger={(props) => (
        <button {...props} type="button" className="sidebar-you-button" aria-label="Your account">
          {children}
        </button>
      )}
    >
      <MenuItem icon="settings" onSelect={() => void navigate({ to: "/settings" })}>
        Profile and settings
      </MenuItem>
      <MenuItem icon="home" onSelect={() => void navigate({ to: "/admin" })}>
        Workspace and people
      </MenuItem>
      <MenuSeparator />
      <MenuItem icon="rotate-ccw" onSelect={() => switchToClassic()}>
        Switch to classic
      </MenuItem>
    </Menu>
  );
}

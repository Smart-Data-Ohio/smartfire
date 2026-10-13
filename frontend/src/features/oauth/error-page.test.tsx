import { cleanup, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it } from "vitest";
import { OAuthErrorPage } from "./error-page.tsx";
import { oauthErrorText } from "./oauth-format.ts";
import { parseOAuthErrorSearch } from "./oauth-search.ts";

afterEach(cleanup);

describe("the OAuth error page", () => {
  it("explains an app Smartfire can't identify, with its code and the way back", () => {
    render(<OAuthErrorPage search={{ error: "invalid_client" }} />);

    expect(screen.getByRole("heading", { name: "Couldn't identify the app" })).toBeTruthy();
    expect(screen.getByRole("alert").textContent).toMatch(/didn't describe itself/);
    expect(screen.getByText("invalid_client").tagName).toBe("CODE");
    expect(screen.getByRole("link", { name: "Back to Smartfire" }).getAttribute("href")).toBe(
      import.meta.env.BASE_URL,
    );
    expect(document.title).toBe("Couldn't identify the app");
  });

  it("explains each code the server sends, and any other in general terms", () => {
    expect(oauthErrorText("unsupported_client_authentication").message).toMatch(
      /secret of its own/,
    );
    expect(oauthErrorText("invalid_redirect_uri").title).toBe(
      "The app's return address doesn't match",
    );
    expect(oauthErrorText("access_denied").title).toBe("The connection was refused");
    expect(oauthErrorText("server_error").title).toBe("Couldn't connect the app");
    expect(oauthErrorText("toString").title).toBe("Couldn't connect the app");
    expect(oauthErrorText(null).title).toBe("Couldn't connect the app");
  });

  it("shows an app's description as text", () => {
    render(
      <OAuthErrorPage
        search={{ error: "invalid_request", error_description: "<b>missing</b> state" }}
      />,
    );

    expect(screen.getByRole("heading", { name: "Couldn't connect the app" })).toBeTruthy();
    expect(screen.getByText("<b>missing</b> state").tagName).toBe("Q");
  });

  it("reads only the code and description from the query, trimmed and short", () => {
    const query = { error: " invalid_client ", other: "x", error_description: "" };

    expect(parseOAuthErrorSearch(query)).toEqual({ error: "invalid_client" });
    expect(parseOAuthErrorSearch({})).toEqual({});
    expect(
      parseOAuthErrorSearch({ error_description: "a".repeat(400) }).error_description,
    ).toHaveLength(300);
  });
});

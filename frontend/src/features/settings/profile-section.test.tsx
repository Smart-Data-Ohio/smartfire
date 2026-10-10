import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { useState } from "react";
import { afterAll, afterEach, expect, it } from "vitest";
import type { Settings } from "../../gen/Settings.ts";
import { mutations } from "../../store/store.ts";
import { settings } from "../../sync/settings.ts";
import { installMockNetwork } from "../../test/mock-network.ts";
import { ProfileSection } from "./profile-section.tsx";
import { SettingsContext } from "./settings-parts.tsx";

const network = installMockNetwork();

afterAll(() => network.restore());

afterEach(() => mutations.reset());

function Harness({ initial }: { readonly initial: Settings }) {
  const [value, replace] = useState(initial);

  return (
    <SettingsContext value={{ settings: value, replace, update: replace }}>
      <ProfileSection />
    </SettingsContext>
  );
}

it("saves and clears optional pronouns and nickname without changing the account name", async () => {
  const initial = await settings.load();

  render(<Harness initial={initial} />);
  const nickname = screen.getByRole("textbox", { name: "Display nickname" });
  const pronouns = screen.getByRole("textbox", { name: "Pronouns" });

  expect(nickname.getAttribute("maxlength")).toBe("32");
  expect(pronouns.getAttribute("maxlength")).toBe("40");
  fireEvent.change(nickname, { target: { value: "  R  " } });
  fireEvent.change(pronouns, { target: { value: "  they/them  " } });
  fireEvent.click(screen.getByRole("button", { name: "Save profile" }));
  await waitFor(() => expect(nickname).toHaveProperty("value", "R"));
  expect(pronouns).toHaveProperty("value", "they/them");
  expect((await settings.load()).profile.name).toBe(initial.profile.name);
  fireEvent.change(nickname, { target: { value: "" } });
  fireEvent.change(pronouns, { target: { value: "" } });
  fireEvent.click(screen.getByRole("button", { name: "Save profile" }));
  await waitFor(() =>
    expect(screen.getByRole("button", { name: "Save profile" })).toHaveProperty("disabled", true),
  );
  expect((await settings.load()).profile).toMatchObject({ nickname: null, pronouns: null });
});

it("shows the server's identity validation error beside its field", async () => {
  render(<Harness initial={await settings.load()} />);
  fireEvent.change(screen.getByRole("textbox", { name: "Pronouns" }), {
    target: { value: "they\tthem" },
  });
  fireEvent.click(screen.getByRole("button", { name: "Save profile" }));
  expect(await screen.findByText("Pronouns must not contain control characters.")).toBeTruthy();
});

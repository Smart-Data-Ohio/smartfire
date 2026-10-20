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

it("previews the banner slot empty, with the avatar's tint and an upload control", async () => {
  render(<Harness initial={await settings.load()} />);

  expect(screen.getByRole("heading", { name: "Profile banner" })).toBeTruthy();
  expect(screen.getByRole("button", { name: "Upload banner" })).toBeTruthy();
  expect(screen.queryByRole("button", { name: "Remove banner" })).toBeNull();
  expect(
    screen
      .getByRole("figure", { name: "Profile preview" })
      .querySelector<HTMLElement>(".profile-banner")?.dataset.empty,
  ).toBe("true");
});

it("refuses a file that isn't an image before uploading it", async () => {
  render(<Harness initial={await settings.load()} />);

  const input = screen.getByLabelText("Choose banner image");
  const notes = new File(["plain words"], "notes.txt", { type: "text/plain" });

  fireEvent.change(input, { target: { files: [notes] } });
  expect((await screen.findByRole("alert")).textContent).toContain(
    "Choose a PNG, JPEG, GIF or WebP image.",
  );
});

it("shows a saved banner in the preview and removes it", async () => {
  const loaded = await settings.load();
  const url = "/users/token/banner?v=3";

  render(
    <Harness
      initial={{ ...loaded, profile: { ...loaded.profile, bannerUrl: url, bannerStillUrl: null } }}
    />,
  );

  const name = `${loaded.profile.name}'s profile banner`;

  expect(screen.getByRole("img", { name }).getAttribute("src")).toBe(url);
  fireEvent.click(screen.getByRole("button", { name: "Remove banner" }));
  await waitFor(() => expect(screen.queryByRole("img", { name })).toBeNull());
  expect(screen.getByRole("button", { name: "Upload banner" })).toBeTruthy();
});

it("shows the name, nickname and pronouns in the preview card as they are typed", async () => {
  const loaded = await settings.load();

  render(<Harness initial={loaded} />);

  const card = screen.getByRole("figure", { name: "Profile preview" });

  fireEvent.change(screen.getByRole("textbox", { name: "Name" }), {
    target: { value: "Rowan Vale" },
  });
  expect(card.querySelector(".profile-editor-name")?.textContent).toBe("Rowan Vale");
  fireEvent.change(screen.getByRole("textbox", { name: "Display nickname" }), {
    target: { value: "Ro" },
  });
  fireEvent.change(screen.getByRole("textbox", { name: "Pronouns" }), {
    target: { value: "they/them" },
  });
  expect(card.querySelector(".profile-editor-name")?.textContent).toBe("Ro");
  expect(card.querySelector(".profile-editor-aside")?.textContent).toBe("Rowan Vale · they/them");
  expect((await settings.load()).profile.name).toBe(loaded.profile.name);
});

it("keeps the typed name in the preview card when saving it fails", async () => {
  const loaded = await settings.load();

  render(<Harness initial={loaded} />);
  fireEvent.change(screen.getByRole("textbox", { name: "Name" }), {
    target: { value: "Rowan Vale" },
  });
  fireEvent.change(screen.getByRole("textbox", { name: "Pronouns" }), {
    target: { value: "they\tthem" },
  });
  fireEvent.click(screen.getByRole("button", { name: "Save profile" }));
  expect(await screen.findByText("Pronouns must not contain control characters.")).toBeTruthy();

  const card = screen.getByRole("figure", { name: "Profile preview" });

  expect(card.querySelector(".profile-editor-name")?.textContent).toBe("Rowan Vale");
  expect(screen.getByRole("textbox", { name: "Name" })).toHaveProperty("value", "Rowan Vale");
  expect((await settings.load()).profile.name).toBe(loaded.profile.name);
});

import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, expect, it } from "vitest";
import { avatarHue } from "../../ui/avatar-palette.ts";
import { ProfileBanner, ProfileHero } from "./profile-banner.tsx";

afterEach(() => {
  cleanup();
  delete document.documentElement.dataset.motion;
});

const MOVING = "/users/token/banner?v=7";

const STILL = "/users/token/banner?v=7&still=1";

it("shows someone's banner with alt text naming them", () => {
  render(<ProfileBanner userId={4} name="Ada" url={MOVING} stillUrl={STILL} />);

  const image = screen.getByRole("img", { name: "Ada's profile banner" });

  expect(image.getAttribute("src")).toBe(MOVING);
});

it("shows an animated banner's first frame when motion is reduced", () => {
  document.documentElement.dataset.motion = "reduce";
  render(<ProfileBanner userId={4} name="Ada" url={MOVING} stillUrl={STILL} />);

  expect(screen.getByRole("img", { name: "Ada's profile banner" }).getAttribute("src")).toBe(STILL);
});

it("keeps a still banner under reduced motion", () => {
  document.documentElement.dataset.motion = "reduce";
  render(<ProfileBanner userId={4} name="Ada" url={MOVING} stillUrl={null} />);

  expect(screen.getByRole("img", { name: "Ada's profile banner" }).getAttribute("src")).toBe(
    MOVING,
  );
});

it("falls back to a wash of the avatar's tint without a banner", () => {
  const { container } = render(<ProfileBanner userId={4} name="Ada" url={null} stillUrl={null} />);
  const strip = container.querySelector<HTMLElement>(".profile-banner");

  expect(screen.queryByRole("img")).toBeNull();
  expect(strip?.dataset.empty).toBe("true");
  expect(strip?.style.getPropertyValue("--avatar-hue")).toBe(`${avatarHue(4)}`);
});

it("falls back to the tint when the banner fails to load", () => {
  const { container } = render(
    <ProfileBanner userId={4} name="Ada" url={MOVING} stillUrl={null} />,
  );

  fireEvent.error(screen.getByRole("img", { name: "Ada's profile banner" }));

  expect(screen.queryByRole("img")).toBeNull();
  expect(container.querySelector<HTMLElement>(".profile-banner")?.dataset.empty).toBe("true");
});

it("draws the avatar over the banner in the hero", () => {
  const { container } = render(
    <ProfileHero userId={4} name="Ada" url={null} stillUrl={null} size="card">
      <span data-testid="avatar" />
    </ProfileHero>,
  );

  expect(container.querySelector(".profile-hero")?.getAttribute("data-size")).toBe("card");
  expect(
    container.querySelector(".profile-hero-avatar")?.contains(screen.getByTestId("avatar")),
  ).toBe(true);
});

import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it } from "vitest";
import App from "../src/App";

describe("RealmBox fake player journey", () => {
  it("guides onboarding to a playable dashboard", async () => {
    const user = userEvent.setup();
    render(<App />);
    await user.click(screen.getByRole("button", { name: /préparer mon monde/i }));
    expect(screen.getByRole("heading", { name: /trouvons vos données/i })).toBeVisible();
    await user.click(screen.getByRole("button", { name: /dossier de démonstration/i }));
    await user.click(screen.getByRole("button", { name: /préparer automatiquement/i }));
    expect(await screen.findByRole("button", { name: /jouer/i }, { timeout: 3000 })).toBeVisible();
    expect(screen.getByText(/aucun service réel/i)).toBeVisible();
  });

  it("starts, chats, and stops the fake session", async () => {
    const user = userEvent.setup();
    render(<App />);
    await user.click(screen.getByRole("button", { name: /préparer mon monde/i }));
    await user.click(screen.getByRole("button", { name: /dossier de démonstration/i }));
    await user.click(screen.getByRole("button", { name: /préparer automatiquement/i }));
    await user.click(await screen.findByRole("button", { name: /jouer/i }, { timeout: 3000 }));
    const input = await screen.findByLabelText(/parler à melya/i, {}, { timeout: 3000 });
    await user.type(input, "On est prêts ?");
    await user.click(screen.getByRole("button", { name: /envoyer/i }));
    expect(await screen.findByText(/Melya :/)).toBeVisible();
    await user.click(screen.getByRole("button", { name: /fermer la session/i }));
    expect(await screen.findByRole("button", { name: /jouer/i })).toBeVisible();
  });
});

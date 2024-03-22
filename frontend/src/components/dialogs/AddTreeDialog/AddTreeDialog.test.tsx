import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import "@testing-library/jest-dom";
import MockAdapter from "axios-mock-adapter";
import { vi } from "vitest";

import { AddTreeDialog } from "./AddTreeDialog";

vi.mock("axios", async () => {
  const actual = await vi.importActual<typeof import("axios")>("axios");

  return {
    default: {
      ...actual.default,
      create: vi.fn().mockReturnThis(),
    }
  };
});

describe("AddTreeDialog", () => {
  let mock: MockAdapter;

  beforeEach(async () => {
    const actual = await vi.importActual<typeof import("axios")>("axios");

    mock = new MockAdapter(actual.default, {
      onNoMatch: "throwException",
    });

    mock.reset();
  });

  test("cannot submit without a name", () => {
    render(<AddTreeDialog center={{
      lat: 1,
      lon: 2,
    }} onSuccess={vi.fn()} />);

    const submitButton = screen.getByRole("button", { name: /save/i });
    expect(submitButton).toBeDisabled();
  });

  test("submit a tree", async () => {
    const user = userEvent.setup();

    const onSuccess = vi.fn();

    mock.onPost("/v1/trees").reply(200, {
      id: 1,
      lat: 56.26,
      lon: 28.48,
      name: "Oak",
    });

    render(<AddTreeDialog center={{
      lat: 1,
      lon: 2,
    }} onSuccess={onSuccess} />);

    const input = screen.getByRole("textbox", { name: /species/i });
    expect(input).toBeInTheDocument();

    await user.type(input, "Oak");

    const submitButton = screen.getByRole("button", { name: /save/i });
    expect(submitButton).not.toBeDisabled();

    await user.click(submitButton);

    expect(onSuccess).toBeCalledWith({
      id: 1,
      lat: 56.26,
      lon: 28.48,
      name: "Oak",
    });
  });

  test("cancel without submit", async () => {
    const user = userEvent.setup();
    const handleCancel = vi.fn();

    render(<AddTreeDialog center={{
      lat: 1,
      lon: 2,
    }} onSuccess={vi.fn()} onCancel={handleCancel} />);

    const cancelButton = screen.getByRole("button", { name: /cancel/i });
    expect(cancelButton).not.toBeDisabled();

    await user.click(cancelButton);

    expect(handleCancel).toBeCalled();
  });
});
